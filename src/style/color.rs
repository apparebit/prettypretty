#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use super::Layer;
use crate::error::OutOfBoundsError;
use crate::{Color, ColorSpace};

// ====================================================================================================================
// Default Color
// ====================================================================================================================

/// The default foreground and background colors.
///
/// The default colors are ordered because they are ordered as theme colors.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, eq_int, frozen, hash, ord, module = "prettypretty.color.style")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DefaultColor {
    Foreground,
    Background,
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl DefaultColor {
    /// Get the default color's human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Foreground => "default foreground color",
            Self::Background => "default background color",
        }
    }
}

impl TryFrom<usize> for DefaultColor {
    type Error = OutOfBoundsError;

    fn try_from(value: usize) -> Result<Self, OutOfBoundsError> {
        match value {
            0 => Ok(DefaultColor::Foreground),
            1 => Ok(DefaultColor::Background),
            _ => Err(OutOfBoundsError::new(value, 0..=1)),
        }
    }
}

// ====================================================================================================================
// Ansi Color
// ====================================================================================================================

/// The 16 extended ANSI colors.
///
/// Rust code converts between 8-bit color codes and enumeration variants with
/// [`AnsiColor as
/// TryFrom<u8>`](enum.AnsiColor.html#impl-TryFrom%3Cu8%3E-for-AnsiColor) and
/// [`u8 as
/// From<AnsiColor>`](enum.AnsiColor.html#impl-From%3CAnsiColor%3E-for-u8).
#[cfg_attr(
    feature = "pyffi",
    doc = "In contrast, Python code uses the [`AnsiColor::try_from_8bit`] and
    [`AnsiColor::to_8bit`] methods."
)]
/// Since ANSI colors have no intrinsic color values, conversion from/to
/// high-resolution colors requires additional machinery, as provided by
/// [`Translator`](crate::trans::Translator).
///
/// The ANSI colors are ordered because they are ordered as theme colors and as
/// indexed colors.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, eq_int, frozen, hash, ord, module = "prettypretty.color.style")
)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AnsiColor {
    #[default]
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
    /// Instantiate an ANSI color from its 8-bit code. <i
    /// class=python-only>Python only!</i>
    ///
    /// This method offers the same functionality as [`AnsiColor as
    /// TryFrom<u8>`](enum.AnsiColor.html#impl-TryFrom%3Cu8%3E-for-AnsiColor)
    /// and is available in Python only.
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn try_from_8bit(value: u8) -> Result<Self, OutOfBoundsError> {
        Self::try_from(value)
    }

    /// Get the 8-bit code for this ANSI color. <i class=python-only>Python
    /// only!</i>
    ///
    /// This method offers the same functionality as [`u8 as
    /// From<AnsiColor>`](enum.AnsiColor.html#impl-From%3CAnsiColor%3E-for-u8)
    /// and is available in Python only.
    #[cfg(feature = "pyffi")]
    pub fn to_8bit(&self) -> u8 {
        *self as u8
    }

    /// Determine whether this ANSI color is achromatic.
    pub fn is_achromatic(&self) -> bool {
        use AnsiColor::*;
        matches!(self, Black | White | BrightBlack | BrightWhite)
    }

    /// Determine whether this ANSI color is bright.
    pub fn is_bright(&self) -> bool {
        8 <= *self as u8
    }

    /// Get the corresponding 3-bit ANSI color.
    ///
    /// If this color is bright, this method returns the corresponding nonbright
    /// color. Otherwise, it returns the color.
    pub fn to_3bit(&self) -> AnsiColor {
        let mut index = *self as u8;
        if 8 <= index {
            index -= 8;
        }
        AnsiColor::try_from(index).unwrap()
    }

    /// Get this ANSI color's name.
    ///
    /// This method returns the human-readable name, e.g., `"bright green"` for
    /// [`AnsiColor::BrightGreen`].
    pub fn name(&self) -> &'static str {
        use AnsiColor::*;

        match self {
            Black => "black",
            Red => "red",
            Green => "green",
            Yellow => "yellow",
            Blue => "blue",
            Magenta => "magenta",
            Cyan => "cyan",
            White => "white",
            BrightBlack => "bright black",
            BrightRed => "bright red",
            BrightGreen => "bright green",
            BrightYellow => "bright yellow",
            BrightBlue => "bright blue",
            BrightMagenta => "bright magenta",
            BrightCyan => "bright cyan",
            BrightWhite => "bright white",
        }
    }
}

