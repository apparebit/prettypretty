//! High-definition colors

mod core;

pub use self::core::ColorSpace;
use self::core::{
    clip, convert, delta_e_ok, in_gamut, map_to_gamut, normalize, to_contrast,
    to_contrast_luminance, P3_CONTRAST, SRGB_CONTRAST,
};
use super::serde::parse;
pub use super::serde::ColorFormatError;
use super::util::Coordinate;

/// A high-resolution color object.
///
///
/// # Managing Gamut
///
/// Color objects have a uniform representation for all supported color spaces,
/// combining a color space tag with a three-element array of coordinates. That
/// does get in the way of color objects enforcing invariants specific to color
/// spaces, notably that coordinates always fall within gamut limits. For
/// example, in-gamut red, green, and blue coordinates for the RGB color spaces
/// sRGB, linear sRGB, Display P3, and linear Display P3 have unit range
/// `0..=1`. But such automatic enforcement of gamut limits might also lead to
/// information loss (by converting from a larger to a smaller space).
///
/// That's why this crate preserves computed coordinates, even if they are out
/// of gamut. Instead, code using this library needs to make sure colors are in
/// gamut before trying to display them:
///
///   * Use [`Color::in_gamut`] to test whether a color is in gamut;
///   * Use [`Color::clip`] to quickly calculate an in-gamut color that may be a
///     subpar stand-in for the original color;
///   * Use [`Color::map_to_gamut`] to more slowly search for a more accurate
///     stand-in;
///   * Use [`Color::difference`] and [`Color::closest`] to implement custom
///     search strategies.
///
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
    /// let blueish = Color::oklab(0.78, -0.1, -0.1);
    /// assert_eq!(blueish.space(), ColorSpace::Oklab);
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
    /// let blueish = Color::oklrab(0.48, -0.1, -0.1);
    /// assert_eq!(blueish.space(), ColorSpace::Oklrab);
    /// assert!(
    ///     (blueish.to(ColorSpace::Oklab).coordinates()[0] - 0.5514232757779728).abs()
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
    /// let deep_purple = Color::oklch(0.5, 0.25, 308);
    /// assert_eq!(deep_purple.space(), ColorSpace::Oklch);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklch(0.5 0.25 308);"></div>
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
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let deep_purple = Color::oklrch(0.5, 0.25, 308);
    /// let also_purple = deep_purple.to(ColorSpace::Oklch);
    /// assert_eq!(also_purple, Color::oklch(0.568838198942395, 0.25, 308));
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklch(0.569 0.25 308);"></div>
    /// </div>
    pub fn oklrch(lr: impl Into<f64>, c: impl Into<f64>, h: impl Into<f64>) -> Self {
        Color {
            space: ColorSpace::Oklrch,
            coordinates: [lr.into(), c.into(), h.into()],
        }
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Access the color space.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let blue = Color::srgb(0, 0, 1);
    /// assert_eq!(blue.space(), ColorSpace::Srgb);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(srgb 0 0 1);"></div>
    /// </div>
    #[inline]
    pub fn space(&self) -> ColorSpace {
        self.space
    }

    /// Access the coordinates.
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

    /// Convert this color to the target color space.
    ///
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
            coordinates: convert(self.space, target, &self.coordinates),
        }
    }

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
    ///
    /// # Example
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
            coordinates: clip(self.space, &self.coordinates),
        }
    }

    /// Map this color into the gamut of its color space and return the result.
    ///
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
    ///
    /// # Example
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
            coordinates: map_to_gamut(self.space, &self.coordinates),
        }
    }

    /// Determine the difference between the two colors. This method computes
    /// the Euclidian distance in the Oklrab color space, calculating the
    /// equivalent of the Delta-E for Oklab.
    #[inline]
    pub fn difference(&self, other: &Self) -> f64 {
        delta_e_ok(
            &self.to(ColorSpace::Oklrab).coordinates,
            &other.to(ColorSpace::Oklrab).coordinates,
        )
    }

    /// Find the position of the candidate color closest to this color. This
    /// method measures distance as the Euclidian distance in Oklrab. If there
    /// are no candidates, the position of the closest color is `None`.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let colors = [
    ///     &Color::srgb(1, 0, 0),
    ///     &Color::srgb(0, 1, 0),
    ///     &Color::srgb(0, 0, 1),
    /// ];
    /// let rose = Color::srgb(1, 0.5, 0.5);
    /// let closest = rose.closest(colors);
    /// assert_eq!(closest, Some(0));
    ///
    /// let closest = Color::srgb(0.5, 1, 0.6).closest(colors);
    /// assert_eq!(closest, Some(1))
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(srgb 1 0 0);"></div>
    /// <div style="background-color: color(srgb 0 1 0);"></div>
    /// <div style="background-color: color(srgb 0 0 1);"></div>
    /// <div style="background-color: color(srgb 1 0.5 0.5);"></div>
    /// <div style="background-color: color(srgb 0.5 1 0.6);"></div>
    /// </div>
    ///
    pub fn closest<'c, C>(&self, candidates: C) -> Option<usize>
    where
        C: IntoIterator<Item = &'c Color>,
    {
        let origin = self.to(ColorSpace::Oklrab);
        let mut min_difference = f64::INFINITY;
        let mut min_index = None;

        for (index, candidate) in candidates.into_iter().enumerate() {
            let difference = delta_e_ok(
                &origin.coordinates,
                &candidate.to(ColorSpace::Oklrab).coordinates,
            );

            if difference < min_difference {
                min_difference = difference;
                min_index = Some(index);
            }
        }

        min_index
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Determine the perceptual contrast of text against a solidly colored
    /// background.
    ///
    /// This method computes the asymmetric, perceptual contrast of text with
    /// this color against a background with the given color. It uses an
    /// algorithm that is surprisingly similar to [Accessible Perceptual
    /// Contrast Algorithm](https://github.com/Myndex/apca-w3), version
    /// 0.0.98G-4g.
    pub fn contrast_against(&self, background: Self) -> f64 {
        let fg = self.to(ColorSpace::Srgb);
        let bg = background.to(ColorSpace::Srgb);

        // Try sRGB
        if fg.in_gamut() && bg.in_gamut() {
            return to_contrast(
                to_contrast_luminance(&SRGB_CONTRAST, &fg.coordinates),
                to_contrast_luminance(&SRGB_CONTRAST, &bg.coordinates),
            );
        };

        // Fall back on Display P3
        let fg = self.to(ColorSpace::DisplayP3);
        let bg = background.to(ColorSpace::DisplayP3);
        to_contrast(
            to_contrast_luminance(&P3_CONTRAST, &fg.coordinates),
            to_contrast_luminance(&P3_CONTRAST, &bg.coordinates),
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
            to_contrast_luminance(&SRGB_CONTRAST, &background.coordinates)
        } else {
            to_contrast_luminance(&P3_CONTRAST, &self.to(ColorSpace::DisplayP3).coordinates)
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
            to_contrast_luminance(&SRGB_CONTRAST, &text.coordinates)
        } else {
            to_contrast_luminance(&P3_CONTRAST, &self.to(ColorSpace::DisplayP3).coordinates)
        };

        to_contrast(luminance, 0.0) <= -to_contrast(luminance, 1.0)
    }
}

