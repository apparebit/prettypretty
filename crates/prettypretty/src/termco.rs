//! Terminal color representations.
//!
//! This module offers a choice of [`AnsiColor`], [`EmbeddedRgb`],
//! [`GrayGradient`], and [`Rgb`] as the primary color representations used bys
//! terminals.
//!
//! Taken together, the first three types are the 8-bit colors and can be
//! wrapped as [`EightBitColor`]s. All color representations including
//! high-resolution [`Color`]s can be wrapped as [`Colorant`]s. That type also
//! includes a variant for the [`Colorant::Default`].
#[cfg(feature = "pyffi")]
use pyo3::{prelude::*, types::PyInt};

use crate::error::{HiResColorantError, OutOfBoundsError};
use crate::style::Layer;
use crate::{Color, ColorSpace};

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
    pyclass(eq, eq_int, frozen, hash, ord, module = "prettypretty.color.termco")
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

#[cfg(not(feature = "pyffi"))]
impl AnsiColor {
    /// Get an iterator over all ANSI colors in order.
    pub fn all() -> AnsiColorIterator {
        AnsiColorIterator::new()
    }
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl AnsiColor {
    /// Get an iterator over all ANSI colors in order.
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn all() -> AnsiColorIterator {
        AnsiColorIterator::new()
    }

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

    /// Get the base version of this ANSI color.
    ///
    /// If this color is bright, this method returns its non-bright version.
    /// Otherwise, it returns the same color.
    pub fn to_base(&self) -> AnsiColor {
        let mut index = *self as u8;
        if 8 <= index {
            index -= 8;
        }
        // SAFETY: index is within bounds by construction
        AnsiColor::try_from(index).unwrap()
    }

    /// Get the bright version of this ANSI color.
    ///
    /// If the color is not bright, this method returns its bright version.
    /// Otherwise, it returns the same color.
    pub fn to_bright(&self) -> AnsiColor {
        let mut index = *self as u8;
        if index < 8 {
            index += 8;
        }
        // SAFETY: index is within bounds by construction
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

    /// Get an abbreviation for this ANSI color.
    ///
    /// This method returns a two-letter abbreviation for this ANSI color. The
    /// abbreviations for each pair of nonbright and bright colors only differ
    /// in case, with the nonbright color's abbreviation in lower case and the
    /// bright color's abbreviation in upper case.
    pub fn abbr(&self) -> &'static str {
        use AnsiColor::*;

        match self {
            Black => "bk",
            Red => "rd",
            Green => "gn",
            Yellow => "yl",
            Blue => "bu",
            Magenta => "mg",
            Cyan => "cn",
            White => "wt",
            BrightBlack => "BK",
            BrightRed => "RD",
            BrightGreen => "GN",
            BrightYellow => "YL",
            BrightBlue => "BU",
            BrightMagenta => "MG",
            BrightCyan => "CN",
            BrightWhite => "WT",
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

// --------------------------------------------------------------------------------------------------------------------

/// A helper for iterating over ANSI colors.
///
/// This iterator is fused, i.e., after returning `None` once, it will keep
/// returning `None`. This iterator also is exact, i.e., its `size_hint()`
/// returns the exact number of remaining items.
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.termco"))]
#[derive(Debug)]
pub struct AnsiColorIterator {
    index: usize,
}

impl AnsiColorIterator {
    fn new() -> Self {
        Self { index: 0 }
    }
}

impl Iterator for AnsiColorIterator {
    type Item = AnsiColor;

    fn next(&mut self) -> Option<Self::Item> {
        if 16 <= self.index {
            None
        } else {
            let index = self.index;
            self.index += 1;
            Some(AnsiColor::try_from(index as u8).unwrap())
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = 16 - self.index;
        (remaining, Some(remaining))
    }
}

impl std::iter::ExactSizeIterator for AnsiColorIterator {
    fn len(&self) -> usize {
        16 - self.index
    }
}

impl std::iter::FusedIterator for AnsiColorIterator {}

#[cfg(feature = "pyffi")]
#[pymethods]
impl AnsiColorIterator {
    /// Get the number of remaining ANSI colors. <i class=python-only>Python
    /// only!</i>
    pub fn __len__(&self) -> usize {
        self.len()
    }

    /// Return this iterator. <i class=python-only>Python only!</i>
    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Return the next ANSI color. <i class=python-only>Python only!</i>
    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<AnsiColor> {
        slf.next()
    }

    /// Get a debug representation for this iterator.
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
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
/// # use prettypretty::termco::EmbeddedRgb;
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
/// # use prettypretty::termco::EmbeddedRgb;
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
/// to a true color with [`Rgb as
/// From<EmbeddedRgb>`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-Rgb),
/// or to a high-resolution color with [`Color as
/// From<EmbeddedRgb>`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-Color).
/// ```
/// # use prettypretty::Color;
/// # use prettypretty::termco::{EmbeddedRgb, Rgb};
/// # use prettypretty::error::OutOfBoundsError;
/// let rose = EmbeddedRgb::new(5, 4, 5)?;
/// assert_eq!(u8::from(rose), 225);
///
/// let also_rose = Rgb::from(rose);
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
    pyclass(eq, frozen, hash, sequence, module = "prettypretty.color.termco")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EmbeddedRgb([u8; 3]);

#[cfg(feature = "pyffi")]
#[pymethods]
impl EmbeddedRgb {
    /// Create a new embedded RGB value from its coordinates. <i
    /// class=python-only>Python only!</i>
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

impl From<&EmbeddedRgb> for Color {
    fn from(value: &EmbeddedRgb) -> Self {
        Color::from(Rgb::from(*value))
    }
}

impl From<EmbeddedRgb> for Color {
    fn from(value: EmbeddedRgb) -> Self {
        Color::from(Rgb::from(value))
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
/// # use prettypretty::termco::GrayGradient;
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
/// # use prettypretty::termco::GrayGradient;
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
/// to a true color with [`Rgb as
/// From<GrayGradient>`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-Rgb),
/// or to a high-resolution color with [`Color as
/// From<GrayGradient>`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-Color).
/// ```
/// # use prettypretty::Color;
/// # use prettypretty::termco::{GrayGradient, Rgb};
/// # use prettypretty::error::OutOfBoundsError;
/// let light_gray = GrayGradient::new(20)?;
/// assert_eq!(u8::from(light_gray), 252);
///
/// let also_light_gray = Rgb::from(light_gray);
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
    pyclass(eq, frozen, hash, ord, module = "prettypretty.color.termco")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GrayGradient(u8);

#[cfg(feature = "pyffi")]
#[pymethods]
impl GrayGradient {
    /// Instantiate a new gray gradient from its level `0..=23`. <i
    /// class=python-only>Python only!</i>
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

impl From<&GrayGradient> for Color {
    fn from(value: &GrayGradient) -> Self {
        Color::from(Rgb::from(*value))
    }
}

impl From<GrayGradient> for Color {
    fn from(value: GrayGradient) -> Self {
        Color::from(Rgb::from(value))
    }
}

// ====================================================================================================================
// Eight-Bit Color
// ====================================================================================================================

/// An 8-bit color wrapping an [`AnsiColor`], [`EmbeddedRgb`], or [`GrayGradient`].
///
#[cfg_attr(
    feature = "pyffi",
    doc = "Since there is no Python feature equivalent to trait implementations in
    Rust, the Python class for `EightBitColor` provides equivalent functionality
    through [`EightBitColor::from_8bit`], [`EightBitColor::to_8bit`], and
    [`EightBitColor::__repr__`]. These methods are not available in Rust."
)]
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.termco")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EightBitColor {
    Ansi(AnsiColor),
    Embedded(EmbeddedRgb),
    Gray(GrayGradient),
}

#[cfg(feature = "pyffi")]
#[pymethods]
impl EightBitColor {
    /// Convert the byte into an 8-bit color. <i class=python-only>Python
    /// only!</i>
    #[staticmethod]
    pub fn from_8bit(byte: u8) -> Self {
        Self::from(byte)
    }

    /// Convert the 8-bit color to a byte. <i class=python-only>Python only!</i>
    pub fn to_8bit(&self) -> u8 {
        u8::from(*self)
    }

    /// Get a debug representation for this 8-bit color. <i
    /// class=python-only>Python only!</i>
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl From<AnsiColor> for EightBitColor {
    fn from(value: AnsiColor) -> Self {
        Self::Ansi(value)
    }
}

impl From<EmbeddedRgb> for EightBitColor {
    fn from(value: EmbeddedRgb) -> Self {
        Self::Embedded(value)
    }
}

impl From<GrayGradient> for EightBitColor {
    fn from(value: GrayGradient) -> Self {
        Self::Gray(value)
    }
}

impl From<u8> for EightBitColor {
    fn from(value: u8) -> Self {
        // SAFETY: explicit range checks
        if (0..=15).contains(&value) {
            Self::Ansi(AnsiColor::try_from(value).unwrap())
        } else if (16..=231).contains(&value) {
            Self::Embedded(EmbeddedRgb::try_from(value).unwrap())
        } else {
            Self::Gray(GrayGradient::try_from(value).unwrap())
        }
    }
}

impl From<EightBitColor> for u8 {
    fn from(value: EightBitColor) -> Self {
        match value {
            EightBitColor::Ansi(c) => c.into(),
            EightBitColor::Embedded(c) => c.into(),
            EightBitColor::Gray(c) => c.into(),
        }
    }
}

impl From<EightBitColor> for Colorant {
    fn from(value: EightBitColor) -> Self {
        match value {
            EightBitColor::Ansi(c) => Colorant::Ansi(c),
            EightBitColor::Embedded(c) => Colorant::Embedded(c),
            EightBitColor::Gray(c) => Colorant::Gray(c),
        }
    }
}

// ====================================================================================================================
// Rgb ("True Color")
// ====================================================================================================================

/// A "true," 24-bit RGB color.
///
/// # Examples
///
/// Rust code can create a new true color with either [`Rgb::new`] or
/// [`Rgb as
/// From<&Color>`](struct.Rgb.html#impl-From%3C%26Color%3E-for-Rgb).
///
/// ```
/// # use prettypretty::Color;
/// # use prettypretty::termco::Rgb;
/// let blue = Color::from_24bit(0xae, 0xe8, 0xfb);
/// let blue_too = Rgb::new(0xae, 0xe8, 0xfb);
/// assert_eq!(Rgb::from(&blue), blue_too);
/// ```
/// <div class=color-swatch>
/// <div style="background-color: #aee8fb;"></div>
/// </div>
/// <br>
///
/// It can access the coordinates with [`Rgb as AsRef<[u8;
/// 3]>`](struct.Rgb.html#impl-AsRef%3C%5Bu8;+3%5D%3E-for-Rgb) or
/// with [`Rgb as
/// Index<usize>`](struct.Rgb.html#impl-Index%3Cusize%3E-for-Rgb).
/// ```
/// # use prettypretty::termco::Rgb;
/// let sea_foam = Rgb::new(0xb6, 0xeb, 0xd4);
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
/// From<Rgb>`](struct.Rgb.html#impl-From%3CRgb%3E-for-Color)
/// or format it in hashed hexadecimal notation with [`Rgb as
/// Display`](struct.Rgb.html#impl-Display-for-Rgb).
/// ```
/// # use prettypretty::Color;
/// # use prettypretty::termco::Rgb;
/// let sand = Rgb::new(0xee, 0xdc, 0xad);
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
    Rust, the Python class for `Rgb` provides equivalent functionality
    through [`Rgb::from_color`], [`Rgb::to_color`], [`Rgb::coordinates`],
    [`Rgb::__len__`], and [`Rgb::__getitem__`]. These methods are not available
    in Rust."
)]
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, sequence, module = "prettypretty.color.termco")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Rgb([u8; 3]);

#[cfg(feature = "pyffi")]
#[pymethods]
impl Rgb {
    /// Create a new true RGB color from its coordinates. <i
    /// class=python-only>Python only!</i>
    #[new]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b])
    }

    /// Create a new true RGB color from the given high-resolution color. <i
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

    /// Convert this true RGB color to a high-resolution color. <i
    /// class=python-only>Python only!</i>
    ///
    /// This method offers the same functionality as [`Color as
    /// From<Rgb>`](struct.Rgb.html#impl-From%3CRgb%3E-for-Color)
    /// and is available in Python only.
    pub fn to_color(&self) -> Color {
        Color::from(*self)
    }

    /// Access this true RGB color's coordinates. <i class=python-only>Python
    /// only!</i>
    pub fn coordinates(&self) -> [u8; 3] {
        self.0
    }

    /// Get this true RGB color's length, which is 3. <i
    /// class=python-only>Python only!</i>
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
    pub fn weighted_euclidian_distance(&self, other: &Rgb) -> u32 {
        self.do_weighted_euclidian_distance(other)
    }

    /// Convert this true color to its debug representation. <i
    /// class=python-only>Python only!</i>
    pub fn __repr__(&self) -> String {
        format!("Rgb({}, {}, {})", self.0[0], self.0[1], self.0[2])
    }

    /// Convert this true color to hashed hexadecimal notation. <i
    /// class=python-only>Python only!</i>
    pub fn __str__(&self) -> String {
        format!("{}", self)
    }
}

#[cfg(not(feature = "pyffi"))]
impl Rgb {
    /// Create a new true color from its coordinates.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b])
    }

    /// Calculate the weighted Euclidian distance between the two colors.
    ///
    /// This method reimplements the distance metric used by the [anstyle
    /// crate](https://github.com/rust-cli/anstyle/blob/main/crates/anstyle-lossy/src/lib.rs).
    pub fn weighted_euclidian_distance(&self, other: &Rgb) -> u32 {
        self.do_weighted_euclidian_distance(other)
    }
}

impl Rgb {
    fn do_weighted_euclidian_distance(&self, other: &Rgb) -> u32 {
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

impl AsRef<[u8; 3]> for Rgb {
    fn as_ref(&self) -> &[u8; 3] {
        &self.0
    }
}

impl std::ops::Index<usize> for Rgb {
    type Output = u8;

    /// Access the coordinate with the given index.
    ///
    /// # Panics
    ///
    /// This method panics if `2 < index`.
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl From<EmbeddedRgb> for Rgb {
    fn from(value: EmbeddedRgb) -> Self {
        let [r, g, b] = Into::<[u8; 3]>::into(value);
        Rgb::new(r, g, b)
    }
}

impl From<GrayGradient> for Rgb {
    fn from(value: GrayGradient) -> Rgb {
        let [r, g, b] = Into::<[u8; 3]>::into(value);
        Rgb::new(r, g, b)
    }
}

impl From<[u8; 3]> for Rgb {
    fn from(value: [u8; 3]) -> Self {
        Rgb::new(value[0], value[1], value[2])
    }
}

impl From<&Color> for Rgb {
    /// Convert the given color to a true color.
    ///
    /// This method first converts the color to gamut-mapped sRGB and then
    /// converts each coordinate to `u8`
    fn from(value: &Color) -> Self {
        let [r, g, b] = value.to(ColorSpace::Srgb).to_gamut().to_24bit();
        Self::new(r, g, b)
    }
}

impl From<Color> for Rgb {
    fn from(value: Color) -> Self {
        Rgb::from(&value)
    }
}

impl From<&Rgb> for Color {
    fn from(value: &Rgb) -> Self {
        Self::from_24bit(value.0[0], value.0[1], value.0[2])
    }
}

impl From<Rgb> for Color {
    fn from(value: Rgb) -> Self {
        Self::from_24bit(value.0[0], value.0[1], value.0[2])
    }
}

impl core::fmt::Display for Rgb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [r, g, b] = *self.as_ref();
        f.write_fmt(format_args!("#{:02x}{:02x}{:02x}", r, g, b))
    }
}

// ====================================================================================================================
// Colorant
// ====================================================================================================================

/// A colorant combines all of prettypretty's color representations.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.termco")
)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Colorant {
    Default(),
    Ansi(AnsiColor),
    Embedded(EmbeddedRgb),
    Gray(GrayGradient),
    Rgb(Rgb),
    HiRes(Color),
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Colorant {
    /// Wrap any color as colorant. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn of(#[pyo3(from_py_with = "crate::termco::into_colorant")] colorant: Colorant) -> Self {
        colorant
    }

    /// Determine whether this colorant is the default.
    pub fn is_default(&self) -> bool {
        matches!(self, Colorant::Default())
    }

    /// Get the SGR parameters for this colorant.
    ///
    /// This method returns `None` if this colorant is a high-resolution color.
    pub fn sgr_parameters(&self, layer: Layer) -> Option<Vec<u8>> {
        match self {
            Self::Default() => Some(vec![39 + layer.offset()]),
            Self::Ansi(c) => {
                let base = if c.is_bright() { 90 } else { 30 } + layer.offset();
                Some(vec![base + c.to_base() as u8])
            }
            Self::Embedded(c) => Some(vec![38 + layer.offset(), 5, u8::from(*c)]),
            Self::Gray(c) => Some(vec![38 + layer.offset(), 5, u8::from(*c)]),
            Self::Rgb(c) => Some(vec![38 + layer.offset(), 2, c[0], c[1], c[2]]),
            Self::HiRes(_) => None,
        }
    }

    /// Convert this terminal color to an 8-bit index color. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn try_to_8bit(&self) -> PyResult<u8> {
        u8::try_from(self).map_err(|_| {
            pyo3::exceptions::PyValueError::new_err("unable to convert to 8-bit index")
        })
    }

    /// Convert this terminal color to 24-bit. <i class=python-only>Python
    /// only!</i>
    #[cfg(feature = "pyffi")]
    pub fn try_to_24bit(&self) -> PyResult<[u8; 3]> {
        <[u8; 3]>::try_from(self).map_err(|_| {
            pyo3::exceptions::PyValueError::new_err("unable to convert to 24-bit coordinates")
        })
    }

    /// Negate this colorant.
    ///
    /// This method computes the color that restores the terminal's default
    /// appearance again, which is `None` if this colorant is the default color.
    fn negate(&self) -> Option<Self> {
        if self.is_default() {
            None
        } else {
            Some(Colorant::Default())
        }
    }

    /// Negate this colorant. <i class=python-only>Python only!</i>
    ///
    /// This method determines the color that restores the terminal's default
    /// appearance again. The result is `None` if the colorant is the default
    /// and  `Some(Colorant::Default())` otherwise.
    #[cfg(feature = "pyffi")]
    pub fn __neg__(&self) -> Option<Self> {
        self.negate()
    }

    /// Convert this colorant to its debug representation. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        match self {
            Self::Default() => "Colorant(default)".to_string(),
            Self::Ansi(c) => format!("Colorant({:?})", c),
            Self::Embedded(c) => format!("Colorant({})", c.__repr__()),
            Self::Gray(c) => format!("Colorant({})", c.__repr__()),
            Self::Rgb(c) => format!("Colorant({})", c.__repr__()),
            Self::HiRes(c) => format!("Colorant({})", c.__repr__()),
        }
    }

