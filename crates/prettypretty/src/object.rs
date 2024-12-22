use std::str::FromStr;

#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use crate::core::{
    clip, convert, delta_e_ok, format, from_24bit, in_gamut, interpolate, is_achromatic, normalize,
    parse, prepare_to_interpolate, scale_lightness, to_24bit, to_contrast,
    to_contrast_luminance_p3, to_contrast_luminance_srgb, to_eq_coordinates, to_gamut, ColorSpace,
    HueInterpolation,
};

use crate::Float;

/// Create a new sRGB color from 24-bit integer coordinates.
///
/// Like [`Color::from_24bit`], this macro creates a new color from 24-bit
/// integer coordinates. However, it also is safe to use in const expressions.
///
/// Rust currently does not allow floating point operations in const functions.
/// That makes it impossible to write a const function that constructs a new
/// high-resolution color object from integer coordinates. However, Rust does
/// currently allow floating point operations in const expressions, notably as
/// arguments to a const function such as a constructor. Hence, a macro can
/// convert and normalize the integer coordinates before passing them to the
/// const function. That's just what this macro does.
#[macro_export]
macro_rules! rgb {
    ($r:expr, $g:expr, $b:expr) => {
        $crate::Color::new(
            $crate::ColorSpace::Srgb,
            [
                $r as $crate::Float / 255.0,
                $g as $crate::Float / 255.0,
                $b as $crate::Float / 255.0,
            ],
        )
    };
}

