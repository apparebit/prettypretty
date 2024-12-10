//! Controlling the terminal with ANSI escape sequences.
//!
//! This module uses three traits to represent ANSI escape sequences:
//!
//!   * [`Command`] instructs a terminal to do something with an in-band signal,
//!     i.e., an ANSI escape sequence. The trait delegates to the `Display`
//!     trait for the mechanics of generating that control sequence.
//!   * [`Sgr`] is a special command using SGR escape sequences. Since
//!     [`Sgr::write_param`] writes only the parameter, it enables composition
//!     of several SGR commands with the [`sgr`](crate::sgr) macro into a
//!     single, compound ANSI escape sequence.
//!   * [`Query`] is a special command that expects a response from the terminal
//!     in form of another ANSI escape sequence and knows how to parse the
//!     payload of that sequence.
//!
//!
//! # Command Library
//!
//! This modules also implements a library of useful commands. It covers the
//! following features and commands:
//!
//!   * Terminal management:
//!       * [`RequestTerminalId`]
//!   * Window title management:
//!       * [`SaveWindowTitle`] and [`RestoreWindowTitle`]
//!       * [`SetWindowTitle`]
//!   * Screen management:
//!       * [`RequestScreenSize`]
//!       * [`EnterAlternateScreen`] and [`ExitAlternateScreen`]
//!       * [`EraseScreen`] and [`EraseLine`]
//!   * Cursor management:
//!       * [`HideCursor`] and [`ShowCursor`]
//!       * [`RequestCursorPosition`]
//!       * Relative [`MoveUp`], [`MoveDown`], [`MoveLeft`], and [`MoveRight`]
//!       * Absolute [`MoveToColumn`], [`MoveToRow`], and [`MoveTo`]
//!       * [`SaveCursorPosition`] and [`RestoreCursorPosition`]
//!   * Managing content:
//!       * [`RequestBatchMode`]
//!       * [`BeginBatch`] and [`EndBatch`]
//!       * [`BeginPaste`] and [`EndPaste`] to perform a
//!         [bracketed paste](https://cirw.in/blog/bracketed-paste) operation
//!       * [`Link`]
//!   * Styling content:
//!       * [`ResetStyle`]
//!       * [`RequestActiveStyle`]
//!       * [`SetForeground8`], [`SetForeground24`], and the
//!         [`SetForeground`](crate::SetForeground) macro
//!       * [`SetBackground8`], [`SetBackground24`], and the
//!         [`SetBackground`](crate::SetBackground) macro
//!       * [`FormatBold`], [`FormatThin`], and [`FormatRegular`]
//!       * [`FormatItalic`] and [`FormatUpright`]
//!       * [`FormatUnderlined`] and [`FormatNotUnderlined`]
//!       * [`FormatBlinking`] and [`FormatNotBlinking`]
//!       * [`FormatReversed`] and [`FormatNotReversed`]
//!       * [`FormatHidden`] and [`FormatNotHidden`]
//!       * [`FormatStricken`] and [`FormatNotStricken`]
//!
//! If a command starts with `Request` in its name, it implements the [`Query`]
//! trait in addition to `Display`.
//!

#![allow(dead_code)]

use crate::term::{is_semi_colon, Control, Radix, SliceExt};
use std::io::{Error, ErrorKind, Result};

/// A command.
///
/// A command's display is its ANSI escape sequence. If a type implements this
/// trait (instead of the more general `Display` trait), it is expected to abide
/// by that requirement.
pub trait Command: std::fmt::Display {}

impl<C: Command + ?Sized> Command for &C {}

/// A command that uses the select-graphic-rendition ANSI escape sequence.
///
/// To facilitate composition, SGR commands implement [`Sgr::write_param`],
/// which only writes the parameter(s) for the command but not the leading CSI
/// control or trailing `m`. A generic implementation then ensures that every
/// `Sgr` also is a `Command`; its `write_ansi` method composes prefix,
/// parameter(s), and suffix. More importantly, a separate [`sgr`](crate::sgr)
/// macro composes several SGR commands into a single ANSI escape sequence.
pub trait Sgr: Command {
    /// Write the parameter(s) for this SGR command.
    fn write_param(&self, out: &mut impl ::std::fmt::Write) -> ::std::fmt::Result;
}

