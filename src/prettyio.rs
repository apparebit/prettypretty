#[cfg(target_family = "unix")]
use prettypretty::term::{render, terminal};
#[cfg(target_family = "unix")]
use prettypretty::trans::ThemeEntry;
#[cfg(target_family = "unix")]
use std::io::{Read, Result, Write};

#[cfg(target_family = "unix")]
pub fn main() -> Result<()> {
    let mut tty = terminal().access()?;
    let mut entries = ThemeEntry::all();

    let info = format!("{:#?}", tty);
    tty.print(info)?;
    tty.print("\r\n\r\n\x1b[1mpress ‹t› to query rotating theme color, ‹q› to quit\x1b[m\r\n\r\n")?;

    let mut iterations = 0;
    let mut line = 0;

    write!(tty, "\x1b[38;5;247m")?;
    loop {
        iterations += 1;
        if 1000 <= iterations {
            tty.print("\x1b[m✋")?;
            break;
        }

        if 70 < line {
            tty.print("\r\n")?;
            line = 0;
        }

        let mut buffer = [0; 32];
        let count = tty.read(&mut buffer)?;
        if count == 0 {
            tty.print("◦")?;
            line += 1;
            continue;
        }

        write!(tty, "〈\x1b[m")?;
        line += 2;

        let mut terminate = false;
        let mut query = None;

        for b in buffer.iter().take(count) {
            line += render(*b, &mut tty)?;

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

        tty.print("\x1b[38;5;247m〉")?;
        line += 2;

        if terminate {
            tty.print("\x1b[m")?;
            break;
        } else if let Some(entry) = query {
            write!(tty, "{}", entry)?;
            tty.flush()?;
        }
    }

    drop(tty);
    println!("\n\nbye bye!");

    Ok(())
}

#[cfg(not(target_family = "unix"))]
pub fn main() {
    println!("Sorry, but this utility only compiles and runs on Unix-like systems!");
}
