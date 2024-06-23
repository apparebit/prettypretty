//! High-definition colors

pub(crate) mod core;

use self::core::{
    clip, convert, delta_e_ok_internal, from_24_bit, in_gamut, interpolate, map_to_gamut,
    normalize_eq, normalize_nan_mut, normalize_range_mut, prepare_to_interpolate, scale_lightness,
    to_24_bit, to_contrast, to_contrast_luminance_p3, to_contrast_luminance_srgb,
};
pub use self::core::{ColorSpace, HueInterpolation, OkVersion};
use super::parser::{format, parse};
use super::util::ColorFormatError;

/// A color interpolator.
///
/// An interpolator performs linear interpolation between the coordinates of two
/// colors according to [CSS Color
/// 4](https://www.w3.org/TR/css-color-4/#interpolation). While linear
/// interpolation itself is rather simple, preparing color coordinates for
/// interpolation is quite a bit more work. It requires converting the colors
/// into the desired color space, carrying forward any missing components, and
/// adjusting hues according to interpolation strategy. An interpolator performs
/// this work only once to facilitate amortization across several
/// interpolations. That makes no difference when mixing two colors, but is
/// effective when computing a gradient.
pub struct Interpolator {
    space: ColorSpace,
    coordinates1: [f64; 3],
    coordinates2: [f64; 3],
}

impl Interpolator {
    /// Create a new color interpolator.
    pub fn new(
        color1: &Color,
        color2: &Color,
        space: ColorSpace,
        strategy: HueInterpolation,
    ) -> Self {
        let [coordinates1, coordinates2] = prepare_to_interpolate(
            color1.space,
            &color1.coordinates,
            color2.space,
            &color2.coordinates,
            space,
            strategy,
        )
        .data;

        Self {
            space,
            coordinates1: coordinates1.data,
            coordinates2: coordinates2.data,
        }
    }

    /// Compute the interpolated color at the given fraction.
    pub fn at(&self, fraction: f64) -> Color {
        let [c1, c2, c3] = interpolate(fraction, &self.coordinates1, &self.coordinates2).data;
        Color::new(self.space, c1, c2, c3)
    }
}

// ====================================================================================================================

/// A high-resolution color object.
///
/// Every color object has a color space and three coordinates.
///
/// # Color Coordinates
///
/// A coordinate may be not-a-number either because it is a [powerless
/// component](https://www.w3.org/TR/css-color-4/#powerless), such as hue when
/// chroma is zero, or a [missing
/// component](https://www.w3.org/TR/css-color-4/#missing), i.e., a component
/// intentionally omitted by the user, notably for interpolation.
///
/// For RGB color spaces, the coordinates of in-gamut colors have unit range.
///
/// For Oklab et al., the (revised) lightness must be `0.0..=1.0`, chroma must
/// be `0.0..`, and hue must be `0..360`. There are no a-priori limits on a/b or
/// upper limit on chroma. However, in practice, a/b are `-0.4..=0.4` and chroma
/// is `0.0..=0.4`.
///
/// Color objects do not enforce any of these invariants automatically because
/// doing so may entail some information loss, notably due to reduced numerical
/// range or resolution. At the same time, color objects provide the methods
/// necessary for more constrained representations. Notably, it includes:
///
///   * [`Color::normalize_nan`] zeros out not-a-number components, so they do
///     not interfere with conversion and other operations;
///   * [`Color::normalize_range`] clamps Oklab component values so they are
///     minimally defined;
///   * [`Color::in_gamut`] tests whether color is in gamut;
///   * [`Color::clip`] quickly calculates an in-gamut color that may not be a
///     good match for the original color.
///   * [`Color::map_to_gamut`] maps an out-of-gamut color to an in-gamut color
///     by searching along the chroma axis until it finds an out-of-gamut color
///     whose clipped version is close-by and in-gamut.
///
/// # Equality Testing and Hashing
///
/// The key requirement for equality testing and hashing is that colors that
/// compare [`Self::eq`] also have the same
/// [`Self::hash`](struct.Color.html#method.hash). To maintain this invariant,
/// the implementation of the two methods normalizes coordinates:
///
///   * To make coordinates comparable, replace not-a-numbers with positive
///     zero;
///   * To preserve not-a-number semantics for hues, also zero out chroma for
///     not-a-number hues in Oklch;
///   * To preserve rotation semantics for hues, remove all full rotations;
///   * To prepare for rounding, scale down hues to unit range;
///   * To allow for floating point error, multiply by 1e14 and then round,
///     which drops the least significant digit;
///   * To make zeros comparable, replace negative zero with positive zero (but
///     only after rounding, which may produce zeros);
///   * To convince Rust that coordinates are comparable, convert to bits.
///
/// While rounding isn't strictly necessary for correctness, it makes for a more
/// robust comparison without meaningfully reducing precision.
///
///
/// <style>
/// .color-swatch {
///     display: flex;
/// }
/// .color-swatch > div {
///     height: 4em;
///     width: 4em;
///     border: black 0.5pt solid;
///     display: flex;
///     align-items: center;
///     justify-content: center;
/// }
/// </style>
#[derive(Clone, Debug)]
pub struct Color {
    space: ColorSpace,
    coordinates: [f64; 3],
}