impl<S: Sgr + ?Sized> Sgr for &S {
    fn write_param(&self, out: &mut impl ::std::fmt::Write) -> ::std::fmt::Result {
        (*self).write_param(out)
    }
}

/// Macro to combine several SGR commands into a single ANSI escape sequence.
#[macro_export]
macro_rules! sgr {
    ( $sgr:expr, $( $sgr2:expr ),* $(,)? ) => {{
        use $crate::cmd::Sgr;

        struct SgrSeq;

        impl std::fmt::Display for SgrSeq {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("\x1b[")?;
                $sgr.write_param(f)?;
                $(
                    f.write_str(";")?;
                    $sgr2.write_param(f)?;
                )*
                f.write_str("m")
            }
        }

        SgrSeq
    }};
}

// -------------------------------------------------------------------------------------

/// A command that receives a response.
///
/// Since UTF-8 is more restricted than byte slices, this trait treats the
/// payload of an ANSI escape sequence as `&[u8]`. Use `str::from_utf8`, if you
/// absolutely need a string slice.
///
/// `Query` does not declare `Command` as a supertrait to maintain a loose
/// coupling between the request and response. In particular, performance
/// experiments with querying for a terminal's current theme show that a
/// two-stage approach, which first writes all 18 requests and then reads all 18
/// responses, is faster than a one-stage or three-stage approach (with the
/// latter separating scanning and parsing into two distinct stages).
pub trait Query: Command {
    /// The type of the response data.
    type Response;

    /// Get the response's control.
    fn control(&self) -> Control;

    /// Parse the payload into a response object.
    fn parse(&self, payload: &[u8]) -> Result<Self::Response>;
}

/// A borrowed query is a query.
impl<Q: Query + ?Sized> Query for &Q {
    type Response = Q::Response;

    fn control(&self) -> Control {
        (*self).control()
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        (*self).parse(payload)
    }
}

// =================================== Local Macros ====================================

macro_rules! declare_struct {
    ($name:ident) => {
        #[doc = stringify!("The 0-ary `", $name, "` command.")]
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $name;
    };
}

macro_rules! define_expr_impl {
    ($name:ident { $repr:expr }) => {
        impl std::fmt::Display for $name {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str($repr)
            }
        }
    };
}

macro_rules! implement_display_sgr {
    ($name:ident { $repr:expr }) => {
        impl crate::cmd::Command for $name {}

        impl crate::cmd::Sgr for $name {
            #[inline]
            fn write_param(&self, out: &mut impl std::fmt::Write) -> std::fmt::Result {
                out.write_str($repr)
            }
        }

        impl std::fmt::Display for $name {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!("\x1b[", $repr, "m"))
            }
        }
    };
}

macro_rules! implement_display_expr {
    ($name:ident { $repr:expr }) => {
        impl crate::cmd::Command for $name {}

        impl std::fmt::Display for $name {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str($repr)
            }
        }
    };
}

macro_rules! implement_display {
    ($name:ident : $selfish:ident ; $output:ident $body:block ) => {
        impl crate::cmd::Command for $name {}

        impl std::fmt::Display for $name {
            #[inline]
            fn fmt(&$selfish, $output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                $body
            }
        }
    }
}

macro_rules! define_sgr {
    ($name:ident, $ansi:tt) => {
        declare_struct!($name);
        implement_display_sgr!($name { $ansi });
    };
}

macro_rules! define_command0 {
    ($name:ident, $ansi:tt) => {
        declare_struct!($name);
        implement_display_expr!($name { $ansi });
    };
}

macro_rules! define_command1 {
    ($name:ident : $typ:ty, $prefix:literal, $suffix:literal) => {
        #[doc = stringify!("The 1-ary `", $name, "(‹n›)` command")]
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $name(pub $typ);

        implement_display!($name: self; f {
            f.write_str($prefix)?;
            write!(f, "{}", self.0)?;
            f.write_str($suffix)
        });
    };
}

