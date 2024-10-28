//! Terminal integration.
//!
//! Bringing your own terminal I/O sounds great at first. It doesn't force sync
//! or async I/O on your application. If your application is async, it doesn't
//! pick the runtime for you. In short, it promises flexibility straight out of
//! the crate. But without additional library support, bringing your own
//! terminal I/O also gets gnarly real fast. That's because command line
//! applications communicate with the terminal (really the terminal emulator)
//! through a protocol (bytes with embedded ANSI escape sequences) that has none
//! of the niceties of actual network protocols. Notably, it doesn't just lack
//! framing, but whether, say, 0x1b represents a press of the escape key or the
//! first byte of an ANSI escape sequence is entirely dependent on context.
//!
//!
//! # Integration of Terminal I/O
//!
//! Producing a protocol stream is as simple as writing to standard output;
//! that's why the displays for [`Style`](crate::style::Style) and
//! [`ThemeEntry`](crate::trans::ThemeEntry) produce ANSI escape sequences.
//! However, consuming such a stream requires three features:
//!
//!  1. [`Terminal`] lets the application disable a terminal's line discipline
//!     and restore it on exit again.
//!  2. [`TerminalReader`] provides the ability to read a terminal's input
//!     stream without blocking indefinitely (with [`Terminal`] doing most of
//!     the work).
//!  3. [`VtScanner`] parses a terminal's byte stream into characters and ANSI
//!     escape sequences, while ignoring malformed byte sequences.
//!
//! [`TerminalReader`] and the corresponding [`TerminalWriter`] are accessible
//! through [`Terminal::reader`] and [`Terminal::writer`]. Alas, like
//! [`Terminal`], they are only available on Unix-like operating systems
//! supported by the [libc](https://github.com/rust-lang/libc) crate. If your
//! application requires async I/O or Windows support, please consider using a
//! more fully-featured terminal crate such as
//! [Crossterm](https://github.com/crossterm-rs/crossterm).
//!
//!
//! # Timing Out Reads
//!
//! There are at least three different approaches to supporting timeouts when
//! reading from the terminal:
//!
//!  1. The first approach relies on the operating system's polling mechanism,
//!     such as `epoll` or `kqueue`. However, polling for a single resource from
//!     within a library seems like an antipattern. Also, macOS supports
//!     `select` only when polling devices including terminals.
//!  2. The second approach uses a helper thread that uses blocking reads for
//!     terminal input and forwards the data to a Rust channel (which supports
//!     timeouts). This approach actually is nicely platform-independent. But
//!     terminating the helper thread seems impossible, unless the operating
//!     system's `TIOCSTI` ioctl or equivalent can be used to inject a poison
//!     value into the input stream.
//!  3. The third approach configures the terminal to time out read operations.
//!     Raw and cbreak modes for terminals usually set the `VMIN` pseudo control
//!     character to 1 and `VTIME` to 0, which instructs the terminal to block
//!     reads until at least one character is available. However, when setting
//!     `VMIN` to 0 and `VTIME` to n>0, the terminal times out waiting after
//!     n*0.1 seconds.
//!
//! This module implements the third approach because it is simple and robust.
//! Notably, it only requires a couple more changes to the terminal
//! configuration over and above the ones already required for cbreak or raw
//! mode. However, since it effectively polls the terminal, the third approach
//! also has higher CPU overhead. That is mitigated somewhat by the large
//! minimum timeout. Alas, that also puts a hard limit on reactivity.
//!
//!
//! # Sans I/O
//!
//! Prettypretty's Python version follows that community's "batteries included"
//! approach and includes a generally useful [terminal
//! abstraction](https://github.com/apparebit/prettypretty/blob/main/prettypretty/terminal.py).
//! By contrast, the Rust version provides just enough functionality to query a
//! terminal for its color theme and only on Unix. If applications have more
//! complex needs, they are expected to bring their own terminal I/O. The latter
//! approach, commonly called *Sans I/O*, is recognized by both
//! [Python](https://sans-io.readthedocs.io) and
//! [Rust](https://www.firezone.dev/blog/sans-io) communities as an effective
//! means for building libraries that avoid the function coloring challenge
//! and equally work with synchronous and asynchronous I/O.

mod escape;
mod render;
#[cfg(target_family = "unix")]
mod unix;

pub use escape::{Action, Control, VtScanner};
pub use render::render;
#[cfg(target_family = "unix")]
pub use unix::{
    Open, ReadWrite, Start, Terminal, TerminalReader, TerminalWriter, TERMINAL_TIMEOUT,
};