impl TryFrom<u8> for AnsiColor {
    type Error = OutOfBoundsError;

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
/// [`EmbeddedRgb::new`] or [`EmbeddedRgb as
/// TryFrom<u8>`](struct.EmbeddedRgb.html#impl-TryFrom%3Cu8%3E-for-EmbeddedRgb).
///
/// ```
/// # use prettypretty::style::EmbeddedRgb;
/// # use prettypretty::error::OutOfBoundsError;
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
/// It can access the coordinates with [`EmbeddedRgb as AsRef<[u8;
/// 3]>`](struct.EmbeddedRgb.html#impl-AsRef%3C%5Bu8;+3%5D%3E-for-EmbeddedRgb)
/// or with [`EmbeddedRgb as
/// Index<usize>`](struct.EmbeddedRgb.html#impl-Index%3Cusize%3E-for-EmbeddedRgb).
/// ```
/// # use prettypretty::style::EmbeddedRgb;
/// # use prettypretty::error::OutOfBoundsError;
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
/// Finally, it can convert an embedded RGB color to `u8` with [`u8 as
/// From<EmbeddedRgb>`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-u8),
/// to a true color with [`TrueColor as
/// From<EmbeddedRgb>`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-TrueColor),
/// or to a high-resolution color with [`Color as
/// From<EmbeddedRgb>`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-Color).
/// ```
/// # use prettypretty::Color;
/// # use prettypretty::style::{EmbeddedRgb, TrueColor};
/// # use prettypretty::error::OutOfBoundsError;
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
    through [`EmbeddedRgb::try_from_8bit`], [`EmbeddedRgb::to_8bit`],
    [`EmbeddedRgb::to_24bit`], [`EmbeddedRgb::to_color`], [`EmbeddedRgb::coordinates`],
    [`EmbeddedRgb::__len__`], [`EmbeddedRgb::__getitem__`], and
    [`EmbeddedRgb::__repr__`]. These methods are not available in Rust."
)]
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, sequence, module = "prettypretty.color.style")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EmbeddedRgb([u8; 3]);

#[cfg(feature = "pyffi")]
#[pymethods]
impl EmbeddedRgb {
    /// Create a new embedded RGB value from its coordinates.
    #[new]
    pub fn new(r: u8, g: u8, b: u8) -> Result<Self, OutOfBoundsError> {
        if 6 <= r {
            Err(OutOfBoundsError::new(r, 0..=5))
        } else if 6 <= g {
            Err(OutOfBoundsError::new(g, 0..=5))
        } else if 6 <= b {
            Err(OutOfBoundsError::new(b, 0..=5))
        } else {
            Ok(Self([r, g, b]))
        }
    }

    /// Instantiate an embedded RGB color from its 8-bit code. <i
    /// class=python-only>Python only!</i>
    ////
    /// This method offers the same functionality as [`EmbeddedRgb as
    /// TryFrom<u8>`](struct.EmbeddedRgb.html#impl-TryFrom%3Cu8%3E-for-EmbeddedRgb)
    /// and is available in Python only.
    #[staticmethod]
    #[inline]
    pub fn try_from_8bit(value: u8) -> Result<Self, OutOfBoundsError> {
        Self::try_from(value)
    }

    /// Get the 8-bit code for this embedded RGB color. <i
    /// class=python-only>Python only!</i>
    ///
    /// This method offers the same functionality as [`u8 as
    /// From<EmbeddedRgb>`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-u8)
    /// and is available in Python only.
    #[inline]
    pub fn to_8bit(&self) -> u8 {
        u8::from(*self)
    }

    /// Convert this embedded RGB color to 24-bit. <i class=python-only>Python
    /// only!</i>
    #[inline]
    pub fn to_24bit(&self) -> [u8; 3] {
        (*self).into()
    }

