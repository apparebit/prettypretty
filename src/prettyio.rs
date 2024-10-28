#![cfg(target_family = "unix")]

use prettypretty::termio::{render, Terminal, TERMINAL_TIMEOUT};
use prettypretty::trans::ThemeEntry;
use std::io::{Read, Result, Write};

pub fn main() -> Result<()> {
    let terminal = Terminal::open()?.cbreak_mode(TERMINAL_TIMEOUT)?;
    let mut reader = terminal.reader();
    let mut writer = terminal.writer();
    let mut entries = ThemeEntry::all();

    write!(
        writer,
        "press ‹t› to query rotating theme color, ‹q› to quit\r\n\r\n"
    )?;

    let mut iterations = 0;
    loop {
        iterations += 1;
        if 1000 <= iterations {
            write!(writer, "✋")?;
            break;
        }

        let mut buffer = [0; 32];
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            write!(writer, "◦")?;
            continue;
        }

        write!(writer, "〈")?;
        let mut terminate = false;
        let mut query = None;

        for b in buffer.iter().take(count) {
            render(*b, &mut writer)?;

            if *b == b'q' {
                terminate = true;
            } else if *b == b't' {
                let mut entry = entries.next();
                if entry.is_none() {
                    entries = ThemeEntry::all();
                    entry = entries.next();
                }

                query = Some(entry.unwrap());
            }
        }

        write!(writer, "〉")?;

        if terminate {
            break;
        } else if let Some(entry) = query {
            write!(writer, "{}", entry)?;
        }
    }

    terminal.restore()?;
    println!("\n\nbye bye!");

    Ok(())
}
