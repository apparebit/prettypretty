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
        // Make sure that letters are math sans-serif Unicode letters and at
        // least one letter is not a hexadecimal number. In UTF-8, ‹› are 3
        // bytes each and the math letters are 4 bytes each.
        0x00 => "‹𝗇𝗎𝗅›",
        0x07 => "‹𝖻𝖾𝗅›",
        0x08 => "‹𝖻s›",
        0x09 => "‹𝗁𝗍›",
        0x0a => "‹𝗅𝖿›",
        0x0b => "‹𝗏𝗍›",
        0x0d => "‹𝖼𝗋›",

        0x18 => "‹𝖼𝖺𝗇›",
        0x1a => "‹𝗌𝗎𝖻›",
        0x1b => "‹𝖾𝗌𝖼›",

        0x7f => "‹𝖽𝖾𝗅›",

        0x90 => "‹𝖽𝖼𝗌›",
        0x98 => "‹𝗌𝗈𝗌›",
        0x9b => "‹𝖼𝗌𝗂›",
        0x9c => "‹𝗌𝗍›",
        0x9d => "‹𝗈𝗌𝖼›",
        0x9e => "‹𝗉𝗆›",
        0x9f => "‹𝖺𝗉𝖼›",

        _ => "",
    };
    if !replacement.is_empty() {
        output.write_str(replacement)?;
        return Ok(2 + (replacement.len() - 6) / 4);
    }

    output.write_fmt(format_args!("‹{:02x}›", byte))?;
    Ok(4)
}

/// Write bytes nicely.
///
/// Conveniently, this trait's two methods have default implementations, and the
/// trait has a default implementation for all writers.
///
/// # Example
///
/// Bring the trait into scope and use it to format individual bytes as well as
/// byte strings:
///
/// ```
/// use prettytty::util::WriteNicely;
/// let mut buffer = [0; 20];
/// let mut cursor = buffer.as_mut_slice();
/// let mut size = 0;
///
/// size += cursor.write_slice_nicely(b"yo")?;
/// size += cursor.write_nicely(0x07)?;
/// assert_eq!(size, 7);
///
/// let len = cursor.len();
/// let len = buffer.len() - len;
/// assert_eq!(&buffer[..len], "yo‹𝖻𝖾𝗅›".as_bytes());
/// # Ok::<(), std::io::Error>(())
/// ```
pub trait WriteNicely: Write {
    /// Output a nicely formatted byte with the writer.
    ///
    /// This method formats:
    ///
    ///   * Printable ASCII characters as ASCII characters;
    ///   * Common C0 and C1 controls as two- or three-letter mnemonics, e.g.,
    ///     `‹𝖻𝖾𝗅›`;
    ///   * All other bytes as two-digit hexadecimal numbers, e.g., ‹17› for ETM
    ///     (End of Transmission).
    ///
    /// To ensure that mnemonics and hexadecimal codes are clearly
    /// distinguishable, this function only uses mnemonics that have at least
    /// one letter that is *not* a hexadecimal digit. It also formats them with
    /// Unicode sans-serif math characters. As a result, FF (form-feed) is not
    /// formatted as `‹𝖿𝖿›` but rather as `‹0c›`
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

    /// Write the slice of bytes nicely.
    fn write_slice_nicely(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
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
pub fn nicely_str(bytes: &[u8]) -> impl std::fmt::Debug + std::fmt::Display + use<'_> {
    ByteStringNicely(bytes)
}

// ------------------------------------------------------------------------------------------------

/// A choice of radix for converting byte slices to integers.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub(crate) enum Radix {
    Decimal = 10,
    #[allow(dead_code)]
    Hexadecimal = 16,
}

impl Radix {
    pub const fn max_length(&self) -> usize {
        match self {
            Radix::Decimal => 10,
            Radix::Hexadecimal => 16,
        }
    }

    pub const fn parse(&self, bytes: &[u8]) -> Option<u64> {
        let length = bytes.len();
        if bytes.is_empty() || self.max_length() < length {
            return None;
        }

        let mut index = 0;
        let mut result = 0;

        while index < length {
            // SAFETY: by construction
            let digit = (bytes[index] as char).to_digit(*self as u32).unwrap();
            result = (*self as u64) * result + digit as u64;
            index += 1;
        }

        Some(result)
    }

    pub const fn parse_u32(&self, bytes: &[u8]) -> Option<u32> {
        if let Some(n) = self.parse(bytes) {
            if n <= 0xffff_ffff {
                return Some(n as u32);
            }
        }
        None
    }

    pub const fn parse_u16(&self, bytes: &[u8]) -> Option<u16> {
        if let Some(n) = self.parse(bytes) {
            if n <= 0xffff {
                return Some(n as u16);
            }
        }
        None
    }
}

/// Determine whether the byte is a semi colon, i.e., semicolon or colon.
pub(crate) fn is_semi_colon(b: &u8) -> bool {
    *b == b';' || *b == b':'
}

#[cfg(test)]
mod test {
    use super::{is_semi_colon, Radix, WriteNicely};
    use std::io::Error;

    #[test]
    fn test_radix_semi_colon() {
        assert_eq!(Radix::Decimal.parse(b"665").unwrap(), 665);
        assert_eq!(Radix::Hexadecimal.parse(b"665").unwrap(), 1_637);
        assert!(is_semi_colon(&b';'));
        assert!(!is_semi_colon(&b'@'));
    }

    #[test]
    fn test_nicely() -> std::io::Result<()> {
        let mut buffer = [0; 128];
        let mut cursor = buffer.as_mut_slice();

        assert_eq!(cursor.write_nicely(b'R')?, 1);
        assert_eq!(cursor.write_nicely(0x1b)?, 5);
        assert_eq!(cursor.write_nicely(b'[')?, 1);
        assert_eq!(cursor.write_nicely(0xaf)?, 4);

        let cursor_len = cursor.len();
        let len = buffer.len() - cursor_len;
        let data = &buffer[..len];
        assert_eq!(data, "R‹𝖾𝗌𝖼›[‹af›".as_bytes());
        assert_eq!(len, 28);
        Ok::<(), Error>(())
    }
}