impl Color {
    /// Instantiate a new color with the given color space and coordinates.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let pink = Color::new(ColorSpace::Oklch, 0.7, 0.22, 3.0);
    /// assert_eq!(pink.coordinates(), &[0.7_f64, 0.22_f64, 3.0_f64]);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklch(0.7 0.22 3.0);"></div>
    /// </div>
    pub const fn new(space: ColorSpace, c1: f64, c2: f64, c3: f64) -> Self {
        Color {
            space,
            coordinates: [c1, c2, c3],
        }
    }

    /// Instantiate a new sRGB color with the given red, green, and blue
    /// coordinates.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let fire_brick = Color::srgb(177.0/255.0, 31.0/255.0, 36.0/255.0);
    /// assert_eq!(fire_brick.space(), ColorSpace::Srgb);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: rgb(177 31 36);"></div>
    /// </div>
    pub fn srgb(r: impl Into<f64>, g: impl Into<f64>, b: impl Into<f64>) -> Self {
        Color {
            space: ColorSpace::Srgb,
            coordinates: [r.into(), g.into(), b.into()],
        }
    }

    /// Instantiate a new Display P3 color with the given red, green, and blue
    /// coordinates.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let cyan = Color::p3(0, 0.87, 0.85);
    /// assert_eq!(cyan.space(), ColorSpace::DisplayP3);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(display-p3 0 0.87 0.85);"></div>
    /// </div>
    pub fn p3(r: impl Into<f64>, g: impl Into<f64>, b: impl Into<f64>) -> Self {
        Color {
            space: ColorSpace::DisplayP3,
            coordinates: [r.into(), g.into(), b.into()],
        }
    }

    /// Instantiate a new Oklab color with the given lightness L, a, and b
    /// coordinates.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let sky = Color::oklab(0.78, -0.1, -0.1);
    /// assert_eq!(sky.space(), ColorSpace::Oklab);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklab(0.78 -0.1 -0.1);"></div>
    /// </div>
    pub fn oklab(l: impl Into<f64>, a: impl Into<f64>, b: impl Into<f64>) -> Self {
        Color {
            space: ColorSpace::Oklab,
            coordinates: [l.into(), a.into(), b.into()],
        }
    }

    /// Instantiate a new Oklrab color with the given revised lightness Lr, a,
    /// and b coordinates.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let turquoise = Color::oklrab(0.48, -0.1, -0.1);
    /// assert_eq!(turquoise.space(), ColorSpace::Oklrab);
    /// assert!(
    ///     (turquoise.to(ColorSpace::Oklab).coordinates()[0] - 0.5514232757779728).abs()
    ///     < 1e-13
    /// );
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklab(0.5514232757779728 -0.1 -0.1);"></div>
    /// </div>
    pub fn oklrab(lr: impl Into<f64>, a: impl Into<f64>, b: impl Into<f64>) -> Self {
        Color {
            space: ColorSpace::Oklrab,
            coordinates: [lr.into(), a.into(), b.into()],
        }
    }

    /// Instantiate a new Oklch color with the given lightness L, chroma C, and
    /// hue h coordinates.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let olive = Color::oklch(0.59, 0.1351, 126);
    /// assert_eq!(olive.space(), ColorSpace::Oklch);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklch(0.59 0.1351 126);"></div>
    /// </div>
    pub fn oklch(l: impl Into<f64>, c: impl Into<f64>, h: impl Into<f64>) -> Self {
        Color {
            space: ColorSpace::Oklch,
            coordinates: [l.into(), c.into(), h.into()],
        }
    }

