//! Optional utility module for terminal integration. <i
//! class=term-only>Term only!</i>
//!
//! This module provides just enough terminal integration to query a terminal
//! for its current color theme. Two abstractions stand out:
//!
//!   - [`Terminal`] represents the controlling terminal. It is accessed with
//!     [`terminal()`] and provides access to terminal I/O with
//!     [`TerminalAccess`].
//!   - [`VtScanner`] implements the state machine for recognizing ANSI escape
//!     sequences.
//!
//! When combined with [`Theme`](crate::trans::Theme) and
//! [`ThemeEntry`](crate::trans::ThemeEntry), querying the terminal for its
//! color theme looks straightforward. Let's walk through one possible solution.
//!
//! # Examples
//!
//! ## 1. Taking Care of Not-Unix
//!
//! First, since terminal support is limited to Unix only, lets set up two
//! versions of the same function, one for other platforms and one for Unix.
//! Since we are assembling a color theme by interacting with the terminal,
//! `std::io::Result<Theme>` seems like a good result type, with
//! [`Theme`](crate::trans::Theme) collecting the two default and 16 ANSI colors
//! belonging to the, ahem, theme:
//!
//! ```
//! # use std::io::{ErrorKind, Result};
//! # use prettypretty::trans::Theme;
//! // Return an error indicating that platform is not supported.
//! #[cfg(not(target_family = "unix"))]
//! fn query() -> Result<Theme> {
//!     Err(ErrorKind::Unsupported.into())
//! }
//!
//! #[cfg(target_family = "unix")]
//! fn query() -> Result<Theme> {
//!     // Ooh, 𒌋𒐜 × XYZ's origin... that's pitch black.
//!     Ok(Theme::default())
//! }
//!
//! // Done.
//! let _ = query();
//! ```
//!
//! ## 2. Function Set Up and Outer Loop
//!
//! Great, now that we have a fallback for non-Unix platforms and the function
//! skeleton for Unix, we can start filling in the latter. In particular, in
//! addition to the dummy theme from last time, we need to set up terminal
//! access and an ANSI escape sequence parser. And while we are at it, we might
//! as well add the outer loop, too. It iterates over the
//! [`ThemeEntry`](crate::trans::ThemeEntry) objects:
//!
//! ```
//! # use std::io::{ErrorKind, Result};
//! # #[cfg(target_family = "unix")]
//! # use prettypretty::term::{terminal, VtScanner};
//! # use prettypretty::trans::{Theme, ThemeEntry};
//! # #[cfg(not(target_family = "unix"))]
//! # fn query() -> Result<Theme> {
//! #     Err(ErrorKind::Unsupported.into())
//! # }
//! #[cfg(target_family = "unix")]
//! fn query() -> Result<Theme> {
//!     // Set up state.
//!     let mut tty = terminal().access()?;
//!     let mut scanner = VtScanner::new();
//!     let mut theme = Theme::default();
//!
//!     for entry in ThemeEntry::all() {
//!         // Process theme entry by theme entry.
//!     }
//!
//!     Ok(theme)
//! }
//! # let _ = query();
//! ```
//!
//! By far the most important incantation amongst the code we just added is the
//! invocation of [`Terminal::access`]: That method connects to the terminal
//! device, configures it to use non-canonical mode and a 0.1s read timeout, and
//! returns an object that reads from and writes to the terminal, no matter
//! whether standard streams are redirected or not. Even better, when that `tty`
//! object is dropped, it not only relinquishes its exclusive hold on terminal
//! I/O, but it also restores the terminal's original (cooked) mode and closes
//! the connection again.
//!
//! If your application needs a longer-living connection to the terminal, it can
//! more directly manage the connection with [`Terminal::connect`] and
//! [`Terminal::disconnect`]. In that case, automatic (re)connection is an
//! anti-feature and [`Terminal::try_access`] provides access only if the
//! terminal is still connected. [`Terminal::connect_with`] and
//! [`Terminal::access_with`] provide additional knobs for fine-tuning the
//! terminal configuration.
//!
//!
//! ## 3. Write Query, Ingest Response, Parse Color, Update Theme
//!
//! With that, we are ready to query the terminal for some colors:
//!
//! ```
//! # use std::io::{BufRead, Error, ErrorKind, Result, Write};
//! # #[cfg(target_family = "unix")]
//! # use prettypretty::term::{terminal, VtScanner};
//! # use prettypretty::trans::{Theme, ThemeEntry};
//! # #[cfg(not(target_family = "unix"))]
//! # fn query() -> Result<Theme> {
//! #     Err(ErrorKind::Unsupported.into())
//! # }
//! #[cfg(target_family = "unix")]
//! fn query() -> Result<Theme> {
//!     let mut tty = terminal().access()?;
//!     let mut scanner = VtScanner::new();
//!     let mut theme = Theme::default();
//!
//!     for entry in ThemeEntry::all() {
//!         // Write query as escape sequence.
//!         write!(tty, "{}", entry)?;
//!         tty.flush()?;
//!
//!         // Read response as escape sequence.
//!         let response = scanner.scan_str(&mut tty)?;
//!
//!         // Parse color.
//!         let color = entry
//!             .parse_response(response)
//!             .map_err(|e| Error::new(
//!                 ErrorKind::InvalidData, e
//!             ))?;
//!
//!         // Update theme.
//!         theme[entry] = color;
//!     }
//!
//!     Ok(theme)
//! }
//! # let _ = query();
//! ```
//!
//! Write the query, ingest the response, parse the color, and update the theme.
//! Out of these four steps, parsing the color looks a bit more involved. But
//! that's only because we manually adjust the error type. Instead, the most
//! powerful incantation we just added is the invocation of
//! [`VtScanner::scan_str`]. The method hides an entire inner loop consuming the
//! terminal input byte by byte until a complete ANSI escape sequence has been
//! recognized. The documentation for [`VtScanner`] explores that method's
//! implementation in rather gory detail.
//!
//! In any case, that's it. That's all the code necessary for querying the
//! terminal for its current color theme. That's pretty much also the
//! implementation of
//! [`Theme::query_terminal`](crate::trans::Theme::query_terminal).
//!
//!
//! ## 4. Validate Color Theme
//!
//! While it provides a realistic example for interacting with the terminal, the
//! example code isn't really acceptable for its intended purpose as
//! documentation test. After all, it doesn't do any testing. However,
//! validating the output is more difficult in this case because every terminal
//! may just have its own color theme.
//!
//! While we can't predict exact color values, those colors aren't picked
//! randomly either. Each theme entry has a well-established name, and
//! applications tend to use colors consistently with their names. Hence, ANSI
//! red might be used to highlight errors, but for highlighting successful
//! completion probably not so much.
//!
//! We can leverage that to devise a practical testing strategy. First, we
//! observe that the six non-bright chromatic colors amongst ANSI colors
//! identify the RGB primaries and secondaries. In other words, they are placed
//! in roughly equal intervals around the hue circle in a perceptually uniform,
//! polar color space such as Oklrch. Second, as discussed in the previous
//! paragraph, we can reasonable assume that color values are roughly consistent
//! with their names. Hence the test processes colors in the order of their
//! names around the hue circle and checks whether each hue falls onto an
//! acceptable arc, say, of 135º. Since there are six colors, we rotate the arc
//! for each color by 360º/6=60º.
//!
//! Here's the corresponding code:
//!
//! ```
//! # use std::io::{BufRead, Error, ErrorKind, Result, Write};
//! # use prettypretty::{Color, ColorSpace};
//! # use prettypretty::style::AnsiColor::*;
//! # #[cfg(target_family = "unix")]
//! # use prettypretty::term::{terminal, VtScanner};
//! # use prettypretty::trans::{Theme, ThemeEntry};
//! # #[cfg(not(target_family = "unix"))]
//! # fn query() -> Result<Theme> {
//! #     Err(ErrorKind::Unsupported.into())
//! # }
//! #[cfg(target_family = "unix")]
//! fn query() -> Result<Theme> {
//!     let mut tty = terminal().access()?;
//!     let mut scanner = VtScanner::new();
//!     let mut theme = Theme::default();
//!
//!     for entry in ThemeEntry::all() {
//!         write!(tty, "{}", entry)?;
//!         tty.flush()?;
//!
//!         let response = scanner.scan_str(&mut tty)?;
//!         let color = entry
//!             .parse_response(response)
//!             .map_err(|e| Error::new(
//!                 ErrorKind::InvalidData, e
//!             ))?;
//!         theme[entry] = color;
//!     }
//!
//!     // Prepare the Oklrch version of the theme.
//!     let colors: Vec<Color> = theme
//!         .as_ref()
//!         .iter()
//!         .map(|c| c.to(ColorSpace::Oklrch))
//!         .collect();
//!     let oktheme = Theme::with_slice(&colors).unwrap();
//!
//!     // Let's validate the hues of the six chromatic nonbright colors.
//!     let mut expected = -45.0_f64 .. 90.0_f64;
//!     for index in [Red, Yellow, Green, Cyan, Blue, Magenta] {
//!         // Since the minimum acceptable hue starts negative,
//!         /// we possibly need to adjust the actual hue, too.
//!         let mut hue = oktheme[index][2];
//!         if expected.start < 0.0_f64 && expected.end + 180.0_f64 < hue {
//!             hue -= 360.0_f64;
//!         }
//!
//!         assert!(
//!             expected.contains(&hue),
//!             "{:>20}  {} < {} < {}",
//!             index.name(), expected.start, hue, expected.end
//!         );
//!
//!         // With the six colors spread around hue circle, increment is 60º.
//!         expected = (expected.start + 60.0_f64) .. (expected.end + 60.0_f64);
//!     }
//!
//!     Ok(theme)
//! }
//! # let _ = query();
//! ```
//!
//! The negative starting angle for red ensures that the arc is continuous
//! numerically, which simplifies testing whether the hue falls onto the arc.
//! However, it also requires adjusting the hue so that it starts out
//! consistently. Can you fill in the code for the bright chromatic colors? And
//! what about non-chromatic colors?
//!
//!
//! # Background
//!
//! Integrating terminal I/O is trivial, as long as an application does not need
//! to read terminal input: The application simply writes text and ANSI escape
//! sequences to style the text to standard output or error. For just that
//! reason, the display of [`Style`](crate::style::Style) is the ANSI escape
//! sequence that changes the terminal to use that style.
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
//! Wezterm's [vtparse](https://github.com/wez/wezterm) crates, the
//! [`VtScanner`] struct addresses the third challenge. Together, they make for
//! a lean but functional terminal integration layer.
//!
//! However, they won't meet all application needs. Notably, if your application
//! requires Windows support or async I/O, please consider using a more
//! fully-featured terminal crate such as
//! [Crossterm](https://github.com/crossterm-rs/crossterm). For the same reason,
//! this module is option and requires the `term` feature.
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

mod escape;
mod render;
mod sys;
mod terminal;

pub use escape::{Action, Control, VtScanner};
pub use render::render;
pub use terminal::{terminal, Mode, Options, Terminal, TerminalAccess};