macro_rules! define_command3 {
    ($name:ident : $typ:ty, $prefix:literal, $suffix:literal) => {
        #[doc = stringify!("The 3-ary `", $name, "(‹r›, ‹g›, ‹b›)` command")]
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $name(pub $typ, pub $typ, pub $typ);

        implement_display!($name: self; f {
            f.write_str($prefix)?;
            write!(f, "{}", self.0)?;
            f.write_str(";")?;
            write!(f, "{}", self.1)?;
            f.write_str(";")?;
            write!(f, "{}", self.2)?;
            f.write_str($suffix)
        });
    };
}

// ====================================== Library ======================================

// -------------------------------- Terminal Management --------------------------------

declare_struct!(RequestTerminalId);
implement_display_expr!(RequestTerminalId { "\x1b[>q" });

impl Query for RequestTerminalId {
    type Response = (Option<Vec<u8>>, Option<Vec<u8>>);

    fn control(&self) -> Control {
        Control::DCS
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

define_command0!(SaveWindowTitle, "\x1b[22;2t");
define_command0!(RestoreWindowTitle, "\x1b[23;2t");

define_command1!(SetWindowTitle: String, "\x1b]2;", "\x1b\\");

// --------------------------------- Screen Management ---------------------------------

define_command0!(EnterAlternateScreen, "\x1b[?1049h");
define_command0!(ExitAlternateScreen, "\x1b[?1049l");

define_command0!(EraseScreen, "\x1b[2J");
define_command0!(EraseLine, "\x1b[2K");

/// The 0-ary `RequestScreenSize` command.
///
/// This command moves the cursor to the lower right corner of the screen. To
/// preserve cursor position, execute [`SaveCursorPosition`] before this command
/// and [`RestoreCursorPosition`] after parsing the response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestScreenSize;
impl Command for RequestScreenSize {}

impl std::fmt::Display for RequestScreenSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        MoveTo(999, 999).fmt(f)?;
        RequestCursorPosition.fmt(f)
    }
}

impl Query for RequestScreenSize {
    type Response = <RequestCursorPosition as Query>::Response;

    fn control(&self) -> Control {
        RequestCursorPosition.control()
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        RequestCursorPosition.parse(payload)
    }
}

// --------------------------------- Cursor Management ---------------------------------

define_command0!(HideCursor, "\x1b[?25l");
define_command0!(ShowCursor, "\x1b[?25h");

define_command1!(MoveUp: u16, "\x1b[", "A");
define_command1!(MoveDown: u16, "\x1b[", "B");
define_command1!(MoveLeft: u16, "\x1b[", "C");
define_command1!(MoveRight: u16, "\x1b[", "D");

/// The 2-ary `MoveTo(‹row›, ‹column›)` command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveTo(pub u16, pub u16);

implement_display!(MoveTo: self; out {
    out.write_str("\x1b[")?;
    out.write_fmt(format_args!("{}", self.0))?;
    out.write_str(";")?;
    out.write_fmt(format_args!("{}", self.1))?;
    out.write_str("H")
});

define_command1!(MoveToColumn: u16, "\x1b[", "G");
define_command1!(MoveToRow: u16, "\x1b[", "d");

/// The 0-ary command to request the cursor position in row-column order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestCursorPosition;

implement_display_expr!(RequestCursorPosition { "\x1b[6n" });

impl Query for RequestCursorPosition {
    /// The row and column of the cursor in that order.
    type Response = (u16, u16);

    fn control(&self) -> Control {
        Control::CSI
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

define_command0!(SaveCursorPosition, "\x1b7");
define_command0!(RestoreCursorPosition, "\x1b8");

// -------------------------------- Content Management ---------------------------------

/// The current batch processing mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchMode {
    NotSupported = 0,
    Enabled = 1,
    Disabled = 2,
    Undefined = 3,
    PermanentlyDisabled = 4,
}

declare_struct!(RequestBatchMode);
implement_display_expr!(RequestBatchMode { "\x1b[?2026$p" });

impl Query for RequestBatchMode {
    type Response = BatchMode;

