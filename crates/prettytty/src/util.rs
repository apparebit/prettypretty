//! Helpers for parsing and displaying byte strings.

use core::fmt;
use std::io;

/// Parse a byte string into an unsigned integer.
///
/// This enum parses byte strings comprising decimal or hexadecimal ASCII digits
/// into `u16` or `u32`. Its methods are `const`, with the implementation
/// working around the limitations of `const` Rust, notably by using a macro to
/// unwrap options instead of the `?` operator.
///
/// # Example
///
/// ```
/// # use prettytty::util::ByteParser;
/// assert_eq!(
///     ByteParser::Hexadecimal.to_u16(b"ffff"),
///     Some(0xffff)
/// );
/// assert_eq!(
///     ByteParser::Decimal.to_u16(b"65536"),
///     None
/// );
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ByteParser {
    Decimal = 10,
    Hexadecimal = 16,
}

// Macro to unwrap options, since ? operator can't be used in const functions.
macro_rules! unwrap {
    ($expr:expr) => {
        match $expr {
            Some(value) => value,
            None => return None,
        }
    };
}

impl ByteParser {
    /// Get this ASCII digit's value.
    const fn digit(&self, byte: u8) -> Option<u8> {
        let value = match byte {
            0x30..=0x39 => byte - 0x30,
            0x41..=0x46 => byte - 0x41 + 10,
            0x61..=0x66 => byte - 0x61 + 10,
            _ => return None,
        };

        if (*self as u8) <= value {
            return None;
        }

        Some(value)
    }

    /// Parse the byte string as a u16.
    pub const fn to_u16(&self, bytes: &[u8]) -> Option<u16> {
        let value = unwrap!(self.to_u32(bytes));
        if value <= 0xffff {
            Some(value as u16)
        } else {
            None
        }
    }

    /// Parse the byte string as a u32.
    pub const fn to_u32(&self, bytes: &[u8]) -> Option<u32> {
        let mut value: u32 = 0;
        let mut index = 0;

        while index < bytes.len() {
            let digit = unwrap!(self.digit(bytes[index]));
            value = unwrap!(value.checked_mul(*self as u32));
            value = unwrap!(value.checked_add(digit as u32));
            index += 1;
        }

        Some(value)
    }
}

// -----------------------------------------------------------------------------------------------

