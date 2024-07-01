#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use crate::{Color, ColorSpace, OutOfBoundsError};

// ====================================================================================================================
// Ansi Color
// ====================================================================================================================

/// The 16 extended ANSI colors.
///
/// Rust code converts between 8-bit color codes and enumeration variants with
/// [`TryFrom<u8> as
/// AnsiColor`](enum.AnsiColor.html#impl-TryFrom%3Cu8%3E-for-AnsiColor) and
/// [`From<AnsiColor> as
/// u8`](enum.AnsiColor.html#impl-From%3CAnsiColor%3E-for-u8).
#[doc = include_str!("style.html")]
#[cfg_attr(
    feature = "pyffi",
    doc = "In contrast, Python code uses the [`AnsiColor::from_8bit`] and
    [`AnsiColor::to_8bit`] methods."
)]
/// Since ANSI colors have no intrinsic color values, conversion to
/// high-resolution colors requires additional machinery, which is implemented
/// by [`Theme`](crate::Theme).
#[cfg_attr(feature = "pyffi", pyclass(eq, eq_int, frozen, hash))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AnsiColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl AnsiColor {
    /// Instantiate an ANSI color from its 8-bit code. <span
    /// class=python-only></span>
    ///
    /// This method offers the same functionality as [`TryFrom<u8> as
    /// AnsiColor`](enum.AnsiColor.html#impl-TryFrom%3Cu8%3E-for-AnsiColor) and
    /// is available in Python only.
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn from_8bit(value: u8) -> Result<Self, OutOfBoundsError> {
        Self::try_from(value)
    }

    /// Get the 8-bit code for this ANSI color. <span class=python-only></span>
    ///
    /// This method offers the same functionality as [`From<AnsiColor> as
    /// u8`](enum.AnsiColor.html#impl-From%3CAnsiColor%3E-for-u8) and is
    /// available in Python only.
    #[cfg(feature = "pyffi")]
    pub fn to_8bit(&self) -> u8 {
        *self as u8
    }

    /// Determine whether this ANSI color is bright.
    pub fn is_bright(&self) -> bool {
        *self as u8 >= 8
    }

    /// Get this ANSI color as nonbright.
    ///
    /// If this color is bright, this method returns the equivalent nonbright
    /// color. Otherwise, it returns this color.
    pub fn nonbright(&self) -> AnsiColor {
        let mut index = *self as u8;
        if index >= 8 {
            index -= 8;
        }
        AnsiColor::try_from(index).unwrap()
    }
}

impl TryFrom<u8> for AnsiColor {
    type Error = OutOfBoundsError;

    /// Try to convert an unsigned byte to an ANSI color.
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let ansi = match value {
            0 => AnsiColor::Black,
            1 => AnsiColor::Red,
            2 => AnsiColor::Green,
            3 => AnsiColor::Yellow,
            4 => AnsiColor::Blue,
            5 => AnsiColor::Magenta,
            6 => AnsiColor::Cyan,
            7 => AnsiColor::White,
            8 => AnsiColor::BrightBlack,
            9 => AnsiColor::BrightRed,
            10 => AnsiColor::BrightGreen,
            11 => AnsiColor::BrightYellow,
            12 => AnsiColor::BrightBlue,
            13 => AnsiColor::BrightMagenta,
            14 => AnsiColor::BrightCyan,
            15 => AnsiColor::BrightWhite,
            _ => return Err(OutOfBoundsError::new(value, 0..=15)),
        };

        Ok(ansi)
    }
}

impl From<AnsiColor> for u8 {
    /// Convert an ANSI color to an unsigned byte.
    fn from(value: AnsiColor) -> u8 {
        value as u8
    }
}

// ====================================================================================================================
// The Embedded 6x6x6 RGB Cube
// ====================================================================================================================

