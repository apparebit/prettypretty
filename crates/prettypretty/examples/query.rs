use std::env::var_os;
use std::io::{Error, ErrorKind, Result, Write};
use std::time::SystemTime;

use prettypretty::theme::Theme;
use prettytty::{err::report, opt::Options, Connection};

fn run_queries(tty: Connection) -> Result<()> {
    tty.output().print("\r\nloop-1")?;

    let start = SystemTime::now();
    let theme11 = Theme::query1(&tty)?;
    let theme12 = Theme::query1(&tty)?;
    let theme13 = Theme::query1(&tty)?;
    let duration = start.elapsed().map_err(|e| Error::other(e))?;
    assert_eq!(theme11, theme12);
    assert_eq!(theme11, theme13);

    write!(
        tty.output(),
        " took {:.1}s\r\nloop-2",
        duration.as_secs_f32()
    )?;
    tty.output().flush()?;

    let start = SystemTime::now();
    let theme21 = Theme::query2(&tty)?;
    let theme22 = Theme::query2(&tty)?;
    let theme23 = Theme::query2(&tty)?;
    let duration = start.elapsed().map_err(|e| Error::other(e))?;
    assert_eq!(theme11, theme21);
    assert_eq!(theme21, theme22);
    assert_eq!(theme21, theme23);

    write!(
        tty.output(),
        " took {:.1}s\r\nloop-3",
        duration.as_secs_f32()
    )?;
    tty.output().flush()?;

    let start = SystemTime::now();
    let theme31 = Theme::query3(&tty)?;
    let theme32 = Theme::query3(&tty)?;
    let theme33 = Theme::query3(&tty)?;
    let duration = start.elapsed().map_err(|e| Error::other(e))?;
    assert_eq!(theme21, theme31);
    assert_eq!(theme31, theme32);
    assert_eq!(theme31, theme33);

    write!(tty.output(), " took {:.1}s\r\n\r\n", duration.as_secs_f32())?;
    tty.output().flush()?;

    Ok(())
}

fn run() -> Result<()> {
    let options = Options::with_log();
    let tty = match Connection::with_options(options) {
        Ok(conn) => conn,
        Err(ref err) if err.kind() == ErrorKind::ConnectionRefused && var_os("CI").is_some() => {
            println!("Unable to connect to terminal in CI; skipping queries!");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    run_queries(tty)?;
    Ok(())
}

fn main() -> Result<()> {
    // FIXME: Need to make sure we only run this test in Windows Terminal 1.22
    // or later. Earlier versions do not support the necessary ANSI escape
    // sequences.
    if cfg!(target_family = "windows") {
        return Ok(());
    }

    match run() {
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
