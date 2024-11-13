/// # query: Stressquerying the terminal for its color theme
use std::error::Error;
use std::io::{Result, Write};

use prettypretty::term::{terminal, Options, TerminalAccess, VtScanner};
use prettypretty::theme;

// ----------------------------------------------------------------------------------------------------------

#[derive(Default)]
struct Runner {
    runs: usize,
    passed: usize,
}

impl Runner {
    fn run<F>(&mut self, label: &str, query: F) -> Result<()>
    where
        F: Fn(&mut TerminalAccess, &mut VtScanner, &mut theme::Theme) -> Result<()>,
    {
        let options = Options::builder().verbose(true).build();
        let result = theme::apply(query, options);
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

    let _ = runner.run("one loop, on demand access", theme::query1);
    let _ = runner.run("two loops, on demand access", theme::query2);
    let _ = runner.run("three loops, on demand access", theme::query3);

    let options = Options::builder().verbose(true).build();
    let _ = unsafe { terminal().connect_with(options) };

    let _ = runner.run("one loop, existing connection", theme::query1);
    let _ = runner.run("two loops, existing connections", theme::query2);
    let _ = runner.run("three loops, existing connection", theme::query3);

    terminal().disconnect();

    runner.summary()
}

#[cfg(test)]
mod test {
    #[test]
    fn run_main() {
        super::main().unwrap()
    }
}