/// The 6x6x6 RGB cube embedded in 8-bit terminal colors.
///
///
/// # Examples
///
/// Rust code can create a new embedded RGB color with either
/// [`EmbeddedRgb::new`] or [`TryFrom<u8> as
/// EmbeddedRgb`](struct.EmbeddedRgb.html#impl-TryFrom%3Cu8%3E-for-EmbeddedRgb).
///
#[doc = include_str!("style.html")]
/// ```
/// # use prettypretty::{EmbeddedRgb, OutOfBoundsError};
/// let orange = EmbeddedRgb::new(5, 2, 0)?;
/// let orange_too = EmbeddedRgb::try_from(208)?;
/// assert_eq!(orange, orange_too);
/// # Ok::<(), OutOfBoundsError>(())
/// ```
/// <div class=color-swatch>
/// <div style="background-color: #ff8700;"></div>
/// </div>
/// <br>
///
/// It can access the coordinates with [`AsRef<[u8; 3]> as
/// EmbeddedRgb`](struct.EmbeddedRgb.html#impl-AsRef%3C%5Bu8;+3%5D%3E-for-EmbeddedRgb)
/// or with [`Index<usize> as
/// EmbeddedRgb`](struct.EmbeddedRgb.html#impl-Index%3Cusize%3E-for-EmbeddedRgb).
/// ```
/// # use prettypretty::{EmbeddedRgb, OutOfBoundsError};
/// let blue = EmbeddedRgb::try_from(75)?;
/// assert_eq!(blue.as_ref(), &[1_u8, 3, 5]);
/// assert_eq!(blue[1], 3);
/// # Ok::<(), OutOfBoundsError>(())
/// ```
/// <div class=color-swatch>
/// <div style="background-color: #5fafff;"></div>
/// </div>
/// <br>
///
/// Finally, it can convert an embedded RGB color to `u8` with
/// [`From<EmbeddedRgb> as
/// u8`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-u8), to a true
/// color with [`From<EmbeddedRgb> as
/// TrueColor`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-TrueColor),
/// or to a high-resolution color with [`From<EmbeddedRgb> as
/// Color`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-Color).
/// ```
/// # use prettypretty::{Color, EmbeddedRgb, OutOfBoundsError, TrueColor};
/// let rose = EmbeddedRgb::new(5, 4, 5)?;
/// assert_eq!(u8::from(rose), 225);
///
/// let also_rose = TrueColor::from(rose);
/// assert_eq!(format!("{}", also_rose), "#ffd7ff");
///
/// let rose_too = Color::from(rose);
/// assert_eq!(rose_too.to_hex_format(), "#ffd7ff");
///
/// assert_eq!(Color::from(also_rose), rose_too);
/// # Ok::<(), OutOfBoundsError>(())
/// ```
/// <div class=color-swatch>
/// <div style="background-color: #ffd7ff;"></div>
/// </div>
/// <br>
///
#[cfg_attr(
    feature = "pyffi",
    doc = "Since there is no Python feature equivalent to trait implementations in
    Rust, the Python class for `EmbeddedRgb` provides equivalent functionality
    through [`EmbeddedRgb::from_8bit`], [`EmbeddedRgb::to_8bit`],
    [`EmbeddedRgb::to_color`], [`EmbeddedRgb::coordinates`], [`EmbeddedRgb::__len__`],
    [`EmbeddedRgb::__getitem__`], and [`EmbeddedRgb::__repr__`].
    These methods are not available in Rust."
)]
#[cfg_attr(feature = "pyffi", pyclass(eq, frozen, hash, sequence))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EmbeddedRgb([u8; 3]);

#[cfg(feature = "pyffi")]
#[pymethods]
impl EmbeddedRgb {
    /// Create a new embedded RGB value from its coordinates.
    #[new]
    pub fn new(r: u8, g: u8, b: u8) -> Result<Self, OutOfBoundsError> {
        if r >= 6 {
            Err(OutOfBoundsError::new(r, 0..=5))
        } else if g >= 6 {
            Err(OutOfBoundsError::new(g, 0..=5))
        } else if b >= 6 {
            Err(OutOfBoundsError::new(b, 0..=5))
        } else {
            Ok(Self([r, g, b]))
        }
    }

    /// Instantiate an embedded RGB color from its 8-bit code. <span
    /// class=python-only></span>
    ///
    /// This method offers the same functionality as [`TryFrom<u8> as
    /// EmbeddedRgb`](struct.EmbeddedRgb.html#impl-TryFrom%3Cu8%3E-for-EmbeddedRgb)
    /// and is available in Python only.
    #[staticmethod]
    pub fn from_8bit(value: u8) -> Result<Self, OutOfBoundsError> {
        Self::try_from(value)
    }

    /// Get the 8-bit code for this embedded RGB color. <span
    /// class=python-only></span>
    ///
    /// This method offers the same functionality as [`From<EmbeddedRgb> as
    /// u8`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-u8) and is
    /// available in Python only.
    pub fn to_8bit(&self) -> u8 {
        u8::from(*self)
    }

    /// Convert this embedded RGB color to a high-resolution color. <span
    /// class=python-only></span>
    ///
    /// This method offers the same functionality as [`From<EmbeddedRgb> as
    /// Color`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-Color)
    /// and is available in Python only.
    pub fn to_color(&self) -> Color {
        Color::from(*self)
    }

