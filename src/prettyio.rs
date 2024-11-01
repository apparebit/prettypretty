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

    write!(
        tty,
        "press ‹t› to query rotating theme color, ‹q› to quit\r\n\r\n"
    )?;
    tty.flush()?;

    let mut iterations = 0;
    let mut line = 0;
    loop {
        iterations += 1;
        if 1000 <= iterations {
            write!(tty, "✋")?;
            tty.flush()?;
            break;
        }

        if 70 < line {
            write!(tty, "\r\n")?;
            line = 0;
            tty.flush()?;
        }

        let mut buffer = [0; 32];
        let count = tty.read(&mut buffer)?;
        if count == 0 {
            write!(tty, "◦")?;
            line += 1;
            tty.flush()?;
            continue;
        }

        write!(tty, "〈")?;
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

        write!(tty, "〉")?;
        line += 2;
        tty.flush()?;

        if terminate {
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
