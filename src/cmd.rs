//! Controlling the terminal with ANSI escape sequences.
//!
//! [`Command`] is an instruction to change a terminal's screen, cursor,
//! content, or some such. To execute a command, you write its display to the
//! terminal's output or error stream. The command's implementation then writes
//! the necessary ANSI escape sequence to the terminal. This works even on
//! Windows, which added support in Windows 10 TH2 (v1511).
//!
//! [`Query`] encapsulates the functionality for reading and parsing the
//! response returned by a terminal after receiving the corresponding request.
//! An implementation checks with [`Query::is_valid`] whether the response
//! started with the right control and then uses [`Query::parse`] to parse the
//! response payload. [`Query::query`] builds on these two required methods to
//! coordinate between [`TerminalAccess`](crate::term::TerminalAccess),
//! [`VtScanner`], and [`Query`].
//!
//! This modules also implements a library of useful commands. Each command
//! implements the `Debug` and `Display` traits as well. The `Debug`
//! representation is the usual datatype representation, whereas the `Display`
//! representation is the ANSI escape sequence. As a result, all commands
//! defined by this module can be directly written to terminal output, just like
//! [`Style`](crate::style::Style) and [`ThemeEntry`](crate::theme::ThemeEntry).
//!
//! The core library includes the following commands:
//!
//!   * For terminal management, [`RequestTerminalId`].
//!   * For window title management, [`SaveWindowTitleOnStack`],
//!     [`LoadWindowTitleFromStack`], and [`SetWindowTitle`].
//!   * For screen management, [`EnterAlternateScreen`] and
//!     [`ExitAlternateScreen`], also [`EraseScreen`] and [`EraseLine`].
//!   * For cursor management, [`HideCursor`] and [`ShowCursor`], the relative
//!     [`MoveUp`], [`MoveDown`], [`MoveLeft`], and [`MoveRight`], the absolute
//!     [`MoveToColumn`], [`MoveToRow`], and [`MoveTo`], also
//!     [`SaveCursorPosition`], [`RestoreCursorPosition`], and
//!     [`RequestCursorPosition`].
//!   * For grouping content, [`RequestBatchMode`], [`BeginBatchedOutput`] and
//!     [`EndBatchedOutput`], [`BeginBracketedPaste`] and [`EndBracketedPaste`],
//!     also [`Link`].
//!   * For showing arbitrary content, [`Print`].
//!
//! If a command starts with `Request` in its name, it is a query and implements
//! the [`Query`] trait in addition to [`Command`].
//!

#![allow(dead_code)]
use std::io::{BufRead, Error, ErrorKind, Result};

use crate::term::{Control, VtScanner};

#[macro_export]
macro_rules! csi {
    ( $( $literal:literal ),+ ) => { concat!("\x1b[", $( $literal ),+) };
}

/// A terminal command.
///
/// Every command has its own ANSI escape sequence. Simple commands have no
/// parameters and hence always produce the same escape sequence. They are
/// implemented as zero-sized types. In contrast, parameterized commands require
/// storage space.
pub trait Command {
    /// Write out the command's ANSI escape sequence.
    fn write_ansi(&self, out: &mut impl ::std::fmt::Write) -> ::std::fmt::Result;
}

/// A borrowed command also is a command.
impl<C: Command> Command for &C {
    fn write_ansi(&self, out: &mut impl ::std::fmt::Write) -> ::std::fmt::Result {
        (*self).write_ansi(out)
    }
}

/// A command that receives a response.
///
/// Since UTF-8 has more invariants than byte slices, this trait represents the
/// payload of ANSI escape sequences as `&[u8]`. Use [`VtScanner::to_str`] to
/// convert to string slice if needed.
pub trait Query {
    /// The type of the response data.
    type Response;

    /// Determine whether the control is the expected control for the response.
    fn is_valid(&self, control: Control) -> bool;

    /// Parse the payload into a response object.
    fn parse(&self, payload: &[u8]) -> Result<Self::Response>;

    fn query(&self, reader: &mut impl BufRead, scanner: &mut VtScanner) -> Result<Self::Response> {
        // To avoid a borrow checker error, we scan sequence first but do not
        // retain the result. We access the control second and the text of the
        // ANSI escape sequence third.
        scanner.scan_bytes(reader)?;
        let control = scanner.completed_control();
        let payload = scanner.finished_bytes()?;

        if control.is_none() || !self.is_valid(control.unwrap()) {
            return Err(ErrorKind::InvalidInput.into());
        }

        self.parse(payload)
    }
}

/// A borrowed query is a query.
impl<Q: Query> Query for &Q {
    type Response = Q::Response;

    fn is_valid(&self, control: Control) -> bool {
        (*self).is_valid(control)
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        (*self).parse(payload)
    }
}

