//! A library of useful terminal commands.
//!
//! This module provides a number of trivial struct and enum types that
//! implement the [`Command`] [`Sgr`], and [`Query`] traits (as needed) to
//! provide common terminal interactions. Organized by topic, supported commands
//! are:
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
//!       * [`SetForegroundDefault`], [`SetForeground8`], and [`SetForeground24`]
//!       * [`SetBackgroundDefault`], [`SetBackground8`], and [`SetBackground24`]
//!       * [`Format::Bold`], [`Format::Thin`], and [`Format::Regular`]
//!       * [`Format::Italic`] and [`Format::Upright`]
//!       * [`Format::Underlined`] and [`Format::NotUnderlined`]
//!       * [`Format::Blinking`] and [`Format::NotBlinking`]
//!       * [`Format::Reversed`] and [`Format::NotReversed`]
//!       * [`Format::Hidden`] and [`Format::NotHidden`]
//!       * [`Format::NotStricken`] and [`Format::NotStricken`]
//!       * [`RequestColor::Black`], [`RequestColor::Red`], and so on for all 16
//!         ANSI colors, also [`RequestColor::Foreground`],
//!         [`RequestColor::Background`], [`RequestColor::Cursor`], and
//!         [`RequestColor::Selection`]
//!
//! If a command starts with `Request` in its name, it implements the [`Query`]
//! trait and hence knows how to parse the response's payload.
//!
//!
//! # Example
//!
//! Executing a command is as simple as writing its display:
//! ```
//! # use prettytty::{sgr, Sgr, cmd::{Format, ResetStyle, SetForeground8}};
//! println!(
//!     "{}Wow!{}",
//!     sgr!(Format::Bold, Format::Underlined, SetForeground8(124)),
//!     ResetStyle
//! );
//! ```
//! The macro [`sgr!`](crate::sgr) is not strictly necessary but conveniently
//! emits a single ANSI escape sequence for the first three stylistic commands.
//! The terminal says <img style="display: inline-block; vertical-align: text-top"
//! src="https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/wow.png"
//!      alt="wow!" width="42">. Wow indeed.

use crate::util::{is_semi_colon, Radix};
use crate::{Command, Control, Query, Sgr};
use std::io::{Error, ErrorKind, Result};
use std::iter::successors;

macro_rules! declare_struct_0 {
    ($name:ident) => {
        #[doc = concat!("The `",stringify!($name),"` command.")]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name;
    };
}

macro_rules! declare_struct_n {
    ($name:ident( $( $typ:ty ),+ $(,)? )) => {
        #[doc = concat!("The `",stringify!($name),"(",stringify!($($typ),+),")` command.")]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name( $( pub $typ ),+ );
    }
}