    /// Access this true color's coordinates. <span class=python-only></span>
    pub fn coordinates(&self) -> [u8; 3] {
        self.0
    }

    /// Get this embedded RGB color's length, which is 3. <span
    /// class=python-only></span>
    ///
    /// This method improves integration with Python's runtime and hence is
    /// available in Python only.
    pub fn __len__(&self) -> usize {
        3
    }

    /// Get the coordinate at the given index. <span class=python-only></span>
    ///
    /// This method improves integration with Python's runtime and hence is
    /// available in Python only.
    pub fn __getitem__(&self, index: isize) -> PyResult<u8> {
        match index {
            -3..=-1 => Ok(self.0[(3 + index) as usize]),
            0..=2 => Ok(self.0[index as usize]),
            _ => Err(pyo3::exceptions::PyIndexError::new_err(
                "Invalid coordinate index",
            )),
        }
    }

    /// Convert this embedded RGB color to its debug representation. <span
    /// class=python-only></span>
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[cfg(not(feature = "pyffi"))]
impl EmbeddedRgb {
    /// Create a new embedded RGB value from its coordinates.
    pub fn new(r: u8, g: u8, b: u8) -> Result<Self, OutOfBoundsError> {
        if r >= 6 {
            Err(OutOfBoundsError::new(r, 0..=5))
        } else if g >= 6 {
            Err(OutOfBoundsError::new(g, 0..=5))
        } else if b >= 6 {
            Err(OutOfBoundsError::new(b, 0..=5))
        } else {
            Ok(Self([r, g, b]))
        }
    }
}

impl TryFrom<u8> for EmbeddedRgb {
    type Error = OutOfBoundsError;

    /// Try instantiating an embedded RGB color from an unsigned byte.
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if !(16..=231).contains(&value) {
            Err(OutOfBoundsError::new(value, 16..=231))
        } else {
            let mut b = value - 16;
            let r = b / 36;
            b -= r * 36;
            let g = b / 6;
            b -= g * 6;

            Self::new(r, g, b)
        }
    }
}

impl AsRef<[u8; 3]> for EmbeddedRgb {
    /// Access this color's coordinates by reference.
    fn as_ref(&self) -> &[u8; 3] {
        &self.0
    }
}

impl std::ops::Index<usize> for EmbeddedRgb {
    type Output = u8;

    /// Access the coordinate with the given index.
    ///
    /// # Panics
    ///
    /// This method panics if `index > 2`.
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl From<EmbeddedRgb> for u8 {
    /// Convert an embedded RGB color to an unsigned byte.
    fn from(value: EmbeddedRgb) -> u8 {
        let [r, g, b] = value.0;
        16 + 36 * r + 6 * g + b
    }
}

impl From<EmbeddedRgb> for TrueColor {
    /// Instantiate a true color from an embedded RGB value.
    fn from(value: EmbeddedRgb) -> Self {
        fn convert(value: u8) -> u8 {
            if value == 0 {
                0
            } else {
                55 + 40 * value
            }
        }

        let [r, g, b] = *value.as_ref();
        TrueColor::new(convert(r), convert(g), convert(b))
    }
}

impl From<EmbeddedRgb> for Color {
    /// Instantiate a high-resolution color from an embedded RGB value.
    fn from(value: EmbeddedRgb) -> Self {
        fn convert(value: u8) -> u8 {
            if value == 0 {
                0
            } else {
                55 + 40 * value
            }
        }

        let [r, g, b] = *value.as_ref();
        Color::from_24bit(convert(r), convert(g), convert(b))
    }
}

// ====================================================================================================================
// Gray Gradient
// ====================================================================================================================

/// The 24-step gray gradient embedded in 8-bit terminal colors.
///
/// # Examples
///
/// Rust code can instantiate a new gray gradient color with either
/// [`GrayGradient::new`] or [`TryFrom<u8> as
/// GrayGradient`](struct.GrayGradient.html#impl-TryFrom%3Cu8%3E-for-GrayGradient).
///
#[doc = include_str!("style.html")]
/// ```
/// # use prettypretty::{GrayGradient, OutOfBoundsError};
/// let almost_black = GrayGradient::new(4)?;
/// let almost_black_too = GrayGradient::try_from(236)?;
/// assert_eq!(almost_black, almost_black_too);
/// # Ok::<(), OutOfBoundsError>(())
/// ```
/// <div class=color-swatch>
/// <div style="background-color: #303030;"></div>
/// </div>
/// <br>
///
/// It can access the gray level with [`GrayGradient::level`].
/// ```
/// # use prettypretty::{GrayGradient, OutOfBoundsError};
/// let midgray = GrayGradient::try_from(243)?;
/// assert_eq!(midgray.level(), 11);
/// # Ok::<(), OutOfBoundsError>(())
/// ```
/// <div class=color-swatch>
/// <div style="background-color: #767676;"></div>
/// </div>
/// <br>
///
/// Finally, it can convert a gray gradient color to `u8` with
/// [`From<GrayGradient> as
/// u8`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-u8), to a true
/// color with [`From<GrayGradient> as
/// TrueColor`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-TrueColor),
/// or to a high-resolution color with [`From<GrayGradient> as
/// Color`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-Color).
/// ```
/// # use prettypretty::{Color, GrayGradient, OutOfBoundsError, TrueColor};
/// let light_gray = GrayGradient::new(20)?;
/// assert_eq!(u8::from(light_gray), 252);
///
/// let also_light_gray = TrueColor::from(light_gray);
/// assert_eq!(format!("{}", also_light_gray), "#d0d0d0");
///
/// let light_gray_too = Color::from(light_gray);
/// assert_eq!(light_gray_too.to_hex_format(), "#d0d0d0");
///
/// assert_eq!(Color::from(also_light_gray), light_gray_too);
/// # Ok::<(), OutOfBoundsError>(())
/// ```
/// <div class=color-swatch>
/// <div style="background-color: #d0d0d0;"></div>
/// </div>
/// <br>
///
#[cfg_attr(
    feature = "pyffi",
    doc = "Since there is no Python feature equivalent to trait implementations in
    Rust, the Python class for `GrayGradient` provides equivalent functionality
    through [`GrayGradient::from_8bit`], [`GrayGradient::__repr__`],
    [`GrayGradient::to_8bit`], and [`GrayGradient::to_color`]. These methods are not
    available in Rust."
)]
#[cfg_attr(feature = "pyffi", pyclass(eq, frozen, hash, ord))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GrayGradient(u8);

