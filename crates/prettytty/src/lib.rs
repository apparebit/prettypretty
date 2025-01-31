#![doc(
    html_logo_url = "https://raw.githubusercontent.com/apparebit/prettypretty/refs/heads/main/docs/figures/prettytty.png"
)]

//! # Pretty 🌸 Tty
//!
//! \[  [**Docs.rs**](https://docs.rs/prettypretty/latest/prettytty/)
//! | [**GitHub Pages**](https://apparebit.github.io/prettypretty/prettytty/)
//! | [**Rust Crate**](https://crates.io/crates/prettytty)
//! | [**Repository**](https://github.com/apparebit/prettypretty)
//! \]
//!
//! This crate provides **lightweight and cross-platform terminal access**. Its
//! only dependency is the low-level crate enabling system calls, i.e.,
//! [`libc`](https://crates.io/crates/libc) on Unix and
//! [`windows-sys`](https://crates.io/crates/windows-sys) on Windows.
//!
//! Using its **connection-oriented interface** is easy:
//!
//!   * Open a [`Connection`].
//!   * Issue [`Command`]s by writing their [`Display`](std::fmt::Display) to
//!     the connection's [`Output`].
//!   * [`Scan`] and read [`Query`] responses from its [`Input`].
//!
//! More generally, [`Input`] implements [`Read`](std::io::Read),
//! [`BufRead`](std::io::BufRead), and [`Scan`], with the latter turning bytes
//! into UTF-8 text, control code, and control sequence tokens. Meanwhile,
//! [`Output`] implements [`Write`](std::io::Write) as well as the auto-flushing
//! [`print()`](Output::print), [`println()`](Output::println), and
//! [`exec()`](Output::exec) methods.
//!
//! The [`cmd`] module provides a **library of common [`Command`] and [`Query`]
//! implementations**. It includes, for example, commands to set the window
//! title, erase (parts of) the screen, to move the cursor, and to style text.
//!
//! To facilitate orderly shutdown, **read operations time out** in configurable
//! increments of 0.1s. That suffices for simple polling but is slow when there
//! is no input. If you need faster timeouts or integration with I/O
//! notifications, use a dedicated polling thread with either an
//! [`std::sync::mpsc`] queue or Unix domain socket.
//!
//! Since terminal connections reconfigure the terminal, an application should
//! go out of its way to **always run [`Connection`]'s drop handler**.
//!
//!
//! # Example
//!
//! Prettytty's connection-oriented interface makes interacting with the
//! terminal a breeze:
//!
//! ```
//! # use std::io::{ErrorKind, Result};
//! # use prettytty::{Connection, Query, Scan};
//! # use prettytty::cmd::{MoveToColumn, RequestCursorPosition};
//! # use prettytty::opt::Options;
//! # fn run() -> Result<()> {
//! // Open a terminal connection.
//! let tty = Connection::with_options(Options::default())?;
//!
//! let pos = {
//!     let (mut input, mut output) = tty.io();
//!
//!     // Move cursor, issue query for position.
//!     output.exec(MoveToColumn::<17>)?;
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
//! # Windows
//!
//! Prettytty uses platform-specific APIs for configuring the terminal, notably
//! for setting the correct mode. By contrast, commands and queries are
//! implemented with ANSI escape sequences. Windows started supporting control
//! sequences for styling output with Windows 10 version 1511 only. It started
//! supporting queries for the current color theme with Windows Terminal 1.22
//! only. Hence, we strongly recommend using prettytty with Windows Terminal
//! 1.22 or later.

mod api;
pub mod cmd;
mod conn;
pub mod err;
pub mod opt;
mod read;
mod scan;
mod sys;
pub mod util;

pub use api::{Command, Control, Query, Scan, Sgr, Token};
pub use conn::{Connection, Input, Output};
