//! Optional utility module for terminal integration. <i
//! class=term-only>Term only!</i>
//!
//! This module interfaces with the process' controlling terminal. It manages
//! the connection as well as the terminal's mode. It also provides read and
//! write access, including for arbitrary ANSI escape sequences.
//!
//! **Access controlling terminal with [`terminal()`]**. You can manually manage
//! the connection with [`Terminal::connect`] / [`Terminal::connect_with`] and
//! [`Terminal::disconnect`] or automatically connect with [`Terminal::access`]
//! / [`Terminal::access_with`]. Since manual connection management requires
//! explicit tear down, both `connect` and `connect_with` are unsafe.
//!
//! **Perform I/O with [`TerminalAccess`]**, which provides exclusive access to
//! the terminal. [`Scanner`] implements state machines for UTF-8 and ANSI
//! escape sequences, thereby turning individual bytes into tokens for text,
//! control characters, and control sequences.
//!
//!
//! # Examples
//!
//! Pretty much all it takes is to delegate writing the query and reading the
//! response to theme entries.
//!
//! ```compile_only
//! # use prettypretty::term::{terminal, VtScanner};
//! # use prettypretty::theme::ThemeEntry;
//! // Access terminal and create scanner
//! let mut tty = terminal().access()?;
//! let mut scanner = VtScanner::new();
//!
//! // Write query
//! write!(tty, "{}", ThemeEntry::DefaultForeground)?;
//! tty.flush()?;
//!
//! // Read response
//! let color = ThemeEntry.read_with(&mut tty, &mut scanner)?;
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! This actually is a more general pattern, thanks to the
//! [`Command`](crate::cmd::Command) and [`Query`](crate::cmd::Query) traits.
//!
//!
//! # Background
//!
//! Integrating terminal I/O is trivial on Unix, as long as an application does
//! not need to read terminal input: The application simply writes text and ANSI
//! escape sequences to style the text to standard output or error. For just
//! that reason, the display of [`Style`](crate::style::Style) is the ANSI
//! escape sequence that changes the terminal to use that style.
//!
//!
//! ## 1. Gnarly Input
//!
//! However, when an application also needs to read terminal input, notably for
//! processing individual key presses or querying the terminal with ANSI escape
//! sequences, things get gnarly real fast for three reasons:
//!
//!  1. By default, terminals serve as line editor and hence also expose the
//!     input only line by line. That gets in the way of reading individual key
//!     presses or ANSI escape sequences that aren't enter key presses.
//!  2. Even when the line discipline is disabled, reading terminal input is a
//!     blocking operation that waits until some bytes become available. That
//!     gets in the way of graceful error recovery, which requires timeouts.
//!     Alas, Rust gets confused when reads return zero bytes and treats them as
//!     end-of-file conditions.
//!  3. Key presses and ANSI escape sequences have complex and overlapping
//!     syntax. Furthermore, correctly parsing ANSI escape sequences in the
//!     presence of errors requires one byte lookahead.
//!
//! Taking a cue from the implementation of `stdio` in the Rust standard
//! library, the [`terminal()`] function and [`Terminal`] as well as
//! [`TerminalAccess`] structs address the first and second challenges, for now
//! for Unix only. Meanwhile, taking a cue from Paul Flo Williams' [state
//! machine for DEC's ANSI-compatible video
//! terminals](https://vt100.net/emu/dec_ansi_parser) and the open source
//! implementations for Alacritty's [vte](https://github.com/alacritty/vte) and
//! Wezterm's [vtparse](https://github.com/wez/wezterm) crates, the [`Scanner`]
//! struct addresses the third challenge. Together, they make for a lean but
//! functional terminal integration layer.
//!
//! However, they won't meet all application needs. Notably, if your application
//! requires async I/O, please consider using a more fully-featured terminal
//! crate such as [Crossterm](https://github.com/crossterm-rs/crossterm). For
//! the same reason, this module is optional and requires the `term` feature.
//!
//!
//! ## 2. Ways to Time Out Reads
//!
//! When it comes to changing the terminal mode, there is little choice of
//! mechanism on Unix systems: `tcgetattr` and `tcsetattr` are the only game in
//! town. However, when it comes to timing out reads, there are three major
//! options:
//!
//!  1. Use the operating system's polling mechanism, such as `epoll` or
//!     `kqueue`. However, polling for a single resource from within a library
//!     seems like a definite antipattern. Also, macOS supports the slow and
//!     non-scalable `select` only when polling devices including terminals.
//!  2. Use a helper thread that uses blocking reads for terminal input and
//!     forwards the data to a Rust channel (which supports read timeouts). This
//!     approach has the benefit of being platform-independent. But terminating
//!     the helper thread seems impossible, unless the operating system's
//!     `TIOCSTI` ioctl or equivalent can be used to inject a poison value into
//!     the input stream.
//!  3. Configure the terminal to time out read operations. The cbreak ("rare")
//!     and raw modes for terminals usually set the pseudo-control characters
//!     `VMIN` and `VTIME` to 1 and 0, respectively. That instructs the terminal
//!     to block reads until at least one byte is available with no timeout.
//!     However, when setting `VMIN` and `VTIME` to 0 and n>0, respectively, the
//!     terminal times out after n*0.1 seconds even if there are no bytes
//!     available.
//!
//! Since this module already modifies the terminal configuration, the third
//! option is an attractive choice. Its simplicity and robustness cinch the
//! deal.
//!
//! Alas, there are two potential pitfalls. First, compared to `epoll` and
//! `kqueue`, fixed timeouts may result higher CPU overhead due to polling.
//! Though, that shouldn't be a problem given the (large) 0.1s increments for
//! timeouts. Second, those same increments do put a hard limit on reactivity
//! for other signals. If either becomes an issue, an application should
//! consider switching to `epoll` or `kqueue`.
//!
//! A third pitfall is that Rust turns read operations that return zero bytes
//! into end-of-file errors. This module helps to mitigate those errors, but an
//! application may need to detect them as well.
//!
//!
//! ## 3. Windows
//!
//! Starting with Windows 10 TH2 (v1511), Windows also supports ANSI escape
//! sequences. While applications currently need to explicitly enable virtual
//! terminal processing, they also are the preferred means for interacting with
//! the console host moving forward, and several console functions that provide
//! equivalent functionality have been deprecated.
//!
//! The Windows console host provides much of the same functionality as Unix
//! pseudo-terminals with a completely different API. Two differences stick out:
//!
//!  1. To interact with the terminal independent of redirected streams, a Unix
//!     application connects to one endpoint by opening `/dev/tty`. Windows has
//!     independent abstractions for input and output, with the equivalent code
//!     on Windows opening `CONIN$` and `CONOUT$`. As a direct consequence,
//!     configuring the console on Windows also requires reading and writing two
//!     modes.
//!  2. The Windows API duplicates a large number of functions, with the name
//!     ending either in `A` or `W`, e.g., `ReadConsoleA` vs `ReadConsoleW`. The
//!     former represent strings with 8-bit characters, whose encoding is
//!     determined by the current "code page". The latter represent strings as
//!     UTF-16. Thankfully, there is a code page for UTF-8, but that does
//!     require reading and writing the code page for input and output during
//!     the initial configuration.
//!
//! Meanwhile, timing out reads is easy. `WaitForSingleObject` takes only two
//! arguments, the handle for console input and the timeout, and gets the job
//! done. Potentially adding a future waker is easy as well: Just switch to
//! `WaitForMultipleObjects`.

mod scan;
mod sys;
mod terminal;
mod util;

pub(crate) use util::{is_semi_colon, Radix, SliceExt};

pub use scan::{Control, Error, Scanner, Token};
pub use terminal::{terminal, Mode, OptionBuilder, Options, Terminal, TerminalAccess};
pub use util::{write_nicely, write_nicely_with_column};