    /// Convert this embedded RGB color to a high-resolution color. <i
    /// class=python-only>Python only!</i>
    ///
    /// This method offers the same functionality as [`Color as
    /// From<EmbeddedRgb>`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-Color)
    /// and is available in Python only.
    #[inline]
    pub fn to_color(&self) -> Color {
        Color::from(*self)
    }

    /// Access this true color's coordinates. <i class=python-only>Python
    /// only!</i>
    #[inline]
    pub fn coordinates(&self) -> [u8; 3] {
        self.0
    }

    /// Get this embedded RGB color's length, which is 3. <i
    /// class=python-only>Python only!</i>
    ///
    /// This method improves integration with Python's runtime and hence is
    /// available in Python only.
    #[inline]
    pub fn __len__(&self) -> usize {
        3
    }

    /// Get the coordinate at the given index. <i class=python-only>Python
    /// only!</i>
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

    /// Convert this embedded RGB color to its debug representation. <i
    /// class=python-only>Python only!</i>
    #[inline]
    pub fn __repr__(&self) -> String {
        format!("EmbeddedRgb({}, {}, {})", self.0[0], self.0[1], self.0[2])
    }
}

#[cfg(not(feature = "pyffi"))]
impl EmbeddedRgb {
    /// Create a new embedded RGB value from its coordinates.
    pub fn new(r: u8, g: u8, b: u8) -> Result<Self, OutOfBoundsError> {
        if 6 <= r {
            Err(OutOfBoundsError::new(r, 0..=5))
        } else if 6 <= g {
            Err(OutOfBoundsError::new(g, 0..=5))
        } else if 6 <= b {
            Err(OutOfBoundsError::new(b, 0..=5))
        } else {
            Ok(Self([r, g, b]))
        }
    }
}

impl TryFrom<u8> for EmbeddedRgb {
    type Error = OutOfBoundsError;

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
    /// This method panics if `2 < index`.
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl From<EmbeddedRgb> for u8 {
    fn from(value: EmbeddedRgb) -> u8 {
        let [r, g, b] = value.0;
        16 + 36 * r + 6 * g + b
    }
}

impl From<EmbeddedRgb> for [u8; 3] {
    fn from(value: EmbeddedRgb) -> Self {
        fn convert(value: u8) -> u8 {
            if value == 0 {
                0
            } else {
                55 + 40 * value
            }
        }

        let [r, g, b] = *value.as_ref();
        [convert(r), convert(g), convert(b)]
    }
}

impl From<EmbeddedRgb> for Color {
    fn from(value: EmbeddedRgb) -> Self {
        TrueColor::from(value).into()
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
/// [`GrayGradient::new`] or [`GrayGradient as
/// TryFrom<u8>`](struct.GrayGradient.html#impl-TryFrom%3Cu8%3E-for-GrayGradient).
///
/// ```
/// # use prettypretty::style::GrayGradient;
/// # use prettypretty::error::OutOfBoundsError;
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
/// # use prettypretty::style::GrayGradient;
/// # use prettypretty::error::OutOfBoundsError;
/// let midgray = GrayGradient::try_from(243)?;
/// assert_eq!(midgray.level(), 11);
/// # Ok::<(), OutOfBoundsError>(())
/// ```
/// <div class=color-swatch>
/// <div style="background-color: #767676;"></div>
/// </div>
/// <br>
///
/// Finally, it can convert a gray gradient color to `u8` with [`u8 as
/// From<GrayGradient>`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-u8),
/// to a true color with [`TrueColor as
/// From<GrayGradient>`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-TrueColor),
/// or to a high-resolution color with [`Color as
/// From<GrayGradient>`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-Color).
/// ```
/// # use prettypretty::Color;
/// # use prettypretty::style::{GrayGradient, TrueColor};
/// # use prettypretty::error::OutOfBoundsError;
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
    through [`GrayGradient::try_from_8bit`], [`GrayGradient::__repr__`],
    [`GrayGradient::to_8bit`], and [`GrayGradient::to_color`]. These methods are not
    available in Rust."
)]
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, ord, module = "prettypretty.color.style")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GrayGradient(u8);

