//! Utility module with prettypretty's errors.

#[cfg(feature = "pyffi")]
use pyo3::{exceptions::PyValueError, prelude::*};

/// An out-of-bounds error.
///
/// This error indicates an index value that is out of bounds for some range.
/// The ranges used by this crate include:
///
///   * `0..=5` for coordinates of [`EmbeddedRgb`](crate::termco::EmbeddedRgb);
///   * `0..=15` for index values of [`AnsiColor`](crate::termco::AnsiColor);
///   * `0..=23` for the gay levels of [`GrayGradient`](crate::termco::GrayGradient);
///   * `16..=231` for index values of [`EmbeddedRgb`](crate::termco::EmbeddedRgb);
///   * `232..=255` for index values of [`GrayGradient`](crate::termco::GrayGradient).
///
#[derive(Clone, Debug)]
pub struct OutOfBoundsError {
    pub value: usize,
    pub expected: std::ops::RangeInclusive<usize>,
}

impl OutOfBoundsError {
    /// Create a new out-of-bounds error.
    pub fn new(value: impl Into<usize>, expected: std::ops::RangeInclusive<usize>) -> Self {
        Self {
            value: value.into(),
            expected,
        }
    }
}

impl std::fmt::Display for OutOfBoundsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{} does not fit into range {}..={}",
            self.value,
            self.expected.start(),
            self.expected.end()
        ))
    }
}

#[cfg(feature = "pyffi")]
impl From<OutOfBoundsError> for PyErr {
    fn from(value: OutOfBoundsError) -> Self {
        pyo3::exceptions::PyIndexError::new_err(value.to_string())
    }
}

// ====================================================================================================================

/// An erroneous color format.
///
/// The enumeration started out with additional information but PyO3 only
/// supports unit variants without associated state. Thankfully, the attendant
/// loss of information is rather limited.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColorFormatError {
    /// A color format that does not start with a known prefix such as `#` or
    /// `rgb:`.
    UnknownFormat,

    /// A color format with unexpected characters or an unexpected number of
    /// characters. For example, `#00` is missing a hexadecimal digit, whereas
    /// `#💩00` has the correct length but contains an unsuitable character.
    UnexpectedCharacters,

    /// A parenthesized color format without the opening parenthesis. For
    /// example, `color display-p3 0 0 0)` is missing the opening parenthesis.
    NoOpeningParenthesis,

    /// A parenthesized color format without the closing parenthesis. For
    /// example, `oklab(1 2 3` is missing the closing parenthesis.
    NoClosingParenthesis,

    /// A color format that is using an unknown color space. For example,
    /// `color(unknown 1 1 1)` uses an unknown color space.
    UnknownColorSpace,

    /// A color format that is missing the coordinate with the given index. For
    /// example, `rgb:0` is missing the second and third coordinate, whereas
    /// `rgb:0//0` is missing the second coordinate only.
    MissingCoordinate,

    /// A color format that has too many digits in the coordinate with the given
    /// index. For example, `rgb:12345/1/22` has too many digits in the first
    /// coordinate.
    OversizedCoordinate,

    /// A color format that has a malformed hexadecimal number as coordinate
    /// with the given index. For example, `#efg` has a malformed third
    /// coordinate.
    MalformedHex,

    /// A color format that has a malformed floating point number as coordinate
    /// with the given index. For example, `color(srgb 1.0 0..1 0.0)` has a
    /// malformed second coordinate.
    MalformedFloat,

    /// A color format with more than three coordinates. For example,
    /// `rgb:1/2/3/4` has one coordinate too many.
    TooManyCoordinates,

    /// An OSC escape sequence with a color is for another theme entry.
    WrongThemeColor,

    /// A OSC escape sequence with a color is malformed.
    MalformedThemeColor,
}

impl std::fmt::Display for ColorFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use ColorFormatError::*;

        match self {
            UnknownFormat => f.write_str(
                "color format should start with `#`, `color()`, `oklab()`, `oklch()`, or `rgb:`",
            ),
            UnexpectedCharacters => {
                f.write_str("color format should contain only valid ASCII characters")
            }
            NoOpeningParenthesis => {
                f.write_str("color format should include an opening parenthesis but has none")
            }
            NoClosingParenthesis => {
                f.write_str("color format should include a closing parenthesis but has none")
            }
            UnknownColorSpace => {
                f.write_str("color format should have known color space but does not")
            }
            MissingCoordinate => {
                f.write_str("color format should have 3 coordinates but is missing one")
            }
            OversizedCoordinate => {
                f.write_str("color format coordinates should have 1-4 hex digits but one has more")
            }
            MalformedHex => {
                f.write_str("color format coordinates should be hexadecimal integers but are not")
            }
            MalformedFloat => {
                f.write_str("color format coordinates should be floating point numbers but are not")
            }
            TooManyCoordinates => {
                f.write_str("color format should have 3 coordinates but has more")
            }
            WrongThemeColor => {
                f.write_str("OSC escape sequence with color is for the wrong theme entry")
            }
            MalformedThemeColor => f.write_str("OSC escape sequence with color is malformed"),
        }
    }
}

impl std::error::Error for ColorFormatError {}

#[cfg(feature = "pyffi")]
impl From<ColorFormatError> for PyErr {
    fn from(value: ColorFormatError) -> Self {
        PyValueError::new_err(value.to_string())
    }
}

// ====================================================================================================================

/// An error indicating a colorant wrapping a high-resolution color.
///
/// No standard exists for displaying high-resolution colors in terminals. Hence
/// colorants wrapping high-resolution colors cannot be displayed as ANSI escape
/// sequences.
#[derive(Clone, Copy, Debug)]
pub struct HiResColorantError;

impl std::fmt::Display for HiResColorantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("unable to format high-resolution colorant as ANSI escape sequence")
    }
}

impl std::error::Error for HiResColorantError {}

#[cfg(feature = "pyffi")]
impl From<HiResColorantError> for PyErr {
    fn from(value: HiResColorantError) -> Self {
        PyValueError::new_err(value.to_string())
    }
}

// ====================================================================================================================

use crate::theme::ThemeEntry;

/// The kinds of errors while querying a terminal for its color theme.
#[derive(Clone, Copy, Debug)]
pub enum ThemeErrorKind {
    AccessDevice,
    WriteQuery(ThemeEntry),
    ScanEscape(ThemeEntry),
    ParseColor(ThemeEntry),
}

/// An error while querying the terminal for its color theme.
#[derive(Debug)]
pub struct ThemeError {
    kind: ThemeErrorKind,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl ThemeError {
    /// Create a new theme error.
    pub fn new(kind: ThemeErrorKind, source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self {
            kind,
            source: Some(source),
        }
    }
}

impl From<ThemeError> for std::io::Error {
    fn from(value: ThemeError) -> Self {
        std::io::Error::other(value)
    }
}

impl std::fmt::Display for ThemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let entry = match self.kind {
            ThemeErrorKind::AccessDevice => return f.write_str("could not access terminal device"),
            ThemeErrorKind::WriteQuery(entry) => {
                f.write_str("could not write query for ")?;
                entry
            }
            ThemeErrorKind::ScanEscape(entry) => {
                f.write_str("could not parse ANSI escape sequence for ")?;
                entry
            }
            ThemeErrorKind::ParseColor(entry) => {
                f.write_str("could not parse color for ")?;
                entry
            }
        };
        f.write_str(entry.name())
    }
}

impl std::error::Error for ThemeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref().map(|e| e as _)
    }
}
