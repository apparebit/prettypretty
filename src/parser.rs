use super::color::ColorSpace;
use super::util::ColorFormatError;

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
        let n = u8::from_str_radix(t, 16).map_err(|e| ColorFormatError::MalformedHex(index, e))?;

        Ok(if factor == 1 { 16 * n + n } else { n })
    }

    let c1 = parse_coordinate(s, 0)?;
    let c2 = parse_coordinate(s, 1)?;
    let c3 = parse_coordinate(s, 2)?;
    Ok([c1, c2, c3])
}

// --------------------------------------------------------------------------------------------------------------------

/// Parse a color in X Windows format. If successful, this function returns
/// three pairs with the number of hexadecimal digits and the numeric value for
/// each coordinate.
fn parse_x(s: &str) -> Result<[(u8, u16); 3], ColorFormatError> {
    if !s.starts_with("rgb:") {
        return Err(ColorFormatError::UnknownFormat);
    }

    fn parse_coordinate(s: Option<&str>, index: usize) -> Result<(u8, u16), ColorFormatError> {
        let t = s.ok_or(ColorFormatError::MissingCoordinate(index))?;
        if t.is_empty() {
            return Err(ColorFormatError::MissingCoordinate(index));
        } else if t.len() > 4 {
            return Err(ColorFormatError::OversizedCoordinate(index));
        }

        let n = u16::from_str_radix(t, 16).map_err(|e| ColorFormatError::MalformedHex(index, e))?;
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

const COLOR_SPACES: [(&str, ColorSpace); 10] = [
    ("srgb", ColorSpace::Srgb),
    ("linear-srgb", ColorSpace::LinearSrgb),
    ("display-p3", ColorSpace::DisplayP3),
    ("--linear-display-p3", ColorSpace::LinearDisplayP3),
    ("rec2020", ColorSpace::Rec2020),
    ("--linear-rec2020", ColorSpace::LinearRec2020),
    ("--oklrab", ColorSpace::Oklrab),
    ("--oklrch", ColorSpace::Oklrch),
    ("xyz", ColorSpace::Xyz),
    ("xyz-d65", ColorSpace::Xyz),
];

/// Parse a subset of valid CSS color formats. This function recognizes only the
/// `oklab()`, `oklch()`, and `color()` functions. The color space for the
/// latter must be `srgb`, `linear-srgb`, `display-p3`, `rec2020`, `xyz`, or one
/// of the non-standard color spaces `--linear-display-p3`, `--linear-rec2020`,
/// `--oklrab`, and `--oklrch`. Coordinates must not have units including `%`.
fn parse_css(s: &str) -> Result<(ColorSpace, [f64; 3]), ColorFormatError> {
    use ColorSpace::*;

    // Munge CSS function name
    let (space, rest) = s
        .strip_prefix("oklab")
        .map(|r| (Some(Oklab), r))
        .or_else(|| s.strip_prefix("oklch").map(|r| (Some(Oklch), r)))
        .or_else(|| s.strip_prefix("color").map(|r| (None, r)))
        .ok_or(ColorFormatError::UnknownFormat)?;

    // Munge parentheses after trimming leading whitespace
    let rest = rest
        .trim_start()
        .strip_prefix('(')
        .ok_or(ColorFormatError::NoOpeningParenthesis)
        .and_then(|rest| {
            rest.strip_suffix(')')
                .ok_or(ColorFormatError::NoClosingParenthesis)
        })?;

    let (space, body) = if let Some(s) = space {
        (s, rest) // Pass through
    } else {
        // Munge color space
        let rest = rest.trim_start();
        COLOR_SPACES
            .iter()
            .filter_map(|(p, s)| rest.strip_prefix(p).map(|r| (*s, r)))
            .next() // Take first (and only) result
            .ok_or(ColorFormatError::UnknownColorSpace)?
    };

    #[inline]
    fn parse_coordinate(s: Option<&str>, index: usize) -> Result<f64, ColorFormatError> {
        s.ok_or(ColorFormatError::MissingCoordinate(index))
            .and_then(|t| {
                t.parse()
                    .map_err(|e| ColorFormatError::MalformedFloat(index, e))
            })
    }

    // Munge coordinates. Iterator eats all leading or trailing white space.
    let mut iter = body.split_whitespace();
    let c1 = parse_coordinate(iter.next(), 0)?;
    let c2 = parse_coordinate(iter.next(), 1)?;
    let c3 = parse_coordinate(iter.next(), 2)?;
    if iter.next().is_some() {
        return Err(ColorFormatError::TooManyCoordinates);
    }

    Ok((space, [c1, c2, c3]))
}

// --------------------------------------------------------------------------------------------------------------------

/// Parse the given string as a color in hashed hexadecimal, X Windows, or CSS
/// format. Before trying to parse the string, this function trims leading and
/// trailing whitespace and converts ASCII characters to lower case. Note that a
/// valid string may still contain Unicode white space characters.
pub fn parse(s: &str) -> Result<(ColorSpace, [f64; 3]), ColorFormatError> {
    let lowercase = s.trim().to_ascii_lowercase(); // Keep around for fn scope
    let s = lowercase.as_str();

    if s.starts_with('#') {
        let [c1, c2, c3] = parse_hashed(s)?;
        Ok((
            ColorSpace::Srgb,
            [c1 as f64 / 255.0, c2 as f64 / 255.0, c3 as f64 / 255.0],
        ))
    } else if s.starts_with("rgb:") {
        fn scale(len_and_value: (u8, u16)) -> f64 {
            len_and_value.1 as f64 / (16_i32.pow(len_and_value.0 as u32) - 1) as f64
        }

        let [c1, c2, c3] = parse_x(s)?;
        Ok((ColorSpace::Srgb, [scale(c1), scale(c2), scale(c3)]))
    } else {
        parse_css(s)
    }
}

// ====================================================================================================================

#[cfg(test)]
mod test {
    use super::ColorSpace::*;
    use super::{parse, parse_css, parse_hashed, parse_x, ColorFormatError};

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
        assert!(matches!(result, Err(ColorFormatError::MalformedHex(1, _))));

        let result = parse_hashed("#00g");
        assert!(matches!(result, Err(ColorFormatError::MalformedHex(2, _))));

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
        assert!(matches!(result, Err(ColorFormatError::MalformedHex(1, _))));

        assert_eq!(
            parse("   RGB:00/55/aa   ")?,
            (Srgb, [0.0_f64, 0.33333333333333333, 0.6666666666666666])
        );

        Ok(())
    }

    #[test]
    fn test_parse_css() {
        assert_eq!(parse_css("oklab(0 0 0)"), Ok((Oklab, [0.0, 0.0, 0.0])));
        assert_eq!(
            parse_css("color(xyz   1  1  1)"),
            Ok((Xyz, [1.0, 1.0, 1.0]))
        );
        assert_eq!(
            parse_css("color(  --oklrch   1  1  1)"),
            Ok((Oklrch, [1.0, 1.0, 1.0]))
        );
        assert_eq!(
            parse_css("color  (  --linear-display-p3   1  1.123  0.3333   )"),
            Ok((LinearDisplayP3, [1.0, 1.123, 0.3333]))
        );
        assert_eq!(
            parse_css("whatever(1 1 1)"),
            Err(ColorFormatError::UnknownFormat)
        );
        assert_eq!(
            parse_css("colorsrgb 1 1 1)"),
            Err(ColorFormatError::NoOpeningParenthesis)
        );
        assert_eq!(
            parse_css("color(srgb 1 1 1"),
            Err(ColorFormatError::NoClosingParenthesis)
        );
        assert_eq!(
            parse_css("color(nemo 1 1 1)"),
            Err(ColorFormatError::UnknownColorSpace)
        );
        assert!(matches!(
            parse_css("color(srgb abc 1 1)"),
            Err(ColorFormatError::MalformedFloat(0, _))
        ));
        assert_eq!(
            parse_css("color(srgb 1)"),
            Err(ColorFormatError::MissingCoordinate(1))
        );
        assert_eq!(
            parse_css("color(srgb 1 1 1 1)"),
            Err(ColorFormatError::TooManyCoordinates)
        );

        assert_eq!(
            parse("   COLOR(  --linear-display-p3   1  1.123  0.3333   )    "),
            Ok((LinearDisplayP3, [1.0, 1.123, 0.3333]))
        );
        assert_eq!(
            parse("  color( --Linear-Display-P3  1  1.123  0.3333 )  "),
            Ok((LinearDisplayP3, [1.0, 1.123, 0.3333]))
        );
    }
}