    /// Display the colorant for the given layer.
    #[cfg(feature = "pyffi")]
    #[pyo3(name = "display")]
    pub fn py_display(&self, layer: Layer) -> PyResult<String> {
        Ok(format!("{}", self.display(layer)?))
    }
}

impl Colorant {
    /// Write the colorant's SGR parameters for the given layer with the given
    /// formatter. <i class=rust-only>Rust only!</i>
    pub fn write_sgr_params(
        &self,
        layer: Layer,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::Default() => write!(f, "{}", 39 + layer.offset()),
            Self::Ansi(c) => {
                let base = if c.is_bright() { 90 } else { 30 } + layer.offset();
                write!(f, "{}", base + c.to_base() as u8)
            }
            Self::Embedded(c) => write!(f, "{};5;{}", 38 + layer.offset(), u8::from(*c)),
            Self::Gray(c) => write!(f, "{};5;{}", 38 + layer.offset(), u8::from(*c)),
            Self::Rgb(c) => write!(f, "{};2;{};{};{}", 38 + layer.offset(), c[0], c[1], c[2]),
            Self::HiRes(_) => Ok(()),
        }
    }

    /// Provide a layer to make this colorant displayable.
    ///
    /// For most colorants, this method returns an implementation of the
    /// standard library's [`std::fmt::Display`] trait that produces the
    /// corresponding ANSI escape sequence. However, for colorants wrapping
    /// high-resolution colors, this method returns an error. Such a colorant
    /// needs to be [`Translator::cap`](crate::Translator::cap)ped first.
    pub fn display(
        &self,
        layer: Layer,
    ) -> Result<impl std::fmt::Display + use<'_>, HiResColorantError> {
        if matches!(self, Self::HiRes(_)) {
            Err(HiResColorantError)
        } else {
            Ok(LayeredColorant {
                layer,
                colorant: self,
            })
        }
    }
}

