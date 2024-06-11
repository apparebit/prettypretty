//! # Terminal Color Formats
//!
//! This module provides the abstractions for terminal color formats. Unlike the
//! more general and precise color abstraction, this module is informed by the
//! many restrictions of terminals. One key consequence is that even colors with
//! three coordinates do not use floating point but integral numbers drawn from
//! a specific range.

// ====================================================================================================================
// Errors
// ====================================================================================================================

use super::util::Coordinate;
use std::ops::RangeInclusive;

/// An out-of-bounds error.
///
/// Trying to convert an invalid byte value to a terminal color results in an
/// out-of-bounds error. It combines the invalid value with the expected range
/// of values. The following ranges occur in practice:
///
///   * `0..=5` for individual coordinates of the embedded RGB cube;
///   * `0..=15` for the 16 extended ANSI colors;
///   * `16..=215` for the 8-bit values of the embedded RGB cube;
///   * `232..=255` for the 24-step gray gradient.
#[derive(Clone, Debug)]
pub struct OutOfBoundsError {
    pub value: u32,
    pub expected: RangeInclusive<u8>,
}

impl OutOfBoundsError {
    /// Create a new out-of-bounds error from an unsigned byte value. This
    /// constructor takes care of the common case where the value has the
    /// smallest unsigned integer type.
    pub const fn from_u8(value: u8, expected: RangeInclusive<u8>) -> Self {
        Self {
            value: value as u32,
            expected,
        }
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

impl std::error::Error for OutOfBoundsError {}

// ====================================================================================================================
// Ansi Color
// ====================================================================================================================

/// The 16 extended ANSI colors.
///
/// Despite their names, *white* and *bright black* are obviously distinct from
/// white and black, respectively. Both are gray. *White* is closer to *bright
/// white* than to either shade named black. *Bright black* is closer to *black*
/// than either shade named white. In other words, the 16 extended ANSI colors
/// include a four-color gray gradient from *black* to *bright black* to *white*
/// to *bright white*.
///
/// With [`EightBitColor`]  composing [`AnsiColor`], [`EmbeddedRgb`], and
/// [`GrayGradient`] to represent 8-bit terminal colors, all four support
/// conversions from and to `u8`. In particular, this crate provides
/// implementations of the infallible
/// [`From<AnsiColor>`](enum.AnsiColor.html#impl-From%3CAnsiColor%3E-for-u8),
/// [`From<EmbeddedRgb>`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-u8),
/// [`From<GrayGradient>`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-u8),
/// and
/// [`From<EightBitColor>`](enum.EightBitColor.html#impl-From%3CEightBitColor%3E-for-u8),
/// all for `u8`. In the other direction, it provides implementations of the
/// fallible
/// [`TryFrom<u8>`](enum.AnsiColor.html#impl-TryFrom%3Cu8%3E-for-AnsiColor) for
/// `AnsiColor`,
/// [`TryFrom<u8>`](struct.EmbeddedRgb.html#impl-TryFrom%3Cu8%3E-for-EmbeddedRgb)
/// for `EmbeddedRgb`, and
/// [`TryFrom<u8>`](struct.GrayGradient.html#impl-TryFrom%3Cu8%3E-for-GrayGradient)
/// for `GrayGradient`, as well as the infallible
/// [`From<u8>`](enum.EightBitColor.html#impl-From%3Cu8%3E-for-EightBitColor)
/// for `EightBitColor`.
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
/// Unlike [`TrueColor`] and [`super::Color`], this struct only implements
/// [`EmbeddedRgb.index`] but not `index_mut()`. The latter cannot ensure the
/// critical invariant that coordinates have range `0..=5`. While that could be
/// addressed with a newtype wrapping u8, the resulting notational overhead also
/// seems incommensurate with the benefits. Instead, this struct implements
/// [`EmbeddedRgb.update`] as setter. It may not be as quite as elegant, but it
/// sure works.
///
/// With [`EightBitColor`]  composing [`AnsiColor`], [`EmbeddedRgb`], and
/// [`GrayGradient`] to represent 8-bit terminal colors, all four support
/// conversions from and to `u8`. In particular, this crate provides
/// implementations of the infallible
/// [`From<AnsiColor>`](enum.AnsiColor.html#impl-From%3CAnsiColor%3E-for-u8),
/// [`From<EmbeddedRgb>`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-u8),
/// [`From<GrayGradient>`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-u8),
/// and
/// [`From<EightBitColor>`](enum.EightBitColor.html#impl-From%3CEightBitColor%3E-for-u8),
/// all for `u8`. In the other direction, it provides implementations of the
/// fallible
/// [`TryFrom<u8>`](enum.AnsiColor.html#impl-TryFrom%3Cu8%3E-for-AnsiColor) for
/// `AnsiColor`,
/// [`TryFrom<u8>`](struct.EmbeddedRgb.html#impl-TryFrom%3Cu8%3E-for-EmbeddedRgb)
/// for `EmbeddedRgb`, and
/// [`TryFrom<u8>`](struct.GrayGradient.html#impl-TryFrom%3Cu8%3E-for-GrayGradient)
/// for `GrayGradient`, as well as the infallible
/// [`From<u8>`](enum.EightBitColor.html#impl-From%3Cu8%3E-for-EightBitColor)
/// for `EightBitColor`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EmbeddedRgb([u8; 3]);

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

    /// Access the coordinates of the embedded RGB color.
    #[inline]
    pub const fn coordinates(&self) -> &[u8; 3] {
        &self.0
    }

    /// Update the named coordinate to the given value. This struct implements
    /// this method in lieu of `index_mut()`, which cannot enforce the invariant
    /// that coordinates must be between 0 and 5, inclusive.
    #[must_use = "method fails on out-of-bounds coordinates"]
    pub fn update(&mut self, index: Coordinate, value: u8) -> Result<(), OutOfBoundsError> {
        if value > 5 {
            Err(OutOfBoundsError::from_u8(value, 0..=5))
        } else {
            self.0[index as usize] = value;
            Ok(())
        }
    }
}

impl TryFrom<u8> for EmbeddedRgb {
    type Error = OutOfBoundsError;

