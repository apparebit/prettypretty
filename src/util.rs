/// A safe, symbolic index for the three color coordinates.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Coordinate {
    C1 = 0,
    C2 = 1,
    C3 = 2,
}

/// Determine whether the two floating point numbers are almost equal. This
/// function returns `true` if both arguments are not-a-number, have the same
/// value, or have the same value after rounding the 15th decimal. In other
/// words, this function treats not-a-number as comparable and tolerates a small
/// absolute error for numbers.
#[allow(dead_code)]
pub(crate) fn almost_eq(n1: f64, n2: f64) -> bool {
    if n1.is_nan() {
        return n2.is_nan();
    } else if n2.is_nan() {
        return false;
    } else if n1 == n2 {
        return true;
    }

    let factor = 10.0_f64.powi((f64::DIGITS as i32) - 1);
    (factor * n1).round() == (factor * n2).round()
}

// ====================================================================================================================
// Errors
// ====================================================================================================================

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
    pub expected: std::ops::RangeInclusive<u8>,
}

impl OutOfBoundsError {
    /// Create a new out-of-bounds error from an unsigned byte value. This
    /// constructor takes care of the common case where the value has the
    /// smallest unsigned integer type.
    pub const fn from_u8(value: u8, expected: std::ops::RangeInclusive<u8>) -> Self {
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

// --------------------------------------------------------------------------------------------------------------------

/// An erroneous color format.
///
/// Several variants include a coordinate index, which is zero-based. The
/// formatted description, however, shows a one-based index prefixed with a `#`
/// (for number).
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
    MissingCoordinate(usize),

    /// A color format that has too many digits in the coordinate with the given
    /// index. For example, `rgb:12345/1/22` has too many digits in the first
    /// coordinate.
    OversizedCoordinate(usize),

    /// A color format that has a malformed hexadecimal number as coordinate
    /// with the given index. For example, `#efg` has a malformed third
    /// coordinate.
    MalformedHex(usize, std::num::ParseIntError),

    /// A color format that has a malformed floating point number as coordinate
    /// with the given index. For example, `color(srgb 1.0 0..1 0.0)` has a
    /// malformed second coordinate.
    MalformedFloat(usize, std::num::ParseFloatError),

    /// A color format with more than three coordinates. For example,
    /// `rgb:1/2/3/4` has one coordinate too many.
    TooManyCoordinates,
}

impl std::fmt::Display for ColorFormatError {
    /// Format a description of this color format error.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use ColorFormatError::*;

        match *self {
            UnknownFormat => write!(f, "color format should start with '#' or 'rgb:'"),
            UnexpectedCharacters => {
                write!(f, "color format should contain only valid ASCII characters")
            }
            NoOpeningParenthesis => write!(
                f,
                "color format should include an opening parenthesis but has none"
            ),
            NoClosingParenthesis => write!(
                f,
                "color format should include a closing parenthesis but has none"
            ),
            UnknownColorSpace => {
                write!(f, "color format should have known color space but does not")
            }
            MissingCoordinate(c) => write!(
                f,
                "color format should have 3 coordinates but is missing #{}",
                c + 1
            ),
            OversizedCoordinate(c) => write!(
                f,
                "color format coordinates should have 1-4 digits but #{} has more",
                c + 1
            ),
            MalformedHex(c, _) => write!(
                f,
                "color format coordinates should be hexadecimal integers but #{} is not",
                c + 1
            ),
            MalformedFloat(c, _) => write!(
                f,
                "color format coordinates should be floating point numbers but #{} is not",
                c + 1
            ),
            TooManyCoordinates => write!(f, "color format should have 3 coordinates but has more"),
        }
    }
}

impl std::error::Error for ColorFormatError {
    /// Access the cause for this color format error.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ColorFormatError::MalformedHex(_, error) => Some(error),
            ColorFormatError::MalformedFloat(_, error) => Some(error),
            _ => None,
        }
    }
}

// --------------------------------------------------------------------------------------------------------------------

/// An error wrapper. This enumeration wraps this crate's two, more fundamental
/// errors, one for out-of-bounds numbers and one for malformed strings. It also
/// defines the corresponding `From` traits, so that code using this crate can
/// just use this error.
#[derive(Clone, Debug)]
pub enum Error {
    Number(OutOfBoundsError),
    String(ColorFormatError),
}

impl std::fmt::Display for Error {
    /// Format this error. This method delegates to the wrapped error.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Number(err) => err.fmt(f),
            Error::String(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    /// Get the source for this error. This method delegates to the wrapped
    /// error.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Number(err) => err.source(),
            Error::String(err) => err.source(),
        }
    }
}

impl From<OutOfBoundsError> for Error {
    /// Wrap an out-of-bounds error.
    fn from(err: OutOfBoundsError) -> Self {
        Self::Number(err)
    }
}

impl From<ColorFormatError> for Error {
    /// Wrap a color format error.
    fn from(err: ColorFormatError) -> Self {
        Self::String(err)
    }
}
