use std::io::{Result, Write};

/// Render the given byte with the given writer.
///
/// If the byte is a printable ASCII character, this function renders it as
/// such. Otherwise, it renders angle quotes with a mnemonic name, such as
/// `‹𝖾𝗌𝖼›`, or the two-digit hexadecimal value, such as `‹f4›`.
pub fn render<W: Write>(byte: u8, writer: &mut W) -> Result<()> {
    if (0x20..=0x7e).contains(&byte) {
        return write!(writer, "{}", char::from(byte));
    }

    let replacement = match byte {
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
        write!(writer, "‹{:02x}›", byte)
    } else {
        write!(writer, "{}", replacement)
    }
}
