//! Helper module with utilities for byte strings.

use std::fmt;
use std::io::Write;

/// Nicely format a byte.
fn format_nicely<W>(byte: u8, output: &mut W) -> Result<usize, fmt::Error>
where
    W: fmt::Write + ?Sized,
{
    if (0x20..=0x7e).contains(&byte) {
        output.write_char(byte as char)?;
        return Ok(1);
    }

    let replacement = match byte {
        // Ensure that letters are math sans-serif Unicode letters. In UTF-8, ‹›
        // are 3 bytes each and the math letters are 4 bytes each.
        0x00 => "‹NUL›",
        0x01 => "‹SOH›",
        0x02 => "‹STX›",
        0x03 => "‹ETX›",
        0x04 => "‹EOT›",
        0x05 => "‹ENQ›",
        0x06 => "‹ACK›",
        0x07 => "‹BEL›",
        0x08 => "‹BS›",
        0x09 => "‹HT›",
        0x0a => "‹LF›",
        0x0b => "‹VT›",
        0x0c => "‹FF›",
        0x0d => "‹CR›",
        0x0e => "‹SO›",
        0x0f => "‹SI›",
        0x10 => "‹DLE›",
        0x11 => "‹DC1›",
        0x12 => "‹DC2›",
        0x13 => "‹DC3›",
        0x14 => "‹DC4›",
        0x15 => "‹NAK›",
        0x16 => "‹SYN›",
        0x17 => "‹ETB›",
        0x18 => "‹CAN›",
        0x19 => "‹EM›",
        0x1a => "‹SUB›",
        0x1b => "‹ESC›",
        0x1c => "‹FS›",
        0x1d => "‹GS›",
        0x1e => "‹RS›",
        0x1f => "‹US›",

        0x7f => "‹DEL›",

        0x90 => "‹DCS›",
        0x98 => "‹SOS›",
        0x9b => "‹CSI›",
        0x9c => "‹ST›",
        0x9d => "‹OSC›",
        0x9e => "‹PM›",
        0x9f => "‹APC›",

        _ => "",
    };
    if !replacement.is_empty() {
        output.write_str(replacement)?;
        return Ok(replacement.len() - 6 + 2);
    }

    output.write_fmt(format_args!("「{:02X}」", byte))?;
    Ok(4)
}

/// Write bytes nicely.
///
/// This trait adds two methods for formatting bytes. Conveniently, both methods
/// have default implementations in terms of the [`Write`] supertrait. For that
/// reason, this trait also is implemented for all writers.
///
/// This trait formats each byte as follows:
///
///   * All printable ASCII characters as themselves, e.g., writing byte 0x50 as
///     `P`.
///   * All C0 and some C1 control characters as their two- or three-letter
///     mnemonics between single guillemets, e.g., writing 0x1B as
///     `‹ESC›`.
///   * All remaining bytes as two-digit hexadecimal numbers between corner
///     brackets, e.g., writing byte 0xAF as `「AF」`.
///
///
/// # Example
///
/// Bring the trait into scope and use it to format individual bytes as well as
/// byte strings:
/// ```
/// use prettytty::util::WriteNicely;
/// let mut buffer = [0; 40]; // More than enough space
/// let mut cursor = buffer.as_mut_slice();
/// let mut graphemes = 0;
///
/// graphemes += cursor.write_all_nicely(b"ring")?;
/// graphemes += cursor.write_nicely(0x07)?;
/// graphemes += cursor.write_nicely(0xff)?;
/// assert_eq!(graphemes, 4 + 5 + 4);
///
/// let unused = cursor.len();
/// let bytes = buffer.len() - unused;
/// assert_eq!(bytes, 21);
/// assert_eq!(&buffer[..bytes], "ring‹BEL›「FF」".as_bytes());
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// As already indicated by the variable names, there is a difference between
/// the number of graphemes written and the number of bytes written. The former
/// counts visual characters, whereas the latter counts bytes in the UTF-8
/// encoding, with guillemets and corner brackets requiring 3 bytes each.
pub trait WriteNicely: Write {
    /// Write the byte nicely.
    ///
    /// This method returns the number of characters written out.
    fn write_nicely(&mut self, byte: u8) -> std::io::Result<usize> {
        struct Adapter<'a, T: ?Sized + 'a> {
            inner: &'a mut T,
            error: std::io::Result<usize>,
        }

        impl<T: std::io::Write + ?Sized> fmt::Write for Adapter<'_, T> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                self.inner.write_all(s.as_bytes()).map_err(|e| {
                    self.error = Err(e);
                    fmt::Error
                })
            }
        }

        let mut output = Adapter {
            inner: self,
            error: Ok(0),
        };

        format_nicely(byte, &mut output).map_err(|_| {
            match output.error {
                Ok(_) => panic!("a formatting trait implementation returned an error when the underlying stream did not"),
                Err(err) => err,
            }
        })
    }

    /// Write all of the given bytes nicely.
    ///
    /// This method returns the number of characters written out.
    fn write_all_nicely(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let mut size = 0;
        for byte in bytes.iter() {
            size += self.write_nicely(*byte)?;
        }
        Ok(size)
    }
}