    /// Try instantiating an embedded RGB color from an unsigned byte.
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < 16 || 231 < value {
            Err(OutOfBoundsError::from_u8(value, 16..=231))
        } else {
            let mut b = value - 16;
            let r = b / 36;
            b -= r * 36;
            let g = b / 6;
            b -= g * 6;

            Ok(Self([r, g, b]))
        }
    }
}

impl From<EmbeddedRgb> for u8 {
    /// Convert an embedded RGB color to an unsigned byte.
    fn from(value: EmbeddedRgb) -> u8 {
        let [r, g, b] = value.0;
        16 + 36 * r + 6 * g + b
    }
}

impl std::ops::Index<Coordinate> for EmbeddedRgb {
    type Output = u8;

    /// Access the named coordinate.
    fn index(&self, index: Coordinate) -> &Self::Output {
        &self.0[index as usize]
    }
}

// ====================================================================================================================
// Gray Gradient
// ====================================================================================================================

/// The 24-step gray gradient embedded in 8-bit terminal colors.
///
/// With [`EightBitColor`]  composing [`AnsiColor`], [`EmbeddedRgb`], and
/// [`GrayGradient`] to represent 8-bit terminal colors, all four support
/// conversions from and to `u8`. In particular, this crate provides
/// implementations of the infallible
/// [`From<AnsiColor>`](enum.AnsiColor.html#impl-From%3CAnsiColor%3E-for-u8),
/// [`From<EmbeddedRgb>`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-u8),
/// [`From<GrayGradient>`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-u8),
/// and
/// [`From<EightBitColor>`](enum.EightBitColor.html#impl-From%3CEightBitColor%3E-for-u8),
/// all for `u8`. In the other direction, it provides implementations of the
/// fallible
/// [`TryFrom<u8>`](enum.AnsiColor.html#impl-TryFrom%3Cu8%3E-for-AnsiColor) for
/// `AnsiColor`,
/// [`TryFrom<u8>`](struct.EmbeddedRgb.html#impl-TryFrom%3Cu8%3E-for-EmbeddedRgb)
/// for `EmbeddedRgb`, and
/// [`TryFrom<u8>`](struct.GrayGradient.html#impl-TryFrom%3Cu8%3E-for-GrayGradient)
/// for `GrayGradient`, as well as the infallible
/// [`From<u8>`](enum.EightBitColor.html#impl-From%3Cu8%3E-for-EightBitColor)
/// for `EightBitColor`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GrayGradient(u8);

impl GrayGradient {
    /// Instantiate a new gray gradient from the level value `0..=23`.
    pub const fn new(value: u32) -> Result<Self, OutOfBoundsError> {
        if value >= 24 {
            Err(OutOfBoundsError {
                value,
                expected: 0..=23,
            })
        } else {
            Ok(Self(value as u8))
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
            Ok(Self(value - 232))
        }
    }
}

impl From<GrayGradient> for u8 {
    /// Convert the gray gradient to an unsigned byte.
    fn from(value: GrayGradient) -> u8 {
        232 + value.0
    }
}

// ====================================================================================================================
// 8-bit Color
// ====================================================================================================================

