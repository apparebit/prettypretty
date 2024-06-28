//! # Terminal Color Formats
//!
//! This module provides the abstractions for terminal color formats. Unlike the
//! more general and precise color abstraction, this module is informed by the
//! many restrictions of terminals. One key consequence is that even colors with
//! three coordinates do not use floating point but integral numbers drawn from
//! a specific range.

#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use crate::{Color, Float, Theme};

/// An out-of-bounds error.
///
/// This error indicates an index value that is out of bounds for some subrange
/// of an unsigned byte. Typically, it results from trying to instantiate
/// [`AnsiColor`], [`EmbeddedRgb`], or [`GrayGradient`] from an index invalid
/// for that particular terminal color. Ranges include:
///
///   * `0..=5` for coordinates of [`EmbeddedRgb`];
///   * `0..=15` for index values of the 16 extended [`AnsiColor`]s;
///   * `0..=23` for the levels of the [`GrayGradient`];
///   * `16..=231` for index values of the [`EmbeddedRgb`];
///   * `232..=255` for index values of the [`GrayGradient`].
///
#[derive(Clone, Debug)]
pub struct OutOfBoundsError {
    pub value: usize,
    pub expected: std::ops::RangeInclusive<u8>,
}

impl OutOfBoundsError {
    /// Create a new out-of-bounds error.
    pub const fn new(value: usize, expected: std::ops::RangeInclusive<u8>) -> Self {
        Self { value, expected }
    }

    /// Create a new out-of-bounds error from an unsigned byte value. This
    /// constructor takes care of the common case where the value has the
    /// smallest unsigned integer type.
    pub const fn from_u8(value: u8, expected: std::ops::RangeInclusive<u8>) -> Self {
        OutOfBoundsError::new(value as usize, expected)
    }
}

impl std::fmt::Display for OutOfBoundsError {
    /// Format this out-of-bounds error.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} should fit into range {}..={}",
            self.value,
            self.expected.start(),
            self.expected.end()
        )
    }
}

#[cfg(feature = "pyffi")]
impl From<OutOfBoundsError> for PyErr {
    /// Convert a color format error to a Python exception.
    fn from(value: OutOfBoundsError) -> Self {
        pyo3::exceptions::PyValueError::new_err(value.to_string())
    }
}

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
#[cfg_attr(
    feature = "pyffi",
    doc = "In contrast, Python code uses the [`AnsiColor::from_8bit`] and
    [`AnsiColor::to_8bit`] methods."
)]
/// Since ANSI colors have no intrinsic color values, conversion to
/// high-resolution colors requires additional machinery, provided by
/// [`Theme`](crate::Theme).
///
/// <style>
/// .python-only::before, .rust-only::before {
///     font-size: 0.8em;
///     display: inline-block;
///     border-radius: 0.5em;
///     padding: 0 0.6em;
///     font-family: -apple-system, BlinkMacSystemFont, avenir next, avenir, segoe ui,
///         helvetica neue, helvetica, Cantarell, Ubuntu, roboto, noto, arial, sans-serif;
///     font-weight: bold;
/// }
/// .python-only::before {
///     content: "Python only!";
///     background: #84c5fb;
/// }
/// .rust-only::before {
///     content: "Rust only!";
///     background: #f0ac84;
/// }
/// </style>
///
/// # Black and White
///
/// Despite their names, *white* and *bright black* usually aren't white and
/// black, respectively, but tones of gray. *White* tends to be closer to
/// *bright white* than to either shade named black. Similarly, *bright black*
/// tends to be closer to *black* than either shade named white. In other words,
/// the 16 extended ANSI colors include a four-color gray gradient from *black*
/// to *bright black* to *white* to *bright white*.
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

#[cfg(feature = "pyffi")]
#[pymethods]
impl AnsiColor {
    /// Instantiate an ANSI color from its 8-bit code. <span
    /// class=python-only></span>
    ///
    /// This method offers the same functionality as [`TryFrom<u8> as
    /// AnsiColor`](enum.AnsiColor.html#impl-TryFrom%3Cu8%3E-for-AnsiColor) and
    /// is available in Python only.
    #[staticmethod]
    pub fn from_8bit(value: u8) -> Result<Self, OutOfBoundsError> {
        Self::try_from(value)
    }