#[cfg(feature = "pyffi")]
#[pymethods]
impl GrayGradient {
    /// Instantiate a new gray gradient from its level `0..=23`.
    #[new]
    pub fn new(value: u8) -> Result<Self, OutOfBoundsError> {
        if 24 <= value {
            Err(OutOfBoundsError::new(value, 0..=23))
        } else {
            Ok(Self(value))
        }
    }

    /// Instantiate a gray gradient from its 8-bit code. <i
    /// class=python-only>Python only!</i>
    ///
    /// This method offers the same functionality as [`GrayGradient as
    /// TryFrom<u8>`](struct.GrayGradient.html#impl-TryFrom%3Cu8%3E-for-GrayGradient)
    /// and is available in Python only.
    #[staticmethod]
    #[inline]
    pub fn try_from_8bit(value: u8) -> Result<Self, OutOfBoundsError> {
        Self::try_from(value)
    }

    /// Get the 8-bit code for this gray gradient color. <i
    /// class=python-only>Python only!</i>
    ///
    /// This method offers the same functionality as [`u8 as
    /// From<GrayGradient>`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-u8)
    /// and is available in Python only.
    #[inline]
    pub fn to_8bit(&self) -> u8 {
        u8::from(*self)
    }

    /// Convert this gray gradient color to 24-bit. <i class=python-only>Python
    /// only!</i>
    #[inline]
    pub fn to_24bit(&self) -> [u8; 3] {
        (*self).into()
    }

    /// Convert this gray gradient to a high-resolution color. <i
    /// class=python-only>Python only!</i>
    ///
    /// This method offers the same functionality as [`Color as
    /// From<GrayGradient>`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-Color)
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

    /// Convert this gray gradient to its debug representation. <i
    /// class=python-only>Python only!</i>
    #[inline]
    pub fn __repr__(&self) -> String {
        format!("GrayGradient({})", self.0)
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

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= 231 {
            Err(OutOfBoundsError::new(value, 232..=255))
        } else {
            Self::new(value - 232)
        }
    }
}

impl From<GrayGradient> for u8 {
    fn from(value: GrayGradient) -> u8 {
        232 + value.0
    }
}

impl From<GrayGradient> for [u8; 3] {
    fn from(value: GrayGradient) -> Self {
        let level = 8 + 10 * value.level();
        [level, level, level]
    }
}

impl From<GrayGradient> for Color {
    fn from(value: GrayGradient) -> Self {
        TrueColor::from(value).into()
    }
}

// ====================================================================================================================
// True Color
// ====================================================================================================================

