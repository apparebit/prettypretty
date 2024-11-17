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

use crate::term::{is_semi_colon, Control, Radix, SliceExt, VtScanner};

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
/// Since UTF-8 is more restricted than byte slices, this trait treats the
/// payload of an ANSI escape sequence as `&[u8]`. Use `str::from_utf8`, if you
/// absolutely need a string slice.
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

/// Write a command to a stream.
///
/// This trait is implemented for all `std::io::Write` streams.
pub trait WriteCommand {
    fn write_cmd<C: Command>(&mut self, command: C) -> std::io::Result<()>;
}

struct Adapter<T> {
    inner: T,
    result: std::io::Result<()>,
}

impl<W: std::io::Write> std::fmt::Write for Adapter<W> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.inner
            .write_all(s.as_bytes())
            .map_err(|_| std::fmt::Error)
    }
}

impl<W: std::io::Write> WriteCommand for W {
    fn write_cmd<C: Command>(&mut self, command: C) -> std::io::Result<()> {
        let mut adapter = Adapter {
            inner: self,
            result: Ok(()),
        };

        command.write_ansi(&mut adapter).map_err(|_| {
            if adapter.result.is_ok() {
                panic!(
                    "<{}>::write_ansi() unexpectedly returned error",
                    std::any::type_name::<C>()
                );
            }
            adapter.result.err().unwrap()
        })
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
    };
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
    type Response = (Option<Vec<u8>>, Option<Vec<u8>>);

    fn is_valid(&self, control: Control) -> bool {
        matches!(control, Control::DCS)
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        let s = payload
            .strip_prefix(b">|")
            .and_then(|s| s.strip_bel_st_suffix())
            .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;

        if let Some(s) = s.strip_suffix(b")") {
            let (n, v) = s
                .iter()
                .position(|byte| *byte == b'(')
                .map(|index| s.split_at(index))
                .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
            let n = n.trim();
            let v = v[1..].trim();
            Ok((n.to_some_owned_bytes(), v.to_some_owned_bytes()))
        } else {
            Ok((s.to_some_owned_bytes(), None))
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

/// The 0-ary command to request the cursor position in row-column order.
pub struct RequestCursorPosition;
define_expr_impl!(RequestCursorPosition { "\x1b[6n" });
define_display!(RequestCursorPosition);

impl Query for RequestCursorPosition {
    /// The row and column of the cursor in that order.
    type Response = (u16, u16);

    fn is_valid(&self, control: Control) -> bool {
        matches!(control, Control::CSI)
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        let s = payload
            .strip_prefix(b"\x1b[")
            .and_then(|s| s.strip_suffix(b"R"))
            .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;

        let mut index = 0;
        let mut params = [0_u16; 2];
        for bytes in s.split(is_semi_colon) {
            if 2 <= index {
                return Err(ErrorKind::InvalidData.into());
            }
            params[index] = bytes
                .to_u16(Radix::Decimal)
                .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
            index += 1;
        }

        if index < 2 {
            return Err(ErrorKind::InvalidData.into());
        }

        Ok((params[0], params[1]))
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
        let bytes = payload
            .strip_prefix(b"\x1b[?2026;")
            .and_then(|s| s.strip_suffix(b"$y"))
            .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
        let response = bytes
            .to_u32(Radix::Decimal)
            .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;

        Ok(match response {
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
    use super::{
        BeginBatchedOutput, MoveLeft, MoveTo, Query, RequestCursorPosition, RequestTerminalId,
    };

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
    fn test_parse_terminal_id() -> std::io::Result<()> {
        let (term, version) = RequestTerminalId.parse(b">|Terminal\x1b\\")?;
        assert_eq!(&term.unwrap(), b"Terminal".as_slice());
        assert!(version.is_none());

        let (term, version) = RequestTerminalId.parse(b">|Terminal (6.65)\x1b\\")?;
        assert_eq!(&term.unwrap(), b"Terminal".as_slice());
        assert_eq!(&version.unwrap(), b"6.65".as_slice());

        let (term, version) = RequestTerminalId.parse(b">|Terminal ()\x1b\\")?;
        assert_eq!(&term.unwrap(), b"Terminal".as_slice());
        assert_eq!(version, None);

        let (term, version) = RequestTerminalId.parse(b">|   (    )\x1b\\")?;
        assert_eq!(term, None);
        assert_eq!(version, None);

        let (term, version) = RequestTerminalId.parse(b">|()\x1b\\")?;
        assert_eq!(term, None);
        assert_eq!(version, None);

        let (term, version) = RequestTerminalId.parse(b">|\x1b\\")?;
        assert_eq!(term, None);
        assert_eq!(version, None);
        Ok(())
    }

    #[test]
    fn test_parse_cursor_position() -> std::io::Result<()> {
        let position = RequestCursorPosition.parse(b"\x1b[6;65R")?;
        assert_eq!(position, (6, 65));
        Ok(())
    }
}