/// Display a byte string in a more humane manner.
///
/// The intended use for this enumeration is wrapping byte strings before
/// handing them off to one of Rust's formatting macros. However, the low-level
/// [`ByteFormat::render`] method, especially when combined with a [`Rewriter`]
/// instance, enables other use cases, too.
///
/// # Example
///
/// ```
/// # use prettytty::util::ByteFormat;
/// assert_eq!(
///     format!("{}", ByteFormat::Concise(b"\x1b[1m\x90@\xfe\x07")),
///     "␛[1m.@.␇"
/// );
/// assert_eq!(
///     format!("{}", ByteFormat::Nicely(b"\x1b[1m\x90@\xfe\x07")),
///     "‹ESC›[1m‹DCS›@「FE」‹BEL›"
/// );
/// assert_eq!(
///     format!("{}", ByteFormat::Hexdump(b"\x1b[1m\x90@\xfe\x07")),
///     "0000:  1b5b 316d 9040 fe07  ␛[1m.@.␇"
/// );
/// ```
#[derive(Debug)]
pub enum ByteFormat<'a> {
    /// The concise format uses one character per byte. It displays C0 control
    /// codes with Unicode control pictures (which may be hard to read) and
    /// replaces bytes larger than 0x7F with a period `.`
    Concise(&'a [u8]),
    /// The elaborate format uses more than one character per byte where
    /// necessary. It displays C0 control codes as well as select C1 control
    /// codes as mnemonics between guillemets, e.g., `‹ESC›` for 0x1B. It
    /// displays bytes larger than 0x7F as hexadecimal numbers between corner
    /// brackets, e.g., `「A0」` for 0xA0.
    Nicely(&'a [u8]),
    /// The hexdump format combines hexadecimal and concise formatting. Unlike
    /// the other formats, it is line-oriented, displaying up to 16 bytes per
    /// line.
    Hexdump(&'a [u8]),
}

const C0: [&str; 32] = [
    "‹NUL›",
    "‹SOH›",
    "‹STX›",
    "‹ETX›",
    "‹EOT›",
    "‹ENQ›",
    "‹ACK›",
    "‹BEL›",
    "‹BS›",
    "‹HT›",
    "‹LF›",
    "‹VT›",
    "‹FF›",
    "‹CR›",
    "‹SO›",
    "‹SI›",
    "‹DLE›",
    "‹DC1›",
    "‹DC2›",
    "‹DC3›",
    "‹DC4›",
    "‹NAK›",
    "‹SYN›",
    "‹ETB›",
    "‹CAN›",
    "‹EM›",
    "‹SUB›",
    "‹ESC›",
    "‹FS›",
    "‹GS›",
    "‹RS›",
    "‹US›",
];

const C1: [&str; 5] = ["‹CSI›", "‹ST›", "‹OSC›", "‹PM›", "‹APC›"];

impl ByteFormat<'_> {
    /// Render the bytes with the given writer.
    ///
    /// This method largely is an implementation detail. It differs from the
    /// display trait by accepting arbitrary writers and by returning the number
    /// of characters (not bytes) written. It is public to support applications
    /// that require either of these features.
    ///
    /// Since the hexdump format is line-oriented, it emits newlines for all but
    /// the last line. The number of characters written only covers that last
    /// line.
    pub fn render<W: fmt::Write + ?Sized>(&self, writer: &mut W) -> Result<usize, fmt::Error> {
        match *self {
            ByteFormat::Concise(bytes) => ByteFormat::render_concise(bytes, writer),
            ByteFormat::Nicely(bytes) => ByteFormat::render_nicely(bytes, writer),
            ByteFormat::Hexdump(bytes) => ByteFormat::render_hexdump(bytes, writer),
        }
    }

    fn render_concise<W>(bytes: &[u8], writer: &mut W) -> Result<usize, fmt::Error>
    where
        W: fmt::Write + ?Sized,
    {
        for byte in bytes {
            let display = match *byte {
                0x00..=0x1f => {
                    char::from_u32(0x2400_u32 + *byte as u32).expect("known good Unicode character")
                }
                0x20..=0x7e => *byte as char,
                0x7f => char::from_u32(0x2421).expect("known good Unicode character"),
                _ => '.',
            };
            writer.write_char(display)?;
        }

        Ok(bytes.len())
    }

    fn render_nicely<W>(bytes: &[u8], writer: &mut W) -> Result<usize, fmt::Error>
    where
        W: fmt::Write + ?Sized,
    {
        let mut ascii = [0; 1];
        let mut characters = 0;

        for &byte in bytes {
            let display = match byte {
                0x00..=0x1f => C0[byte as usize],
                0x20..=0x7e => {
                    ascii[0] = byte;
                    // SAFETY: Guaranteed to be ASCII by match arm
                    core::str::from_utf8(&ascii).expect("ASCII characters are valid UTF-8, too")
                }
                0x7f => "‹DEL›",
                0x90 => "‹DCS›",
                0x98 => "‹SOS›",
                0x9b..=0x9f => C1[(byte - 0x9b) as usize],
                _ => "",
            };

            if display.is_empty() {
                writer.write_fmt(format_args!("「{:02X}」", byte))?;
                characters += 4;
            } else {
                writer.write_str(display)?;
                characters += match display.len() {
                    n @ (1 | 2) => n,
                    n => n - 6 + 2,
                };
            }
        }

        Ok(characters)
    }

    // Grr, if I add the annotation to the offending for loop over pairs, it is
    // ineffective.
    #[allow(clippy::missing_asserts_for_indexing)]
    fn render_hexdump<W>(bytes: &[u8], writer: &mut W) -> Result<usize, fmt::Error>
    where
        W: fmt::Write + ?Sized,
    {
        const CHUNK_SIZE: usize = 16;
        let compact = bytes.len() < CHUNK_SIZE;
        let mut chunk_index = 0;
        let mut characters = 0;

        for chunk in bytes.chunks(CHUNK_SIZE) {
            if 0 < chunk_index {
                writer.write_char('\n')?;
            }

            write!(writer, "{:04x}:  ", chunk_index)?;
            characters = 7; // Restart counting so we only count last line

            for pair in chunk.chunks(2) {
                // Allow for uneven number of bytes in final chunk.
                write!(writer, "{:02x}", pair[0])?;
                if pair.len() == 1 {
                    write!(writer, "   ")?;
                } else {
                    write!(writer, "{:02x} ", pair[1])?;
                }
                characters += 5;
            }

            if !compact {
                for _ in 0..(CHUNK_SIZE - chunk.len()) / 2 {
                    // Pad out remaining hexadecimal slots for final chunk.
                    writer.write_str("     ")?;
                    characters += 5;
                }
            }

            // Separate hexadecimal from character display by two columns
            writer.write_str(" ")?;
            characters += 1;

            ByteFormat::render_concise(chunk, writer)?;

            chunk_index += chunk.len();
            characters += chunk.len();
        }

        Ok(characters)
    }
}

impl fmt::Display for ByteFormat<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.render(f)?;
        Ok(())
    }
}

// -----------------------------------------------------------------------------------------------

