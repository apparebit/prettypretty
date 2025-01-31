/// # echo: Visualizing Terminal Input
///
/// This example keeps reading raw bytes from the terminal. If it times out, it
/// prints a small circle. Otherwise, it prints the bytes between large angular
/// brackets like so `〈q〉`. Typing `t` queries the terminal for a theme color.
/// Typing `q` exits the program.
use std::io::{Read, Write};

use prettytty::cmd::{Format, RequestColor, ResetStyle, SetDefaultForeground, SetForeground8};
use prettytty::err::report;
use prettytty::opt::Options;
use prettytty::util::WriteNicely;
use prettytty::Connection;

const GRAY: SetForeground8<244> = SetForeground8::<244>;

#[allow(unused_assignments)]
fn run() -> std::io::Result<()> {
    // Access the terminal
    println!("\n");
    let options = Options::with_log();
    let tty = Connection::with_options(options)?;
    let mut input = tty.input();
    let mut output = tty.output();

    let mut token_buffer = [0_u8; 100];
    let width = 80;

    // Peek into terminal access
    write!(
        output,
        "\r\n\r\n{}press ‹t› to query theme color, ‹q› to quit{}\r\n\r\n",
        Format::Bold,
        Format::Bold.undo()
    )?;
    output.flush()?;

    let mut color_requests = RequestColor::all();
    let mut number_of_reads = 0;
    let mut column = 0;

    macro_rules! wrap {
        ( $offset:expr ) => {
            if width - $offset <= column {
                output.print("\r\n")?;
                column = $offset;
            } else {
                column += $offset;
            }
        };
    }

    output.exec(GRAY)?;
    loop {
        // Stop looping after a while.
        number_of_reads += 1;
        if 1000 <= number_of_reads {
            wrap!(2);
            output.print(format!("{}✋", ResetStyle))?;
            break;
        }

        // Read in some data. Handle timeout.
        let mut buffer = [0; 32];
        let count = input.read(&mut buffer)?;
        if count == 0 {
            wrap!(1);
            output.print("◦")?;
            continue;
        };

        // Format token into intermediate buffer.
        let mut cursor = token_buffer.as_mut();
        let mut char_len = 4;

        cursor.write("〈".as_bytes())?;
        for byte in buffer[..count].iter() {
            char_len += cursor.write_nicely(*byte)?;
        }
        cursor.write("〉".as_bytes())?;

        let cursor_len = cursor.len();
        let token_len = token_buffer.len() - cursor_len;

        // Actually write out token.
        wrap!(char_len);
        output.write(format!("{}", SetDefaultForeground).as_bytes())?;
        output.write(&token_buffer[..token_len])?;
        output.write(format!("{}", GRAY).as_bytes())?;
        output.flush()?;

        // Handle user input.
        if buffer.contains(&b'q') {
            output.exec(ResetStyle)?;
            output.println("\n\n")?;
            break;
        } else if buffer.contains(&b't') {
            let mut entry = color_requests.next();
            if entry.is_none() {
                color_requests = RequestColor::all();
                entry = color_requests.next();
            }

            output.exec(entry.unwrap())?;
        }
    }

    // Relinquish the terminal again.
    drop(input);
    drop(output);
    drop(tty);
    println!("bye bye!");

    Ok(())
}

fn main() {
    if let Err(error) = run() {
        report(&error);
    }
}
