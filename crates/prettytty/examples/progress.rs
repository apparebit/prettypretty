use std::io::{Result, Write};
use std::thread;
use std::time::Duration;

use rand;
use rand::rngs::ThreadRng;
use rand_distr::{Distribution, Normal, Uniform};

use prettytty::cmd::{
    DynMoveTo, EnterAlternateScreen, EraseScreen, ExitAlternateScreen, HideCursor,
    RequestScreenSize, SetDefaultForeground, SetForeground8, ShowCursor,
};
use prettytty::{fuse, Connection, Output, Query};

// -------------------------------------------------------------------------------------

/// Progress is a floating point percentage.
pub type Progress = f32;

/// An iterator over monotonically increasing progress reports.
///
/// The first value is 0.0 and the final value is 100.0. Increments are
/// randomized by a normal distribution with mean 1.0 and standard deviation
/// 2.0/3.0.
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
            rng: rand::rng(),
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

        // Always use old status to ensure iterator produces 0.0
        // Compute min(100.0) to ensure iterator produces 100.0
        let result = self.status.min(100.0);
        if 100.0 <= self.status {
            self.done = true;
        } else {
            // Compute max(0.1) to ensure monotonically increasing progress
            let incr = self.normal.sample(&mut self.rng).max(0.1);
            self.status += incr;
        }

        Some(result)
    }
}

impl std::iter::FusedIterator for ProgressReporter {}

// -------------------------------------------------------------------------------------

/// A progress renderer.
///
/// The renderer displays the progress bar at the current cursor position. It
/// does not adjust that position.s
pub struct Renderer(pub Progress);

// The progress indicator itself has 25 fixed-width cells, each capable of
// displaying 4 steps, which makes for a resolution of 100 distinct steps total.
// Add to that nine more cells with the ┫caps┣, a space, and the percentage
// number, which is 100.0% at most.
const WIDTH: usize = CELLS + 9;
const CELLS: usize = 25;
const STEPS: usize = 4;

impl std::fmt::Display for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let uprog = self.0 as usize;
        let full = uprog / STEPS;
        let partial = uprog % STEPS;
        let empty = CELLS - full - (if 0 < partial { 1 } else { 0 });

        // The 11th 8-bit color is bright green
        write!(f, "┫{}", SetForeground8::<10>)?;
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

/// Animate the progress bar
pub fn animate(output: &mut Output, row: u16, column: u16) -> Result<()> {
    // Nap time is between 1/60 and 1/10 seconds
    let uniform = Uniform::new_inclusive(16, 100).map_err(|e| std::io::Error::other(e))?;
    let mut rng = rand::rng();

    for progress in ProgressReporter::new() {
        write!(output, "{}{}", DynMoveTo(row, column), Renderer(progress))?;
        output.flush()?;

        let nap = Duration::from_millis(uniform.sample(&mut rng));
        thread::sleep(nap);
    }

    thread::sleep(Duration::from_millis(500));
    Ok(())
}

fn main() -> Result<()> {
    let tty = Connection::open()?;
    let (mut input, mut output) = tty.io();

    // Prepare screen
    output.exec_defer(
        fuse!(EnterAlternateScreen, EraseScreen, HideCursor),
        fuse!(ShowCursor, ExitAlternateScreen),
    )?;

    // Determine the bar's coordinates
    let (row, column) = RequestScreenSize.run(&mut input, &mut output)?;
    let (row, column) = (
        row.saturating_sub(1) / 2,
        column.saturating_sub(WIDTH as u16) / 2,
    );

    // Run the animation
    animate(&mut output, row, column)
}