/// A lightweight adapter from [`std::io::Write`] to [`core::fmt::Write`].
///
/// Since Rust encodes strings and string slices in UTF-8, forwarding
/// [`core::fmt::Write::write_str`] to [`std::io::Write::write_all`] is
/// straight-forward. The challenge is that [`std::io::Error`] covers many
/// different error conditions, whereas [`core::fmt::Error`] is a unit-like
/// struct. The primary benefit of this adapter is that it tracks the most
/// recent I/O error. Hence, if the rewriter fails with a format error, code
/// using this struct can recover the underlying I/O error.
///
/// # Example
///
/// The match below illustrates how to do just that:
/// ```
/// # use prettytty::util::Rewriter;
/// # use std::io::{Cursor, Write};
/// # use core::fmt::Write as FmtWrite;
/// # fn main() -> std::io::Result<()> {
/// let mut cursor = Cursor::new(vec![0; 10]);
/// let mut writer = Rewriter::new(&mut cursor);
///
/// match writer.write_str("Hello!") {
///     Ok(()) => (),
///     Err(_) => return Err(writer.into_err()),
/// }
///
/// assert_eq!(&cursor.get_ref()[0..5], b"Hello");
/// # Ok(())
/// # }
/// ```
pub struct Rewriter<'a, W: ?Sized + 'a> {
    writer: &'a mut W,
    result: io::Result<()>,
}

impl<'a, W: ?Sized + 'a> Rewriter<'a, W> {
    /// Create a new rewriter.
    pub fn new(writer: &'a mut W) -> Self {
        Self {
            writer,
            result: Ok(()),
        }
    }

    /// Determine whether this rewriter wraps an error result.
    pub fn is_err(&self) -> bool {
        self.result.is_err()
    }

    /// Consume the rewriter to get its error.
    ///
    /// If the code using this rewriter produced a [`fmt::Error`], this method
    /// produces the underlying I/O error.
    ///
    /// # Panics
    ///
    /// If the rewriter didn't record an error.
    pub fn into_err(self) -> io::Error {
        match self.result {
            Err(err) => err,
            Ok(_) => panic!("display trait returned error without underlying I/O error"),
        }
    }
}

impl<W: io::Write + ?Sized> fmt::Write for Rewriter<'_, W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.writer.write_all(s.as_bytes()).map_err(|err| {
            self.result = Err(err);
            fmt::Error
        })
    }
}

// ------------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use std::io::{Cursor, Error, Write};

    #[test]
    fn test_radix_parse() {
        assert_eq!(ByteParser::Decimal.to_u16(b"665"), Some(665));
        assert_eq!(ByteParser::Decimal.to_u16(b"65536"), None);
        assert_eq!(ByteParser::Decimal.to_u16(b"665A"), None);
        assert_eq!(ByteParser::Hexadecimal.to_u16(b"665"), Some(1_637));
        assert_eq!(ByteParser::Hexadecimal.to_u16(b"665A"), Some(26_202));
        assert_eq!(ByteParser::Hexadecimal.to_u16(b"fFfF"), Some(0xffff));
        assert_eq!(ByteParser::Hexadecimal.to_u16(b"10000"), None);

        assert_eq!(ByteParser::Decimal.to_u32(b"665"), Some(665));
        assert_eq!(ByteParser::Decimal.to_u32(b"65536"), Some(65_536));
        assert_eq!(ByteParser::Decimal.to_u32(b"665A"), None);
        assert_eq!(ByteParser::Hexadecimal.to_u32(b"665"), Some(1_637));
        assert_eq!(ByteParser::Hexadecimal.to_u32(b"665A"), Some(26_202));
        assert_eq!(
            ByteParser::Hexadecimal.to_u32(b"fFfFfFfF"),
            Some(0xffff_ffff)
        );
        assert_eq!(ByteParser::Hexadecimal.to_u32(b"100000000"), None);
    }

    #[test]
    fn test_format() -> std::io::Result<()> {
        let mut buffer = Cursor::new(vec![0; 500]);
        write!(
            buffer,
            "{}",
            ByteFormat::Hexdump(b"\x1bP>|Terminal\x07\x1bP>|Name\x1b\\")
        )?;

        assert_eq!(
            &buffer.get_ref()[0..buffer.position() as usize],
            b"0000:  1b50 3e7c 5465 726d 696e 616c 071b 503e  \xe2\x90\x9bP>|Terminal\
                                                                    \xe2\x90\x87\
                                                                    \xe2\x90\x9bP>\n\
              0010:  7c4e 616d 651b 5c                        |Name\xe2\x90\x9b\\"
        );
        Ok(())
    }

    #[test]
    fn test_nicely() -> std::io::Result<()> {
        let mut buffer = Cursor::new(vec![0; 100]);
        let mut writer = Rewriter::new(&mut buffer);

        assert_eq!(ByteFormat::Nicely(b"R").render(&mut writer), Ok(1));
        assert_eq!(ByteFormat::Nicely(b"\x1b").render(&mut writer), Ok(5));
        assert_eq!(ByteFormat::Nicely(b"#").render(&mut writer), Ok(1));
        assert_eq!(ByteFormat::Nicely(b"\xaf").render(&mut writer), Ok(4));
        assert_eq!(ByteFormat::Nicely(b"\\").render(&mut writer), Ok(1));
        assert_eq!(ByteFormat::Nicely(b"\"").render(&mut writer), Ok(1));

        assert_eq!(
            &buffer.get_ref()[0..buffer.position() as usize],
            "R‹ESC›#「AF」\\\"".as_bytes()
        );
        assert_eq!(buffer.position(), 21);
        Ok::<(), Error>(())
    }
}