impl std::ops::Neg for &Colorant {
    type Output = Option<Colorant>;

    fn neg(self) -> Self::Output {
        self.negate()
    }
}

impl std::ops::Neg for Colorant {
    type Output = Option<Colorant>;

    fn neg(self) -> Self::Output {
        self.negate()
    }
}

/// Convert any color into a colorant.
#[cfg(feature = "pyffi")]
pub(crate) fn into_colorant(obj: &Bound<'_, PyAny>) -> PyResult<Colorant> {
    if obj.is_instance_of::<PyInt>() {
        return obj.extract::<u8>().map(|c| c.into());
    }

    obj.extract::<Colorant>()
        .or_else(|_| obj.extract::<AnsiColor>().map(|c| c.into()))
        .or_else(|_| obj.extract::<EmbeddedRgb>().map(|c| c.into()))
        .or_else(|_| obj.extract::<GrayGradient>().map(|c| c.into()))
        .or_else(|_| obj.extract::<EightBitColor>().map(|c| c.into()))
        .or_else(|_| obj.extract::<Rgb>().map(|c| c.into()))
        .or_else(|_| obj.extract::<Color>().map(|c| c.into()))
}

impl From<AnsiColor> for Colorant {
    fn from(value: AnsiColor) -> Self {
        Self::Ansi(value)
    }
}

