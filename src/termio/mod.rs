//! Utilities for bringing your own terminal I/O.
//!
//! Bringing your own terminal I/O sounds great on paper. It doesn't force sync
//! or async I/O on your application. If async, it doesn't lock down the
//! (possibly wrong) runtime. In short, it promises flexibility straight out of
//! the crate. But the reality of bringing your own terminal I/O gets gnarly
//! real fast because the on-the-wire protocol has none of the niceties of
//! modern network protocols. It's just a stream of bytes with embedded ANSI
//! escape sequences.
//!
//! This module provides the low-level building blocks for processing this
//! protocol, i.e., writing and reading escape sequences:
//!
//!  * [`VtScanner`] implements the state machines for recognizing ANSI escape
//!    sequences.
//!  * [`Control`] enumerates the different kinds of ANSI escape sequences and
//!    their initial bytes.
//!  * [`Action`] enumerates the different ways applications react to state
//!    machine transitions.
//!
//! The documentation for [`VtScanner`] includes example code for querying a
//! terminal for its theme colors and integrating with the
//! [`trans`](crate::trans) module's [`ThemeEntry`](crate::trans::ThemeEntry)
//! abstraction.
//!
//!
//! # More Generally: Sans I/O
//!
//! Prettypretty's Python version follows that community's "batteries included"
//! approach and includes a generally useful [terminal
//! abstraction](https://github.com/apparebit/prettypretty/blob/main/prettypretty/terminal.py).
//! By contrast, the Rust version requires the application to bring its own
//! terminal I/O. The latter approach, commonly called *Sans I/O* is recognized
//! by both [Python](https://sans-io.readthedocs.io) and
//! [Rust](https://www.firezone.dev/blog/sans-io) communities as an effective
//! means for coping with asynchronous I/O tainting functions throughout an
//! application (i.e., the function coloring challenge). Its value proposition
//! is simple: If we keep I/O out of library code, we can reuse the library with
//! synchronous and asynchronous I/O. With Sans I/O, library code still needs to
//! implement protocol processing, only now it provides a clean interface for
//! plugging the actual I/O routines.

mod escape;

pub use escape::{Action, Control, VtScanner};
