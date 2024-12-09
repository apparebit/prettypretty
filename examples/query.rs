/// # query: Stressquerying the terminal for its color theme
use std::error::Error;
use std::io::{Result, Write};

use prettypretty::cmd::{FormatBold, ResetStyle, SetForeground8};
use prettypretty::term::{terminal, Options, Scanner, TerminalAccess};
use prettypretty::theme;

pub fn report<R>(result: Result<R>) {
    match result {
        Ok(_) => (),
        Err(error) => {
            println!(
                "{}{}ERROR {}{}",
                FormatBold,
                SetForeground8(1),
                error,
                ResetStyle
            );

            let mut error: &dyn Error = &error;
            loop {
                match error.source() {
                    Some(inner) => error = inner,
                    None => break,
                }

                println!("    {}", error);
            }
        }
    }
}

// ----------------------------------------------------------------------------------------------------------

#[derive(Default)]
struct Runner {
    runs: usize,
    passed: usize,
}

impl Runner {
    fn run<F>(&mut self, label: &str, query: F) -> Result<()>
    where
        F: Fn(&mut TerminalAccess, &mut Scanner, &mut theme::Theme) -> Result<()>,
    {
        let options = Options::builder().verbose(true).build();
        report(self.handle(label, theme::apply(query, options)));
        Ok(())
    }

    fn handle<R>(&mut self, label: &str, result: Result<R>) -> Result<()> {
        let mut tty = terminal().access_with(Options::in_verbose())?;
        self.runs += 1;

        match result {
            Ok(_) => {
                self.passed += 1;
                write!(tty, "    {}PASS{} {}\r\n", FormatBold, ResetStyle, label)?;
                tty.flush()?;
            }
            Err(ref error) => {
                write!(tty, "    {}FAIL{} {}\r\n", FormatBold, ResetStyle, label)?;

                let mut error: &dyn Error = error;
                loop {
                    write!(tty, "        {}\r\n", format!("{}", error))?;
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
        let msg = format!("\n{}/{} runs passed", self.passed, self.runs);
        let clr = if self.passed == self.runs { 2 } else { 1 };
        println!(
            "{}{}{}{}",
            FormatBold,
            SetForeground8(clr),
            &msg,
            ResetStyle
        );

        if self.passed < self.runs {
            Err(std::io::Error::other(msg))
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
    use std::io::Result;

    #[test]
    fn run_main() -> Result<()> {
        super::main()
    }
}