impl From<EmbeddedRgb> for Colorant {
    fn from(value: EmbeddedRgb) -> Self {
        Self::Embedded(value)
    }
}

impl From<GrayGradient> for Colorant {
    fn from(value: GrayGradient) -> Self {
        Self::Gray(value)
    }
}

impl From<u8> for Colorant {
    fn from(value: u8) -> Self {
        if (0..=15).contains(&value) {
            Self::Ansi(AnsiColor::try_from(value).unwrap())
        } else if (16..=231).contains(&value) {
            Self::Embedded(EmbeddedRgb::try_from(value).unwrap())
        } else {
            Self::Gray(GrayGradient::try_from(value).unwrap())
        }
    }
}

impl From<[u8; 3]> for Colorant {
    fn from(value: [u8; 3]) -> Self {
        Self::Rgb(Rgb(value))
    }
}

impl From<Rgb> for Colorant {
    fn from(value: Rgb) -> Self {
        Self::Rgb(value)
    }
}

impl From<Color> for Colorant {
    fn from(value: Color) -> Self {
        Self::HiRes(value)
    }
}

impl From<&Color> for Colorant {
    fn from(value: &Color) -> Self {
        Self::HiRes(value.clone())
    }
}

impl TryFrom<&Colorant> for u8 {
    type Error = Colorant;

