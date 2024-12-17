#![doc(html_logo_url = "")]

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
//! # use prettytty::{Connection, Query, Scan, cmd::{MoveTo, RequestCursorPosition}};
//! let rcp = RequestCursorPosition;
//! let tty = Connection::open()?;
//! let pos = {
//!     let (mut output, mut input) = (tty.output(), tty.input());
//!     output.exec(MoveTo(6, 65))?;
//!     output.exec(rcp)?;
//!
//!     let response = input.read_sequence(rcp.control())?;
//!     rcp.parse(response)?
//! };
//! drop(tty);
//! assert_eq!(pos, (6, 65));
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
