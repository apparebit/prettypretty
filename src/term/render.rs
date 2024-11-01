use std::io::{Result, Write};

/// Utility for rendering bytes as human-readable text.
///
/// If the byte is a printable ASCII character, this function renders it as
/// such. Otherwise, it renders angle quotes with a mnemonic name, such as
/// `‹𝖾𝗌𝖼›`, or the two-digit hexadecimal value, such as `‹f4›`. This
/// function returns the number of characters (not UTF-8 bytes) rendered.
pub fn render<W: Write>(byte: u8, writer: &mut W) -> Result<usize> {
    if (0x20..=0x7e).contains(&byte) {
        write!(writer, "{}", char::from(byte))?;
        return Ok(1);
    }

    let replacement = match byte {
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
        write!(writer, "‹{:02x}›", byte)?;
        Ok(4)
    } else {
        write!(writer, "{}", replacement)?;
        Ok(2 + (replacement.len() - 6) / 4)
    }
}