    /// Try to convert this colorant to an 8-bit index.
    ///
    /// For ANSI, embedded RGB, and gray gradient colors, this method unwraps
    /// the colorant and returns the corresponding 8-bit index. It returns any
    /// other colorant as the error value.
    fn try_from(value: &Colorant) -> Result<Self, Self::Error> {
        match value {
            Colorant::Default() => Err(value.clone()),
            Colorant::Ansi(c) => Ok(u8::from(*c)),
            Colorant::Embedded(c) => Ok(u8::from(*c)),
            Colorant::Gray(c) => Ok(u8::from(*c)),
            Colorant::Rgb(_) => Err(value.clone()),
            Colorant::HiRes(_) => Err(value.clone()),
        }
    }
}

impl TryFrom<Colorant> for u8 {
    type Error = Colorant;

    fn try_from(value: Colorant) -> Result<Self, Self::Error> {
        u8::try_from(&value)
    }
}

impl TryFrom<&Colorant> for [u8; 3] {
    type Error = Colorant;

    fn try_from(value: &Colorant) -> Result<Self, Self::Error> {
        match value {
            Colorant::Default() | Colorant::Ansi(_) => Err(value.clone()),
            Colorant::Embedded(c) => Ok((*c).into()),
            Colorant::Gray(c) => Ok((*c).into()),
            Colorant::Rgb(c) => Ok(*c.as_ref()),
            Colorant::HiRes(_) => Err(value.clone()),
        }
    }
}