#[cfg(feature = "pyffi")]
#[pymethods]
impl GrayGradient {
    /// Instantiate a new gray gradient from its level `0..=23`.
    #[new]
    pub fn new(value: u8) -> Result<Self, OutOfBoundsError> {
        if value >= 24 {
            Err(OutOfBoundsError::new(value, 0..=23))
        } else {
            Ok(Self(value))
        }
    }

    /// Instantiate a gray gradient from its 8-bit code. <span
    /// class=python-only></span>
    ///
    /// This method offers the same functionality as [`TryFrom<u8> as
    /// GrayGradient`](struct.GrayGradient.html#impl-TryFrom%3Cu8%3E-for-GrayGradient)
    /// and is available in Python only.
    #[staticmethod]
    pub fn from_8bit(value: u8) -> Result<Self, OutOfBoundsError> {
        Self::try_from(value)
    }

    /// Get the 8-bit code for this gray gradient color. <span
    /// class=python-only></span>
    ///
    /// This method offers the same functionality as [`From<GrayGradient> as
    /// u8`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-u8) and is
    /// available in Python only.
    #[inline]
    pub fn to_8bit(&self) -> u8 {
        u8::from(*self)
    }

    /// Convert this gray gradient to a high-resolution color. <span
    /// class=python-only></span>
    ///
    /// This method offers the same functionality as [`From<GrayGradient> as
    /// Color`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-Color)
    /// and is available in Python only.
    #[inline]
    pub fn to_color(&self) -> Color {
        Color::from(*self)
    }

    /// Access the gray level `0..=23`.
    #[inline]
    pub const fn level(&self) -> u8 {
        self.0
    }

    /// Convert this gray gradient to its debug representation. <span
    /// class=python-only></span>
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[cfg(not(feature = "pyffi"))]
impl GrayGradient {
    /// Instantiate a new gray gradient from its level `0..=23`.
    pub fn new(value: u8) -> Result<Self, OutOfBoundsError> {
        if value <= 23 {
            Ok(Self(value))
        } else {
            Err(OutOfBoundsError::new(value, 0..=23))
        }
    }

