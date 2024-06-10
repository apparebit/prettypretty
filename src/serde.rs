// ====================================================================================================================
// Parse Color from String
// ====================================================================================================================

use std::error::Error;

use crate::ColorSpace;

/// An erroneous color format. Several variants include a coordinate index,
/// which is zero-based. The description, however, shows a one-based index
/// prefixed with a #.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColorFormatError {
    /// A color format that does not start with a known prefix such as `#` or
    /// `rgb:`.
    UnknownFormat,

    /// A color format with unexpected characters or an unexpected number of
    /// characters. For example, `#00` is missing a hexadecimal digit, whereas
    /// `#💩00` has the correct length but contains an unsuitable character.
    UnexpectedCharacters,

    /// A color format that is missing the coordinate with the given index. For
    /// example, `rgb:0` is missing the second and third coordinate, whereas
    /// `rgb:0//0` is missing the second coordinate only.
    MissingCoordinate(usize),

    /// A color format that has too many digits in the coordinate with the given
    /// index. For example, `rgb:12345/1/22` has too many digits in the first
    /// coordinate.
    OversizedCoordinate(usize),

    /// A color format that has a malformed number as coordinate with the given
    /// index. For example, `#efg` has a malformed third coordinate.
    MalformedCoordinate(usize, std::num::ParseIntError),

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
            MalformedCoordinate(c, _) => write!(
                f,
                "color format coordinates should be valid numbers but #{} is not",
                c + 1
            ),
            TooManyCoordinates => write!(f, "color format should have 3 coordinates but has more"),
        }
    }
}

impl Error for ColorFormatError {
    /// Access the cause for this color format error.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let ColorFormatError::MalformedCoordinate(_, error) = self {
            Some(error)
        } else {
            None
        }
    }
}

/// Parse a 24-bit color in hashed hexadecimal format. If successful, this
/// function returns the three coordinates as unsigned bytes. It transparently
/// handles single-digit coordinates.
fn parse_hashed(s: &str) -> Result<[u8; 3], ColorFormatError> {
    if !s.starts_with('#') {
        return Err(ColorFormatError::UnknownFormat);
    } else if s.len() != 4 && s.len() != 7 {
        return Err(ColorFormatError::UnexpectedCharacters);
    }

    fn parse_coordinate(s: &str, index: usize) -> Result<u8, ColorFormatError> {
        let factor = s.len() / 3;
        let t = s
            .get(1 + factor * index..1 + factor * (index + 1))
            .ok_or(ColorFormatError::UnexpectedCharacters)?;
        let n = u8::from_str_radix(t, 16)
            .map_err(|e| ColorFormatError::MalformedCoordinate(index, e))?;

        Ok(if factor == 1 { 16 * n + n } else { n })
    }

    let c1 = parse_coordinate(s, 0)?;
    let c2 = parse_coordinate(s, 1)?;
    let c3 = parse_coordinate(s, 2)?;
    Ok([c1, c2, c3])
}

/// Parse a color in X Windows format. If successful, this function returns
/// three pairs with the number of hexadecimal digits and the numeric value for
/// each coordinate.
fn parse_x(s: &str) -> Result<[(u8, u16); 3], ColorFormatError> {
    if !s.starts_with("rgb:") {
        return Err(ColorFormatError::UnknownFormat);
    }

    fn parse_coordinate(s: Option<&str>, index: usize) -> Result<(u8, u16), ColorFormatError> {
        let t = s.ok_or_else(|| ColorFormatError::MissingCoordinate(index))?;
        if t.len() == 0 {
            return Err(ColorFormatError::MissingCoordinate(index));
        } else if t.len() > 4 {
            return Err(ColorFormatError::OversizedCoordinate(index));
        }

        let n = u16::from_str_radix(t, 16)
            .map_err(|e| ColorFormatError::MalformedCoordinate(index, e))?;

        Ok((t.len() as u8, n))
    }

    // SAFETY: unwrap() is safe because we tested for just that prefix above.
    let mut iter = s.strip_prefix("rgb:").unwrap().split('/');
    let c1 = parse_coordinate(iter.next(), 0)?;
    let c2 = parse_coordinate(iter.next(), 1)?;
    let c3 = parse_coordinate(iter.next(), 2)?;
    if iter.next().is_some() {
        return Err(ColorFormatError::TooManyCoordinates);
    }

    Ok([c1, c2, c3])
}

/// Parse the given string as a color in hashed hexadecimal or X Windows format.
pub(crate) fn parse(s: &str) -> Result<(ColorSpace, [f64; 3]), ColorFormatError> {
    if s.starts_with('#') {
        let [c1, c2, c3] = parse_hashed(s)?;
        Ok((
            ColorSpace::Srgb,
            [c1 as f64 / 255.0, c2 as f64 / 255.0, c3 as f64 / 255.0],
        ))
    } else if s.starts_with("rgb:") {
        fn scale(coordinate: (u8, u16)) -> f64 {
            coordinate.1 as f64 / (16_i32.pow(coordinate.0 as u32) - 1) as f64
        }

        let [c1, c2, c3] = parse_x(s)?;

        Ok((ColorSpace::Srgb, [scale(c1), scale(c2), scale(c3)]))
    } else {
        Err(ColorFormatError::UnknownFormat)
    }
}

#[cfg(test)]
mod test {
    use super::{parse_hashed, parse_x, ColorFormatError};

    #[test]
    fn test_parse_hashed() -> Result<(), ColorFormatError> {
        assert_eq!(parse_hashed("#123")?, [0x11_u8, 0x22, 0x33]);
        assert_eq!(parse_hashed("#112233")?, [0x11_u8, 0x22, 0x33]);
        assert_eq!(parse_hashed("fff"), Err(ColorFormatError::UnknownFormat));
        assert_eq!(
            parse_hashed("#ff"),
            Err(ColorFormatError::UnexpectedCharacters)
        );
        assert_eq!(
            parse_hashed("#💩00"),
            Err(ColorFormatError::UnexpectedCharacters)
        );

        let result = parse_hashed("#0g0");
        assert!(matches!(
            result,
            Err(ColorFormatError::MalformedCoordinate(1, _))
        ));

        let result = parse_hashed("#00g");
        assert!(matches!(
            result,
            Err(ColorFormatError::MalformedCoordinate(2, _))
        ));

        Ok(())
    }

    #[test]
    fn test_parse_x() -> Result<(), ColorFormatError> {
        assert_eq!(
            parse_x("rgb:a/bb/ccc")?,
            [(1_u8, 0xa_u16), (2, 0xbb), (3, 0xccc)]
        );
        assert_eq!(
            parse_x("rgb:0123/4567/89ab")?,
            [(4_u8, 0x123_u16), (4, 0x4567), (4, 0x89ab)]
        );
        assert_eq!(
            parse_x("rgbi:0.1/0.1/0.1"),
            Err(ColorFormatError::UnknownFormat)
        );
        assert_eq!(
            parse_x("rgb:0"),
            Err(ColorFormatError::MissingCoordinate(1))
        );
        assert_eq!(
            parse_x("rgb:0//2"),
            Err(ColorFormatError::MissingCoordinate(1))
        );
        assert_eq!(
            parse_x("rgb:1/12345/1"),
            Err(ColorFormatError::OversizedCoordinate(1))
        );
        assert_eq!(
            parse_x("rgb:1/2/3/4"),
            Err(ColorFormatError::TooManyCoordinates)
        );

        let result = parse_x("rgb:f/g/f");
        assert!(matches!(
            result,
            Err(ColorFormatError::MalformedCoordinate(1, _))
        ));

        Ok(())
    }
}