impl TryFrom<Colorant> for [u8; 3] {
    type Error = Colorant;

    fn try_from(value: Colorant) -> Result<Self, Self::Error> {
        <[u8; 3]>::try_from(&value)
    }
}

impl TryFrom<Colorant> for Color {
    type Error = Colorant;

    fn try_from(value: Colorant) -> Result<Self, Self::Error> {
        if let Colorant::HiRes(c) = value {
            Ok(c)
        } else {
            let [r, g, b] = value.try_into()?;
            Ok(Color::from_24bit(r, g, b))
        }
    }
}

struct LayeredColorant<'a> {
    layer: Layer,
    colorant: &'a Colorant,
}

impl std::fmt::Display for LayeredColorant<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\x1b[")?;
        self.colorant.write_sgr_params(self.layer, f)?;
        f.write_str("m")
    }
}

#[cfg(test)]
mod test {
    use super::{AnsiColor, Colorant, EmbeddedRgb, GrayGradient, OutOfBoundsError, Rgb};

    #[test]
    fn test_conversion() -> Result<(), OutOfBoundsError> {
        let magenta = AnsiColor::Magenta;
        assert_eq!(magenta as u8, 5);

        let green = EmbeddedRgb::new(0, 4, 0)?;
        assert_eq!(green.as_ref(), &[0, 4, 0]);
        assert_eq!(Rgb::from(green), Rgb::new(0, 215, 0));

        let gray = GrayGradient::new(12)?;
        assert_eq!(gray.level(), 12);
        assert_eq!(Rgb::from(gray), Rgb::new(128, 128, 128));

        let also_magenta = Colorant::Ansi(AnsiColor::Magenta);
        let also_green = Colorant::Embedded(green);
        let also_gray = Colorant::Gray(gray);

        assert_eq!(also_magenta, Colorant::from(5));
        assert_eq!(also_green, Colorant::from(40));
        assert_eq!(also_gray, Colorant::from(244));

        assert!(<[u8; 3]>::try_from(also_magenta).is_err());
        assert_eq!(<[u8; 3]>::try_from(also_green), Ok([0_u8, 215, 0]));
        assert_eq!(<[u8; 3]>::try_from(also_gray), Ok([128_u8, 128, 128]));

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