/// A "true," 24-bit RGB color.
///
/// # Examples
///
/// Rust code can create a new true color with either [`TrueColor::new`] or
/// [`TrueColor as
/// From<&Color>`](struct.TrueColor.html#impl-From%3C%26Color%3E-for-TrueColor).
///
/// ```
/// # use prettypretty::Color;
/// # use prettypretty::style::TrueColor;
/// let blue = Color::from_24bit(0xae, 0xe8, 0xfb);
/// let blue_too = TrueColor::new(0xae, 0xe8, 0xfb);
/// assert_eq!(TrueColor::from(&blue), blue_too);
/// ```
/// <div class=color-swatch>
/// <div style="background-color: #aee8fb;"></div>
/// </div>
/// <br>
///
/// It can access the coordinates with [`TrueColor as AsRef<[u8;
/// 3]>`](struct.TrueColor.html#impl-AsRef%3C%5Bu8;+3%5D%3E-for-TrueColor) or
/// with [`TrueColor as
/// Index<usize>`](struct.TrueColor.html#impl-Index%3Cusize%3E-for-TrueColor).
/// ```
/// # use prettypretty::style::TrueColor;
/// let sea_foam = TrueColor::new(0xb6, 0xeb, 0xd4);
/// assert_eq!(sea_foam.as_ref(), &[182_u8, 235, 212]);
/// assert_eq!(sea_foam[1], 235);
/// ```
/// <div class=color-swatch>
/// <div style="background-color: #b6ebd4;"></div>
/// </div>
/// <br>
///
/// Finally, it can convert a true color to a high-resolution color with [`Color
/// as
/// From<TrueColor>`](struct.TrueColor.html#impl-From%3CTrueColor%3E-for-Color)
/// or format it in hashed hexadecimal notation with [`TrueColor as
/// Display`](struct.TrueColor.html#impl-Display-for-TrueColor).
/// ```
/// # use prettypretty::Color;
/// # use prettypretty::style::TrueColor;
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
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, sequence, module = "prettypretty.color.style")
)]
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

    /// Create a new true color from the given high-resolution color. <i
    /// class=python-only>Python only!</i>
    ///
    /// This method invokes true color's constructor after converting the given
    /// color to an in-gamut sRGB color and then converting each coordinate to a
    /// `u8`.
    #[staticmethod]
    pub fn from_color(color: &Color) -> Self {
        let [r, g, b] = color.to_24bit();
        Self::new(r, g, b)
    }

    /// Convert this true color to a high-resolution color. <i
    /// class=python-only>Python only!</i>
    ///
    /// This method offers the same functionality as [`Color as
    /// From<TrueColor>`](struct.TrueColor.html#impl-From%3CTrueColor%3E-for-Color)
    /// and is available in Python only.
    pub fn to_color(&self) -> Color {
        Color::from(*self)
    }

    /// Access this true color's coordinates. <i class=python-only>Python
    /// only!</i>
    pub fn coordinates(&self) -> [u8; 3] {
        self.0
    }

    /// Get this true color's length, which is 3. <i class=python-only>Python
    /// only!</i>
    ///
    /// This method improves integration with Python's runtime and hence is
    /// available in Python only.
    pub fn __len__(&self) -> usize {
        3
    }

    /// Get the coordinate at the given index. <i class=python-only>Python
    /// only!</i>
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

    /// Calculate the weighted Euclidian distance between the two colors.
    ///
    /// This method reimplements the distance metric used by the [anstyle
    /// crate](https://github.com/rust-cli/anstyle/blob/main/crates/anstyle-lossy/src/lib.rs).
    pub fn weighted_euclidian_distance(&self, other: &TrueColor) -> u32 {
        self.do_weighted_euclidian_distance(other)
    }

    /// Convert this true color to its debug representation. <i
    /// class=python-only>Python only!</i>
    pub fn __repr__(&self) -> String {
        format!("TrueColor({}, {}, {})", self.0[0], self.0[1], self.0[2])
    }

    /// Convert this true color to hashed hexadecimal notation. <i
    /// class=python-only>Python only!</i>
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

    /// Calculate the weighted Euclidian distance between the two colors.
    ///
    /// This method reimplements the distance metric used by the [anstyle
    /// crate](https://github.com/rust-cli/anstyle/blob/main/crates/anstyle-lossy/src/lib.rs).
    pub fn weighted_euclidian_distance(&self, other: &TrueColor) -> u32 {
        self.do_weighted_euclidian_distance(other)
    }
}

impl TrueColor {
    fn do_weighted_euclidian_distance(&self, other: &TrueColor) -> u32 {
        let r1 = self.0[0] as i32;
        let g1 = self.0[1] as i32;
        let b1 = self.0[2] as i32;
        let r2 = other.0[0] as i32;
        let g2 = other.0[1] as i32;
        let b2 = other.0[2] as i32;

        let r_sum = r1 + r2;
        let r_delta = r1 - r2;
        let g_delta = g1 - g2;
        let b_delta = b1 - b2;

        let r = (2 * 512 + r_sum) * r_delta * r_delta;
        let g = 4 * g_delta * g_delta * (1 << 8);
        let b = (2 * 767 - r_sum) * b_delta * b_delta;

        (r + g + b) as u32
    }
}

