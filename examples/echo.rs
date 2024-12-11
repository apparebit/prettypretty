/// # echo: An interactive visualization of terminal input and output
///
/// At its core, this example reads from terminal input and writes the result to
/// terminal output in a loop. It writes a small circle if it timed out and has
/// no input to write. And it automatically terminates after 1,000 iterations of
/// the loop (i.e., 100 seconds). If you type `t` for *theme*, it also issues as
/// query for a theme color, rotating through the 18 theme colors for each `t`.
/// If you type `q` for *quit*, the example quits.
use std::error::Error;
use std::io::{stdout, IsTerminal, Read};

use prettypretty::style::{stylist, Fidelity, Stylist};
use prettypretty::term::{format_nicely, terminal};
use prettypretty::theme::{Theme, ThemeEntry};
use prettypretty::trans::Translator;
use prettypretty::OkVersion;

#[allow(non_snake_case)]
fn run() -> std::io::Result<()> {
    // Determine runtime context
    let theme = Theme::query_terminal()?;
    let translator = Translator::new(OkVersion::Revised, theme);
    let fidelity = Fidelity::from_environment(stdout().is_terminal());

    // Define and adjust styles
    let BOLD = &stylist().bold().et_voila().cap(fidelity, &translator);
    let GRAY = &stylist()
        .gray(15)
        .fg()
        .et_voila()
        .cap(fidelity, &translator);
    let RESET = &Stylist::with_reset().et_voila().cap(fidelity, &translator);

    // Access terminal
    let mut tty = terminal().access()?;
    let mut entries = ThemeEntry::all();

    // Peek into terminal access
    tty.print(format!(
        "{}press ‹t› to query theme color, ‹q› to quit{}\r\n\r\n",
        BOLD, -BOLD
    ))?;

    let mut iterations = 0;
    let mut column = 0;

    tty.print(format!("{}", GRAY))?;
    loop {
        iterations += 1;
        if 1000 <= iterations {
            tty.print(format!("{}✋", RESET))?;
            break;
        }

        if 70 < column {
            tty.print("\r\n")?;
            column = 0;
        }

        let mut buffer = [0; 32];
        let count = tty.read(&mut buffer)?;
        if count == 0 {
            tty.print("◦")?;
            column += 1;
            continue;
        }

        tty.print(format!("〈{}", -GRAY))?;
        column += 2;

        let mut terminate = false;
        let mut query = None;

        // By passing column 0, we should avoid linebreaks inside write_nicely.
        for byte in buffer[..count].iter() {
            column += format_nicely(*byte, &mut tty)?;
        }

        if buffer.contains(&b'q') {
            terminate = true;
        } else if buffer.contains(&b't') {
            let mut entry = entries.next();
            if entry.is_none() {
                entries = ThemeEntry::all();
                entry = entries.next();
            }

            query = Some(entry.unwrap());
        }

        tty.print(format!("{}〉", GRAY))?;
        column += 2;

        if terminate {
            tty.print(format!("{}", RESET))?;
            break;
        } else if let Some(entry) = query {
            tty.print(format!("{}", entry))?;
        }
    }

    drop(tty);
    println!("\n\nbye bye!");

    Ok(())
}

fn main() {
    let result = run();
    if let Err(error) = result {
        println!("\nError: {}", error);
        if let Some(inner) = error.source() {
            println!("    -> {}", inner);
        }
    }
}