    /// Access the gray level `0..=23`.
    #[inline]
    pub const fn level(&self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for GrayGradient {
    type Error = OutOfBoundsError;

    /// Try instantiating a gray gradient value from an unsigned byte.
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= 231 {
            Err(OutOfBoundsError::new(value, 232..=255))
        } else {
            Self::new(value - 232)
        }
    }
}

impl From<GrayGradient> for u8 {
    /// Convert the gray gradient to an unsigned byte.
    fn from(value: GrayGradient) -> u8 {
        232 + value.0
    }
}

impl From<GrayGradient> for TrueColor {
    /// Convert the gray gradient to a true color.
    fn from(value: GrayGradient) -> TrueColor {
        let level = 8 + 10 * value.level();
        TrueColor::new(level, level, level)
    }
}

impl From<GrayGradient> for Color {
    /// Instantiate a high-resolution color from an embedded RGB value.
    fn from(value: GrayGradient) -> Self {
        let level = 8 + 10 * value.level();
        Color::from_24bit(level, level, level)
    }
}

// ====================================================================================================================
// True Color
// ====================================================================================================================

/// A "true," 24-bit RGB color.
///
/// # Examples
///
/// Rust code can create a new true color with either
/// [`TrueColor::new`] or [`From<&Color> as
/// TrueColor`](struct.TrueColor.html#impl-From%3C%26Color%3E-for-TrueColor).
///
#[doc = include_str!("style.html")]
/// ```
/// # use prettypretty::{Color,TrueColor};
/// let blue = Color::from_24bit(0xae, 0xe8, 0xfb);
/// let blue_too = TrueColor::new(0xae, 0xe8, 0xfb);
/// assert_eq!(TrueColor::from(&blue), blue_too);
/// ```
/// <div class=color-swatch>
/// <div style="background-color: #aee8fb;"></div>
/// </div>
/// <br>
///
/// It can access the coordinates with [`AsRef<[u8; 3]> as
/// TrueColor`](struct.TrueColor.html#impl-AsRef%3C%5Bu8;+3%5D%3E-for-TrueColor)
/// or with [`Index<usize> as
/// TrueColor`](struct.TrueColor.html#impl-Index%3Cusize%3E-for-TrueColor).
/// ```
/// # use prettypretty::TrueColor;
/// let sea_foam = TrueColor::new(0xb6, 0xeb, 0xd4);
/// assert_eq!(sea_foam.as_ref(), &[182_u8, 235, 212]);
/// assert_eq!(sea_foam[1], 235);
/// ```
/// <div class=color-swatch>
/// <div style="background-color: #b6ebd4;"></div>
/// </div>
/// <br>
///
/// Finally, it can convert a true color to a high-resolution color with
/// [`From<TrueColor> as
/// Color`](struct.TrueColor.html#impl-From%3CTrueColor%3E-for-Color) or format
/// it in hashed hexadecimal notation with [`Display as
/// TrueColor`](struct.TrueColor.html#impl-Display-for-TrueColor).
/// ```
/// # use prettypretty::{Color, TrueColor};
/// let sand = TrueColor::new(0xee, 0xdc, 0xad);
/// assert_eq!(Color::from(sand), Color::from_24bit(0xee, 0xdc, 0xad));
/// assert_eq!(format!("{}", sand), "#eedcad");
/// ```
/// <div class=color-swatch>
/// <div style="background-color: #eedcad;"></div>
/// </div>
/// <br>
///
#[cfg_attr(
    feature = "pyffi",
    doc = "Since there is no Python feature equivalent to trait implementations in
    Rust, the Python class for `TrueColor` provides equivalent functionality
    through [`TrueColor::from_color`], [`True Color::to_8bit`], [`TrueColor::to_color`],
    [`TrueColor::coordinates`], [`TrueColor::__len__`], and [`TrueColor::__getitem__`].
    These methods are not available in Rust."
)]
#[cfg_attr(feature = "pyffi", pyclass(eq, frozen, hash, sequence))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TrueColor([u8; 3]);