    fn control(&self) -> Control {
        Control::CSI
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

define_command0!(BeginBatch, "\x1b[?2026h");
define_command0!(EndBatch, "\x1b[?2026l");

define_command0!(BeginPaste, "\x1b[?2004h");
define_command0!(EndPaste, "\x1b[?2004l");

/// The 3-ary `Link(‹id›, ‹href›, ‹text›)` command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link(String);

impl Link {
    /// Create a new hyperlink with the given text, URL, and optional ID.
    pub fn new<'a, ID, HREF, TEXT>(id: ID, href: HREF, text: TEXT) -> Self
    where
        ID: Into<Option<&'a str>>,
        HREF: AsRef<str>,
        TEXT: AsRef<str>,
    {
        let mut s = String::new();
        let id = id.into();
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
pub fn link(href: impl AsRef<str>, text: impl AsRef<str>) -> Link {
    Link::new(None, href, text)
}

implement_display!(Link: self; out { out.write_str(&self.0) } );

// --------------------------------- Style Management ----------------------------------

define_command0!(ResetStyle, "\x1b[m");

define_command1!(SetForeground8: u8, "\x1b[38;5;", "m");
define_command1!(SetBackground8: u8, "\x1b[48;5;", "m");
define_command3!(SetForeground24: u8, "\x1b[38;2;", "m");
define_command3!(SetBackground24: u8, "\x1b[48;2;", "m");

/// 1-/3-ary helper macro to `SetForeground!(‹n› / ‹r›, ‹g›, ‹b›)`
#[macro_export]
macro_rules! SetForeground {
    ($r:expr, $g:expr, $b:expr) => {
        SetForeground24($r, $g, $b)
    };
    ($n:expr) => {
        SetForeground8($n)
    };
}

/// 1-/3-ary helper macro to `SetBackground!(‹n› / ‹r›, ‹g›, ‹b›)`
#[macro_export]
macro_rules! SetBackground {
    ($r:expr, $g:expr, $b:expr) => {
        SetBackground24($r, $g, $b)
    };
    ($n:expr) => {
        SetBackground8($n)
    };
}

define_sgr!(FormatBold, "1");
define_sgr!(FormatThin, "2");
define_sgr!(FormatRegular, "22");
define_sgr!(FormatItalic, "3");
define_sgr!(FormatUpright, "23");
define_sgr!(FormatUnderlined, "4");
define_sgr!(FormatBlinking, "5");
define_sgr!(FormatReversed, "7");
define_sgr!(FormatHidden, "8");
define_sgr!(FormatStricken, "9");
define_sgr!(FormatNotUnderlined, "24");
define_sgr!(FormatNotBlinking, "25");
define_sgr!(FormatNotReversed, "27");
define_sgr!(FormatNotHidden, "28");
define_sgr!(FormatNotStricken, "29");

declare_struct!(RequestActiveStyle);
implement_display_expr!(RequestActiveStyle { "\x1bP$qm\x1b\\" });

impl Query for RequestActiveStyle {
    type Response = Vec<u8>;

    fn control(&self) -> Control {
        Control::DCS
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
        BeginBatch, FormatBold, FormatUnderlined, MoveLeft, MoveTo, Query, RequestCursorPosition,
        RequestTerminalId,
    };
    use std::io::Write;

    #[test]
    fn test_size_and_display() {
        assert_eq!(std::mem::size_of::<BeginBatch>(), 0);
        assert_eq!(std::mem::size_of::<MoveLeft>(), 2);
        assert_eq!(std::mem::size_of::<MoveTo>(), 4);

        assert_eq!(format!("{}", BeginBatch), "\x1b[?2026h");
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

    #[test]
    fn test_sgr() -> std::io::Result<()> {
        let bold_underline = sgr!(FormatBold, FormatUnderlined);
        let mut sink = Vec::new();
        write!(sink, "{}", bold_underline)?;

        assert_eq!(String::from_utf8(sink).unwrap(), "\x1b[1;4m");
        Ok(())
    }
}
