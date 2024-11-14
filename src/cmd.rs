//! Controlling the terminal with ANSI escape sequences.
//!
//! [`Command`] is an instruction to change a terminal's screen, cursor,
//! content, or some such. Its implementation writes an ANSI escape sequence to
//! the terminal, even on Windows, which added support in Windows 10 TH2
//! (v1511). [`Query`] encapsulates the functionality for reading and parsing
//! the response returned by a terminal after receiving the corresponding
//! request. An implementation checks with [`Query::is_valid`] whether the
//! response had the right control and then uses [`Query::parse`] to parse the
//! payload of the terminal's response. [`Query::query`] builds on these two
//! required methods to coordinate between
//! [`TerminalAccess`](crate::term::TerminalAccess), [`VtScanner`], and
//! [`Query`].
//!
//! This modules further defines the core of a command library. Each command
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
//!     [`MoveUp`], [`MoveDown`], [`MoveLeft`], and [`MoveRight`],
//!     the absolute [`MoveToColumn`], [`MoveToRow`], and [`MoveTo`],
//!     also [`RequestCursorPosition`].
//!   * For grouping content, [`RequestBatchMode`], [`BeginBatchedOutput`] and
//!     [`EndBatchedOutput`], [`BeginBracketedPaste`] and [`EndBracketedPaste`],
//!     also [`Link`].
//!
//! If a command starts with `Request` in its name, it is a query and implements
//! the [`Query`] trait in addition to [`Command`].
//!

#![allow(dead_code)]
use std::io::{BufRead, Error, ErrorKind, Result};

use crate::term::{Control, VtScanner};

/// A terminal command.
///
/// Every command has its own ANSI escape sequence. Simple commands have no
/// parameters and hence always produce the same escape sequence. They are
/// implemented as zero-sized types. In contrast, parameterized commands require
/// storage space.
pub trait Command {
    /// Write out the command's ANSI escape sequence.
    fn write_ansi(&self, f: &mut impl ::std::fmt::Write) -> ::std::fmt::Result;
}

/// Item to make references to commands also function as commands.
impl<C: Command> Command for &C {
    fn write_ansi(&self, f: &mut impl ::std::fmt::Write) -> ::std::fmt::Result {
        (*self).write_ansi(f)
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

macro_rules! define_simple_command {
    ($name:ident $(: $selfish:ident)? { $repr:expr }) => {
        impl crate::cmd::Command for $name {
            #[inline]
            fn write_ansi(&self, f: &mut impl ::std::fmt::Write) -> ::std::fmt::Result {
                // If requested, make self available under external name
                $(let $selfish = self;)?
                f.write_str($repr)
            }
        }
    }
}

macro_rules! define_command {
    ($name:ident, $selfish:ident, $format:ident $body:block ) => {
        impl crate::cmd::Command for $name {
            #[inline]
            fn write_ansi(& $selfish, $format: &mut impl ::std::fmt::Write) -> ::std::fmt::Result {
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

macro_rules! define_simple_suite {
    ($name:ident, $ansi:tt) => {
        #[doc = "The 0-ary `"]
        #[doc = stringify!($name)]
        #[doc = "` command."]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name;

        define_simple_command!($name { $ansi });
        define_display!($name);
    };
}

macro_rules! define_one_num_suite {
    ($name:ident, $prefix:literal, $suffix:literal) => {
        #[doc = "The 1-ary `"]
        #[doc = stringify!($name)]
        #[doc = "` command."]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name(u16);

        define_command!($name, self, f {
            f.write_str($prefix)?;
            f.write_fmt(format_args!("{}", self.0))?;
            f.write_str($suffix)
        });
        define_display!($name);
    };
}

// -------------------------------- Terminal Management --------------------------------

define_simple_suite!(RequestTerminalId, "\x1b[>q");

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

define_simple_suite!(SaveWindowTitleOnStack, "\x1b[22;2t");
define_simple_suite!(LoadWindowTitleFromStack, "\x1b[23;2t");

/// The 1-ary `SetWindowTitle` command.
pub struct SetWindowTitle(String);
define_command!(SetWindowTitle, self, f {
    f.write_str("\x1b]2;")?;
    f.write_str(&self.0)?;
    f.write_str("\x1b\\")
});
define_display!(SetWindowTitle);

// --------------------------------- Screen Management ---------------------------------

define_simple_suite!(EnterAlternateScreen, "\x1b[?1049h");
define_simple_suite!(ExitAlternateScreen, "\x1b[?1049l");

define_simple_suite!(EraseScreen, "\x1b[2J");
define_simple_suite!(EraseLine, "\x1b[2K");

// --------------------------------- Cursor Management ---------------------------------

define_simple_suite!(HideCursor, "\x1b[?25l");
define_simple_suite!(ShowCursor, "\x1b[?25h");

define_one_num_suite!(MoveUp, "\x1b[", "A");
define_one_num_suite!(MoveDown, "\x1b[", "B");
define_one_num_suite!(MoveLeft, "\x1b[", "C");
define_one_num_suite!(MoveRight, "\x1b[", "D");

/// The 2-ary `MoveTo` *row, column* command.
#[derive(Clone, Copy, Debug)]
pub struct MoveTo(u16, u16);

define_command!(MoveTo, self, f {
    f.write_str("\x1b[")?;
    f.write_fmt(format_args!("{}", self.0))?;
    f.write_str(";")?;
    f.write_fmt(format_args!("{}", self.1))?;
    f.write_str("H")
});
define_display!(MoveTo);

define_one_num_suite!(MoveToColumn, "\x1b[", "G");
define_one_num_suite!(MoveToRow, "\x1b[", "d");

define_simple_suite!(RequestCursorPosition, "\x1b[6n");

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
            || params[0].is_none()
            || (u16::MAX as u64) < params[0].unwrap()
            || (u16::MAX as u64) < params[1].unwrap()
        {
            return Err(ErrorKind::InvalidData.into());
        }
        Ok((params[0].unwrap() as u16, params[1].unwrap() as u16))
    }
}

// -------------------------------- Content Management ---------------------------------

define_simple_suite!(RequestBatchMode, "\x1b[?2026$p");

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

define_simple_suite!(BeginBatchedOutput, "\x1b[?2026h");
define_simple_suite!(EndBatchedOutput, "\x1b[?2026l");

define_simple_suite!(BeginBracketedPaste, "\x1b[?2004h");
define_simple_suite!(EndBracketedPaste, "\x1b[?2004l");

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
pub fn link(text: impl AsRef<str>, href: impl AsRef<str>, id: Option<&str>) -> Link {
    Link::new(text, href, id)
}

define_command!(Link, self, f { f.write_str(&self.0) } );
define_display!(Link);

// --------------------------------- Style Management ----------------------------------

define_simple_suite!(ResetStyle, "\x1b[m");
define_simple_suite!(RequestActiveStyle, "\x1bP$qm\x1b\\");

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