    /// Get the 8-bit code for this ANSI color. <span class=python-only></span>
    ///
    /// This method offers the same functionality as [`From<AnsiColor> as
    /// u8`](enum.AnsiColor.html#impl-From%3CAnsiColor%3E-for-u8) and is
    /// available in Python only.
    pub fn to_8bit(&self) -> u8 {
        *self as u8
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
            _ => return Err(OutOfBoundsError::from_u8(value, 0..=15)),
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
// The Embedded 6x6x6 RGB
// ====================================================================================================================

/// The 6x6x6 RGB cube embedded in 8-bit terminal colors.
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
/// .python-only::before, .rust-only::before {
///     font-size: 0.8em;
///     display: inline-block;
///     border-radius: 0.5em;
///     padding: 0 0.6em;
///     font-family: -apple-system, BlinkMacSystemFont, avenir next, avenir, segoe ui,
///         helvetica neue, helvetica, Cantarell, Ubuntu, roboto, noto, arial, sans-serif;
///     font-weight: bold;
/// }
/// .python-only::before {
///     content: "Python only!";
///     background: #84c5fb;
/// }
/// .rust-only::before {
///     content: "Rust only!";
///     background: #f0ac84;
/// }
/// </style>
///
/// # Examples
///
/// Rust code can create a new embedded RGB color with either
/// [`EmbeddedRgb::new`] or [`TryFrom<u8> as
/// EmbeddedRgb`](struct.EmbeddedRgb.html#impl-TryFrom%3Cu8%3E-for-EmbeddedRgb).
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
/// u8`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-u8) or to
/// a high-resolution color with [`From<EmbeddedRgb> as
/// Color`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-Color).
/// ```
/// # use prettypretty::{Color, EmbeddedRgb, OutOfBoundsError};
/// let rose = EmbeddedRgb::new(5, 4, 5)?;
/// assert_eq!(u8::from(rose), 225);
///
/// let rose_too = Color::from(rose);
/// assert_eq!(rose_too.to_hex_format(), "#ffd7ff");
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
    through [`EmbeddedRgb::from_8bit`], [`EmbeddedRgb::__len__`],
    [`EmbeddedRgb::__getitem__`], [`EmbeddedRgb::to_8bit`], and
    [`EmbeddedRgb::to_color`]. These methods are not available in Rust."
)]
#[cfg_attr(feature = "pyffi", pyclass(eq, frozen, hash, sequence))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EmbeddedRgb([u8; 3]);