// --------------------------------------------------------------------------------------------------------------------

impl Default for Color {
    /// Create an instance of the default color. The chosen default for this
    /// crate is pitch black, i.e., the origin in XYZ.
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
    /// This method recognizes two hexadecimal notations for RGB colors, the
    /// hashed notation familiar from the web and an older notation used by X
    /// Windows. Even though the latter is intended to represent *device RGB*,
    /// this crate treats both as sRGB.
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
    /// By implementing the `FromStr` trait, `str::parse` works just the same
    /// for parsing color formats—that is, as long as type inference can
    /// determine what type to parse. For that reason, the definition of
    /// `orange` below includes a type whereas the definition of `blue` does
    /// not.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace, ColorFormatError};
    /// # use std::str::FromStr;
    /// let blue = Color::from_str("#35f")?;
    /// assert_eq!(blue, Color::srgb(0.2, 0.3333333333333333, 1));
    ///
    /// let orange: Color = str::parse("rgb:ffff/9696/0000")?;
    /// assert_eq!(orange, Color::srgb(1, 0.5882352941176471, 0));
    /// # Ok::<(), ColorFormatError>(())
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #35f;"></div>
    /// <div style="background-color: #ff9600;"></div>
    /// </div>
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse(s).map(|(space, coordinates)| Color { space, coordinates })
    }
}

// --------------------------------------------------------------------------------------------------------------------

impl std::hash::Hash for Color {
    /// Hash this color.
    ///
    /// See [`Color`] for an overview of equality testing and hashing.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.space.hash(state);

        let n = normalize(self.space, &self.coordinates);
        n[0].hash(state);
        n[1].hash(state);
        n[2].hash(state);
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
        }

        let n1 = normalize(self.space, &self.coordinates);
        let n2 = normalize(other.space, &other.coordinates);
        n1 == n2
    }
}

impl Eq for Color {}

// --------------------------------------------------------------------------------------------------------------------

impl std::ops::Index<Coordinate> for Color {
    type Output = f64;

    /// Access the named coordinate.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// use prettypretty::Coordinate::*;
    ///
    /// let purple = Color::srgb(0.5, 0.4, 0.75);
    /// assert_eq!(purple[C3], 0.75);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(srgb 0.5 0.4 0.75);"></div>
    /// </div>
    fn index(&self, index: Coordinate) -> &Self::Output {
        &self.coordinates[index as usize]
    }
}

impl std::ops::IndexMut<Coordinate> for Color {
    /// Mutably access the named coordinate.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// use prettypretty::Coordinate::*;
    ///
    /// let mut magenta = Color::srgb(0, 0.3, 0.8);
    /// // Oops, we forgot to set the red coordinate. Let's fix that.
    /// magenta[C1] = 0.9;
    /// assert_eq!(magenta.coordinates(), &[0.9_f64, 0.3_f64, 0.8_f64]);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(srgb 0.9 0.3 0.8);"></div>
    /// </div>
    fn index_mut(&mut self, index: Coordinate) -> &mut Self::Output {
        &mut self.coordinates[index as usize]
    }
}
