use std::env::var_os;
use std::io::{ErrorKind, Result};

use prettypretty::theme::Theme;
use prettytty::{err::report, opt::Options, Connection};

fn run_queries(tty: Connection) -> Result<()> {
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

    let options = Options::builder().timeout(50).verbose(true).build();
    let tty = match Connection::with_options(options) {
        Ok(conn) => conn,
        Err(err) if err.kind() == ErrorKind::ConnectionRefused && var_os("CI").is_some() => {
            println!("Unable to connect to terminal in CI; skipping queries!");
            return Ok(());
        }
        Err(err) => {
            report(&err);
            return Err(err);
        }
    };

    match run_queries(tty) {
        Ok(()) => Ok(()),
        Err(err) => {
            report(&err);
            Err(err)
        }
    }
}

#[cfg(test)]
mod test {
    use std::io::Result;

    #[test]
    fn run_main() -> Result<()> {
        super::main()
    }
}