#[cfg(feature = "pyffi")]
#[pymethods]
impl TrueColor {
    /// Create a new true color from its coordinates.
    #[new]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b])
    }

    /// Create a new true color from the given high-resolution color. <span
    /// class=python-only></span>
    ///
    /// This method converts the given color to an in-gamut sRGB color and then
    /// converts each coordinate to a `u8`. changes the components to
    #[staticmethod]
    pub fn from_color(color: &Color) -> Self {
        let [r, g, b] = color.to_24bit();
        Self::new(r, g, b)
    }

    /// Convert this true color to a high-resolution color. <span
    /// class=python-only></span>
    ///
    /// This method offers the same functionality as [`From<TrueColor> as
    /// Color`](struct.TrueColor.html#impl-From%3CTrueColor%3E-for-Color) and is
    /// available in Python only.
    pub fn to_color(&self) -> Color {
        Color::from(*self)
    }

    /// Access this true color's coordinates. <span class=python-only></span>
    pub fn coordinates(&self) -> [u8; 3] {
        self.0
    }

    /// Get this true color's length, which is 3. <span
    /// class=python-only></span>
    ///
    /// This method improves integration with Python's runtime and hence is
    /// available in Python only.
    pub fn __len__(&self) -> usize {
        3
    }

    /// Get the coordinate at the given index. <span class=python-only></span>
    ///
    /// This method improves integration with Python's runtime and hence is
    /// available in Python only.
    pub fn __getitem__(&self, index: isize) -> PyResult<u8> {
        match index {
            -3..=-1 => Ok(self.0[(3 + index) as usize]),
            0..=2 => Ok(self.0[index as usize]),
            _ => Err(pyo3::exceptions::PyIndexError::new_err(
                "Invalid coordinate index",
            )),
        }
    }

    /// Convert this true color to its debug representation. <span
    /// class=python-only></span>
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    /// Convert this true color to hashed hexadecimal notation. <span
    /// class=python-only></span>
    pub fn __str__(&self) -> String {
        format!("{}", self)
    }
}

#[cfg(not(feature = "pyffi"))]
impl TrueColor {
    /// Create a new true color from its coordinates.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b])
    }
}

impl AsRef<[u8; 3]> for TrueColor {
    /// Access this color's coordinates by reference.
    fn as_ref(&self) -> &[u8; 3] {
        &self.0
    }
}

impl std::ops::Index<usize> for TrueColor {
    type Output = u8;

    /// Access the coordinate with the given index.
    ///
    /// # Panics
    ///
    /// This method panics if `index > 2`.
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl From<&Color> for TrueColor {
    /// Convert the given color to a true color.
    ///
    /// This method first converts the color to gamut-mapped sRGB and then
    /// converts each coordinate to `u8`
    fn from(value: &Color) -> Self {
        let [r, g, b] = value.to(ColorSpace::Srgb).to_gamut().to_24bit();
        Self::new(r, g, b)
    }
}

impl From<TrueColor> for Color {
    /// Instantiate a high-resolution color from a true color value.
    fn from(value: TrueColor) -> Self {
        Self::from_24bit(value.0[0], value.0[1], value.0[2])
    }
}

impl core::fmt::Display for TrueColor {
    /// Format this true color in hashed hexadecimal notation.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [r, g, b] = *self.as_ref();
        write!(f, "#{:02x}{:02x}{:02x}", r, g, b)
    }
}

// ====================================================================================================================
// Terminal Color
// ====================================================================================================================

/// A terminal color.
///
/// ANSI escape codes distinguish between four kinds of color:
///
///  1. The default foreground and background colors
///  2. [`AnsiColor`], the 16 extended ANSI colors
///  3. 8-bit indexed colors, which comprise [`AnsiColor`], [`EmbeddedRgb`], and
///     [`GrayGradient`]
///  4. [`TrueColor`], i.e., 24-bit RGB
///
/// This enumeration captures the four kinds by offering variants that wrap the
/// types for ANSI, embedded RGB, gray gradient, and 24-bit colors while also
/// adding a variant for the default colors.
///
/// There is no wrapper type for 8-bit colors. While one existed in an early
/// version of this crate, it did not offer any useful functionality besides
/// possibly avoiding an `unwrap()` and hence was removed again.
///
/// Variants other than the one for the default colors wrap more specific
/// colors. They depart from common Rust practice of using tuple variants for
/// wrapper types and instead are implemented as struct variants with a single
/// `color` field. That does add some, small notational overhead in Rust. But it
/// also results in a cleaner and more approachable interface in Python, hence
/// their use here. The variants for embedded RGB and 24-bit RGB colors derive
/// their names from the number of levels per channel.
#[doc = include_str!("style.html")]
#[cfg_attr(feature = "pyffi", pyclass(eq, frozen, hash))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TerminalColor {
    Default(),
    Ansi { color: AnsiColor },
    Rgb6 { color: EmbeddedRgb },
    Gray { color: GrayGradient },
    Rgb256 { color: TrueColor },
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl TerminalColor {
    /// Convert the high-resolution color to a terminal color. <span
    /// class=python-only></span>
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn from_color(color: &Color) -> Self {
        Self::from(color)
    }

    /// Convert the 8-bit index to a terminal color. <span
    /// class=python-only></span>
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn from_8bit(color: u8) -> Self {
        Self::from(color)
    }

    /// Instantiate a new terminal color from the 24-bit RGB coordinates.
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn from_24bit(r: u8, g: u8, b: u8) -> Self {
        Self::Rgb256 {
            color: TrueColor::new(r, g, b),
        }
    }

    /// Convert this terminal color to an 8-bit index color.
    #[cfg(feature = "pyffi")]
    pub fn to_8bit(&self) -> PyResult<u8> {
        u8::try_from(*self)
            .map_err(|_| pyo3::exceptions::PyValueError::new_err("unable to convert to 8-bit index"))
    }

    /// Determine whether this terminal color is the default color.
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Default())
    }

    /// Get the SGR parameters for this terminal color.
    ///
    /// This method determines the SGR parameters for setting the given layer,
    /// i.e., foreground or background, to this terminal color. It returns 1, 3,
    /// or 5 parameters that may be combined with other SGR parameters into one
    /// escape sequence, as long as they are properly separated by semicolons.
    pub fn sgr_parameters(&self, layer: Layer) -> Vec<u8> {
        match self {
            TerminalColor::Default() => vec![30 + layer.offset()],
            TerminalColor::Ansi { color: c } => {
                let base = if c.is_bright() { 90 } else { 30 } + layer.offset();
                vec![base + c.nonbright() as u8]
            }
            TerminalColor::Rgb6 { color: c } => {
                vec![38 + layer.offset(), 5, u8::from(*c)]
            }
            TerminalColor::Gray { color: c } => {
                vec![38 + layer.offset(), 5, u8::from(*c)]
            }
            TerminalColor::Rgb256 { color: c } => {
                vec![38 + layer.offset(), 2, c[0], c[1], c[2]]
            }
        }
    }

    /// Convert to a debug representation. <span class=python-only></span>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[cfg(not(feature = "pyffi"))]