    /// Instantiate a new Oklrch color with the given revised lightness Lr,
    /// chroma C, and hue h coordinates.
    ///
    /// # Examples
    ///
    /// When you compare the example code below with that for [`Color::oklch`],
    /// the impact of revised lightness becomes plainly visible, with Oklrch
    /// producing a clearly lighter olive tone at the same magnitude of
    /// lightness. In other words, Oklrab and Oklrch decompress lighter tones
    /// while compressing darker ones.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let olive = Color::oklrch(0.59, 0.1351, 126);
    /// let same_olive = olive.to(ColorSpace::Oklch);
    /// assert_eq!(same_olive, Color::oklch(0.6469389611084363, 0.1351, 126));
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklch(0.647 0.1351 126);"></div>
    /// </div>
    pub fn oklrch(lr: impl Into<f64>, c: impl Into<f64>, h: impl Into<f64>) -> Self {
        Color {
            space: ColorSpace::Oklrch,
            coordinates: [lr.into(), c.into(), h.into()],
        }
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Instantiate a new sRGB color with the given red, green, and blue
    /// coordinates scaled by 1.0/255.0.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let tangerine = Color::from_24_bit(0xff, 0x93, 0x00);
    /// assert_eq!(tangerine, Color::srgb(1, 0.5764705882352941, 0));
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #ff9300;"></div>
    /// </div>
    pub fn from_24_bit(r: u8, g: u8, b: u8) -> Self {
        Color {
            space: ColorSpace::Srgb,
            coordinates: from_24_bit(r, g, b).data,
        }
    }

    /// Convert this color to 24-bit representation.
    ///
    /// This function returns the coordinates converted to unsigned bytes.
    ///
    /// # Panics
    ///
    /// This function assumes that coordinate values range `0.0..=1.0` and
    /// panics if that is not the case. The assumption holds for in-gamut RGB
    /// colors, e.g., for colors in sRGB, Display P3, Rec. 2020, and their
    /// linear versions. However, it does *not* hold for Oklab et al.
    pub fn to_24_bit(&self) -> [u8; 3] {
        to_24_bit(&self.coordinates).data
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Determine whether this color is the default color, i.e., is the origin
    /// of the XYZ color space.
    pub fn is_default(&self) -> bool {
        self.space == ColorSpace::Xyz && self.coordinates == [0.0, 0.0, 0.0]
    }

    /// Access the color space.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let aqua = Color::oklch(0.66, 0.1867, 250);
    /// assert_eq!(aqua.space(), ColorSpace::Oklch);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklch(0.66 0.1867 250);"></div>
    /// </div>
    #[inline]
    pub fn space(&self) -> ColorSpace {
        self.space
    }

    /// Access the coordinates.
    ///
    /// This method's intended use is for iterating over the three coordinates.
    /// To read *and write* individual coordinates, this class also implements
    /// [`Color::index`](struct.Color.html#method.index) and
    /// [`Color::index_mut`](struct.Color.html#method.index_mut), which take a
    /// `usize` as argument.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let green = Color::p3(0, 1, 0);
    /// assert_eq!(green.coordinates(), &[0.0, 1.0, 0.0]);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(display-p3 0 1 0);"></div>
    /// </div>
    #[inline]
    pub fn coordinates(&self) -> &[f64; 3] {
        &self.coordinates
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Normalize not-a-number coordinates by zeroing them.
    ///
    /// If the hue for Oklch/Oklrch is not-a-number, this method also zeros out
    /// the chroma.
    #[inline]
    #[must_use = "method returns owned color and does not mutate original value"]
    pub fn normalize_nan(mut self) -> Self {
        normalize_nan_mut(self.space, &mut self.coordinates);
        self
    }

    /// Normalize coordinate ranges.
    ///
    /// This method clamps (revised) lightness and chroma to the ranges
    /// `0.0..=1.0` and `0.0..`, respectively. It also folds hue to the range
    /// `0..360`.
    #[inline]
    #[must_use = "method returns owned color and does not mutate original value"]
    pub fn normalize_range(mut self) -> Self {
        normalize_range_mut(self.space, &mut self.coordinates);
        self
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Lighten this color by the given factor in Oklrch.
    ///
    /// This method converts this color to Oklrch, then multiplies its lightness
    /// Lr by the given factor, and finally returns the result—which may or may
    /// not be in-gamut for another color space. This method does not include an
    /// option for selecting Oklch because of its non-uniform lightness L.
    ///
    /// # Examples
    ///
    /// The code example leverages the fact that lightening by a factor f is the
    /// same as darkening by factor 1/f and vice versa. Note that the example
    /// computes the colors out of order but then validates them in order. The
    /// color swatch shows them in order, from darkest to lightest.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace::*};
    /// let goldenrod1 = Color::from_24_bit(0x8b, 0x65, 0x08);
    /// let goldenrod3 = goldenrod1.lighten(1.4).to(Srgb);
    /// let goldenrod2 = goldenrod3.lighten(1.2/1.4).to(Srgb);
    /// assert_eq!(goldenrod1.to_24_bit(), [0x8b_u8, 0x65, 0x08]);
    /// assert_eq!(goldenrod2.to_24_bit(), [0xa4_u8, 0x7d, 0x2c]);
    /// assert_eq!(goldenrod3.to_24_bit(), [0xbd_u8, 0x95, 0x47]);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #8b6508;"></div>
    /// <div style="background-color: #a47d2c;"></div>
    /// <div style="background-color: #bd9547;"></div>
    /// </div>
    #[allow(non_snake_case)]
    #[inline]
    #[must_use = "method returns new color and does not mutate original value"]
    pub fn lighten(&self, factor: f64) -> Color {
        Color {
            space: ColorSpace::Oklrch,
            coordinates: scale_lightness(self.space, &self.coordinates, factor).data,
        }
    }

    /// Darken this color by the given factor in Oklrch.
    ///
    /// Since darkening by some factor is just lightening by the inverse, this
    /// method delegates to [`Color::lighten`] with just that value.
    #[inline]
    #[must_use = "method returns new color and does not mutate original value"]
    pub fn darken(&self, factor: f64) -> Color {
        Color {
            space: ColorSpace::Oklrch,
            coordinates: scale_lightness(self.space, &self.coordinates, factor.recip()).data,
        }
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Convert this color to the target color space.
    ///
    /// # Approach
    ///
    /// A color space is usually defined through a conversion from and to
    /// another color space. The color module includes handwritten functions
    /// that implement just those single-hop conversions. The basic challenge
    /// for arbitrary conversions, as implemented by this method, is to find a
    /// path through the graph of single-hop conversions. Dijkstra's algorithm
    /// would certainly work. But it also incurs substantial dynamic overhead on
    /// every conversion.
    ///
    /// The algorithm used by this method can avoid much of this dynamic
    /// overhead. It is based on the observation that single-hop conversions
    /// form a tree rooted in XYZ. That suggests taking a divide and conquer
    /// approach towards the most general conversions, which go through XYZ:
    /// Split the path into two, from the source color space to XYZ and from XYZ
    /// to the target color space.
    ///
    /// Alas, conversions that do not go through XYZ need to be handled
    /// separately and the cluster of Oklab, Oklrab, Oklch, and Oklrch—with
    /// Oklab converting to Oklrab and Oklch, which in turn both convert to
    /// Oklrch—requires 4 single-hop and 4 double-hop conversion functions in
    /// addition to the 2 single-hop, 4 double-hop, and 2 triple-hop functions
    /// for converting from and to XYZ.
    ///
    /// With those conversion functions in place, routing through the conversion
    /// graph is a straightforward linear case analysis that first matches pairs
    /// of color spaces to handle conversions within subtrees, then matches on
    /// the source color space, and finally matches on the target color space.
    /// Conveniently, a match during the first step also eliminates the need for
    /// the second and third match. See the source code for the full details.
    #[inline]
    #[must_use = "method returns a new color and does not mutate original value"]
    pub fn to(&self, target: ColorSpace) -> Self {
        Self {
            space: target,
            coordinates: convert(self.space, target, &self.coordinates).data,
        }
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Determine whether this color is in-gamut for its color space.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let red = Color::srgb(1, 0, 0);
    /// assert!(red.in_gamut());
    ///
    /// let green = Color::p3(0, 1, 0);
    /// assert!(!green.to(ColorSpace::Srgb).in_gamut());
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(srgb 1 0 0);"></div>
    /// <div style="background-color: color(display-p3 0 1 0);"></div>
    /// </div>
    #[inline]
    pub fn in_gamut(&self) -> bool {
        in_gamut(self.space, &self.coordinates)
    }

    /// Clip this color to the gamut of its color space.
    ///
    /// # Examples
    ///
    /// Display P3's green primary is out of gamut in sRGB. Clipping the
    /// converted color does bring it into gamut, though the result may be a
    /// rough match for the original color.
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let too_green = Color::new(ColorSpace::DisplayP3, 0.0, 1.0, 0.0)
    ///     .to(ColorSpace::Srgb);
    /// assert!(!too_green.in_gamut());
    ///
    /// let green = too_green.clip();
    /// assert!(green.in_gamut());
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(display-p3 0 1 0);"></div>
    /// <div style="background-color: color(srgb 0 1 0);"></div>
    /// </div>
    #[inline]
    #[must_use = "method returns a new color and does not mutate original value"]
    pub fn clip(&self) -> Self {
        Self {
            space: self.space,
            coordinates: clip(self.space, &self.coordinates).data,
        }
    }

    /// Map this color into the gamut of its color space and return the result.
    ///
    /// # Algorithm
    ///
    /// This method uses the [CSS Color 4
    /// algorithm](https://drafts.csswg.org/css-color/#css-gamut-mapping) for
    /// gamut mapping. It performs a binary search in Oklch for a color with
    /// less chroma than the original (but the same lightness and hue), whose
    /// clipped version is within the *just noticeable difference* and in gamut
    /// for the current color space. That clipped color is the result.
    ///
    /// The algorithm nicely illustrates how different color spaces are best
    /// suited to different needs. First, it performs clipping and in-gamut
    /// testing in the current color space. After all, that's the color space
    /// the application requires the color to be in. Second, it performs color
    /// adjustments in Oklch. It is eminently suited to color manipulation
    /// because it is both perceptually uniform and has polar coordinates.
    /// Third, it measures distance in Oklab. Since the color space is
    /// perceptually uniform and has Cartesian coordinates, Euclidian distance
    /// is easy to compute and still accurate.
    ///
    /// # Examples
    ///
    /// Display P3's yellow secondary is out of gamut in sRGB. Gamut mapping the
    /// converted color does bring it into gamut while also perserving the hue
    /// and maximizing the chroma, all within sRGB's gamut.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let too_green = Color::new(ColorSpace::DisplayP3, 0.0, 1.0, 0.0)
    ///     .to(ColorSpace::Srgb);
    /// assert!(!too_green.in_gamut());
    ///
    /// let green = too_green.map_to_gamut();
    /// assert!(green.in_gamut());
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(display-p3 0 1 0);"></div>
    /// <div style="background-color: color(srgb 0.0 0.9857637107710325 0.15974244397344017);"></div>
    /// </div>
    #[inline]
    #[must_use = "method returns a new color and does not mutate original value"]
    pub fn map_to_gamut(&self) -> Self {
        Self {
            space: self.space,
            coordinates: map_to_gamut(self.space, &self.coordinates).data,
        }
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Compute the Euclidian distance between the two colors in Oklab.
    ///
    /// This method computes the color difference *Delta E OK*, which is the
    /// Euclidian distance in the Oklab color space, using either original or
    /// revised version.
    ///
    /// # Examples
    ///
    /// The example code computes the distance between two rather light colors,
    /// with lightness L(honeydew) = 0.94 and L(cantaloupe) = 0.87. Since the
    /// revised lightness Lr corrects the original's dark bias, we'd expect
    /// light colors to be more spread out in Oklrab. That is indeed the case.
    /// ```
    /// # use prettypretty::{Color, ColorSpace, OkVersion, ColorFormatError};
    /// # use std::str::FromStr;
    /// let honeydew = Color::from_str("#d4fb79")?;
    /// let cantaloupe = Color::from_str("#ffd479")?;
    /// let d1 = honeydew.distance(&cantaloupe, OkVersion::Original);
    /// let d2 = honeydew.distance(&cantaloupe, OkVersion::Revised);
    /// assert!((d1 - 0.11174969799958659).abs() < f64::EPSILON);
    /// assert!((d2 - 0.11498895250174994).abs() < f64::EPSILON);
    /// # Ok::<(), ColorFormatError>(())
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #d4fb79;"></div>
    /// <div style="background-color: #ffd479;"></div>
    /// </div>
    #[inline]
    pub fn distance(&self, other: &Self, version: OkVersion) -> f64 {
        delta_e_ok_internal(
            &self.to(version.cartesian_space()).coordinates,
            &other.to(version.cartesian_space()).coordinates,
        )
    }

    /// Find the index position of the candidate color closest to this color.
    ///
    /// This method delegates to [`Color::find_closest`] using the Delta E
    /// metric for Oklab/Oklrab, which is the Euclidian distance.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace, OkVersion};
    /// let colors = [
    ///     &Color::from_24_bit(0xc4, 0x13, 0x31),
    ///     &Color::from_24_bit(0, 0x80, 0x25),
    ///     &Color::from_24_bit(0x30, 0x78, 0xea),
    /// ];
    /// let rose = Color::srgb(1, 0.5, 0.5);
    /// let closest = rose.find_closest_ok(colors, OkVersion::Revised);
    /// assert_eq!(closest, Some(0));
    ///
    /// let green = Color::srgb(0.5, 1, 0.6);
    /// let closest = green.find_closest_ok(colors, OkVersion::Revised);
    /// assert_eq!(closest, Some(1))
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #c41331;"></div>
    /// <div style="background-color: #008025;"></div>
    /// <div style="background-color: #3078ea;"></div>
    /// <div style="background-color: color(srgb 1 0.5 0.5);"></div>
    /// <div style="background-color: color(srgb 0.5 1 0.6);"></div>
    /// </div>
    pub fn find_closest_ok<'c, C>(&self, candidates: C, version: OkVersion) -> Option<usize>
    where
        C: IntoIterator<Item = &'c Self>,
    {
        self.find_closest(candidates, version.cartesian_space(), delta_e_ok_internal)
    }

    /// Find the index position of the candidate color closest to this color.
    ///
    /// This method compares this color to every candidate color by computing
    /// the distance with the given function and returns the index position of
    /// the candidate with smallest distance. If there are no candidates, it
    /// returns `None`. The distance metric is declared `mut` to allow for
    /// stateful comparisons.
    pub fn find_closest<'c, C, F>(
        &self,
        candidates: C,
        space: ColorSpace,
        mut compute_distance: F,
    ) -> Option<usize>
    where
        C: IntoIterator<Item = &'c Color>,
        F: FnMut(&[f64; 3], &[f64; 3]) -> f64,
    {
        // Reimplement search loop for color objects (instead of coordinates):
        // We need to convert candidates to comparison color space, which has a
        // simple lifetime (the loop body) in this case, not so much when
        // wrapping iterators.

        let origin = self.to(space);
        let mut min_distance = f64::INFINITY;
        let mut min_index = None;

        for (index, candidate) in candidates.into_iter().enumerate() {
            let candidate = candidate.to(space);
            let distance = compute_distance(&origin.coordinates, &candidate.coordinates);
            if distance < min_distance {
                min_distance = distance;
                min_index = Some(index);
            }
        }

        min_index
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Interpolate the two colors.
    ///
    /// This method creates a new interpolator for this and the given color.
    /// [`Interpolator::at`] generates the actual, interpolated colors.
    ///
    /// # Examples
    ///
    /// As illustrated below, [`Color::interpolate`] takes care of the mechanics
    /// of interpolation. However, the resulting color may not be displayable
    /// and hence require further processing, such as gamut mapping.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace, HueInterpolation};
    /// let red = Color::srgb(0.8, 0, 0);
    /// let yellow = Color::from_24_bit(0xff, 0xca, 0);
    /// let orange = red
    ///     .interpolate(&yellow, ColorSpace::Oklch, HueInterpolation::Shorter)
    ///     .at(0.5);
    /// assert_eq!(orange, Color::oklch(0.6960475282872609, 0.196904718808239, 59.33737836604695));
    /// assert!(!orange.to(ColorSpace::Rec2020).in_gamut());
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(srgb 0.8 0 0);"></div>
    /// <div style="background-color: #ffca00;"></div>
    /// <div style="background-color: oklch(0.6960475282872609 0.196904718808239 59.33737836604695);"></div>
    /// </div>
    /// <br>
    ///
    /// As illustrated below, the interpolation color space and, for polar color
    /// spaces, the interpolation strategy have considerable impact on the
    /// colors generated by interpolation.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace, HueInterpolation};
    /// let purple = Color::from_24_bit(0xe1, 0x87, 0xfd);
    /// let orange = Color::from_24_bit(0xf7, 0xaa, 0x31);
    /// let salmon = purple
    ///     .interpolate(&orange, ColorSpace::Oklab, HueInterpolation::Shorter)
    ///     .at(0.5)
    ///     .to(ColorSpace::DisplayP3)
    ///     .map_to_gamut();
    /// let pink = purple
    ///     .interpolate(&orange, ColorSpace::Oklch, HueInterpolation::Shorter)
    ///     .at(0.5)
    ///     .to(ColorSpace::DisplayP3)
    ///     .map_to_gamut();
    /// let cyan = purple
    ///     .interpolate(&orange, ColorSpace::Oklch, HueInterpolation::Longer)
    ///     .at(0.5)
    ///     .to(ColorSpace::DisplayP3)
    ///     .map_to_gamut();
    /// assert_eq!(salmon, Color::p3(0.8741728617760183, 0.6327954633247381, 0.6763509691329291));
    /// assert_eq!(pink, Color::p3(1.0, 0.5471696596453801, 0.583554480600142));
    /// assert_eq!(cyan, Color::p3(0.14993363501776769, 0.82564322454698, 0.841871415351839));
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #e187fd;"></div>
    /// <div style="background-color: #f7aa31;"></div>
    /// <div style="background-color: color(display-p3 0.8741728617760183 0.6327954633247381 0.6763509691329291);"></div>
    /// <div style="background-color: color(display-p3 1.0 0.5471696596453801 0.583554480600142);"></div>
    /// <div style="background-color: color(display-p3 0.14993363501776769 0.82564322454698 0.841871415351839);"></div>
    /// </div>
    /// <br>
    ///
    /// It may help to locate the five colors on Oklab's a/b or chroma/hue plane
    /// (i.e., without accounting for their lightness).
    ///
    /// ![The colors plotted on Oklab's chroma and hue plane](https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/interpolate.svg)
    ///
    /// As shown in the figure above:
    ///
    ///  1. Since Oklab uses Cartesian coordinates, the first interpolated
    ///     color, a salmon tone, sits midway on the line connecting the two
    ///     source colors.
    ///  2. With Oklch using polar coordinates, the second interpolated color, a
    ///     pink, sits midway on the shorter arc connecting the two source
    ///     colors. That arc is not a circle segment because the two source
    ///     colors have different chroma values, 0.18546 and 0.15466, in
    ///     addition to different hues, 317.8 and 73.1.
    ///  3. The third interpolated color, a cyan, sits midway on the longer arc
    ///     connecting the two source colors. Its hue is exactly 180º apart from
    ///     that of the second interpolated color.
    ///
    /// Interestingly, all three interpolated colors have similar lightness
    /// values, 0.77761, 0.77742, and 0.77761. That speaks for Oklab's
    /// perceptual uniformity, even if Oklab/Oklch are biased towards dark
    /// tones.
    pub fn interpolate(
        &self,
        color: &Self,
        interpolation_space: ColorSpace,
        interpolation_strategy: HueInterpolation,
    ) -> Interpolator {
        Interpolator::new(self, color, interpolation_space, interpolation_strategy)
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Determine the perceptual contrast of text against a solidly colored
    /// background.
    ///
    /// This method computes the asymmetric, perceptual contrast of text with
    /// this color against a background with the given color. It uses an
    /// algorithm that is surprisingly similar to the [Accessible Perceptual
    /// Contrast Algorithm](https://github.com/Myndex/apca-w3), version
    /// 0.0.98G-4g.
    pub fn contrast_against(&self, background: &Self) -> f64 {
        let fg = self.to(ColorSpace::Srgb);
        let bg = background.to(ColorSpace::Srgb);

        // Try sRGB
        if fg.in_gamut() && bg.in_gamut() {
            return to_contrast(
                to_contrast_luminance_srgb(&fg.coordinates),
                to_contrast_luminance_srgb(&bg.coordinates),
            );
        };

        // Fall back on Display P3
        let fg = self.to(ColorSpace::DisplayP3);
        let bg = background.to(ColorSpace::DisplayP3);
        to_contrast(
            to_contrast_luminance_p3(&fg.coordinates),
            to_contrast_luminance_p3(&bg.coordinates),
        )
    }

    /// Determine whether black or white text maximizes perceptual contrast
    /// against a solid background with this color. This function uses the
    /// same algorithm as [`Color::contrast_against`].
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace, ColorFormatError};
    /// let blue: Color = str::parse("#6872ff")?;
    /// assert!(!blue.use_black_text());
    /// # Ok::<(), ColorFormatError>(())
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #6872ff;">
    ///     <span style="color: #000;">Don't!</span>
    /// </div>
    /// <div style="background-color: #6872ff;">
    ///     <span style="color: #fff;">Do!</span>
    /// </div>
    /// </div>
    pub fn use_black_text(&self) -> bool {
        let background = self.to(ColorSpace::Srgb);
        let luminance = if background.in_gamut() {
            to_contrast_luminance_srgb(&background.coordinates)
        } else {
            to_contrast_luminance_p3(&self.to(ColorSpace::DisplayP3).coordinates)
        };

        to_contrast(0.0, luminance) >= -to_contrast(1.0, luminance)
    }

    /// Determine whether a black or white background maximizes perceptual
    /// contrast behind text with this color. This function uses the same
    /// algorithm as [`Color::contrast_against`].
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace, ColorFormatError};
    /// let blue: Color = str::parse("#68a0ff")?;
    /// assert!(blue.use_black_background());
    /// # Ok::<(), ColorFormatError>(())
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #000;">
    /// <span style="color: #68a0ff;">Do!</span>
    /// </div>
    /// <div style="background-color: #fff;">
    /// <span style="color: #68a0ff;">Don't!</span>
    /// </div>
    /// </div>
    pub fn use_black_background(&self) -> bool {
        let text = self.to(ColorSpace::Srgb);
        let luminance = if text.in_gamut() {
            to_contrast_luminance_srgb(&text.coordinates)
        } else {
            to_contrast_luminance_p3(&self.to(ColorSpace::DisplayP3).coordinates)
        };

        to_contrast(luminance, 0.0) <= -to_contrast(luminance, 1.0)
    }
}

// --------------------------------------------------------------------------------------------------------------------

impl Default for Color {
    /// Create an instance of the default color. The chosen default for
    /// high-resolution colors is pitch black, i.e., the origin in XYZ.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let default = Color::default();
    /// assert_eq!(default.space(), ColorSpace::Xyz);
    /// assert_eq!(default.coordinates(), &[0.0_f64, 0.0, 0.0]);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(xyz 0 0 0);"></div>
    /// </div>
    fn default() -> Self {
        Color {
            space: ColorSpace::Xyz,
            coordinates: [0.0, 0.0, 0.0],
        }
    }
}

impl std::str::FromStr for Color {
    type Err = ColorFormatError;

    /// Instantiate a color from its string representation.
    ///
    /// Before parsing the string slice, this method trims any leading and
    /// trailing white space and converts to ASCII lower case. The latter makes
    /// parsing effectively case-insensitive.
    ///
    /// This method recognizes two hexadecimal notations for RGB colors, the
    /// hashed notation familiar from the web and an older notation used by X
    /// Windows. Even though the latter was originally just specifying *device
    /// RGB*, this crate treats both as notations as sRGB.
    ///
    /// The *hashed notation* has three or six hexadecimal digits, e.g., `#123` or
    /// #`cafe00`. Note that the three digit version is a short form of the six
    /// digit version with every digit repeated. In other words, the red
    /// coordinate in `#123` is not 0x1/0xf but 0x11/0xff.
    ///
    /// The *X Windows notation* has between one and four hexadecimal digits per
    /// coordinate, e.g., `rgb:1/00/cafe`. Here, every coordinate is scaled,
    /// i.e., the red coordinate in the example is 0x1/0xf.
    ///
    /// This method also recognizes a subset of the *CSS color syntax*. In
    /// particular, it recognizes the `color()`, `oklab()`, and `oklch` CSS
    /// functions. For `color()`, the color space right after the opening
    /// parenthesis must be `srgb`, `linear-srgb`, `display-p3`,
    /// `--linear-display-p3`, `rec2020`, `--linear-rec2020`, `--oklrab`,
    /// `--oklrch`, or `xyz`. As indicated by the leading double-dashes, the
    /// linear versions of Display P3 and Rec. 2020 as well as OkLrab and Oklrch
    /// are not included in CSS 4 Color. Coordinates must be space-separated and
    /// unitless (i.e., no `%` or `deg`).
    ///
    /// By implementing the `FromStr` trait, `str::parse` works just the same
    /// for parsing color formats—that is, as long as type inference can
    /// determine what type to parse. For that reason, the definition of
    /// `orange` below includes a type whereas the definition of `blue` does
    /// not.
    ///
    /// Don't forget the `use` statement bringing `FromStr` into scope.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace, ColorFormatError};
    /// use std::str::FromStr;
    ///
    /// let navy = Color::from_str("#011480")?;
    /// assert_eq!(navy, Color::srgb(
    ///     0.00392156862745098,
    ///     0.0784313725490196,
    ///     0.5019607843137255,
    /// ));
    ///
    /// let rose: Color = str::parse("rgb:ffff/dada/cccc")?;
    /// assert_eq!(rose, Color::srgb(1, 0.8549019607843137, 0.8));
    /// # Ok::<(), ColorFormatError>(())
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #011480;"></div>
    /// <div style="background-color: #ffdacc;"></div>
    /// </div>
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse(s).map(|(space, coordinates)| Color { space, coordinates })
    }
}

// --------------------------------------------------------------------------------------------------------------------

mod from_term {
    use crate::{EmbeddedRgb, GrayGradient, TrueColor};

    impl From<TrueColor> for super::Color {
        /// Convert the "true" color object into a *true* color object... 🤪
        fn from(value: TrueColor) -> super::Color {
            let [r, g, b] = *value.coordinates();
            super::Color::srgb(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0)
        }
    }

    impl From<EmbeddedRgb> for super::Color {
        /// Instantiate a new color from the embedded RGB value.
        fn from(value: EmbeddedRgb) -> super::Color {
            TrueColor::from(value).into()
        }
    }

    impl From<GrayGradient> for super::Color {
        /// Instantiate a new color from the embedded RGB value.
        fn from(value: GrayGradient) -> super::Color {
            TrueColor::from(value).into()
        }
    }
}

// --------------------------------------------------------------------------------------------------------------------

impl std::hash::Hash for Color {
    /// Hash this color.
    ///
    /// See [`Color`] for an overview of equality testing and hashing.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.space.hash(state);

        let [n1, n2, n3] = normalize_eq(self.space, &self.coordinates).data;
        n1.hash(state);
        n2.hash(state);
        n3.hash(state);
    }
}

impl PartialEq for Color {
    /// Determine whether this color equals the other color.
    ///
    /// As discussed in the overview for [`Color`], [`Self::eq`] and
    /// [`Self::hash`](struct.Color.html#method.hash) normalize color
    /// coordinates before testing/hashing them. The following *equalities*
    /// illustrate how normalization handles not-a-numbers, very small numbers,
    /// and polar coordinates:
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// assert_eq!(
    ///     Color::srgb(f64::NAN, 3e-15, 8e-15),
    ///     Color::srgb(0,        0,     1e-14)
    /// );
    ///
    /// assert_eq!(
    ///     Color::oklch(0.5, 0.1, 665),
    ///     Color::oklch(0.5, 0.1, 305)
    /// );
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(srgb 0 0 0.00000000000001);"></div>
    /// <div style="background-color: oklch(0.5 0.1 305);"></div>
    /// </div>
    fn eq(&self, other: &Self) -> bool {
        if self.space != other.space {
            return false;
        } else if self.coordinates == other.coordinates {
            return true;
        }

        let n1 = normalize_eq(self.space, &self.coordinates).data;
        let n2 = normalize_eq(other.space, &other.coordinates).data;
        n1 == n2
    }
}

impl Eq for Color {}

// --------------------------------------------------------------------------------------------------------------------

impl AsRef<[f64; 3]> for Color {
    /// Treat the color as a reference to its coordinate array.
    fn as_ref(&self) -> &[f64; 3] {
        &self.coordinates
    }
}

impl AsMut<[f64; 3]> for Color {
    /// Treat the color as a mutable reference to its coordinate array.
    fn as_mut(&mut self) -> &mut [f64; 3] {
        &mut self.coordinates
    }
}

impl std::ops::Index<usize> for Color {
    type Output = f64;

    /// Access the coordinate with the given index.
    ///
    /// # Panics
    ///
    /// This method panics if `index > 2`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    ///
    /// let purple = Color::srgb(0.5, 0.4, 0.75);
    /// assert_eq!(purple[2], 0.75);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(srgb 0.5 0.4 0.75);"></div>
    /// </div>
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.coordinates[index]
    }
}

impl std::ops::IndexMut<usize> for Color {
    /// Mutably access the coordinate with the given index.
    ///
    /// # Panics
    ///
    /// This method panics if `index > 2`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let mut magenta = Color::srgb(0, 0.3, 0.8);
    /// // Oops, we forgot to set the red coordinate. Let's fix that.
    /// magenta[0] = 0.9;
    /// assert_eq!(magenta.coordinates(), &[0.9_f64, 0.3_f64, 0.8_f64]);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(srgb 0.9 0.3 0.8);"></div>
    /// </div>
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.coordinates[index]
    }
}

impl std::fmt::Display for Color {
    /// Format this color.
    ///
    /// This method formats the color in CSS format using either a `color()`,
    /// `oklab()`, or `oklch()` CSS function and three space-separated
    /// coordinates. It respects the formatter's precision, defaulting to 5
    /// digits past the decimal. Since degrees for Oklch/Oklrch are up to two
    /// orders of magnitude larger than other coordinates, this method uses a
    /// precision smaller by 2 for degrees.
    ///
    /// # Examples
    ///
    /// The example code takes a color specified in hashed hexadecimal notation
    /// and formats it as sRGB with 5 and 3 significant digits after the decimal
    /// as well as Oklch with 5 digits for L and C as well as 3 digits for hº.
    /// The color swatch repeats the four different notations (adjusted for CSS)
    /// and hence should show the same color four times over.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorFormatError, ColorSpace::*};
    /// # use std::str::FromStr;
    /// let lime = Color::from_str("#a1d2ae")?;
    /// assert_eq!(format!("{}", lime), "color(srgb 0.63137 0.82353 0.68235)");
    /// assert_eq!(format!("{:.3}", lime), "color(srgb 0.631 0.824 0.682)");
    /// assert_eq!(format!("{}", lime.to(Oklch)), "oklch(0.81945 0.07179 152.812)");
    /// # Ok::<(), ColorFormatError>(())
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #a1d2ae;"></div>
    /// <div style="background-color: color(srgb 0.63137 0.82353 0.68235);"></div>
    /// <div style="background-color: color(srgb 0.631 0.824 0.682);"></div>
    /// <div style="background-color: oklch(0.81945 0.07179 152.812);"></div>
    /// </div>
    /// <br>
    ///
    /// In the above example, all coordinates have at least 5 non-zero decimals.
    /// But that need not be the case. The following example formats a gray in
    /// Oklch, which has no chroma and no hue. The lightness has only three
    /// decimals and serializes with as many. The chroma has no non-zero
    /// decimals and serializes as `0`. Finally, the hue is not-a-number and
    /// serializes as `none`.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorFormatError, ColorSpace::*};
    /// # use std::str::FromStr;
    /// let gray = Color::oklch(0.665, 0.0, f64::NAN);
    /// assert_eq!(format!("{}", gray), "oklch(0.665 0 none)");
    /// # Ok::<(), ColorFormatError>(())
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklch(0.665 0 none);"></div>
    /// </div>
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format(self.space, &self.coordinates, f)
    }
}
