use std::error::Error;
use std::io::{Result, Write};

use prettypretty::error::{ThemeError, ThemeErrorKind};
use prettypretty::term::{terminal, Options, TerminalAccess, VtScanner};
use prettypretty::trans::{Theme, ThemeEntry};
use prettypretty::Color;

// ----------------------------------------------------------------------------------------------------------

fn connect() -> Result<()> {
    unsafe { terminal().connect_with(Options::in_verbose()) }?;
    Ok(())
}

fn disconnect() {
    terminal().disconnect()
}

fn access() -> Result<TerminalAccess<'static>> {
    let tty = terminal()
        .access_with(Options::in_verbose())
        .map_err(|e| ThemeError::new(ThemeErrorKind::AccessDevice, e.into()))?;
    Ok(tty)
}

fn write(tty: &mut TerminalAccess, entry: ThemeEntry) -> Result<()> {
    write!(tty, "{}", entry)
        .map_err(|e| ThemeError::new(ThemeErrorKind::WriteQuery(entry), e.into()))?;
    Ok(())
}

fn write_and_flush(tty: &mut TerminalAccess, entry: ThemeEntry) -> Result<()> {
    write!(tty, "{}", entry)
        .and_then(|()| tty.flush())
        .map_err(|e| ThemeError::new(ThemeErrorKind::WriteQuery(entry), e.into()))?;
    Ok(())
}

fn read<'a>(
    tty: &mut TerminalAccess,
    scanner: &'a mut VtScanner,
    entry: ThemeEntry,
) -> Result<&'a str> {
    let response = scanner
        .scan_str(tty)
        .map_err(|e| ThemeError::new(ThemeErrorKind::ScanEscape(entry), e.into()))?;
    Ok(response)
}

fn parse(entry: ThemeEntry, response: &str) -> Result<Color> {
    let color = entry
        .parse_response(response)
        .map_err(|e| ThemeError::new(ThemeErrorKind::ParseColor(entry), e.into()))?;
    Ok(color)
}

fn read_and_parse(
    tty: &mut TerminalAccess,
    scanner: &mut VtScanner,
    entry: ThemeEntry,
) -> Result<Color> {
    let response = read(tty, scanner, entry)?;
    parse(entry, response)
}

// ----------------------------------------------------------------------------------------------------------

fn write_read_parse_loop() -> Result<Theme> {
    let tty = &mut access()?;
    let scanner = &mut VtScanner::new();
    let mut theme = Theme::default();

    for entry in ThemeEntry::all() {
        write_and_flush(tty, entry)?;
        theme[entry] = read_and_parse(tty, scanner, entry)?;
    }

    Ok(theme)
}

fn write_loop_read_parse_loop() -> Result<Theme> {
    let tty = &mut access()?;
    let scanner = &mut VtScanner::new();
    let mut theme = Theme::default();

    for entry in ThemeEntry::all() {
        write(tty, entry)?;
    }

    tty.flush()?;

    for entry in ThemeEntry::all() {
        theme[entry] = read_and_parse(tty, scanner, entry)?;
    }

    Ok(theme)
}

fn write_loop_read_loop_parse_loop() -> Result<Theme> {
    let tty = &mut access()?;
    let scanner = &mut VtScanner::new();
    let mut theme = Theme::default();

    for entry in ThemeEntry::all() {
        write(tty, entry)?;
    }

    tty.flush()?;

    let mut all_responses = Vec::new();
    for entry in ThemeEntry::all() {
        let response = read(tty, scanner, entry)?;
        all_responses.push(String::from(response));
    }

    for (entry, response) in ThemeEntry::all().zip(all_responses.into_iter()) {
        theme[entry] = parse(entry, &response)?;
    }

    Ok(theme)
}

// ----------------------------------------------------------------------------------------------------------

#[derive(Default)]
struct Runner {
    runs: usize,
    passed: usize,
}

impl Runner {
    fn run<F>(&mut self, label: &str, experiment: F) -> Result<()>
    where
        F: Fn() -> Result<Theme>,
    {
        let result = experiment();
        self.runs += 1;

        let mut tty = terminal().access_with(Options::in_verbose())?;
        match result {
            Ok(_) => {
                self.passed += 1;
                write!(tty, "│ PASS {:<70} │\r\n", label)?;
                tty.flush()?;
            }
            Err(ref error) => {
                write!(tty, "│ FAIL {:<70} │\r\n", label)?;

                let mut error: &dyn Error = error;
                loop {
                    write!(tty, "│      {:<70} │\r\n", error)?;
                    match error.source() {
                        Some(source) => error = source,
                        None => break,
                    }
                }

                tty.flush()?;
            }
        }

        Ok(())
    }

    fn summary(&self) -> Result<()> {
        println!("{}/{} runs passed", self.runs, self.passed);
        if self.passed < self.runs {
            Err(std::io::ErrorKind::Other.into())
        } else {
            Ok(())
        }
    }
}

fn main() -> Result<()> {
    if cfg!(target_family = "windows") {
        return Ok(());
    }

    let mut runner = Runner::default();

    let _ = runner.run("one loop, on demand access", write_read_parse_loop);
    let _ = runner.run("two loops, on demand access", write_loop_read_parse_loop);
    let _ = runner.run(
        "three loops, on demand access",
        write_loop_read_loop_parse_loop,
    );

    let _ = connect();

    let _ = runner.run("one loop, existing connection", write_read_parse_loop);
    let _ = runner.run(
        "two loops, existing connections",
        write_loop_read_parse_loop,
    );
    let _ = runner.run(
        "three loops, existing connection",
        write_loop_read_loop_parse_loop,
    );

    disconnect();

    runner.summary()
}

#[cfg(test)]
mod test {
    #[test]
    fn run_main() {
        super::main().unwrap()
    }
}
