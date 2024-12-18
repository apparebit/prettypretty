use std::io::{ErrorKind, Result};

use prettypretty::theme::Theme;
use prettytty::{err::report, opt::Options, Connection};

fn run_queries() -> Result<()> {
    let options = Options::builder().timeout(50).verbose(true).build();
    let tty = Connection::with_options(options)?;

    {
        tty.output().print("1-loop\r\n")?;
    }
    let theme1 = Theme::query1(&tty)?;

    {
        tty.output().print("2-loops\r\n")?;
    }
    let theme2 = Theme::query2(&tty)?;

    {
        tty.output().print("3-loops\r\n")?;
    }
    let theme3 = Theme::query3(&tty)?;

    assert_eq!(theme1, theme2);
    assert_eq!(theme1, theme3);
    Ok(())
}

fn main() -> Result<()> {
    // FIXME: Need to make sure we only run this test in Windows Terminal 1.22
    // or later. Earlier versions do not support the necessary ANSI escape
    // sequences.
    if cfg!(target_family = "windows") {
        return Ok(());
    }

    let mut result = run_queries();
    if let Err(err) = &result {
        if err.kind() == ErrorKind::ConnectionRefused {
            println!("Unable to connect to terminal. Skipping queries.");
            result = Ok(());
        } else {
            report(err);
        }
    }
    result
}

#[cfg(test)]
mod test {
    use std::io::Result;

    #[test]
    fn run_main() -> Result<()> {
        super::main()
    }
}