impl AsRef<[u8; 3]> for TrueColor {
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
    /// This method panics if `2 < index`.
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl From<EmbeddedRgb> for TrueColor {
    fn from(value: EmbeddedRgb) -> Self {
        let [r, g, b] = value.into();
        TrueColor::new(r, g, b)
    }
}

impl From<GrayGradient> for TrueColor {
    fn from(value: GrayGradient) -> TrueColor {
        let [r, g, b] = value.into();
        TrueColor::new(r, g, b)
    }
}

impl From<[u8; 3]> for TrueColor {
    fn from(value: [u8; 3]) -> Self {
        TrueColor::new(value[0], value[1], value[2])
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
    fn from(value: TrueColor) -> Self {
        Self::from_24bit(value.0[0], value.0[1], value.0[2])
    }
}

impl core::fmt::Display for TrueColor {
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
/// This enumeration unifies all five terminal color types, [`DefaultColor`],
/// [`AnsiColor`], [`EmbeddedRgb`], [`GrayGradient`], and [`TrueColor`]. It does
/// not distinguish between ANSI colors as themselves and as 8-bit colors. An
/// early version of this crate included the corresponding wrapper type, but it
/// offered no distinct functionality and hence was removed again.
///
/// In a departure from common practice, variants are implemented as struct
/// variants with a single `color` field. This does result in slightly more
/// verbose Rust patterns, but it also makes the Python classes much easier to
/// use. The variants for the embedded RGB and 24-bit RGB colors derive their
/// names from the number of levels per channel.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TerminalColor {
    Default { color: DefaultColor },
    Ansi { color: AnsiColor },
    Embedded { color: EmbeddedRgb },
    Gray { color: GrayGradient },
    Bits24 { color: TrueColor },
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl TerminalColor {
    /// The default foreground color.
    pub const FOREGROUND: TerminalColor = TerminalColor::Default {
        color: DefaultColor::Foreground,
    };

    /// The default background color.
    pub const BACKGROUND: TerminalColor = TerminalColor::Default {
        color: DefaultColor::Background,
    };

    /// Convert the high-resolution color to a terminal color. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn from_color(color: &Color) -> Self {
        Self::from(color)
    }

    /// Convert the 8-bit index to a terminal color. <i class=python-only>Python
    /// only!</i>
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn from_8bit(color: u8) -> Self {
        Self::from(color)
    }

    /// Instantiate a new terminal color from the 24-bit RGB coordinates.
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn from_24bit(r: u8, g: u8, b: u8) -> Self {
        Self::Bits24 {
            color: TrueColor::new(r, g, b),
        }
    }

    /// Convert this terminal color to an 8-bit index color. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn try_to_8bit(&self) -> PyResult<u8> {
        u8::try_from(*self).map_err(|_| {
            pyo3::exceptions::PyValueError::new_err("unable to convert to 8-bit index")
        })
    }

    /// Convert this terminal color to 24-bit. <i class=python-only>Python
    /// only!</i>
    #[cfg(feature = "pyffi")]
    pub fn try_to_24bit(&self) -> PyResult<[u8; 3]> {
        <[u8; 3]>::try_from(*self).map_err(|_| {
            pyo3::exceptions::PyValueError::new_err("unable to convert to 24-bit coordinates")
        })
    }

    /// Determine whether this terminal color is the default color.
    #[inline]
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Default { .. })
    }

    /// Negate this terminal color.
    ///
    /// This method determines the terminal color for restoring the terminal
    /// layer's default appearance again. If this color is the default color for
    /// that layer, the appearance does not need to change and this method
    /// returns `None`. Otherwise, this method returns the default color for the
    /// layer.
    pub fn negate(&self, layer: Layer) -> Option<TerminalColor> {
        match (*self, layer) {
            (
                Self::Default {
                    color: DefaultColor::Foreground,
                },
                Layer::Foreground,
            ) => None,
            (
                Self::Default {
                    color: DefaultColor::Background,
                },
                Layer::Background,
            ) => None,
            (_, Layer::Foreground) => Some(Self::FOREGROUND),
            (_, Layer::Background) => Some(Self::BACKGROUND),
        }
    }

    /// Get the SGR parameters for this terminal color.
    ///
    /// This method determines the SGR parameters for setting the given layer,
    /// i.e., foreground or background, to this terminal color. It returns 1, 3,
    /// or 5 parameters that may be combined with other SGR parameters into one
    /// escape sequence, as long as they are properly separated by semicolons.
    ///
    /// # Panics
    ///
    /// This method panics if it is invoked on a default color with an
    /// inconsistent layer.
    pub fn sgr_parameters(&self, layer: Layer) -> Vec<u8> {
        match self {
            TerminalColor::Default { color: c } => {
                if *c as u8 != layer as u8 {
                    panic!("unable to use default color {:?} for layer {:?}", c, layer);
                }

                match c {
                    DefaultColor::Foreground => vec![39],
                    DefaultColor::Background => vec![49],
                }
            }
            TerminalColor::Ansi { color: c } => {
                let base = if c.is_bright() { 90 } else { 30 } + layer.offset();
                vec![base + c.to_3bit() as u8]
            }
            TerminalColor::Embedded { color: c } => {
                vec![38 + layer.offset(), 5, u8::from(*c)]
            }
            TerminalColor::Gray { color: c } => {
                vec![38 + layer.offset(), 5, u8::from(*c)]
            }
            TerminalColor::Bits24 { color: c } => {
                vec![38 + layer.offset(), 2, c[0], c[1], c[2]]
            }
        }
    }

    /// Convert to a debug representation. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        match self {
            TerminalColor::Default { color: c } => format!("TerminalColor.Default({:?})", c),
            TerminalColor::Ansi { color: c } => format!("TerminalColor.Ansi({:?})", c),
            TerminalColor::Embedded { color: c } => {
                format!("TerminalColor.Embedded({})", c.__repr__())
            }
            TerminalColor::Gray { color: c } => format!("TerminalColor.Gray({})", c.__repr__()),
            TerminalColor::Bits24 { color: c } => format!("TerminalColor.Bits24({})", c.__repr__()),
        }
    }
}