/// 8-bit terminal colors combine ANSI colors, embedded RGB colors, and the gray
/// gradient.
///
/// With [`EightBitColor`]  composing [`AnsiColor`], [`EmbeddedRgb`], and
/// [`GrayGradient`] to represent 8-bit terminal colors, all four support
/// conversions from and to `u8`. In particular, this crate provides
/// implementations of the infallible
/// [`From<AnsiColor>`](enum.AnsiColor.html#impl-From%3CAnsiColor%3E-for-u8),
/// [`From<EmbeddedRgb>`](struct.EmbeddedRgb.html#impl-From%3CEmbeddedRgb%3E-for-u8),
/// [`From<GrayGradient>`](struct.GrayGradient.html#impl-From%3CGrayGradient%3E-for-u8),
/// and
/// [`From<EightBitColor>`](enum.EightBitColor.html#impl-From%3CEightBitColor%3E-for-u8),
/// all for `u8`. In the other direction, it provides implementations of the
/// fallible
/// [`TryFrom<u8>`](enum.AnsiColor.html#impl-TryFrom%3Cu8%3E-for-AnsiColor) for
/// `AnsiColor`,
/// [`TryFrom<u8>`](struct.EmbeddedRgb.html#impl-TryFrom%3Cu8%3E-for-EmbeddedRgb)
/// for `EmbeddedRgb`, and
/// [`TryFrom<u8>`](struct.GrayGradient.html#impl-TryFrom%3Cu8%3E-for-GrayGradient)
/// for `GrayGradient`, as well as the infallible
/// [`From<u8>`](enum.EightBitColor.html#impl-From%3Cu8%3E-for-EightBitColor)
/// for `EightBitColor`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EightBitColor {
    Ansi(AnsiColor),
    Rgb(EmbeddedRgb),
    Gray(GrayGradient),
}

impl EightBitColor {
    /// Determine whether this 8-bit color is an ANSI color.
    pub fn is_ansi(&self) -> bool {
        matches!(*self, Self::Ansi(_))
    }

    /// Access this 8-bit color as an ANSI color.
    pub fn ansi(&self) -> Option<AnsiColor> {
        if let Self::Ansi(color) = *self {
            Some(color)
        } else {
            None
        }
    }

    /// Determine whether this 8-bit color is an embedded RGB color.
    pub fn is_rgb(&self) -> bool {
        matches!(*self, Self::Rgb(_))
    }

    /// Access this 8-bit color as an embedded RGB color.
    pub fn rgb(&self) -> Option<EmbeddedRgb> {
        if let Self::Rgb(color) = *self {
            Some(color)
        } else {
            None
        }
    }

    /// Determine whether this 8-bit color is a gray gradient.
    pub fn is_gray(&self) -> bool {
        matches!(self, Self::Gray(_))
    }

    /// Access this 8-bit color as a gray gradient.
    pub fn gray(&self) -> Option<GrayGradient> {
        if let Self::Gray(color) = *self {
            Some(color)
        } else {
            None
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
// True Color (24-bit RGB)
// ====================================================================================================================

/// A true color, i.e., 24-bit color.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TrueColor([u8; 3]);

impl TrueColor {
    /// Create a new true color from its coordinates.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b])
    }

    /// Access the coordinates.
    #[inline]
    pub const fn coordinates(&self) -> &[u8; 3] {
        &self.0
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

        let [r, g, b] = *value.coordinates();
        Self([convert(r), convert(g), convert(b)])
    }
}

impl From<GrayGradient> for TrueColor {
    /// Instantiate a true color from a gray gradient value.
    fn from(value: GrayGradient) -> Self {
        let level = 8 + 10 * value.level();
        Self([level, level, level])
    }
}

impl std::ops::Index<Coordinate> for TrueColor {
    type Output = u8;

    /// Access the named coordinate.
    fn index(&self, index: Coordinate) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl std::ops::IndexMut<Coordinate> for TrueColor {
    /// Mutably access the named coordinate.
    fn index_mut(&mut self, index: Coordinate) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

// ====================================================================================================================
// Fidelity
// ====================================================================================================================

/// Terminal fidelity.
// #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
// pub enum Fidelity {
//     /// The equivalent of true color.
//     FullColor,
//     /// The equivalent of 8-bit color.
//     ReducedColor,
//     /// The equivalent of ANSI colors.
//     MinimalColor,
//     /// No colors, but ANSI escape codes are fine.
//     NoColor,
//     /// No colors, no ANSI escape codes.
//     None,
// }

// ====================================================================================================================

#[cfg(test)]
mod test {
    use super::{AnsiColor, EightBitColor, EmbeddedRgb, GrayGradient, OutOfBoundsError};

    #[test]
    fn test_conversion() -> Result<(), OutOfBoundsError> {
        let magenta = AnsiColor::Magenta;
        assert_eq!(magenta as u8, 5);

        let green = EmbeddedRgb::new(0, 4, 0)?;
        assert_eq!(green.coordinates(), &[0, 4, 0]);

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
        assert_eq!(*black_rgb.coordinates(), [0_u8, 0_u8, 0_u8]);
        assert_eq!(u8::from(black_rgb), 16);
        let white_rgb = EmbeddedRgb::try_from(231)?;
        assert_eq!(*white_rgb.coordinates(), [5_u8, 5_u8, 5_u8]);
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