// -------------------------------------- Macros ---------------------------------------

macro_rules! define_expr_impl {
    ($name:ident { $repr:expr }) => {
        impl crate::cmd::Command for $name {
            #[inline]
            fn write_ansi(&self, out: &mut impl ::std::fmt::Write) -> ::std::fmt::Result {
                out.write_str($repr)
            }
        }
    }
}

macro_rules! define_impl {
    ($name:ident : $selfish:ident ; $output:ident $body:block ) => {
        impl crate::cmd::Command for $name {
            #[inline]
            fn write_ansi(& $selfish, $output: &mut impl ::std::fmt::Write) -> ::std::fmt::Result {
                $body
            }
        }
    }
}

macro_rules! define_display {
    ($name:ident) => {
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.write_ansi(f)
            }
        }
    };
}

macro_rules! define_simple_command {
    ($name:ident, $ansi:tt) => {
        #[doc = "The 0-ary `"]
        #[doc = stringify!($name)]
        #[doc = "` command."]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name;

        define_expr_impl!($name { $ansi });
        define_display!($name);
    };
}

macro_rules! define_num_arg_command {
    ($name:ident, $prefix:literal, $suffix:literal) => {
        #[doc = "The 1-ary `"]
        #[doc = stringify!($name)]
        #[doc = "` command."]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name(u16);

        define_impl!($name: self; out {
            out.write_str($prefix)?;
            write!(out, "{}", self.0)?;
            out.write_str($suffix)
        });
        define_display!($name);
    };
}

// -------------------------------- Terminal Management --------------------------------

define_simple_command!(RequestTerminalId, "\x1b[>q");

impl Query for RequestTerminalId {
    type Response = (Vec<u8>, Option<Vec<u8>>);

    fn is_valid(&self, control: Control) -> bool {
        matches!(control, Control::DCS)
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        let s = payload
            .strip_prefix(b">|")
            .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;

        if let Some(s) = s.strip_suffix(b")") {
            let (n, v) = s
                .iter()
                .position(|byte| *byte == b'(')
                .map(|index| s.split_at(index))
                .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
            let name = n.to_owned();
            let version = v.to_owned();
            Ok((name, Some(version)))
        } else {
            let name = s.to_owned();
            Ok((name, None))
        }
    }
}

// --------------------------------- Window Management ---------------------------------

define_simple_command!(SaveWindowTitleOnStack, "\x1b[22;2t");
define_simple_command!(LoadWindowTitleFromStack, "\x1b[23;2t");

/// The 1-ary `SetWindowTitle` command.
pub struct SetWindowTitle(String);
define_impl!(SetWindowTitle: self; out {
    out.write_str("\x1b]2;")?;
    out.write_str(&self.0)?;
    out.write_str("\x1b\\")
});
define_display!(SetWindowTitle);

// --------------------------------- Screen Management ---------------------------------

define_simple_command!(EnterAlternateScreen, "\x1b[?1049h");
define_simple_command!(ExitAlternateScreen, "\x1b[?1049l");

define_simple_command!(EraseScreen, "\x1b[2J");
define_simple_command!(EraseLine, "\x1b[2K");

// --------------------------------- Cursor Management ---------------------------------

define_simple_command!(HideCursor, "\x1b[?25l");
define_simple_command!(ShowCursor, "\x1b[?25h");

define_num_arg_command!(MoveUp, "\x1b[", "A");
define_num_arg_command!(MoveDown, "\x1b[", "B");
define_num_arg_command!(MoveLeft, "\x1b[", "C");
define_num_arg_command!(MoveRight, "\x1b[", "D");

/// The 2-ary `MoveTo` *row, column* command.
#[derive(Clone, Copy, Debug)]
pub struct MoveTo(u16, u16);

define_impl!(MoveTo: self; out {
    out.write_str("\x1b[")?;
    out.write_fmt(format_args!("{}", self.0))?;
    out.write_str(";")?;
    out.write_fmt(format_args!("{}", self.1))?;
    out.write_str("H")
});
define_display!(MoveTo);

define_num_arg_command!(MoveToColumn, "\x1b[", "G");
define_num_arg_command!(MoveToRow, "\x1b[", "d");

define_simple_command!(RequestCursorPosition, "\x1b[6n");

impl Query for RequestCursorPosition {
    type Response = (u16, u16);

    fn is_valid(&self, control: Control) -> bool {
        matches!(control, Control::CSI)
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        let s = payload
            .strip_prefix(b"\x1b[")
            .and_then(|s| s.strip_suffix(b"R"))
            .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;

        let params = VtScanner::split_params(s)?;
        if params.len() != 2
            || params[0].is_none()
            || params[1].is_none()
            || (u16::MAX as u64) < params[0].unwrap()
            || (u16::MAX as u64) < params[1].unwrap()
        {
            return Err(ErrorKind::InvalidData.into());
        }
        Ok((params[0].unwrap() as u16, params[1].unwrap() as u16))
    }
}

