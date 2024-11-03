#[cfg(target_family = "unix")]
#[allow(non_snake_case)]
pub fn main() -> std::io::Result<()> {
    use prettypretty::style::{stylist, Fidelity, GrayGradient, Stylist};
    use prettypretty::term::{render, terminal};
    use prettypretty::trans::{Theme, ThemeEntry, Translator};
    use prettypretty::OkVersion;
    use std::io::{stdout, IsTerminal, Read, Write};

    // Determine runtime context
    let theme = Theme::query_terminal()?;
    let translator = Translator::new(OkVersion::Revised, theme);
    let fidelity = Fidelity::from_environment(stdout().is_terminal());

    // Define and adjust styles
    let BOLD = &stylist().bold().et_voila().cap(fidelity, &translator);
    let GRAY = &stylist()
        .fg(GrayGradient::new(15).unwrap())
        .et_voila()
        .cap(fidelity, &translator);
    let RESET = &Stylist::with_reset().et_voila().cap(fidelity, &translator);

    // Access terminal
    let mut tty = terminal().access()?;
    let mut entries = ThemeEntry::all();

    // Peek into terminal access
    let info = format!("{:#?}", tty);
    tty.print(info)?;
    write!(
        tty,
        "\r\n\r\n{}press ‹t› to query rotating theme color, ‹q› to quit{}\r\n\r\n",
        BOLD, !BOLD
    )?;

    let mut iterations = 0;
    let mut line = 0;

    write!(tty, "{}", GRAY)?;
    loop {
        iterations += 1;
        if 1000 <= iterations {
            write!(tty, "{}✋", RESET)?;
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

        write!(tty, "〈{}", !GRAY)?;
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

        write!(tty, "{}〉", GRAY)?;
        line += 2;

        if terminate {
            write!(tty, "{}", RESET)?;
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
    println!("Sorry, but this utility only runs on Unix-like systems!");
}