#[cfg(feature = "pyffi")]
#[pymethods]
impl EmbeddedRgb {
    /// Create a new embedded RGB value from its coordinates.
    #[new]
    pub const fn new(r: u8, g: u8, b: u8) -> Result<Self, OutOfBoundsError> {
        if r >= 6 {
            Err(OutOfBoundsError::from_u8(r, 0..=5))
        } else if g >= 6 {
            Err(OutOfBoundsError::from_u8(g, 0..=5))
        } else if b >= 6 {
            Err(OutOfBoundsError::from_u8(b, 0..=5))
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
}

#[cfg(not(feature = "pyffi"))]
impl EmbeddedRgb {
    /// Create a new embedded RGB value from its coordinates.
    pub const fn new(r: u8, g: u8, b: u8) -> Result<Self, OutOfBoundsError> {
        if r >= 6 {
            Err(OutOfBoundsError::from_u8(r, 0..=5))
        } else if g >= 6 {
            Err(OutOfBoundsError::from_u8(g, 0..=5))
        } else if b >= 6 {
            Err(OutOfBoundsError::from_u8(b, 0..=5))
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
            Err(OutOfBoundsError::from_u8(value, 16..=231))
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
/// .python-only::before, .rust-only::before {
///     font-size: 0.8em;
///     display: inline-block;
///     border-radius: 0.5em;
///     padding: 0 0.6em;
///     font-family: -apple-system, BlinkMacSystemFont, avenir next, avenir, segoe ui,
///         helvetica neue, helvetica, Cantarell, Ubuntu, roboto, noto, arial, sans-serif;
///     font-weight: bold;
/// }
/// .python-only::before {
///     content: "Python only!";
///     background: #84c5fb;
/// }
/// .rust-only::before {
///     content: "Rust only!";
///     background: #f0ac84;
/// }
/// </style>
///
/// # Examples
///
/// Rust code can instantiate a new gray gradient color with either
/// [`GrayGradient::new`] or [`TryFrom<u8> as
/// GrayGradient`](struct.GrayGradient.html#impl-TryFrom%3Cu8%3E-for-GrayGradient).
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
/// u8`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-u8) or to a
/// high-resolution color with [`From<GrayGradient> as
/// Color`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-Color).
/// ```
/// # use prettypretty::{Color, GrayGradient, OutOfBoundsError};
/// let light_gray = GrayGradient::new(20)?;
/// assert_eq!(u8::from(light_gray), 252);
///
/// let light_gray_too = Color::from(light_gray);
/// assert_eq!(light_gray_too.to_hex_format(), "#d0d0d0");
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
    through [`GrayGradient::from_8bit`], [`GrayGradient::to_8bit`], and
    [`GrayGradient::to_color`]. These methods are not available in Rust, though
    [`GrayGradient::new`] and [`GrayGradient::level`] are."
)]
#[cfg_attr(feature = "pyffi", pyclass(eq, frozen, hash, ord))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GrayGradient(u8);

#[cfg(feature = "pyffi")]
#[pymethods]
impl GrayGradient {
    /// Instantiate a new gray gradient from its level `0..=23`.
    #[new]
    pub const fn new(value: u8) -> Result<Self, OutOfBoundsError> {
        if value >= 24 {
            Err(OutOfBoundsError::from_u8(value, 0..=23))
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
}

#[cfg(not(feature = "pyffi"))]
impl GrayGradient {
    /// Instantiate a new gray gradient from its level `0..=23`.
    pub const fn new(value: u8) -> Result<Self, OutOfBoundsError> {
        if value <= 23 {
            Ok(Self(value))
        } else {
            Err(OutOfBoundsError::from_u8(value, 0..=23))
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
            Err(OutOfBoundsError::from_u8(value, 232..=255))
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

impl From<GrayGradient> for Color {
    /// Instantiate a high-resolution color from an embedded RGB value.
    fn from(value: GrayGradient) -> Self {
        let level = 8 + 10 * value.level();
        Color::from_24bit(level, level, level)
    }
}

// ====================================================================================================================
// 8-bit Color
// ====================================================================================================================

/// 8-bit terminal colors.
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
/// .python-only::before, .rust-only::before {
///     font-size: 0.8em;
///     display: inline-block;
///     border-radius: 0.5em;
///     padding: 0 0.6em;
///     font-family: -apple-system, BlinkMacSystemFont, avenir next, avenir, segoe ui,
///         helvetica neue, helvetica, Cantarell, Ubuntu, roboto, noto, arial, sans-serif;
///     font-weight: bold;
/// }
/// .python-only::before {
///     content: "Python only!";
///     background: #84c5fb;
/// }
/// .rust-only::before {
///     content: "Rust only!";
///     background: #f0ac84;
/// }
/// </style>
///
/// # Black and White
///
/// The ANSI colors, the 6x6x6 RGB cube, and the gray gradient all include
/// colors that are pretty close to black and white. They may even be called
/// black or white. Which one should we use?
///
/// If the terminal only supports ANSI colors, then there is no choice. We have
/// to use the ANSI black and bright white. But since ANSI colors are themeable
/// in most terminal emulators, we cannot count on those colors actually
/// rendering as black and white. Furthermore, even rather conservative color
/// themes, such as the default light theme in macOS Terminal.app, may not use
/// `#000` and `#fff` for black and white.
///
/// Instead, if the terminal supports 8-bit colors, we should use the first and
/// last color belonging to the embedded RGB cube, i.e., 16 and 231. Within that
/// low-resolution RGB cube, they correspond to the extrema 0, 0, 0 and 5, 5, 5,
/// i.e., black and white. Even better, they retain their extremism under
/// conversion to sRGB, turning into `#000` and `#fff`, respectively. By
/// comparison, the darkest and lightest color of the gray gradient are
/// `#121212` and `#f8f8f8`, respectively.
///
/// <div class=color-swatch>
/// <div style="background-color: #000;"></div>
/// <div style="background-color: #fff;"></div>
/// <div style="background-color: #121212;"></div>
/// <div style="background-color: #f8f8f8;"></div>
/// </div>
#[cfg_attr(feature = "pyffi", pyclass(eq, frozen, hash))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EightBitColor {
    Ansi(AnsiColor),
    Rgb(EmbeddedRgb),
    Gray(GrayGradient),
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl EightBitColor {
    /// Instantiate an 8-bit color from its numeric code. <span
    /// class=python-only></span>
    ///
    /// This method offers the same functionality as [`From<u8> as
    /// EightBitColor`](enum.EightBitColor.html#impl-From%3Cu8%3E-for-EightBitColor)
    /// and is available in Python only.
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn from_8bit(value: u8) -> Self {
        Self::from(value)
    }

    /// Get the numeric code for this 8-bit color. <span
    /// class=python-only></span>
    ///
    /// This method offers the same functionality as [`From<EightBitColor> as
    /// u8`](enum.EightBitColor.html#impl-From%3CEightBitColor%3E-for-u8) and is
    /// available in Python only.
    #[cfg(feature = "pyffi")]
    pub fn to_8bit(&self) -> u8 {
        u8::from(*self)
    }

    /// Convert this 8-bit color to a 3+1-bit RGB color.
    ///
    /// This method treats embedded RGB and gray gradient colors as RGB colors,
    /// downsamples them to 3-bit RGB, i.e., one bit per component, and then
    /// uses the magnitude of the corresponding integer as signal for possibly
    /// switching to bright colors, i.e., the +1-bit. While pretty coarse, this
    /// heuristic is already more sophisticated than the one employed by
    /// [Chalk](https://github.com/chalk/chalk/blob/main/source/vendor/ansi-styles/index.js),
    /// one of the more popular terminal color libraries for JavaScript.
    pub fn to_4bit_rgb(&self) -> AnsiColor {
        let (r, g, b) = match *self {
            Self::Ansi(color) => return color,
            Self::Rgb(color) => {
                (color[0] as Float / 5.0, color[1] as Float / 5.0, color[2] as Float / 5.0)
            }
            Self::Gray(color) => {
                let c = color.level() as Float / 23.0;
                (c, c, c)
            }
        };

        // (0..=1).round() effectively downsamples to 1-bit per component.
        // Then shift bits into 3-bit binary number, b most significant.
        let mut c = (b.round() as u8) << 2 + (g.round() as u8) << 1 + r.round() as u8;
        // Use magnitude of bgr to maybe switch (+1 bit) to bright colors.
        if c >= 2 {
            c += 8;
        }

        AnsiColor::try_from(c).unwrap()
    }

    /// Convert this 8-bit color to a high-resolution color.
    ///
    /// Since the 16 extended ANSI colors do not have intrinsic color values,
    /// this method requires access to the current color theme for possibly
    /// resolving an ANSI color to its current value.
    pub fn to_color(&self, theme: &Theme) -> Color {
        match *self {
            Self::Ansi(color) => theme[color].clone(),
            Self::Rgb(color) => color.into(),
            Self::Gray(color) => color.into(),
        }
    }
}

impl From<u8> for EightBitColor {
    /// Convert an unsigned byte to an 8-bit color.
    fn from(value: u8) -> Self {
        use EightBitColor::*;

        if value <= 15 {
            Ansi(AnsiColor::try_from(value).unwrap())
        } else if value <= 231 {
            Rgb(EmbeddedRgb::try_from(value).unwrap())
        } else {
            Gray(GrayGradient::try_from(value).unwrap())
        }
    }
}

impl From<EightBitColor> for u8 {
    /// Convert an 8-bit color to an unsigned byte.
    fn from(value: EightBitColor) -> u8 {
        match value {
            EightBitColor::Ansi(color) => color.into(),
            EightBitColor::Rgb(color) => color.into(),
            EightBitColor::Gray(color) => color.into(),
        }
    }
}

// ====================================================================================================================

#[cfg(test)]
mod test {
    use super::{AnsiColor, EightBitColor, EmbeddedRgb, GrayGradient, OutOfBoundsError};

    #[test]
    fn test_conversion() -> Result<(), OutOfBoundsError> {
        let magenta = AnsiColor::Magenta;
        assert_eq!(magenta as u8, 5);

        let green = EmbeddedRgb::new(0, 4, 0)?;
        assert_eq!(green.as_ref(), &[0, 4, 0]);

        let gray = GrayGradient::new(12)?;
        assert_eq!(gray.level(), 12);

        let also_magenta = EightBitColor::Ansi(AnsiColor::Magenta);
        let also_green = EightBitColor::Rgb(green);
        let also_gray = EightBitColor::Gray(gray);

        assert_eq!(u8::from(also_magenta), 5);
        assert_eq!(u8::from(also_green), 40);
        assert_eq!(u8::from(also_gray), 244);

        assert_eq!(EightBitColor::from(5), also_magenta);
        assert_eq!(EightBitColor::from(40), also_green);
        assert_eq!(EightBitColor::from(244), also_gray);

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
