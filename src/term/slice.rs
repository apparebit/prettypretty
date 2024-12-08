//! Utility module to bridge the gap between `str` and `[u8]`.

/// A choice of radix for converting byte slices to integers.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Radix {
    Decimal = 10,
    #[allow(dead_code)]
    Hexadecimal = 16,
}

#[inline]
const fn to_digit(byte: u8, radix: u32) -> Option<u32> {
    (byte as char).to_digit(radix)
}

fn parse(bytes: &[u8], radix: u32) -> Option<u64> {
    let max_length = match radix {
        10 => 10,
        16 => 8,
        _ => panic!("radix {} is neither 10 nor 16", radix),
    };

    if bytes.is_empty() || max_length < bytes.len() {
        return None;
    }

    let mut result = 0;
    for byte in bytes.iter() {
        let digit = to_digit(*byte, radix)?;
        result = (radix as u64) * result + digit as u64;
    }

    Some(result)
}

/// An extension trait for byte slices.
pub(crate) trait SliceExt {
    /// Trim ASCII whitespace from both sides.
    fn trim(&self) -> &Self;

    /// Strip either BEL or ST from end.
    fn strip_bel_st_suffix(&self) -> Option<&Self>;

    /// Convert to some owned bytes.
    fn to_some_owned_bytes(&self) -> Option<Vec<u8>>;

    /// Convert to a `u32`.
    fn to_u32(&self, radix: Radix) -> Option<u32>;

    /// Convert to a `u16`.
    fn to_u16(&self, radix: Radix) -> Option<u16>;

    /// Convert to a `u8`.
    #[allow(dead_code)]
    fn to_u8(&self, radix: Radix) -> Option<u8>;
}

impl SliceExt for [u8] {
    fn trim(&self) -> &Self {
        let start = match self.iter().position(|b| !b.is_ascii_whitespace()) {
            Some(index) => index,
            None => return &[],
        };

        let stop = self.iter().rposition(|b| !b.is_ascii_whitespace()).unwrap();
        &self[start..=stop]
    }

    fn strip_bel_st_suffix(&self) -> Option<&Self> {
        self.strip_suffix(b"\x07")
            .or_else(|| self.strip_suffix(b"\x1b\\"))
    }

    fn to_some_owned_bytes(&self) -> Option<Vec<u8>> {
        if self.is_empty() {
            None
        } else {
            Some(self.to_owned())
        }
    }

    fn to_u32(&self, radix: Radix) -> Option<u32> {
        u32::try_from(parse(self, radix as u32)?).ok()
    }

    fn to_u16(&self, radix: Radix) -> Option<u16> {
        u16::try_from(parse(self, radix as u32)?).ok()
    }

    fn to_u8(&self, radix: Radix) -> Option<u8> {
        u8::try_from(parse(self, radix as u32)?).ok()
    }
}

/// Determine whether the byte is a semicolon or colon.
pub(crate) fn is_semi_colon(b: &u8) -> bool {
    *b == b';' || *b == b':'
}

#[cfg(test)]
mod test {
    use super::{Radix, SliceExt};

    #[test]
    fn test_slice_ext() {
        assert_eq!(b"665".to_u32(Radix::Decimal).unwrap(), 665);
        assert_eq!(b"665".to_u32(Radix::Hexadecimal).unwrap(), 1_637);
        assert_eq!(b"".to_some_owned_bytes(), None);
        assert_eq!(b"665".to_some_owned_bytes(), Some(vec!(b'6', b'6', b'5')));
        assert_eq!(b"".trim(), b"");
        assert_eq!(b" \r\n \t ".trim(), b"");
        assert_eq!(b"  space  ".trim(), b"space");
        assert_eq!(b"space".trim(), b"space");
        assert_eq!(b"boo\x07".strip_bel_st_suffix(), Some(b"boo".as_slice()));
        assert_eq!(b"boo\x1b\\".strip_bel_st_suffix(), Some(b"boo".as_slice()));
    }
}