/// A high-resolution color object.
///
/// Every color object has a [color space](ColorSpace) and three coordinates.
///
/// # Color Coordinates
///
/// For RGB color spaces, the coordinates of in-gamut colors have unit range.
/// For the other color spaces, there are no gamut bounds.
///
/// However, the coordinates of colors in Oklab et al. still need to meet the
/// following constraints to be well-formed. The (revised) lightness must be
/// `0.0..=1.0` and chroma must be `0.0..`. There are no a-priori limits on a/b
/// or upper limit on chroma. However, in practice, a/b are `-0.4..=0.4` and
/// chroma is `0.0..=0.4`. The hue may have any magnitude, though `0..360` are
/// preferred.
///
/// A coordinate may be not-a-number either because it is a [powerless
/// component](https://www.w3.org/TR/css-color-4/#powerless), such as the hue in
/// Oklch/Oklrch when chroma is zero, or a [missing
/// component](https://www.w3.org/TR/css-color-4/#missing), i.e., a component
/// intentionally set to not-a-number, notably for interpolation.
///
/// ## Normalization
///
/// While coordinates may be not-a-number, that representation of powerless or
/// missing components can easily render any computation on colors useless. For
/// that reason, this class automatically normalizes colors with
/// [`Color::normalize`] if necessary. Normalization replaces not-a-numbers with
/// zero and also ensures that lightness and chroma have meaningful quantities.
///
/// ## Equality Testing and Hashing
///
/// Normalization isn't sufficient for equality testing and hashing, which have
/// the additional requirement that equal colors also have equal hashes. Hence
/// this class performs the following steps to prepare coordinates for either
/// operation:
///
///   * To turn coordinates into comparable entities, replace not-a-numbers with
///     positive zero;
///   * To preserve not-a-number semantics for hues, also zero out chroma for
///     not-a-number hues in Oklch;
///   * To preserve rotation semantics for hues, remove all full rotations;
///   * To prepare for rounding, scale down hues to unit range;
///   * To allow for floating point error, multiply by 1e5/1e14 and then round,
///     which drops the least significant digit;
///   * To make zeros comparable, replace negative zero with positive zero (but
///     only after rounding, which may produce zeros);
///   * To convince Rust that coordinates are comparable, convert to bits.
///
/// While rounding isn't strictly necessary for correctness, it makes for a more
/// robust comparison without meaningfully reducing precision.
///
/// ## Coordinate Access
///
/// Both Rust and Python code can access individual coordinates by indexing a
/// color object with integers `0..2`.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, sequence, module = "prettypretty.color")
)]
#[derive(Clone)]
pub struct Color {
    space: ColorSpace,
    coordinates: [Float; 3],
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Color {
    // The following constructors come in pairs, once for pyffi and once without
    // pyffi. Unfortunately, the #[new] and #[staticmethod] attributes seem to
    // be incompatible with #[cfg_attr()]. It might be worth wrapping all this
    // in a macro.

    /// Instantiate a new color with the given color space and coordinates.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let pink = Color::new(ColorSpace::Oklch, [0.7, 0.22, 3.0]);
    /// assert_eq!(pink.as_ref(), &[0.7_f64, 0.22_f64, 3.0_f64]);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklch(0.7 0.22 3.0);"></div>
    /// </div>
    #[cfg(feature = "pyffi")]
    #[new]
    #[inline]
    pub const fn new(space: ColorSpace, coordinates: [Float; 3]) -> Self {
        Self { space, coordinates }
    }

    /// Instantiate a new color with the given color space and coordinates.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let pink = Color::new(ColorSpace::Oklch, [0.7, 0.22, 3.0]);
    /// assert_eq!(pink.as_ref(), &[0.7_f64, 0.22_f64, 3.0_f64]);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklch(0.7 0.22 3.0);"></div>
    /// </div>
    #[cfg(not(feature = "pyffi"))]
    #[inline]
    pub const fn new(space: ColorSpace, coordinates: [Float; 3]) -> Self {
        Self { space, coordinates }
    }

    /// Parse a color from its string representation. <i
    /// class=python-only>Python only!</i>
    ///
    /// This method implements the same functionality as `Color`'s [`Color as
    /// FromStr`](struct.Color.html#impl-FromStr-for-Color) and is available in
    /// Python only.
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn parse(s: &str) -> Result<Color, crate::error::ColorFormatError> {
        use std::str::FromStr;

        Color::from_str(s)
    }

    /// Instantiate a new sRGB color with the given red, green, and blue
    /// coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let fire_brick = Color::srgb(177.0/255.0, 31.0/255.0, 36.0/255.0);
    /// assert_eq!(fire_brick.space(), ColorSpace::Srgb);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: rgb(177 31 36);"></div>
    /// </div>
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn srgb(r: Float, g: Float, b: Float) -> Self {
        Self::new(ColorSpace::Srgb, [r, g, b])
    }

    /// Instantiate a new Display P3 color with the given red, green, and blue
    /// coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let cyan = Color::p3(0, 0.87, 0.85);
    /// assert_eq!(cyan.space(), ColorSpace::DisplayP3);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(display-p3 0 0.87 0.85);"></div>
    /// </div>
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn p3(r: Float, g: Float, b: Float) -> Self {
        Self::new(ColorSpace::DisplayP3, [r, g, b])
    }

    /// Instantiate a new Oklab color with the given lightness L, a, and b
    /// coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let sky = Color::oklab(0.78, -0.1, -0.1);
    /// assert_eq!(sky.space(), ColorSpace::Oklab);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklab(0.78 -0.1 -0.1);"></div>
    /// </div>
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn oklab(l: Float, a: Float, b: Float) -> Self {
        Self::new(ColorSpace::Oklab, [l, a, b])
    }

    /// Instantiate a new Oklrab color with the given revised lightness Lr, a,
    /// and b coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let turquoise = Color::oklrab(0.48, -0.1, -0.1);
    /// assert_eq!(turquoise.space(), ColorSpace::Oklrab);
    /// assert!(
    ///     (turquoise.to(ColorSpace::Oklab).as_ref()[0] - 0.5514232757779728).abs()
    ///     < 1e-13
    /// );
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklab(0.5514232757779728 -0.1 -0.1);"></div>
    /// </div>
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn oklrab(lr: Float, a: Float, b: Float) -> Self {
        Self::new(ColorSpace::Oklab, [lr, a, b])
    }

    /// Instantiate a new Oklch color with the given lightness L, chroma C, and
    /// hue h coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let olive = Color::oklch(0.59, 0.1351, 126);
    /// assert_eq!(olive.space(), ColorSpace::Oklch);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklch(0.59 0.1351 126);"></div>
    /// </div>
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn oklch(l: Float, c: Float, h: Float) -> Self {
        Self::new(ColorSpace::Oklch, [l, c, h])
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
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn oklrch(lr: Float, c: Float, h: Float) -> Self {
        Self::new(ColorSpace::Oklch, [lr, c, h])
    }

    /// Instantiate a new sRGB color from its 24-bit representation.
    ///
    /// This function returns a new sRGB color with the given red, green, and
    /// blue coordinates scaled by 1/255. The [`rgb`] macro does the same thing
    /// but is safe to use inside const expressions.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let tangerine = Color::from_24bit(0xff, 0x93, 0x00);
    /// assert_eq!(tangerine, Color::srgb(1.0, 0.5764705882352941, 0.0));
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #ff9300;"></div>
    /// </div>
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    #[inline]
    pub fn from_24bit(r: u8, g: u8, b: u8) -> Self {
        Self::new(ColorSpace::Srgb, from_24bit(r, g, b))
    }

    /// Instantiate a new sRGB color from its 24-bit representation.
    ///
    /// This function returns a new sRGB color with the given red, green, and
    /// blue coordinates scaled by 1/255. The [`rgb`] macro does the same thing
    /// but is safe to use inside const expressions.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let tangerine = Color::from_24bit(0xff, 0x93, 0x00);
    /// assert_eq!(tangerine, Color::srgb(1.0, 0.5764705882352941, 0.0));
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #ff9300;"></div>
    /// </div>
    #[cfg(not(feature = "pyffi"))]
    #[inline]
    pub fn from_24bit(r: u8, g: u8, b: u8) -> Self {
        Self::new(ColorSpace::Srgb, from_24bit(r, g, b))
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Access the color space.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let aqua = Color::oklch(0.66, 0.1867, 250.0);
    /// assert_eq!(aqua.space(), ColorSpace::Oklch);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklch(0.66 0.1867 250.0);"></div>
    /// </div>
    #[inline]
    pub fn space(&self) -> ColorSpace {
        self.space
    }

    /// Access the coordinates. <i class=python-only>Python only!</i>
    ///
    /// This method provides access to this color's coordinates as a single
    /// sequence instead of piecemeal by index. However, it critically differs
    /// from [`AsRef<[Float;3]> as Color`](struct.Color.html) because it returns
    /// the coordinates by value instead of reference.
    #[cfg(feature = "pyffi")]
    pub fn coordinates(&self) -> [Float; 3] {
        self.coordinates
    }

    /// Get this color's length, which is 3. <i class=python-only>Python
    /// only!</i>
    ///
    /// This method is available from Python only.
    #[cfg(feature = "pyffi")]
    pub fn __len__(&self) -> usize {
        3
    }

    /// Read coordinates by index. <i class=python-only>Python only!</i>
    ///
    /// This method is available from Python only.
    #[cfg(feature = "pyffi")]
    pub fn __getitem__(&self, index: isize) -> PyResult<Float> {
        match index {
            -3..=-1 => Ok(self.coordinates[(3 + index) as usize]),
            0..=2 => Ok(self.coordinates[index as usize]),
            _ => Err(pyo3::exceptions::PyIndexError::new_err(
                "Invalid coordinate index",
            )),
        }
    }

    /// Determine whether this color is the default color, i.e., is the origin
    /// of the XYZ color space.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let black = Color::p3(0, 0, 0);
    /// let default_black = Color::new(ColorSpace::Xyz, [0.0, 0.0, 0.0]);
    /// assert!(black != default_black);
    /// assert!(black.to(ColorSpace::Xyz) == default_black);
    /// assert!(!black.is_default());
    /// assert!(default_black.is_default());
    /// assert!(Color::default() == default_black);
    /// assert!(Color::default().is_default());
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(display-p3 0 0 0)"></div>
    /// <div style="background-color: color(xyz 0 0 0)"></div>
    /// </div>
    #[inline]
    pub fn is_default(&self) -> bool {
        self.space == ColorSpace::Xyz && self.coordinates == [0.0, 0.0, 0.0]
    }

    /// The threshold used by [`is_achromatic`](Color::is_achromatic).
    #[cfg(feature = "pyffi")]
    #[classattr]
    pub const ACHROMATIC_THRESHOLD: Float = 0.01;

    /// The threshold used by [`is_achromatic`](Color::is_achromatic).
    #[cfg(not(feature = "pyffi"))]
    pub const ACHROMATIC_THRESHOLD: Float = 0.01;

    /// Determine whether this color is achromatic.
    ///
    /// For consistent, high-quality results, this method tests wether hue is
    /// not-a-number or chroma is less equal than
    /// [`ACHROMATIC_THRESHOLD`](Color::ACHROMATIC_THRESHOLD) in Oklch or
    /// Oklrch, converting this color if necessary.
    ///
    /// # Algorithmic Considerations
    ///
    /// Such a threshold-based predicate seems poorly suited to color spaces
    /// with Cartesian coordinates, as it carves a cuboid out of such spaces. Of
    /// course, the perceptual implications of the cuboid are highly dependent
    /// on the specific color space. (They also needn't be uniform; after all,
    /// gamma correction isn't uniform either.)
    ///
    /// As perceptually uniform color spaces, the differences between
    /// Oklab/Oklrab and Oklch/Oklrch are instructive here: In all four
    /// variations, achromatic colors form a thin column centered around the
    /// lightness axis. For Oklch/Oklrch, the column is circular. In other
    /// words, lightness and hue have *no* impact on whether colors are
    /// classified as achromatic. Only chroma, up to and including some
    /// threshold 𝚾, makes a difference. That nicely matches our informal
    /// expectations for lightness, chroma, and hue.
    ///
    /// In contrast, for Oklab/Oklrab, the column has a square profile. It
    /// intersects with the a/b axes at 𝚾 units from the lightness axis—just as
    /// the circular column does. But whereas that distance is constant for the
    /// surface of the circular column, the square column's corners are
    /// positioned ±𝚾 units along both the a/b axes from the origin. That is
    /// sqrt(𝚾²+𝚾²) = sqrt(2)⋅𝚾 = 1.41⋅𝚾 units from the lightness axis. In
    /// other words, the chroma threshold itself has a variability of 1.41×.
    ///
    /// # Examples
    ///
    /// The swatch below shows the Oklab colors at the four corners and the four
    /// axis intersections for a lightness of 0.6 and 𝚾=0.1 in order of
    /// increasing hue. The 1.41× difference in chroma may not be glaring, but
    /// it *is* clearly noticeable.
    ///
    /// <div class=color-swatch>
    /// <div style="background-color: oklab(0.6 0.1 0)"></div>
    /// <div style="background-color: oklab(0.6 0.1 0.1)"></div>
    /// <div style="background-color: oklab(0.6 0 0.1)"></div>
    /// <div style="background-color: oklab(0.6 -0.1 0.1)"></div>
    /// <div style="background-color: oklab(0.6 -0.1 0)"></div>
    /// <div style="background-color: oklab(0.6 -0.1 -0.1)"></div>
    /// <div style="background-color: oklab(0.6 0 -0.1)"></div>
    /// <div style="background-color: oklab(0.6 0.1 -0.1)"></div>
    /// </div>
    /// <br>
    ///
    /// Clearly, 𝚾=0.1 is on the large side when it comes to practical
    /// applications of achromatic testing. By comparison, this method uses
    /// 𝚾=0.01, which is an order-of-magnitude smaller and hence far more
    /// precise. Though, as illustrated by the example below, even that
    /// threshold allows for numerically significant divergence amongst, say,
    /// RGB coordinates.
    ///
    /// ```
    /// # use prettypretty::Color;
    /// let gray = Color::srgb(0.5, 0.5, 0.5);
    /// assert!(gray.is_achromatic());
    /// let gray_enough = Color::srgb(0.5, 0.5, 0.526);
    /// assert!(gray_enough.is_achromatic());
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(srgb 0.5 0.5 0.5)"></div>
    /// <div style="background-color: color(srgb 0.5 0.5 0.526)"></div>
    /// </div>
    /// <br>
    ///
    /// If you carefully look at the above swatch, you'll notice that the
    /// difference between the two colors isn't just numerical. It also is large
    /// enough to be visible!
    ///
    /// The next example leverages the above analysis of the geometry of the
    /// achromatic subspaces in Oklab/Oklch to systematically test boundary
    /// conditions. Yet again, the 1.41× difference in chroma is large enough to
    /// be perceptible.
    ///
    /// ```
    /// # use prettypretty::Color;
    /// let long = Color::ACHROMATIC_THRESHOLD;
    /// let short = Color::ACHROMATIC_THRESHOLD / 2.0_f64.sqrt();
    ///
    /// assert!(Color::oklab(0.5, long, 0).is_achromatic());
    /// assert!(Color::oklab(0.5, 0, long).is_achromatic());
    /// assert!(Color::oklab(0.5, -long, 0).is_achromatic());
    /// assert!(Color::oklab(0.5, 0, -long).is_achromatic());
    ///
    /// assert!(!Color::oklab(0.5, long, long).is_achromatic());
    /// assert!(!Color::oklab(0.5, -long, long).is_achromatic());
    /// assert!(!Color::oklab(0.5, -long, -long).is_achromatic());
    /// assert!(!Color::oklab(0.5, long, -long).is_achromatic());
    ///
    /// assert!(Color::oklab(0.5, short, short).is_achromatic());
    /// assert!(Color::oklab(0.5, -short, short).is_achromatic());
    /// assert!(Color::oklab(0.5, -short, -short).is_achromatic());
    /// assert!(Color::oklab(0.5, short, -short).is_achromatic());
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklab(0.5 0.01 0)"></div>
    /// <div style="background-color: oklab(0.5 0 0.01)"></div>
    /// <div style="background-color: oklab(0.5 -0.01 0)"></div>
    /// <div style="background-color: oklab(0.5 0 -0.01)"></div>
    /// <hr>
    /// <div style="background-color: oklab(0.5 0.01 0.01)"></div>
    /// <div style="background-color: oklab(0.5 -0.01 0.01)"></div>
    /// <div style="background-color: oklab(0.5 -0.01 -0.01)"></div>
    /// <div style="background-color: oklab(0.5 0.01 -0.01)"></div>
    /// <hr>
    /// <div style="background-color: oklab(0.5 0.007 0.007)"></div>
    /// <div style="background-color: oklab(0.5 -0.007 0.007)"></div>
    /// <div style="background-color: oklab(0.5 -0.007 -0.007)"></div>
    /// <div style="background-color: oklab(0.5 0.007 -0.007)"></div>
    /// </div>
    #[inline]
    pub fn is_achromatic(&self) -> bool {
        is_achromatic(self.space, &self.coordinates, Color::ACHROMATIC_THRESHOLD)
    }

    /// Determine whether this color is achromatic given the threshold.
    ///
    /// For consistent, high-quality results, this method tests wether hue is
    /// not-a-number or chroma is less equal than the threshold in Oklch or
    /// Oklrch, converting this color if necessary.
    ///
    /// The threshold must be non-negative, since chroma in Oklch/Oklrch is
    /// non-negative and hence cannot possibly be less equal than a negative
    /// threshold. Upon violation of this invariant, this method returns the
    /// offending threshold as an error object.
    ///
    /// # Algorithm
    ///
    /// Unlike [`is_achromatic`](Color::is_achromatic), this method accepts an
    /// explicit threshold argument. While that does necessitate testing the
    /// argument's sign on every single invocation, it also addresses a real
    /// need for applications such as
    /// [prettypretty.plot](https://github.com/apparebit/prettypretty/blob/main/prettypretty/plot.py)
    /// that require consistent but coarse-grained detection of achromatic
    /// colors. The plot script, for instance, scatter-plots colors on the
    /// two-dimensional hue-chroma plane of the Oklab variations. Since that
    /// would project achromatic colors to the origin and nearly achromatic
    /// colors near the origin, the script uses a relatively high threshold,
    /// 𝜲=0.05, to prevent a large blob of partially overlappig colors close to
    /// the origin. Instead, close-by colors are projected to the origin, which
    /// shows only one (average) gray.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::Color;
    /// assert_eq!(
    ///     Color::oklab(0.35, 0.11, -0.03).is_achromatic_threshold(-0.0),
    ///     Err(-0.0)
    /// );
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklab(0.35 0.11 -0.03)"></div>
    /// </div>
    #[cfg(not(feature = "pyffi"))]
    pub fn is_achromatic_threshold(&self, threshold: Float) -> Result<bool, Float> {
        if threshold.is_sign_negative() {
            Err(threshold)
        } else {
            Ok(is_achromatic(self.space, &self.coordinates, threshold))
        }
    }

    /// Determine whether this color is achromatic given the threshold.
    ///
    /// For consistent, high-quality results, this method tests wether hue is
    /// not-a-number or chroma is less equal than the threshold in Oklch or
    /// Oklrch, converting this color if necessary.
    ///
    /// The threshold must be non-negative, since chroma in Oklch/Oklrch is
    /// non-negative and hence cannot possibly be less equal than a negative
    /// threshold. Upon violation of this invariant, this method returns the
    /// offending threshold as an error object.
    ///
    /// # Algorithm
    ///
    /// Unlike [`is_achromatic`](Color::is_achromatic), this method accepts an
    /// explicit threshold argument. While that does necessitate testing the
    /// argument's sign on every single invocation, it also addresses a real
    /// need for applications such as
    /// [prettypretty.plot](https://github.com/apparebit/prettypretty/blob/main/prettypretty/plot.py)
    /// that require consistent but coarse-grained detection of achromatic
    /// colors. The plot script, for instance, scatter-plots colors on the
    /// two-dimensional hue-chroma plane of the Oklab variations. Since that
    /// would project achromatic colors to the origin and nearly achromatic
    /// colors near the origin, the script uses a relatively high threshold,
    /// 𝜲=0.05, to prevent a large blob of partially overlappig colors close to
    /// the origin. Instead, close-by colors are projected to the origin, which
    /// shows only one (average) gray.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::Color;
    /// assert_eq!(
    ///     Color::oklab(0.35, 0.11, -0.03).is_achromatic_threshold(-0.0),
    ///     Err(pyo3::exceptions::PyValueError::new_err(format!(
    ///         "negative achromatic threshold -0.0"
    ///     )))
    /// );
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklab(0.35 0.11 -0.03)"></div>
    /// </div>
    #[cfg(feature = "pyffi")]
    pub fn is_achromatic_threshold(&self, threshold: Float) -> PyResult<bool> {
        if threshold.is_sign_negative() {
            Err(pyo3::exceptions::PyValueError::new_err(format!(
                "negative achromatic threshold {}",
                threshold
            )))
        } else {
            Ok(is_achromatic(self.space, &self.coordinates, threshold))
        }
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Normalize this color.
    ///
    /// This function replaces not-a-number coordinates with zero. For semantic
    /// coherence, if the hue in Oklch/Oklrch is not-a-number, it also replaces
    /// chroma with zero. Furthermore, it clamps (revised) lightness to `0..=1`
    /// and chroma to `0..`.
    ///
    /// Many methods automatically normalize colors. A statement to that effect
    /// is included in their documentation. Methods that do *not* normalize
    /// their colors include [`Color::clip`], [`Color::distance`],
    ///  [`Color::in_gamut`], and [`Color::is_default`].
    #[inline]
    pub fn normalize(&self) -> Self {
        Self::new(self.space, normalize(self.space, &self.coordinates))
    }

    /// Determine the hue (in radians) and chroma of this color.
    ///
    /// If this color is not in the Oklch or Oklrch color space, this method
    /// converts the color and then returns the hue in radians and the chroma.
    pub fn hue_chroma(&self) -> (Float, Float) {
        let [_, c, h] = match self.space {
            ColorSpace::Oklch | ColorSpace::Oklrch => self.coordinates,
            ColorSpace::Oklrab => self.to(ColorSpace::Oklrch).coordinates,
            _ => self.to(ColorSpace::Oklch).coordinates,
        };

        (h.to_radians(), c)
    }

    /// Determine the x, y chromaticity coordinates of this color.
    ///
    /// This method determines the x, y coordinates for the 1931 version of the
    /// CIE chromaticity diagram.
    pub fn xy_chromaticity(&self) -> (Float, Float) {
        let [x, y, z] = self.to(ColorSpace::Xyz).coordinates;
        let sum = x + y + z;
        (x / sum, y / sum)
    }

    /// Determine the u', v' chromaticity coordinates of this color.
    ///
    /// This method determines the u', v' coordinates for the 1976 version of
    /// the CIE chromaticity diagram.
    pub fn uv_prime_chromaticity(&self) -> (Float, Float) {
        let (x, y) = self.xy_chromaticity();
        (
            4.0 * x / (-2.0 * x + 12.0 * y + 3.0),
            9.0 * y / (-2.0 * x + 12.0 * y + 3.0),
        )
    }

    /// Convert this color to the target color space.
    ///
    /// This method normalizes the color before conversion.
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
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let gray = Color::from_24bit(0x6c, 0x74, 0x79);
    /// assert_eq!(gray, Color::new(
    ///     ColorSpace::Srgb,
    ///     [108.0/255.0, 116.0/255.0, 121.0/255.0]
    /// ));
    /// let same_gray = gray.to(ColorSpace::Oklrch);
    /// assert_eq!(same_gray, Color::new(
    ///     ColorSpace::Oklrch,
    ///     [0.4827939631351205, 0.012421260273578993, 234.98550533688365]
    /// ));
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: rgb(108 116 121)"></div>
    /// </div>
    #[inline]
    #[must_use = "method returns a new color and does not mutate original value"]
    pub fn to(&self, target: ColorSpace) -> Self {
        Self::new(target, convert(self.space, target, &self.coordinates))
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Determine whether this color is in-gamut for its color space.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let red = Color::srgb(1.0, 0.0, 0.0);
    /// assert!(red.in_gamut());
    ///
    /// let green = Color::p3(0.0, 1.0, 0.0);
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
    /// let too_green = Color::new(ColorSpace::DisplayP3, [0.0, 1.0, 0.0])
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
        Self::new(self.space, clip(self.space, &self.coordinates))
    }

    /// Map this color into the gamut of its color space.
    ///
    /// This method normalizes the color before gamut-mapping.
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
    /// adjustments in Oklch. It is nicely suited to color manipulation because
    /// it is both perceptually uniform and has polar coordinates. Third, it
    /// measures distance in Oklab. Since the color space is perceptually
    /// uniform and has Cartesian coordinates, computing that distance is as
    /// simple as calculating Euclidian distance, i.e., the square root of the
    /// coordinate differences squared and summed.
    ///
    /// # Examples
    ///
    /// Display P3's yellow secondary is out of gamut in sRGB. Gamut mapping the
    /// converted color does bring it into gamut while also perserving the hue
    /// and maximizing the chroma, all within sRGB's gamut.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let too_green = Color::new(ColorSpace::DisplayP3, [0.0, 1.0, 0.0])
    ///     .to(ColorSpace::Srgb);
    /// assert!(!too_green.in_gamut());
    ///
    /// let green = too_green.to_gamut();
    /// assert!(green.in_gamut());
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(display-p3 0 1 0);"></div>
    /// <div style="background-color: color(srgb 0.0 0.9857637107710325 0.15974244397344017);"></div>
    /// </div>
    #[inline]
    #[must_use = "method returns a new color and does not mutate original value"]
    pub fn to_gamut(&self) -> Self {
        Self::new(self.space, to_gamut(self.space, &self.coordinates))
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
    /// # use prettypretty::{assert_close_enough, Color, ColorSpace, OkVersion};
    /// # use prettypretty::error::ColorFormatError;
    /// # use std::str::FromStr;
    /// let honeydew = Color::from_str("#d4fb79")?;
    /// let cantaloupe = Color::from_str("#ffd479")?;
    /// let d1 = honeydew.distance(&cantaloupe, OkVersion::Original);
    /// let d2 = honeydew.distance(&cantaloupe, OkVersion::Revised);
    /// assert_close_enough!(d1, 0.11174969799958659);
    /// assert_close_enough!(d2, 0.11498895250174994);
    /// # Ok::<(), ColorFormatError>(())
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #d4fb79;"></div>
    /// <div style="background-color: #ffd479;"></div>
    /// </div>
    #[inline]
    pub fn distance(&self, other: &Self, version: OkVersion) -> f64 {
        delta_e_ok(
            &self.to(version.cartesian_space()).coordinates,
            &other.to(version.cartesian_space()).coordinates,
        )
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Interpolate the two colors.
    ///
    /// This method creates a new interpolator for this and the given color.
    /// [`Interpolator::at`] generates the actual, interpolated colors. It
    /// normalizes both colors.
    ///
    /// # Examples
    ///
    /// As illustrated below, [`Color::interpolate`] takes care of the mechanics
    /// of interpolation. However, the resulting color may not be displayable
    /// and hence require further processing, such as gamut mapping.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace, HueInterpolation};
    /// let red = Color::srgb(0.8, 0.0, 0.0);
    /// let yellow = Color::from_24bit(0xff, 0xca, 0);
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
    /// let purple = Color::from_24bit(0xe1, 0x87, 0xfd);
    /// let orange = Color::from_24bit(0xf7, 0xaa, 0x31);
    /// let salmon = purple
    ///     .interpolate(&orange, ColorSpace::Oklab, HueInterpolation::Shorter)
    ///     .at(0.5)
    ///     .to(ColorSpace::DisplayP3)
    ///     .to_gamut();
    /// let pink = purple
    ///     .interpolate(&orange, ColorSpace::Oklch, HueInterpolation::Shorter)
    ///     .at(0.5)
    ///     .to(ColorSpace::DisplayP3)
    ///     .to_gamut();
    /// let cyan = purple
    ///     .interpolate(&orange, ColorSpace::Oklch, HueInterpolation::Longer)
    ///     .at(0.5)
    ///     .to(ColorSpace::DisplayP3)
    ///     .to_gamut();
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
    #[inline]
    #[must_use = "method returns interpolator and does not mutate original values"]
    pub fn interpolate(
        &self,
        color: &Self,
        interpolation_space: ColorSpace,
        interpolation_strategy: HueInterpolation,
    ) -> Interpolator {
        Interpolator::new(self, color, interpolation_space, interpolation_strategy)
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Lighten this color by the given factor in Oklrch.
    ///
    /// This method normalizes this color, converts it to Oklrch, multiplies its
    /// lightness Lr by the given factor, and returns the result—which may or
    /// may not be in-gamut for another color space. This method does not
    /// include an option for selecting Oklch because of its non-uniform
    /// lightness L.
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
    /// let goldenrod1 = Color::from_24bit(0x8b, 0x65, 0x08);
    /// let goldenrod3 = goldenrod1.lighten(1.4).to(Srgb);
    /// let goldenrod2 = goldenrod3.lighten(1.2/1.4).to(Srgb);
    /// assert_eq!(goldenrod1.to_24bit(), [0x8b_u8, 0x65, 0x08]);
    /// assert_eq!(goldenrod2.to_24bit(), [0xa4_u8, 0x7d, 0x2c]);
    /// assert_eq!(goldenrod3.to_24bit(), [0xbd_u8, 0x95, 0x47]);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #8b6508;"></div>
    /// <div style="background-color: #a47d2c;"></div>
    /// <div style="background-color: #bd9547;"></div>
    /// </div>
    #[inline]
    #[must_use = "method returns a new color and does not mutate original value"]
    pub fn lighten(&self, factor: Float) -> Self {
        Self::new(
            ColorSpace::Oklrch,
            scale_lightness(self.space, &self.coordinates, factor),
        )
    }

    /// Darken this color by the given factor.
    ///
    /// Darkening is the same as lightening, except that it is using the inverse
    /// factor. See [`Color::lighten`]. This method normalizes this color.
    #[inline]
    #[must_use = "method returns a new color and does not mutate original value"]
    pub fn darken(&self, factor: Float) -> Self {
        Self::new(
            ColorSpace::Oklrch,
            scale_lightness(self.space, &self.coordinates, factor.recip()),
        )
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Determine the perceptual contrast of text against a solidly colored
    /// background.
    ///
    /// This method computes the asymmetric, perceptual contrast of text with
    /// this color against a background with the given color. It uses an
    /// algorithm that is surprisingly similar to the [Accessible Perceptual
    /// Contrast Algorithm](https://github.com/Myndex/apca-w3) (APCA), version
    /// 0.0.98G-4g. This method normalizes both colors.
    ///
    /// According to the [bronze level conformance
    /// criteria](https://readtech.org/ARC/tests/bronze-simple-mode/?tn=criterion)
    /// for APCA, a contrast of 75 is the minimum for body text and a contrast
    /// of 90 is desirable.
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

    /// Determine the text with maximal perceptual contrast.
    ///
    /// This method determines whether black or white text maximizes perceptual
    /// contrast against a background with this color. This method normalizes
    /// the color. It uses the same algorithm as [`Color::contrast_against`].
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// # use prettypretty::error::ColorFormatError;
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

        -to_contrast(1.0, luminance) <= to_contrast(0.0, luminance)
    }

    /// Determine the background with maximal perceptual contrast.
    ///
    /// This method determines whether a black or white background maximizes
    /// perceptual contrast behind text with this color. This method normalizes
    /// the color. It uses the same algorithm as [`Color::contrast_against`].
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// # use prettypretty::error::ColorFormatError;
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

    // ----------------------------------------------------------------------------------------------------------------

    /// Convert this color to 24-bit RGB representation.
    ///
    /// This method converts the color to a gamut-mapped sRGB color before
    /// converting each coordinate to a `u8`.
    pub fn to_24bit(&self) -> [u8; 3] {
        to_24bit(
            ColorSpace::Srgb,
            self.to(ColorSpace::Srgb).to_gamut().as_ref(),
        )
    }

    /// Format this color in familiar `#123abc` hashed hexadecimal representation.
    ///
    /// Like [`Color::to_24bit`], this method converts the color to a
    /// gamut-mapped sRGB color before formatting its coordinates in hashed
    /// hexadecimal notation.
    ///
    /// # Examples
    ///
    /// The example code illustrates formatting in hashed hexadecimal format for
    /// a hot pink in Display P3. Together with the color swatch below, it also
    /// demonstrates that clipped and gamut-mapped color can substantially
    /// differ.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let pink = Color::p3(1, 0.2, 1).to(ColorSpace::Srgb);
    /// assert!(!pink.in_gamut());
    /// let clip = pink.clip();
    /// assert_eq!(clip, Color::srgb(1, 0, 1));
    /// let hex = pink.to_hex_format();
    /// assert_eq!(hex, "#ff41fb");
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(display-p3 1 0.2 1);"></div>
    /// <div style="background-color: color(srgb 1 0 1);"></div>
    /// <div style="background-color: #ff41fb;"></div>
    /// </div>
    #[inline]
    pub fn to_hex_format(&self) -> String {
        let [r, g, b] = self.to_24bit();
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    }

    // ----------------------------------------------------------------------------------------------------------------

    /// Convert this color to its debug representation. <i
    /// class=python-only>Python only!</i>
    ///
    /// This method is available from Python only.
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    /// Convert this color to its (CSS-based) string representation. <i
    /// class=python-only>Python only!</i>
    ///
    /// This method is available from Python only.
    #[cfg(feature = "pyffi")]
    pub fn __str__(&self) -> String {
        format!("{}", self)
    }
}

// --------------------------------------------------------------------------------------------------------------------

// Use separate block, so that methods are not exposed to Python.
// Use cfg(), so that methods are not documented again.
#[cfg(not(feature = "pyffi"))]
impl Color {
    /// Instantiate a new sRGB color with the given red, green, and blue
    /// coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let fire_brick = Color::srgb(177.0/255.0, 31.0/255.0, 36.0/255.0);
    /// assert_eq!(fire_brick.space(), ColorSpace::Srgb);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: rgb(177 31 36);"></div>
    /// </div>
    pub fn srgb(r: impl Into<Float>, g: impl Into<Float>, b: impl Into<Float>) -> Self {
        Self::new(ColorSpace::Srgb, [r.into(), g.into(), b.into()])
    }

    /// Instantiate a new Display P3 color with the given red, green, and blue
    /// coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let cyan = Color::p3(0, 0.87, 0.85);
    /// assert_eq!(cyan.space(), ColorSpace::DisplayP3);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(display-p3 0 0.87 0.85);"></div>
    /// </div>
    pub fn p3(r: impl Into<Float>, g: impl Into<Float>, b: impl Into<Float>) -> Self {
        Self::new(ColorSpace::DisplayP3, [r.into(), g.into(), b.into()])
    }

    /// Instantiate a new Oklab color with the given lightness L, a, and b
    /// coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let sky = Color::oklab(0.78, -0.1, -0.1);
    /// assert_eq!(sky.space(), ColorSpace::Oklab);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklab(0.78 -0.1 -0.1);"></div>
    /// </div>
    pub fn oklab(l: impl Into<Float>, a: impl Into<Float>, b: impl Into<Float>) -> Self {
        Self::new(ColorSpace::Oklab, [l.into(), a.into(), b.into()])
    }

    /// Instantiate a new Oklrab color with the given revised lightness Lr, a,
    /// and b coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let turquoise = Color::oklrab(0.48, -0.1, -0.1);
    /// assert_eq!(turquoise.space(), ColorSpace::Oklrab);
    /// assert!(
    ///     (turquoise.to(ColorSpace::Oklab).as_ref()[0] - 0.5514232757779728).abs()
    ///     < 1e-13
    /// );
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklab(0.5514232757779728 -0.1 -0.1);"></div>
    /// </div>
    pub fn oklrab(lr: impl Into<Float>, a: impl Into<Float>, b: impl Into<Float>) -> Self {
        Self::new(ColorSpace::Oklrab, [lr.into(), a.into(), b.into()])
    }

    /// Instantiate a new Oklch color with the given lightness L, chroma C, and
    /// hue h coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let olive = Color::oklch(0.59, 0.1351, 126);
    /// assert_eq!(olive.space(), ColorSpace::Oklch);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: oklch(0.59 0.1351 126);"></div>
    /// </div>
    pub fn oklch(l: impl Into<Float>, c: impl Into<Float>, h: impl Into<Float>) -> Self {
        Self::new(ColorSpace::Oklch, [l.into(), c.into(), h.into()])
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
    pub fn oklrch(lr: impl Into<Float>, c: impl Into<Float>, h: impl Into<Float>) -> Self {
        Self::new(ColorSpace::Oklrch, [lr.into(), c.into(), h.into()])
    }
}

// Use separate block, so that methods are not exposed to Python.
// Do not use cfg(), so that methods are documented.
impl Color {
    /// Find the index position of the candidate color closest to this color.
    /// <i class=rust-only>Rust only!</i>
    ///
    /// This method delegates to [`Color::find_closest`] using the Delta E
    /// metric for Oklab/Oklrab, which is the Euclidian distance.
    ///
    /// Since this method converts every color to either Oklab or Oklrab, it
    /// also normalizes every color before use.
    ///
    /// Because it is generic, this method is available in Rust only. A
    /// specialized version is available in Python through
    /// [`Translator`](crate::trans::Translator).
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace, OkVersion};
    /// let colors = [
    ///     &Color::from_24bit(0xc4, 0x13, 0x31),
    ///     &Color::from_24bit(0, 0x80, 0x25),
    ///     &Color::from_24bit(0x30, 0x78, 0xea),
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
        self.find_closest(candidates, version.cartesian_space(), delta_e_ok)
    }

    /// Find the index position of the candidate color closest to this color.
    /// <i class=rust-only>Rust only!</i>
    ///
    /// This method compares this color to every candidate color by computing
    /// the distance with the given function and returns the index position of
    /// the candidate with smallest distance. If there are no candidates, it
    /// returns `None`. The distance metric is declared `mut` to allow for
    /// stateful comparisons.
    ///
    /// Since this method converts every color to the given color space, it also
    /// normalizes every color before use.
    ///
    /// Because it is generic, this method is available in Rust only. A
    /// specialized version is available in Python through
    /// [`Translator`](crate::trans::Translator).
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
}

impl Default for Color {
    /// Create an instance of the default color.
    ///
    /// The chosen default for high-resolution colors is the origin in XYZ,
    /// i.e., pitch black.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let default = Color::default();
    /// assert_eq!(default.space(), ColorSpace::Xyz);
    /// assert_eq!(default.as_ref(), &[0.0_f64, 0.0, 0.0]);
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(xyz 0 0 0);"></div>
    /// </div>
    #[inline]
    fn default() -> Self {
        Self::new(ColorSpace::Xyz, [0.0, 0.0, 0.0])
    }
}

impl std::str::FromStr for Color {
    type Err = crate::error::ColorFormatError;

    /// Instantiate a color from its string representation.
    ///
    /// Before parsing the string slice, this method trims any leading and
    /// trailing white space while also converting ASCII letters to lower case.
    /// That makes parsing effectively case-insensitive.
    ///
    /// This method recognizes two hexadecimal notations for RGB colors, the
    /// hashed notation familiar from the web and the XParseColor notation
    /// familiar from X Windows. While the latter originally specified *device
    /// RGB*, this crate treats `rgb:` strings as specifying sRGB colors.
    ///
    /// The *hashed notation* has three or six hexadecimal digits, e.g., `#123` or
    /// #`cafe00`. Note that the three digit version is a short form of the six
    /// digit version with every digit repeated. In other words, the red
    /// coordinate in `#123` is not 0x1/0xf but 0x11/0xff.
    ///
    /// The *XParseColor notation* has between one and four hexadecimal digits
    /// per coordinate, e.g., `rgb:1/00/cafe`. Here, every coordinate is scaled,
    /// i.e., the red coordinate in the example is 0x1/0xf.
    ///
    /// This method also recognizes a subset of the *CSS color syntax*. In
    /// particular, it recognizes the `color()`, `oklab()`, and `oklch` CSS
    /// functions. For `color()`, the color space right after the opening
    /// parenthesis may be `srgb`, `linear-srgb`, `display-p3`,
    /// `--linear-display-p3`, `rec2020`, `--linear-rec2020`, `--oklrab`,
    /// `--oklrch`, or `xyz`. As indicated by the leading double-dashes, the
    /// linear versions of Display P3 and Rec. 2020 as well as OkLrab and Oklrch
    /// are not included in [CSS 4 Color](https://www.w3.org/TR/css-color-4/).
    /// Coordinates must be space-separated and unitless (i.e., no `%` or
    /// `deg`).
    ///
    /// By implementing the `FromStr` trait, `str::parse` works just the same
    /// for parsing color formats—that is, as long as type inference can
    /// determine what type to parse. Hence, the definition of `rose` in the
    /// code example below explicitly declares the type of that variable,
    /// whereas the definition of `navy` gets by without such an annotation.
    ///
    /// Don't forget the `use` statement bringing `FromStr` into scope.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// # use prettypretty::error::ColorFormatError;
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
        parse(s).map(|(space, coordinates)| Self::new(space, coordinates))
    }
}

impl TryFrom<&str> for Color {
    type Error = crate::error::ColorFormatError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Color::from_str(value)
    }
}

impl TryFrom<String> for Color {
    type Error = crate::error::ColorFormatError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Color::from_str(value.as_str())
    }
}

impl AsRef<[Float; 3]> for Color {
    fn as_ref(&self) -> &[Float; 3] {
        &self.coordinates
    }
}

impl std::ops::Index<usize> for Color {
    type Output = f64;

    /// Access the coordinate with the given index.
    ///
    /// # Panics
    ///
    /// This method panics if `2 < index`.
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

impl std::hash::Hash for Color {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.space.hash(state);

        let [n1, n2, n3] = to_eq_coordinates(self.space, &self.coordinates);
        n1.hash(state);
        n2.hash(state);
        n3.hash(state);
    }
}

impl PartialEq for Color {
    /// Determine whether this color equals the other color.
    ///
    /// A key requirement for data structures that implement the `Eq` and `Hash`
    /// traits is that [`Self::hash`](struct.Color.html#method.hash)  produces
    /// the same results for colors that are [`Color::eq`]. [`Color`] enforces
    /// that invariant by normalizing coordinates and turning them into bit
    /// strings before equality testing or hashing. In particular, both methods
    /// perform the following steps:
    ///
    ///   * To turn coordinates into comparable entities, replace not-a-numbers with
    ///     positive zero;
    ///   * To preserve not-a-number semantics for hues, also zero out chroma for
    ///     not-a-number hues in Oklch;
    ///   * To preserve rotation semantics for hues, remove all full rotations;
    ///   * To prepare for rounding, scale down hues to unit range;
    ///   * To allow for floating point error, multiply by 1e5/1e14 (depending
    ///     on `Float`'s type) and then round to drop least significant digit;
    ///   * To make zeros comparable, replace negative zero with positive zero
    ///     (but only after rounding, as it may produce zeros);
    ///   * To convince Rust that coordinates are comparable, convert them to
    ///     bits.
    ///
    /// While rounding isn't strictly necessary for correctness, it makes for a
    /// more robust comparison without meaningfully reducing precision, at least
    /// for the default representation using `f64`.
    ///
    /// # Examples
    ///
    /// The following example code illustrates how equality testing handles
    /// not-a-numbers, numbers with very small differences, and hues:
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace, Float};
    /// let delta = 2.0 * (10.0 as Float).powi(-(Float::DIGITS as i32));
    /// assert_eq!(
    ///     Color::srgb(Float::NAN, 4.0 * delta, 0.12 + delta),
    ///     Color::srgb(0,          5.0 * delta, 0.12        )
    /// );
    ///
    /// assert_eq!(Color::oklch(0.5, 0.1, 665), Color::oklch(0.5, 0.1, 305));
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: color(srgb 0 0.00000000000001 0.12);"></div>
    /// <div style="background-color: oklch(0.5 0.1 305);"></div>
    /// </div>
    fn eq(&self, other: &Self) -> bool {
        if self.space != other.space {
            return false;
        } else if self.coordinates == other.coordinates {
            return true;
        }

        let n1 = to_eq_coordinates(self.space, &self.coordinates);
        let n2 = to_eq_coordinates(other.space, &other.coordinates);
        n1 == n2
    }
}

impl Eq for Color {}

impl std::fmt::Debug for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [c1, c2, c3] = self.coordinates;
        f.write_fmt(format_args!(
            "Color({:?}, [{}, {}, {}])",
            self.space, c1, c2, c3
        ))
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
    /// # use prettypretty::{Color, ColorSpace::*};
    /// # use prettypretty::error::ColorFormatError;
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
    /// # use prettypretty::{Color, ColorSpace::*};
    /// # use prettypretty::error::ColorFormatError;
    /// # use std::str::FromStr;
    /// let gray = Color::oklch(0.665, 0, f64::NAN);
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

// ====================================================================================================================

/// A choice of Oklab versions.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, eq_int, frozen, hash, module = "prettypretty.color")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum OkVersion {
    /// The original Oklab/Oklch color spaces.
    Original,
    /// The revised Oklrab/Oklrch color spaces.
    Revised,
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl OkVersion {
    /// Determine the Cartesion color space corresponding to this version of the
    /// Oklab color spaces.
    pub const fn cartesian_space(&self) -> ColorSpace {
        match *self {
            Self::Original => ColorSpace::Oklab,
            Self::Revised => ColorSpace::Oklrab,
        }
    }

    /// Determine the polar color space corresponding to this version of the
    /// Oklab color space.
    pub const fn polar_space(&self) -> ColorSpace {
        match *self {
            Self::Original => ColorSpace::Oklch,
            Self::Revised => ColorSpace::Oklrch,
        }
    }
}

// ====================================================================================================================

/// Helper struct returned by [`Color::interpolate`].
///
/// An interpolator performs linear interpolation between the coordinates of two
/// colors according to [CSS Color
/// 4](https://www.w3.org/TR/css-color-4/#interpolation). While the linear
/// interpolation itself is straight-forward, preparing color coordinates in
/// accordance with the specification is surprisingly complicated because it
/// requires carrying forward missing components and adjusting hue according to
/// interpolation strategy.  However, instead of performing this preparatory
/// work for every interpolation, this struct can perform an arbitrary number of
/// interpolations for the its two source colors and thus potentially amortize
/// the cost of preparation.
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color"))]
#[derive(Clone, Debug)]
pub struct Interpolator {
    space: ColorSpace,
    coordinates1: [Float; 3],
    coordinates2: [Float; 3],
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Interpolator {
    /// Create a new color interpolator.
    ///
    /// See [`Color::interpolate`] for detailed examples.
    #[cfg(feature = "pyffi")]
    #[new]
    pub fn new(
        color1: &Color,
        color2: &Color,
        space: ColorSpace,
        strategy: HueInterpolation,
    ) -> Self {
        let (coordinates1, coordinates2) = prepare_to_interpolate(
            color1.space,
            &color1.coordinates,
            color2.space,
            &color2.coordinates,
            space,
            strategy,
        );

        Self {
            space,
            coordinates1,
            coordinates2,
        }
    }

    /// Create a new color interpolator.
    ///
    /// See [`Color::interpolate`] for detailed examples.
    #[cfg(not(feature = "pyffi"))]
    #[inline]
    pub fn new(
        color1: &Color,
        color2: &Color,
        space: ColorSpace,
        strategy: HueInterpolation,
    ) -> Self {
        let (coordinates1, coordinates2) = prepare_to_interpolate(
            color1.space,
            &color1.coordinates,
            color2.space,
            &color2.coordinates,
            space,
            strategy,
        );

        Self {
            space,
            coordinates1,
            coordinates2,
        }
    }

    /// Compute the interpolated color for the given fraction.
    ///
    /// See [`Color::interpolate`] for detailed examples.
    #[inline]
    pub fn at(&self, fraction: f64) -> Color {
        let [c1, c2, c3] = interpolate(fraction, &self.coordinates1, &self.coordinates2);
        Color::new(self.space, [c1, c2, c3])
    }

    /// Create a debug representation of this interpolator. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!(
            "Interpolator({:?}, {:?}, {:?})",
            self.space, self.coordinates1, self.coordinates2
        )
    }
}