impl TerminalColor {
    /// Instantiate a new terminal color from the 24-bit RGB coordinates.
    pub fn from_24bit(r: impl Into<u8>, g: impl Into<u8>, b: impl Into<u8>) -> Self {
        Self::Rgb256 {
            color: TrueColor::new(r.into(), g.into(), b.into()),
        }
    }
}

impl From<u8> for TerminalColor {
    /// Convert 8-bit index to a terminal color.
    ///
    /// Depending on the 8-bit number, this method returns either a wrapped
    /// ANSI, embedded RGB, or gray gradient color.
    fn from(value: u8) -> Self {
        if (0..=15).contains(&value) {
            Self::Ansi {
                color: AnsiColor::try_from(value).unwrap(),
            }
        } else if (16..=231).contains(&value) {
            Self::Rgb6 {
                color: EmbeddedRgb::try_from(value).unwrap(),
            }
        } else {
            Self::Gray {
                color: GrayGradient::try_from(value).unwrap(),
            }
        }
    }
}

impl From<&Color> for TerminalColor {
    /// Convert a high-resolution color to a terminal color.
    ///
    /// This method first converts the color to gamut-mapped sRGB and then
    /// converts each coordinate to `u8` before returning a wrapped
    /// [`TrueColor`].
    fn from(value: &Color) -> Self {
        Self::Rgb256 {
            color: TrueColor::from(value),
        }
    }
}

impl TryFrom<TerminalColor> for u8 {
    type Error = TerminalColor;

    /// Try to convert this terminal color to an 8-bit index.
    ///
    /// For ANSI, embedded RGB, and gray gradient colors, this method unwraps
    /// the color and converts it to an 8-bit index. It returns any other
    /// terminal color as the error value.
    fn try_from(value: TerminalColor) -> Result<Self, Self::Error> {
        match value {
            TerminalColor::Default() => Err(value),
            TerminalColor::Ansi { color: c } => Ok(u8::from(c)),
            TerminalColor::Rgb6 { color: c } => Ok(u8::from(c)),
            TerminalColor::Gray { color: c } => Ok(u8::from(c)),
            TerminalColor::Rgb256 { .. } => Err(value),
        }
    }
}

// ====================================================================================================================
// Layer and Fidelity

/// The layer for rendering to the terminal.
#[doc = include_str!("style.html")]
#[cfg_attr(feature = "pyffi", pyclass(eq, eq_int, frozen, hash))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Layer {
    /// The foreground or text layer.
    Foreground = 0,
    /// The background layer.
    Background = 10,
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Layer {
    /// Determine the offset for this layer.
    ///
    /// The offset is added to CSI parameter values for foreground colors.
    #[inline]
    pub fn offset(&self) -> u8 {
        *self as u8
    }

    /// Return a humane description for this layer. <span
    /// class=python-only></span>
    #[cfg(feature = "pyffi")]
    pub fn __str__(&self) -> String {
        format!("{}", self)
    }
}

