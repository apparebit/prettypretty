//! Helper module with utilities for byte strings.

/// Nicely format the byte with the given writer.
///
/// This function writes
///
///   * printable ASCII characters as just that, ASCII characters;
///   * replaces common C0 and C1 controls with two- or three-letter mnemonics
///     between `‹›`, e.g., `‹𝖻𝖾𝗅›`;
///   * formats all other bytes as two-digit hexadecimal numbers, again between
///     `‹›`, e.g., ‹17› for ETM (End of Transmission).
///
/// To ensure that mnemonics and hexadecimal codes are easily distinguishable,
/// this function only uses mnemonics that have at least one letter that is not
/// a hexadecimal digit and formats them with Unicode sans-serif math
/// characters. For that reason, FF (form-feed) is not formatted as `‹𝖿𝖿›` but
/// rather as `‹0c›`
///
/// This function does *not* flush the output.
///
///
/// # Examples
///
/// ```
/// # use std::io::Write;
/// # use prettytty::util::format_nicely;
/// let mut buffer = [0_u8; 100];
/// let mut cursor = buffer.as_mut();
/// for byte in b"\x1bP>|tty\x07" {
///     format_nicely(*byte, &mut cursor)?;
/// }
///
/// // Since cursor mutably borrows buffer,
/// // we first get cursor's length only:
/// let len = cursor.len();
///
/// // Cursor's lifetime ends with this comment,
/// // restoring access to buffer:
/// let len = buffer.len() - len;
///
/// assert_eq!(&buffer[..len], "‹𝖾𝗌𝖼›P>|tty‹𝖻𝖾𝗅›".as_bytes());
/// # Ok::<(), std::io::Error>(())
/// ```
pub fn format_nicely(byte: u8, writer: &mut impl std::io::Write) -> std::io::Result<usize> {
    if (0x20..=0x7e).contains(&byte) {
        writer.write_all(&[byte])?;
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
        writer.write_all(replacement.as_bytes())?;
        return Ok(2 + (replacement.len() - 6) / 4);
    }

    writer.write_fmt(format_args!("‹{:02x}›", byte))?;
    Ok(4)
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
    use super::{is_semi_colon, Radix};

    #[test]
    fn test_slice_ext() {
        assert_eq!(Radix::Decimal.parse(b"665").unwrap(), 665);
        assert_eq!(Radix::Hexadecimal.parse(b"665").unwrap(), 1_637);
        assert!(is_semi_colon(&b';'));
        assert!(!is_semi_colon(&b'@'));
    }
}
