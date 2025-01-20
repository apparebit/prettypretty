use std::io::{stdout, Result, Write};
use std::thread;
use std::time::Duration;

use rand::{rngs::ThreadRng, thread_rng};
use rand_distr::{Distribution, Normal, Uniform};

use prettytty::cmd::{
    HideCursor, MoveToColumn, MoveUp, SetDefaultForeground, SetForeground8, ShowCursor,
};

// -------------------------------------------------------------------------------------

/// Progress is a floating point percentage.
pub type Progress = f32;

/// An iterator over monotonically increasing progress reports.
///
/// The first value is 0.0 and the final value is 100.0. Increments are
/// randomized by a normal distribution with mean 1.0 and standard deviation
/// 1.0/3.0.
pub struct ProgressReporter {
    normal: Normal<Progress>,
    rng: ThreadRng,
    status: Progress,
    done: bool,
}

impl ProgressReporter {
    /// Create a new progress reporter.
    pub fn new() -> Self {
        Self {
            normal: Normal::new(1.0, 2.0 / 3.0).unwrap(),
            rng: thread_rng(),
            status: 0.0,
            done: false,
        }
    }
}

impl std::iter::Iterator for ProgressReporter {
    type Item = Progress;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let result = self.status.min(100.0);
        if 100.0 <= self.status {
            self.done = true;
        } else {
            // Ensure that progress is monotonically increasing.
            let incr = self.normal.sample(&mut self.rng).max(0.1);
            self.status += incr;
        }

        Some(result)
    }
}

impl std::iter::FusedIterator for ProgressReporter {}

// -------------------------------------------------------------------------------------

/// A progress renderer.
pub struct Renderer(pub Progress);

const WIDTH: usize = 25;
const STEPS: usize = 4;

impl std::fmt::Display for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let uprog = self.0 as usize;
        let full = uprog / STEPS;
        let partial = uprog % STEPS;
        let empty = WIDTH - full - (if 0 < partial { 1 } else { 0 });

        write!(f, "{}  ┫{}", MoveToColumn::<0>, SetForeground8::<10>)?;

        for _ in 0..full {
            f.write_str("█")?;
        }
        if 0 < partial {
            f.write_str(["▎", "▌", "▊"][partial - 1])?;
        }
        for _ in 0..empty {
            f.write_str(" ")?;
        }

        write!(f, "{}┣ {:5.1}%", SetDefaultForeground, self.0)
    }
}

// -------------------------------------------------------------------------------------

/// Animate a progress bar's progress from 0 to 100 percent.
pub fn animate(output: &mut impl Write) -> Result<()> {
    let uniform = Uniform::new_inclusive(16, 100);
    let mut rng = thread_rng();

    // Flushed by first write_bar()
    write!(output, "{}", HideCursor)?;

    for progress in ProgressReporter::new() {
        write!(output, "{}", Renderer(progress))?;
        output.flush()?;

        let nap = Duration::from_millis(uniform.sample(&mut rng));
        thread::sleep(nap);
    }

    Ok(())
}

fn main() -> Result<()> {
    print!("\n\n{}", MoveUp::<1>);
    let result = animate(&mut stdout());
    // Clean up: Always make cursor visible again.
    print!("{}\n\n", ShowCursor);
    result
}