#[cfg(not(feature = "pyffi"))]
impl TerminalColor {
    /// Instantiate a new terminal color from the 24-bit RGB coordinates.
    pub fn from_24bit(r: impl Into<u8>, g: impl Into<u8>, b: impl Into<u8>) -> Self {
        Self::Bits24 {
            color: TrueColor::new(r.into(), g.into(), b.into()),
        }
    }
}

impl From<DefaultColor> for TerminalColor {
    fn from(color: DefaultColor) -> Self {
        TerminalColor::Default { color }
    }
}

impl From<AnsiColor> for TerminalColor {
    fn from(color: AnsiColor) -> Self {
        TerminalColor::Ansi { color }
    }
}

impl From<EmbeddedRgb> for TerminalColor {
    fn from(color: EmbeddedRgb) -> Self {
        TerminalColor::Embedded { color }
    }
}

impl From<GrayGradient> for TerminalColor {
    fn from(color: GrayGradient) -> Self {
        TerminalColor::Gray { color }
    }
}

impl From<TrueColor> for TerminalColor {
    fn from(color: TrueColor) -> Self {
        TerminalColor::Bits24 { color }
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
            Self::Embedded {
                color: EmbeddedRgb::try_from(value).unwrap(),
            }
        } else {
            Self::Gray {
                color: GrayGradient::try_from(value).unwrap(),
            }
        }
    }
}

impl From<[u8; 3]> for TerminalColor {
    fn from(value: [u8; 3]) -> Self {
        Self::Bits24 {
            color: TrueColor(value),
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
        Self::Bits24 {
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
            TerminalColor::Default { .. } => Err(value),
            TerminalColor::Ansi { color: c } => Ok(u8::from(c)),
            TerminalColor::Embedded { color: c } => Ok(u8::from(c)),
            TerminalColor::Gray { color: c } => Ok(u8::from(c)),
            TerminalColor::Bits24 { .. } => Err(value),
        }
    }
}

impl TryFrom<TerminalColor> for [u8; 3] {
    type Error = TerminalColor;

    fn try_from(value: TerminalColor) -> Result<Self, Self::Error> {
        match value {
            TerminalColor::Default { .. } => Err(value),
            TerminalColor::Ansi { .. } => Err(value),
            TerminalColor::Embedded { color } => Ok(color.into()),
            TerminalColor::Gray { color } => Ok(color.into()),
            TerminalColor::Bits24 { color } => Ok(*color.as_ref()),
        }
    }
}

impl TryFrom<TerminalColor> for Color {
    type Error = TerminalColor;

    fn try_from(value: TerminalColor) -> Result<Self, Self::Error> {
        let [r, g, b] = value.try_into()?;
        Ok(Color::from_24bit(r, g, b))
    }
}
