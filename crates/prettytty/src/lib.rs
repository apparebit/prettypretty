#![doc(
    html_logo_url = "https://raw.githubusercontent.com/apparebit/prettypretty/refs/heads/main/docs/figures/prettytty.png"
)]

//! # The prettytty terminal library 🌸
//!
//! This crate provides **lightweight and cross-platform terminal access**. Its
//! only dependency is the low-level crate enabling system calls, i.e.,
//! [`libc`](https://crates.io/crates/libc) on Unix and
//! [`windows-sys`](https://crates.io/crates/windows-sys) on Windows. Similar to
//! [crossterm](https://crates.io/crates/crossterm), prettytty sports a
//! [`Command`] trait for terminal instructions. But it doesn't stop there and
//! also supports a [`Query`] trait for processing responses. While those traits
//! ensure easy extensibility, the [`cmd`] library with over 70 built-in
//! commands probably covers your needs already.
//!
//! To facilitate integration with sync and async I/O, **terminal input times
//! out**. That suffices for simple polling but is slow when there is no input.
//! Otherwise, it's easy enough to integrate a dedicated polling thread with
//! your favorite notification primitive.
//!
//! Accessing the terminal is as simple as **opening a [`Connection`] and using
//! its [`Input`] and [`Output`]**. The former not only supports buffered input
//! with [`Read`](std::io::Read) and [`BufRead`](std::io::BufRead) but, more
//! importantly, reading text and control sequence [`Token`]s with [`Scan`]. The
//! latter implements [`Write`](std::io::Write) and provides auto-flushing
//! [`print()`](Output::print) and [`exec`](Output::exec) as well.
//!
//!
//! # Example
//!
//! Thanks to [`Connection`], [`Input`], and [`Output`], interacting with the
//! terminal is a breeze:
//!
//! ```
//! # use std::io::{ErrorKind, Result};
//! # use prettytty::{Connection, Query, Scan};
//! # use prettytty::cmd::{MoveToColumn, RequestCursorPosition};
//! # use prettytty::opt::Options;
//! # fn run() -> Result<()> {
//! // Open a terminal connection with 1s timeout.
//! let tty = Connection::with_options(
//!     Options::builder().timeout(10).build())?;
//!
//! let pos = {
//!     let (mut input, mut output) = tty.io();
//!
//!     // Move cursor, issue query for position.
//!     output.exec(MoveToColumn(17))?;
//!     output.exec(RequestCursorPosition)?;
//!
//!     // Read and parse response.
//!     let response = input.read_sequence(
//!         RequestCursorPosition.control())?;
//!     RequestCursorPosition.parse(response)?
//! };
//!
//! assert_eq!(pos.1, 17);
//! # Ok(())
//! # }
//! # // Treat connection refused errors in CI as implying no TTY.
//! # match run() {
//! #     Ok(()) => (),
//! #     Err(err) if err.kind() == ErrorKind::ConnectionRefused &&
//! #         std::env::var_os("CI").is_some() => (),
//! #     Err(err) => return Err(err),
//! # };
//! # Ok::<(), std::io::Error>(())
//! ```
//!

mod api;
pub mod cmd;
mod conn;
pub mod err;
pub mod opt;
mod scan;
mod sys;
pub mod util;

pub use api::{Command, Control, Query, Scan, Sgr, Token};
pub use conn::{Connection, Input, Output};