impl std::fmt::Display for Layer {
    /// Format this layer name.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Foreground => f.write_str("foreground"),
            Self::Background => f.write_str("background"),
        }
    }
}

/// The stylistic fidelity of terminal output.
///
/// This enumeration captures levels of stylistic fidelity. It can describe the
/// capabilities of a terminal or runtime environment (such as CI) as well as
/// the preferences of a user (notably, `NoColor`).
///
#[doc = include_str!("style.html")]
#[cfg_attr(feature = "pyffi", pyclass(eq, eq_int, frozen, hash, ord))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Fidelity {
    /// Plain text, no ANSI escape codes
    Plain,
    /// ANSI escape codes but no colors
    NoColor,
    /// ANSI and default colors only
    Ansi,
    /// 8-bit indexed colors including ANSI and default colors
    EightBit,
    /// Full fidelity including 24-bit RGB color.
    Full,
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Fidelity {
    /// Determine the fidelity required for rendering the given terminal color.
    /// <span class=python-only></span>
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn from_color(color: TerminalColor) -> Self {
        color.into()
    }

    /// Determine whether this fidelity level suffices for rendering the
    /// terminal color.
    pub fn covers(&self, color: TerminalColor) -> bool {
        Fidelity::from(color) <= *self
    }

    /// Return a humane description for this fidelity. <span
    /// class=python-only></span>
    #[cfg(feature = "pyffi")]
    pub fn __str__(&self) -> String {
        format!("{}", self)
    }
}

impl From<TerminalColor> for Fidelity {
    /// Determine the necessary fidelity level for the given terminal color.
    fn from(value: TerminalColor) -> Self {
        match value {
            TerminalColor::Default() | TerminalColor::Ansi { .. } => Self::Ansi,
            TerminalColor::Rgb6 { .. } | TerminalColor::Gray { .. } => Self::EightBit,
            TerminalColor::Rgb256 { .. } => Self::Full,
        }
    }
}

impl std::fmt::Display for Fidelity {
    /// Format a humane description for this fidelity.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Plain => "plain text",
            Self::NoColor => "no colors",
            Self::Ansi => "ANSI colors",
            Self::EightBit => "8-bit colors",
            Self::Full => "24-bit colors",
        };

        f.write_str(s)
    }
}

// ====================================================================================================================

#[cfg(test)]
mod test {
    use super::{AnsiColor, EmbeddedRgb, GrayGradient, OutOfBoundsError, TerminalColor};

    #[test]
    fn test_conversion() -> Result<(), OutOfBoundsError> {
        let magenta = AnsiColor::Magenta;
        assert_eq!(magenta as u8, 5);

        let green = EmbeddedRgb::new(0, 4, 0)?;
        assert_eq!(green.as_ref(), &[0, 4, 0]);

        let gray = GrayGradient::new(12)?;
        assert_eq!(gray.level(), 12);

        let also_magenta = TerminalColor::Ansi {
            color: AnsiColor::Magenta,
        };
        let also_green = TerminalColor::Rgb6 { color: green };
        let also_gray = TerminalColor::Gray { color: gray };

        assert_eq!(also_magenta, TerminalColor::from(5));
        assert_eq!(also_green, TerminalColor::from(40));
        assert_eq!(also_gray, TerminalColor::from(244));

        Ok(())
    }

    #[test]
    fn test_limits() -> Result<(), OutOfBoundsError> {
        let black_ansi = AnsiColor::try_from(0)?;
        assert_eq!(black_ansi, AnsiColor::Black);
        assert_eq!(u8::from(black_ansi), 0);
        let white_ansi = AnsiColor::try_from(15)?;
        assert_eq!(white_ansi, AnsiColor::BrightWhite);
        assert_eq!(u8::from(white_ansi), 15);

        let black_rgb = EmbeddedRgb::try_from(16)?;
        assert_eq!(*black_rgb.as_ref(), [0_u8, 0_u8, 0_u8]);
        assert_eq!(u8::from(black_rgb), 16);
        let white_rgb = EmbeddedRgb::try_from(231)?;
        assert_eq!(*white_rgb.as_ref(), [5_u8, 5_u8, 5_u8]);
        assert_eq!(u8::from(white_rgb), 231);

        let black_gray = GrayGradient::try_from(232)?;
        assert_eq!(black_gray.level(), 0);
        assert_eq!(u8::from(black_gray), 232);
        let white_gray = GrayGradient::try_from(255)?;
        assert_eq!(white_gray.level(), 23);
        assert_eq!(u8::from(white_gray), 255);

        Ok(())
    }
}