macro_rules! implement_sgr_expr {
    ($name:ident { $repr:expr }) => {
        impl crate::cmd::Command for $name {}

        impl crate::cmd::Sgr for $name {
            #[inline]
            fn write_param(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

macro_rules! implement_sgr {
    ($name:ident : $selfish:ident ; $output:ident $body:block) => {
        impl crate::cmd::Command for $name {}

        impl crate::cmd::Sgr for $name {
            #[inline]
            fn write_param(&$selfish, $output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                $body
            }
        }

        impl std::fmt::Display for $name {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("\x1b[")?;
                self.write_param(f)?;
                f.write_str("m")
            }
        }
    };
}

macro_rules! implement_command_expr {
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

macro_rules! implement_command {
    ($name:ident : $selfish:ident ; $output:ident $body:block) => {
        impl crate::cmd::Command for $name {}

        impl std::fmt::Display for $name {
            #[inline]
            fn fmt(&$selfish, $output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                $body
            }
        }
    }
}

macro_rules! define_sgr_0 {
    ($name:ident, $ansi:tt) => {
        declare_struct_0!($name);
        implement_sgr_expr!($name { $ansi });
    };
}

macro_rules! define_command_0 {
    ($name:ident, $ansi:tt) => {
        declare_struct_0!($name);
        implement_command_expr!($name { $ansi });
    };
}

macro_rules! define_sgr_1 {
    ($name:ident : $typ:ty, $prefix:literal) => {
        declare_struct_n!($name($typ));
        implement_sgr!($name: self; f {
            f.write_str($prefix)?;
            write!(f, "{}", self.0)
        });
    }
}

macro_rules! define_sgr_3 {
    ($name:ident : $typ:ty, $prefix:literal) => {
        declare_struct_n!($name($typ, $typ, $typ));
        implement_sgr!($name: self; f {
            f.write_str($prefix)?;
            write!(f, "{}", self.0)?;
            f.write_str(";")?;
            write!(f, "{}", self.1)?;
            f.write_str(";")?;
            write!(f, "{}", self.2)
        });
    }
}

macro_rules! define_command_1 {
    ($name:ident : $typ:ty, $prefix:literal, $suffix:literal) => {
        declare_struct_n!($name($typ));
        implement_command!($name: self; f {
            f.write_str($prefix)?;
            write!(f, "{}", self.0)?;
            f.write_str($suffix)
        });
    };
}

// ====================================== Library ======================================

// -------------------------------- Terminal Management --------------------------------

declare_struct_0!(RequestTerminalId);
implement_command_expr!(RequestTerminalId { "\x1b[>q" });

impl Query for RequestTerminalId {
    type Response = (Option<Vec<u8>>, Option<Vec<u8>>);

    fn control(&self) -> Control {
        Control::DCS
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        fn prepare(value: &[u8]) -> Option<Vec<u8>> {
            if value.is_empty() {
                None
            } else {
                Some(value.to_owned())
            }
        }

        let s = payload
            .strip_prefix(b">|")
            .and_then(|s| {
                s.strip_suffix(b"\0x7")
                    .or_else(|| s.strip_suffix(b"\x1b\\"))
            })
            .ok_or_else(|| Error::from(ErrorKind::InvalidData))?
            .trim_ascii();

        if let Some(s) = s.strip_suffix(b")") {
            let (n, v) = s
                .iter()
                .position(|byte| *byte == b'(')
                .map(|index| s.split_at(index))
                .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
            let n = n.trim_ascii();
            let v = v[1..].trim_ascii();

            Ok((prepare(n), prepare(v)))
        } else {
            Ok((prepare(s), None))
        }
    }
}

// --------------------------------- Window Management ---------------------------------

define_command_0!(SaveWindowTitle, "\x1b[22;2t");
define_command_0!(RestoreWindowTitle, "\x1b[23;2t");

/// The `SetWindowTitle(String)` command.
///
/// This is one of few commands that cannot be copied, only cloned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetWindowTitle(String);
implement_command!(SetWindowTitle: self; f {
    f.write_str("\x1b]2;")?;
    write!(f, "{}", self.0)?;
    f.write_str("\x1b\\")
});

// --------------------------------- Screen Management ---------------------------------

define_command_0!(EnterAlternateScreen, "\x1b[?1049h");
define_command_0!(ExitAlternateScreen, "\x1b[?1049l");

define_command_0!(EraseScreen, "\x1b[2J");
define_command_0!(EraseLine, "\x1b[2K");

declare_struct_0!(RequestScreenSize);
impl Command for RequestScreenSize {}

impl std::fmt::Display for RequestScreenSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        MoveTo(u16::MAX, u16::MAX).fmt(f)?;
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

define_command_0!(HideCursor, "\x1b[?25l");
define_command_0!(ShowCursor, "\x1b[?25h");

define_command_1!(MoveUp: u16, "\x1b[", "A");
define_command_1!(MoveDown: u16, "\x1b[", "B");
define_command_1!(MoveLeft: u16, "\x1b[", "C");
define_command_1!(MoveRight: u16, "\x1b[", "D");

/// The `MoveTo(‹row›, ‹column›)` command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveTo(pub u16, pub u16);

implement_command!(MoveTo: self; out {
    out.write_str("\x1b[")?;
    out.write_fmt(format_args!("{}", self.0))?;
    out.write_str(";")?;
    out.write_fmt(format_args!("{}", self.1))?;
    out.write_str("H")
});

define_command_1!(MoveToColumn: u16, "\x1b[", "G");
define_command_1!(MoveToRow: u16, "\x1b[", "d");

define_command_0!(SaveCursorPosition, "\x1b7");
define_command_0!(RestoreCursorPosition, "\x1b8");

declare_struct_0!(RequestCursorPosition);
implement_command_expr!(RequestCursorPosition { "\x1b[6n" });

impl Query for RequestCursorPosition {
    /// The row and column of the cursor in that order.
    type Response = (u16, u16);

    fn control(&self) -> Control {
        Control::CSI
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        let s = payload
            .strip_suffix(b"R")
            .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;

        let mut index = 0;
        let mut params = [0_u16; 2];
        for bytes in s.split(is_semi_colon) {
            if 2 <= index {
                return Err(ErrorKind::InvalidData.into());
            }
            params[index] = Radix::Decimal
                .parse_u16(bytes)
                .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
            index += 1;
        }

        if index < 2 {
            return Err(ErrorKind::InvalidData.into());
        }

        Ok((params[0], params[1]))
    }
}

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

declare_struct_0!(RequestBatchMode);
implement_command_expr!(RequestBatchMode { "\x1b[?2026$p" });

impl Query for RequestBatchMode {
    type Response = BatchMode;

    fn control(&self) -> Control {
        Control::CSI
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        let bytes = payload
            .strip_prefix(b"?2026;")
            .and_then(|s| s.strip_suffix(b"$y"))
            .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
        let response = Radix::Decimal
            .parse_u32(bytes)
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

define_command_0!(BeginBatch, "\x1b[?2026h");
define_command_0!(EndBatch, "\x1b[?2026l");

define_command_0!(BeginPaste, "\x1b[?2004h");
define_command_0!(EndPaste, "\x1b[?2004l");

/// The `Link(‹id›, ‹href›, ‹text›)` command.
///
/// This is one of few commands that cannot be copied, only cloned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link(Option<String>, String, String);

impl Link {
    /// Create a new hyperlink with the given text, URL, and optional ID.
    pub fn new<I, H, T>(id: Option<I>, href: H, text: T) -> Self
    where
        I: Into<String>,
        H: Into<String>,
        T: Into<String>,
    {
        Self(id.map(|s| s.into()), href.into(), text.into())
    }
}

implement_command!(Link: self; out {
    if let Some(ref id) = self.0 {
        out.write_str("\x1b]8;id=")?;
        out.write_str(id)?;
        out.write_str(";")?;
    } else {
        out.write_str("\x1b]8;;")?;
    }

    out.write_str(self.1.as_ref())?;
    out.write_str("\x1b\\")?;
    out.write_str(self.2.as_ref())?;
    out.write_str("\x1b]8;;\x1b\\")
});

// --------------------------------- Style Management ----------------------------------

define_command_0!(ResetStyle, "\x1b[m");

define_sgr_0!(SetForegroundDefault, "39");
define_sgr_0!(SetBackgroundDefault, "49");
define_sgr_1!(SetForeground8: u8, "38;5;");
define_sgr_1!(SetBackground8: u8, "48;5;");
define_sgr_3!(SetForeground24: u8, "38;2;");
define_sgr_3!(SetBackground24: u8, "48;2;");

/// The enumeration of `Format` commands.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum Format {
    Bold = 1,
    Thin = 2,
    Regular = 22,
    Italic = 3,
    Upright = 23,
    Underlined = 4,
    NotUnderlined = 24,
    Blinking = 5,
    NotBlinking = 25,
    Reversed = 7,
    NotReversed = 27,
    Hidden = 8,
    NotHidden = 28,
    Stricken = 9,
    NotStricken = 29,
}

impl Format {
    /// Get the format that restores default appearance.
    pub fn undo(&self) -> Self {
        use self::Format::*;

        match self {
            Bold | Thin => Regular,
            Italic => Upright,
            Underlined => NotUnderlined,
            Blinking => NotBlinking,
            Reversed => NotReversed,
            Hidden => NotHidden,
            Stricken => NotStricken,
            _ => *self,
        }
    }
}

impl Sgr for Format {
    fn write_param(&self, out: &mut std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(out, "{}", *self as u8)
    }
}

impl Command for Format {}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\x1b[")?;
        self.write_param(f)?;
        f.write_str("m")
    }
}

declare_struct_0!(RequestActiveStyle);
implement_command_expr!(RequestActiveStyle { "\x1bP$qm\x1b\\" });

impl Query for RequestActiveStyle {
    type Response = Vec<u8>;

    fn control(&self) -> Control {
        Control::DCS
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        let s = payload
            .strip_prefix(b"1$r")
            .and_then(|s| s.strip_suffix(b"m"))
            .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;

        Ok(s.to_owned())
    }
}

/// The enumeration of `RequestColor` commands.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum RequestColor {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
    BrightBlack = 8,
    BrightRed = 9,
    BrightGreen = 10,
    BrightYellow = 11,
    BrightBlue = 12,
    BrightMagenta = 13,
    BrightCyan = 14,
    BrightWhite = 15,
    Foreground = 110,
    Background = 111,
    Cursor = 112,
    Selection = 117,
}

impl RequestColor {
    /// The number of possible color requests.
    pub const COUNT: usize = 20;

    /// Get the successor.
    fn successor(&self) -> Option<RequestColor> {
        use self::RequestColor::*;

        Some(match self {
            Black => Red,
            Red => Green,
            Green => Yellow,
            Yellow => Blue,
            Blue => Magenta,
            Magenta => Cyan,
            Cyan => White,
            White => BrightBlack,
            BrightBlack => BrightRed,
            BrightRed => BrightGreen,
            BrightGreen => BrightYellow,
            BrightYellow => BrightBlue,
            BrightBlue => BrightMagenta,
            BrightMagenta => BrightCyan,
            BrightCyan => BrightWhite,
            BrightWhite => Foreground,
            Foreground => Background,
            Background => Cursor,
            Cursor => Selection,
            Selection => return None,
        })
    }

    /// Get an iterator over all color requests.
    pub fn all() -> impl Iterator<Item = RequestColor> {
        successors(Some(Self::Black), |c| c.successor())
    }
}

impl Command for RequestColor {}

impl std::fmt::Display for RequestColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = *self as u32;
        if code < 16 {
            write!(f, "\x1b]4;{};?\x1b\\", code)
        } else {
            write!(f, "\x1b]{};?\x1b\\", code - 100)
        }
    }
}

impl Query for RequestColor {
    /// An RGB color.
    ///
    /// The parsed response comprises one pair per RGB channel, with the first
    /// number the signal strength and the second number the signal width. The
    /// signal width is the number of hexadecimal digits, always between 1 and 4
    /// inclusive, and usually 4.
    type Response = [(u16, u16); 3];

    fn control(&self) -> Control {
        Control::OSC
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        use crate::err::ErrorKind;

        let code = *self as u8;
        let bytes = if code < 20 {
            let bytes = payload
                .strip_prefix(b"4;")
                .ok_or(Error::from(ErrorKind::BadSequence))?;
            if code < 10 {
                bytes.strip_prefix(&[b'0' + code])
            } else {
                bytes.strip_prefix(&[b'1', b'0' + code - 10])
            }
        } else {
            payload.strip_prefix(match self {
                Self::Foreground => b"10",
                Self::Background => b"11",
                Self::Cursor => b"12",
                Self::Selection => b"17",
                _ => panic!("unknown theme color"),
            })
        }
        .and_then(|bytes| bytes.strip_prefix(b";rgb:"))
        .ok_or(Error::from(ErrorKind::BadSequence))?;

        fn parse(bytes: Option<&[u8]>) -> std::result::Result<(u16, u16), Error> {
            let bytes = bytes.ok_or(Error::from(ErrorKind::TooFewCoordinates))?;
            if bytes.is_empty() {
                return Err(ErrorKind::EmptyCoordinate.into());
            } else if 4 < bytes.len() {
                return Err(ErrorKind::OversizedCoordinate.into());
            }

            let n = Radix::Hexadecimal
                .parse_u16(bytes)
                .ok_or(Error::from(ErrorKind::MalformedCoordinate))?;
            Ok((n, bytes.len() as u16))
        }

        let mut iter = bytes.split(|b| *b == b'/');
        let r = parse(iter.next())?;
        let g = parse(iter.next())?;
        let b = parse(iter.next())?;
        if iter.next().is_some() {
            return Err(ErrorKind::TooManyCoordinates.into());
        }

        Ok([r, g, b])
    }
}

// =====================================================================================

#[cfg(test)]
mod test {
    use super::{
        BeginBatch, MoveLeft, MoveTo, Query, RequestColor, RequestCursorPosition, RequestTerminalId,
    };

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
        let position = RequestCursorPosition.parse(b"6;65R")?;
        assert_eq!(position, (6, 65));
        Ok(())
    }

    #[test]
    fn test_parse_theme_color() -> std::io::Result<()> {
        let color = RequestColor::Background.parse(b"11;rgb:a/b/cdef")?;
        assert_eq!(color, [(10, 1), (11, 1), (52_719, 4)]);
        let color = RequestColor::Magenta.parse(b"4;5;rgb:12/345/6789")?;
        assert_eq!(color, [(18, 2), (837, 3), (26_505, 4)]);
        let color = RequestColor::BrightMagenta.parse(b"4;13;rgb:ff/00/ff")?;
        assert_eq!(color, [(255, 2), (0, 2), (255, 2)]);
        Ok(())
    }
}