define_simple_command!(SaveCursorPosition, "\x1b7");
define_simple_command!(RestoreCursorPosition, "\x1b8");

// -------------------------------- Content Management ---------------------------------

define_simple_command!(RequestBatchMode, "\x1b[?2026$p");

/// The current batch processing mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchMode {
    NotSupported = 0,
    Enabled = 1,
    Disabled = 2,
    Undefined = 3,
    PermanentlyDisabled = 4,
}

impl Query for RequestBatchMode {
    type Response = BatchMode;

    fn is_valid(&self, control: Control) -> bool {
        matches!(control, Control::CSI)
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        let s = payload
            .strip_prefix(b"\x1b[?2026;")
            .and_then(|s| s.strip_suffix(b"$y"))
            .ok_or_else(|| Error::from(ErrorKind::ConnectionRefused))?;
        Ok(match VtScanner::to_u64(s)? {
            0 => BatchMode::NotSupported,
            1 => BatchMode::Enabled,
            2 => BatchMode::Disabled,
            4 => BatchMode::PermanentlyDisabled,
            _ => BatchMode::Undefined,
        })
    }
}

define_simple_command!(BeginBatchedOutput, "\x1b[?2026h");
define_simple_command!(EndBatchedOutput, "\x1b[?2026l");

define_simple_command!(BeginBracketedPaste, "\x1b[?2004h");
define_simple_command!(EndBracketedPaste, "\x1b[?2004l");

/// The 3-ary `Link` command.
#[derive(Clone, Debug)]
pub struct Link(String);

impl Link {
    /// Create a new hyperlink with the given text, URL, and optional ID.
    pub fn new(text: impl AsRef<str>, href: impl AsRef<str>, id: Option<&str>) -> Self {
        let mut s = String::new();
        match id {
            Some(id) => {
                s.push_str("\x1b]8;id=");
                s.push_str(id);
                s.push(';');
            }
            None => s.push_str("\x1b]8;;"),
        }
        s.push_str(href.as_ref());
        s.push_str("\x1b\\");
        s.push_str(text.as_ref());
        s.push_str("\x1b]8;;\x1b\\");

        Self(s)
    }
}

/// Create a new hyperlink for terminal display.
pub fn link(text: impl AsRef<str>, href: impl AsRef<str>) -> Link {
    Link::new(text, href, None)
}

define_impl!(Link: self; out { out.write_str(&self.0) } );
define_display!(Link);

pub struct Print<D: std::fmt::Display>(D);

impl<D: std::fmt::Display> Command for Print<D> {
    fn write_ansi(&self, out: &mut impl ::std::fmt::Write) -> ::std::fmt::Result {
        write!(out, "{}", self.0)
    }
}

impl<D: std::fmt::Display> std::fmt::Display for Print<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// --------------------------------- Style Management ----------------------------------

define_simple_command!(ResetStyle, "\x1b[m");

define_simple_command!(RequestActiveStyle, "\x1bP$qm\x1b\\");

impl Query for RequestActiveStyle {
    type Response = Vec<u8>;

    fn is_valid(&self, control: Control) -> bool {
        matches!(control, Control::DCS)
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        let s = payload
            .strip_prefix(b"\x1bP1$r")
            .and_then(|s| s.strip_suffix(b"m"))
            .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;

        Ok(s.to_owned())
    }
}

// =====================================================================================

#[cfg(test)]
mod test {
    use super::{BeginBatchedOutput, MoveLeft, MoveTo, Query, RequestTerminalId};
    use crate::term::VtScanner;

    #[test]
    fn test_size_and_display() {
        assert_eq!(std::mem::size_of::<BeginBatchedOutput>(), 0);
        assert_eq!(std::mem::size_of::<MoveLeft>(), 2);
        assert_eq!(std::mem::size_of::<MoveTo>(), 4);

        assert_eq!(format!("{}", BeginBatchedOutput), "\x1b[?2026h");
        assert_eq!(format!("{}", MoveLeft(2)), "\x1b[2C");
        assert_eq!(format!("{}", MoveTo(5, 7)), "\x1b[5;7H")
    }

    #[test]
    fn test_parsing() -> std::io::Result<()> {
        let mut input = b"\x1bP>|Terminal\x1b\\".as_slice();
        let mut scanner = VtScanner::new();

        let response = scanner.scan_bytes(&mut input)?;
        let (name, version) = RequestTerminalId.parse(response)?;

        assert_eq!(&name, b"Terminal");
        assert_eq!(version, None);
        Ok(())
    }
}
