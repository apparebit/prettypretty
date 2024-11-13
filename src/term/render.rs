use std::io::Write;

/// A newtype providing `u8` with a readable display.
#[derive(Clone, Copy)]
pub struct ByteNicely(u8);

impl ByteNicely {
    /// Get the printed length in characters.
    pub fn len(&self) -> usize {
        match self.0 {
            0x07 | 0x09 | 0x1b | 0x7f | 0x90 | 0x98 | 0x9b | 0x9d | 0x9f => 5,
            0x08 | 0x0a | 0x1d | 0x9c | 0x9e => 4,
            0x20..=0x7e => 1,
            _ => 4,
        }
    }
}

impl std::fmt::Display for ByteNicely {
    /// Display the nice byte nicely.
    ///
    /// This method prints regular ASCII characters (including the space) as is,
    /// common control characters as two or three letter mnemonics between `‹›`,
    /// and all remaining bytes as two hexadecimal digits between `‹›`.
    /// Mnemonics use sans-serif Unicode letters.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if (0x20..=0x7e).contains(&self.0) {
            return f.write_str(char::from(self.0).encode_utf8(&mut [0; 4]))
        }

        let replacement = match self.0 {
            // ‹› are 3 bytes each in UTF8, the math letters are 4 bytes each
            0x07 => "‹𝖻𝖾𝗅›",
            0x08 => "‹𝖻s›",
            0x09 => "‹𝗍𝖺𝖻›",
            0x0a => "‹𝗇𝗅›",
            0x1b => "‹𝖾𝗌𝖼›",
            0x1d => "‹𝖼𝗋›",
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

        if replacement.is_empty() {
            f.write_fmt(format_args!("‹{:02x}›", self.0))
        } else {
            f.write_str(replacement)
        }
    }
}

/// Write the byte nicely.
pub fn write_nicely<W: Write>(byte: u8, writer: &mut W) -> std::io::Result<usize> {
    let nice = ByteNicely(byte);
    write!(writer, "{}", nice)?;
    Ok(nice.len())
}