impl<W: Write> WriteNicely for W {}

// -----------------------------------------------------------------------------------------------

/// A newtype for nicely formatting a byte slice.
struct ByteStringNicely<'a>(&'a [u8]);

impl std::fmt::Display for ByteStringNicely<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\"")?;
        for byte in self.0.iter() {
            if *byte == b'"' {
                f.write_str("\\\"")?;
            } else {
                format_nicely(*byte, f)?;
            }
        }
        f.write_str("\"")
    }
}

impl std::fmt::Debug for ByteStringNicely<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

/// Turn the slice into a value that displays nicely.
///
/// Also see [`WriteNicely`].
pub fn nicely(bytes: &[u8]) -> impl std::fmt::Debug + std::fmt::Display + use<'_> {
    ByteStringNicely(bytes)
}

// ------------------------------------------------------------------------------------------------

macro_rules! make_parser {
    (@internal u16) => { u32 };
    (@internal u32) => { u64 };
    (@base dec) => { 10 };
    (@base hex) => { 16 };
    (@digitize dec: $bytes:ident[$index:ident]) => {
        match $bytes[$index] {
            n @ 0x30..=0x39 => n - 0x30,
            _ => return None,
        }
    };
    (@digitize hex: $bytes:ident[$index:ident]) => {
        match $bytes[$index] {
            n @ 0x30..=0x39 => n - 0x30,
            n @ 0x41..=0x46 => n - 0x41 + 10,
            n @ 0x61..=0x66 => n - 0x61 + 10,
            _ => return None,
        }
    };
    ($ident:ident : $radix:ident -> $ty:ident) => {
        #[allow(dead_code)]
        pub(crate) const fn $ident(bytes: &[u8]) -> Option<$ty> {
            type Internal = make_parser!(@internal $ty);
            const MAX: Internal = <$ty>::MAX as Internal;
            const BASE: Internal = make_parser!(@base $radix);

            let length = bytes.len();
            if length == 0 {
                return None;
            }

            let mut index = 0;
            let mut result = 0;

            while index < length {
                let digit = make_parser!(@digitize $radix: bytes[index]);
                result = BASE * result + digit as Internal;
                if MAX < result {
                    return None;
                }

                index += 1;
            }

            Some(result as $ty)
        }
    };
}

make_parser!(parse_dec_u16 : dec -> u16);
make_parser!(parse_hex_u16 : hex -> u16);
make_parser!(parse_dec_u32 : dec -> u32);
make_parser!(parse_hex_u32 : hex -> u32);

/// Determine whether the byte is a semi colon, i.e., semicolon or colon.
pub(crate) fn is_semi_colon(b: &u8) -> bool {
    *b == b';' || *b == b':'
}

// ------------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Error;

    #[test]
    fn test_parse_semi_colon() {
        assert_eq!(parse_dec_u16(b"665"), Some(665));
        assert_eq!(parse_dec_u16(b"65536"), None);
        assert_eq!(parse_dec_u16(b"665A"), None);
        assert_eq!(parse_hex_u16(b"665"), Some(1_637));
        assert_eq!(parse_hex_u16(b"665A"), Some(26_202));
        assert_eq!(parse_hex_u16(b"fFfF"), Some(0xffff));
        assert_eq!(parse_hex_u16(b"10000"), None);

        assert_eq!(parse_dec_u32(b"665"), Some(665));
        assert_eq!(parse_dec_u32(b"65536"), Some(65_536));
        assert_eq!(parse_dec_u32(b"665A"), None);
        assert_eq!(parse_hex_u32(b"665"), Some(1_637));
        assert_eq!(parse_hex_u32(b"665A"), Some(26_202));
        assert_eq!(parse_hex_u32(b"fFfFfFfF"), Some(0xffff_ffff));
        assert_eq!(parse_hex_u32(b"100000000"), None);

        assert!(is_semi_colon(&b';'));
        assert!(!is_semi_colon(&b'@'));
    }

    #[test]
    fn test_nicely() -> std::io::Result<()> {
        let mut buffer = [0; 128];
        let mut cursor = buffer.as_mut_slice();

        assert_eq!(cursor.write_nicely(b'R')?, 1);
        assert_eq!(cursor.write_nicely(0x1b)?, 5);
        assert_eq!(cursor.write_nicely(b'#')?, 1);
        assert_eq!(cursor.write_nicely(0xaf)?, 4);

        let cursor_len = cursor.len();
        let len = buffer.len() - cursor_len;
        let data = &buffer[..len];
        assert_eq!(data, "R‹ESC›#「AF」".as_bytes());
        assert_eq!(len, 19);
        Ok::<(), Error>(())
    }
}
