//! A library of useful terminal commands.
//!
//! This module provides a number of straight-forward struct and enum types that
//! implement the [`Command`] trait and, where needed, also the [`Sgr`] and
//! [`Query`] traits. Organized by topic, this library covers the following 87
//! commands:
//!
//!   * Terminal identification:
//!       * [`RequestTerminalId`]
//!   * Window title management:
//!       * [`SaveWindowTitle`] and [`RestoreWindowTitle`]
//!       * [`DynSetWindowTitle`]
//!   * Screen management:
//!       * [`RequestScreenSize`]
//!       * [`EnterAlternateScreen`] and [`ExitAlternateScreen`]
//!       * [`EnableReverseMode`] and [`DisableReverseMode`]
//!       * [`EraseScreen`]
//!   * Scrolling:
//!       * [`ScrollUp`], [`ScrollDown`], [`DynScrollUp`], and [`DynScrollDown`]
//!       * [`SetScrollRegion`] and [`DynSetScrollRegion`]
//!       * [`ResetScrollRegion`]
//!       * [`EnableAutowrap`] and [`DisableAutowrap`]
//!   * Cursor management:
//!       * [`SetCursor::Default`], [`SetCursor::BlinkingBlock`],
//!         [`SetCursor::SteadyBlock`], [`SetCursor::BlinkingUnderscore`],
//!         [`SetCursor::SteadyUnderscore`], [`SetCursor::BlinkingBar`], and
//!         [`SetCursor::SteadyBar`].
//!       * [`HideCursor`] and [`ShowCursor`]
//!       * [`RequestCursorPosition`]
//!       * Relative [`MoveUp`], [`MoveDown`], [`MoveLeft`], [`MoveRight`],
//!         [`DynMoveUp`], [`DynMoveDown`], [`DynMoveLeft`], and
//!         [`DynMoveRight`]
//!       * Absolute [`MoveToColumn`], [`MoveToRow`], [`MoveTo`],
//!         [`DynMoveToColumn`], [`DynMoveToRow`], and [`DynMoveTo`]
//!       * [`SaveCursorPosition`] and [`RestoreCursorPosition`]
//!   * Managing content:
//!       * [`EraseLine`] and [`EraseRestOfLine`]
//!       * [`BeginBatch`] and [`EndBatch`] to [group
//!         updates](https://gist.github.com/christianparpart/d8a62cc1ab659194337d73e399004036)
//!       * [`BeginPaste`] and [`EndPaste`] to perform
//!         [bracketed paste](https://cirw.in/blog/bracketed-paste) operations
//!       * [`DynLink`] to [add
//!         hyperlinks](https://gist.github.com/christianparpart/180fb3c5f008489c8afcffb3fa46cd8e)
//!   * Managing modes:
//!       * [`RequestMode`] and [`DynRequestMode`] to query the status of a mode.
//!   * Styling content:
//!       * [`ResetStyle`]
//!       * [`RequestActiveStyle`]
//!       * [`SetDefaultForeground`], [`SetForeground8`], [`SetForeground24`],
//!         [`DynSetForeground8`], and [`DynSetForeground24`]
//!       * [`SetDefaultBackground`], [`SetBackground8`], [`SetBackground24`],
//!         [`DynSetBackground8`], and [`DynSetBackground24`]
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
//! Most commands are implemented by zero-sized unit structs and enum variants.
//! Commands that require arguments may come in one or both of two flavors, a
//! static flavor relying on const generics and a dynamic flavor storing the
//! arguments. The command name for the latter flavor starts with `Dyn`; it
//! obviously is *not* zero-sized.
//!
//! If a command name starts with `Request`, it also implements the [`Query`]
//! trait and hence knows how to parse the response's payload. When implementing
//! your own queries, you may find [`util::ByteParser`](crate::util::ByteParser)
//! useful.
//!
//! You can easily combine several commands into a compound command with the
//! [`fuse!`](crate::fuse) and [`fuse_sgr!`](crate::fuse_sgr) macros.
//!
//!
//! # Example
//!
//! Executing a command is as simple as writing its display:
//! ```
//! # use prettytty::{fuse_sgr, Sgr, cmd::{Format, ResetStyle, SetForeground8}};
//! println!(
//!     "{}Wow!{}",
//!     fuse_sgr!(Format::Bold, Format::Underlined, SetForeground8::<124>),
//!     ResetStyle
//! );
//! ```
//! The invocation of the [`fuse_sgr!`](crate::fuse_sgr) macro in the above
//! example is not strictly necessary. Separately writing `Format::Bold`,
//! `Format::Underlined`, and `SetForeground8::<124>` to the console would set
//! the same style. However, that would also write three distinct ANSI escape
//! sequences, whereas `fuse_sgr!` returns a value that writes only one ANSI
//! escape sequence. After receiving the above text, the terminal prints <img
//! style="display: inline-block; vertical-align: text-top"
//!      src="https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/wow.png"
//!      alt="wow!" width="42">. Wow indeed 😜

use crate::util::ByteParser;
use crate::{Command, Control, Query, Sgr};
use core::iter::successors;
use std::io::{Error, ErrorKind, Result};

macro_rules! declare_unit_struct {
    ($name:ident) => {
        #[doc = concat!("The unit `",stringify!($name),"` command.")]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name;
    };
}

macro_rules! declare_n_struct {
    ($name:ident( $( $arg:ident : $typ:ty ),+ $(,)? )) => {
        #[doc = concat!("The dynamic `",stringify!($name),"(",stringify!($($arg),+),")` command.")]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name( $( pub $typ ),+ );
    };
    ($name:ident< $( $arg:ident : $typ:ty ),+ >) => {
        #[doc = concat!("The static `",stringify!($name),"<",stringify!($($arg),+),">` command.")]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name< $(const $arg: $typ),+ >;
    }
}

macro_rules! implement_command {
    ($name:ident $(< $( $arg:ident : $typ:ty ),+ >)? : $selfish:ident ; $output:ident $body:block) => {
        impl $(< $(const $arg: $typ),+ >)? $crate::Command for $name $(< $($arg),+ >)? {}

        impl $(< $(const $arg: $typ),+ >)? ::core::fmt::Display for $name $(< $($arg),+ >)? {
            #[inline]
            fn fmt(&$selfish, $output: &mut ::core::fmt::Formatter<'_>) -> core::fmt::Result {
                $body
            }
        }
    }
}

macro_rules! define_unit_command {
    ($name:ident, $ansi:tt) => {
        declare_unit_struct!($name);
        implement_command!($name: self; f { f.write_str($ansi) });
    };
}

macro_rules! define_cmd_1 {
    ($name:ident <$arg:ident : $typ:ty>, $dyn_name:ident, $prefix:literal, $suffix:literal) => {
        declare_n_struct!($name<$arg : $typ>);
        implement_command!($name<$arg : $typ>: self; f {
            f.write_str($prefix)?;
            <_ as ::core::fmt::Display>::fmt(&$arg, f)?;
            f.write_str($suffix)
        });

        declare_n_struct!($dyn_name($arg : $typ));
        implement_command!($dyn_name: self; f {
            f.write_str($prefix)?;
            <_ as ::core::fmt::Display>::fmt(&self.0, f)?;
            f.write_str($suffix)
        });
    }
}

macro_rules! define_cmd_2 {
    (
        $name:ident <$arg1:ident : $typ1:ty, $arg2:ident : $typ2:ty>,
            $dyn_name:ident, $prefix:literal, $suffix:literal
    ) => {
        declare_n_struct!($name<$arg1 : $typ1, $arg2 : $typ2>);
        implement_command!($name<$arg1 : $typ1, $arg2 : $typ2>: self; f {
            f.write_str($prefix)?;
            <_ as ::core::fmt::Display>::fmt(&$arg1, f)?;
            f.write_str(";")?;
            <_ as ::core::fmt::Display>::fmt(&$arg2, f)?;
            f.write_str($suffix)
        });

        declare_n_struct!($dyn_name($arg1 : $typ1, $arg2 : $typ2));
        implement_command!($dyn_name: self; f {
            f.write_str($prefix)?;
            <_ as ::core::fmt::Display>::fmt(&self.0, f)?;
            f.write_str(";")?;
            <_ as ::core::fmt::Display>::fmt(&self.1, f)?;
            f.write_str($suffix)
        });
    }
}

macro_rules! implement_sgr {
    ($name:ident $(< $( $arg:ident : $typ:ty ),+ >)? : $selfish:ident ; $output:ident $body:block) => {
        impl $(< $(const $arg: $typ),+ >)? $crate::Command for $name $(< $($arg),+ >)? {}

        impl $(< $(const $arg: $typ),+ >)? $crate::Sgr for $name $(< $($arg),+ >)? {
            #[inline]
            fn write_param(&$selfish, $output: &mut ::core::fmt::Formatter<'_>) -> core::fmt::Result {
                $body
            }
        }

        impl $(< $(const $arg: $typ),+ >)?  ::core::fmt::Display for $name $(< $($arg),+ >)? {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str("\x1b[")?;
                self.write_param(f)?;
                f.write_str("m")
            }
        }
    };
}

macro_rules! define_unit_sgr {
    ($name:ident, $ansi:tt) => {
        declare_unit_struct!($name);
        implement_sgr!($name: self; f { f.write_str($ansi) });
    };
}

macro_rules! define_8bit_color {
    ($name:ident, $dyn_name:ident, $dark_base:expr, $bright_base:expr, $prefix:literal) => {
        declare_n_struct!($name<COLOR: u8>);
        implement_sgr!($name<COLOR: u8>: self; f {
            match COLOR {
                0..=7 => <_ as ::core::fmt::Display>::fmt(&($dark_base + COLOR), f),
                8..=15 => <_ as ::core::fmt::Display>::fmt(&($bright_base + COLOR), f),
                _ => {
                    f.write_str($prefix)?;
                    <_ as ::core::fmt::Display>::fmt(&COLOR, f)
                }
            }
        });

        declare_n_struct!($dyn_name(COLOR: u8));
        implement_sgr!($dyn_name: self; f {
            match self.0 {
                0..=7 => <_ as ::core::fmt::Display>::fmt(&($dark_base + self.0), f),
                8..=15 => <_ as ::core::fmt::Display>::fmt(&($bright_base + self.0), f),
                _ => {
                    f.write_str($prefix)?;
                    <_ as ::core::fmt::Display>::fmt(&self.0, f)
                }
            }
        });
    }
}

macro_rules! define_24bit_color {
    ($name:ident, $dyn_name:ident, $prefix:literal) => {
        declare_n_struct!($name<R: u8, G: u8, B: u8>);
        implement_sgr!($name<R: u8, G: u8, B: u8>: self; f {
            f.write_str($prefix)?;
            <_ as ::core::fmt::Display>::fmt(&R, f)?;
            f.write_str(";")?;
            <_ as ::core::fmt::Display>::fmt(&G, f)?;
            f.write_str(";")?;
            <_ as ::core::fmt::Display>::fmt(&B, f)
        });

        declare_n_struct!($dyn_name(R: u8, G: u8, B: u8));
        implement_sgr!($dyn_name: self; f {
            f.write_str($prefix)?;
            <_ as ::core::fmt::Display>::fmt(&self.0, f)?;
            f.write_str(";")?;
            <_ as ::core::fmt::Display>::fmt(&self.1, f)?;
            f.write_str(";")?;
            <_ as ::core::fmt::Display>::fmt(&self.2, f)
        });
    }
}

// ====================================== Library ======================================

// -------------------------------- Terminal Management --------------------------------

define_unit_command!(RequestTerminalId, "\x1b[>q");

impl Query for RequestTerminalId {
    type Response = (Option<Vec<u8>>, Option<Vec<u8>>);

    #[inline]
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

define_unit_command!(SaveWindowTitle, "\x1b[22;2t");
define_unit_command!(RestoreWindowTitle, "\x1b[23;2t");

/// The dynamic `DynSetWindowTitle(String)` command.
///
/// This command cannot be copied, only cloned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynSetWindowTitle(String);
implement_command!(DynSetWindowTitle: self; f {
    f.write_str("\x1b]2;")?;
    f.write_str(self.0.as_str())?;
    f.write_str("\x1b\\")
});

// --------------------------------- Screen Management ---------------------------------

define_unit_command!(EnterAlternateScreen, "\x1b[?1049h");
define_unit_command!(ExitAlternateScreen, "\x1b[?1049l");

define_unit_command!(EnableReverseMode, "\x1b[?5h");
define_unit_command!(DisableReverseMode, "\x1b[?5l");

define_unit_command!(EraseScreen, "\x1b[2J");

declare_unit_struct!(RequestScreenSize);
impl Command for RequestScreenSize {}

impl core::fmt::Display for RequestScreenSize {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        MoveTo::<{ u16::MAX }, { u16::MAX }>.fmt(f)?;
        RequestCursorPosition.fmt(f)
    }
}

impl Query for RequestScreenSize {
    type Response = <RequestCursorPosition as Query>::Response;

    #[inline]
    fn control(&self) -> Control {
        RequestCursorPosition.control()
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        RequestCursorPosition.parse(payload)
    }
}

// ------------------------------------- Scrolling -------------------------------------

define_cmd_1!(ScrollUp<ROWS: u16>, DynScrollUp, "\x1b[", "S");
define_cmd_1!(ScrollDown<ROWS: u16>, DynScrollDown, "\x1b[", "S");

define_cmd_2!(SetScrollRegion<TOP: u16, BOTTOM: u16>, DynSetScrollRegion, "\x1b[", "r");
define_unit_command!(ResetScrollRegion, "\x1b[r");
define_unit_command!(EnableAutowrap, "\x1b[?7h");
define_unit_command!(DisableAutowrap, "\x1b[?7l");

// --------------------------------- Cursor Management ---------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetCursor {
    Default = 0,
    BlinkingBlock = 1,
    SteadyBlock = 2,
    BlinkingUnderscore = 3,
    SteadyUnderscore = 4,
    BlinkingBar = 5,
    SteadyBar = 6,
}

impl Command for SetCursor {}

impl core::fmt::Display for SetCursor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("\x1b[")?;
        <_ as core::fmt::Display>::fmt(&(*self as u8), f)?;
        f.write_str(" q")
    }
}

define_unit_command!(HideCursor, "\x1b[?25l");
define_unit_command!(ShowCursor, "\x1b[?25h");

define_cmd_1!(MoveUp<ROWS: u16>, DynMoveUp, "\x1b[", "A");
define_cmd_1!(MoveDown<ROWS: u16>, DynMoveDown, "\x1b[", "B");
define_cmd_1!(MoveLeft<COLUMNS: u16>, DynMoveLeft, "\x1b[", "C");
define_cmd_1!(MoveRight<COLUMNS: u16>, DynMoveRight, "\x1b[", "D");

define_cmd_2!(MoveTo<ROW: u16, COLUMN: u16>, DynMoveTo, "\x1b[", "H");

define_cmd_1!(MoveToColumn<COLUMN: u16>, DynMoveToColumn, "\x1b[", "G");
define_cmd_1!(MoveToRow<ROW: u16>, DynMoveToRow, "\x1b[", "d");

define_unit_command!(SaveCursorPosition, "\x1b7");
define_unit_command!(RestoreCursorPosition, "\x1b8");

define_unit_command!(RequestCursorPosition, "\x1b[6n");

impl Query for RequestCursorPosition {
    /// The row and column of the cursor in that order.
    type Response = (u16, u16);

    #[inline]
    fn control(&self) -> Control {
        Control::CSI
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        let s = payload
            .strip_suffix(b"R")
            .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;

        let mut index = 0;
        let mut params = [0_u16; 2];
        for bytes in s.split(|b| *b == b';' || *b == b':') {
            if 2 <= index {
                return Err(ErrorKind::InvalidData.into());
            }
            params[index] = ByteParser::Decimal
                .to_u16(bytes)
                .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
            index += 1;
        }

        if index < 2 {
            return Err(ErrorKind::InvalidData.into());
        }

        Ok(params.into())
    }
}

// -------------------------------- Content Management ---------------------------------

define_unit_command!(EraseLine, "\x1b[2K");
define_unit_command!(EraseRestOfLine, "\x1b[K");

define_unit_command!(BeginBatch, "\x1b[?2026h");
define_unit_command!(EndBatch, "\x1b[?2026l");

define_unit_command!(BeginPaste, "\x1b[?2004h");
define_unit_command!(EndPaste, "\x1b[?2004l");

/// The dynamic `DynLink(ID, HREF, TEXT)` command.
///
/// This command cannot be copied, only cloned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynLink(Option<String>, String, String);

impl DynLink {
    /// Create a new hyperlink with the given URL and text.
    pub fn new<H, T>(href: H, text: T) -> Self
    where
        H: Into<String>,
        T: Into<String>,
    {
        Self(None, href.into(), text.into())
    }

    /// Create a new hyperlink with the given ID, URL, and text.
    pub fn with_id<I, H, T>(id: Option<I>, href: H, text: T) -> Self
    where
        I: Into<String>,
        H: Into<String>,
        T: Into<String>,
    {
        Self(id.map(core::convert::Into::into), href.into(), text.into())
    }
}

implement_command!(DynLink: self; f {
    if let Some(ref id) = self.0 {
        f.write_str("\x1b]8;id=")?;
        f.write_str(id)?;
        f.write_str(";")?;
    } else {
        f.write_str("\x1b]8;;")?;
    }

    f.write_str(self.1.as_str())?;
    f.write_str("\x1b\\")?;
    f.write_str(self.2.as_str())?;
    f.write_str("\x1b]8;;\x1b\\")
});

// --------------------------------------- Modes ---------------------------------------

/// The current batch processing mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeStatus {
    NotSupported = 0,
    Enabled = 1,
    Disabled = 2,
    PermanentlyEnabled = 3,
    PermanentlyDisabled = 4,
}

define_cmd_1!(RequestMode<MODE: u16>, DynRequestMode, "\x1b[?", "$p");

fn parse_mode_status(payload: &[u8], expected_mode: u16) -> Result<ModeStatus> {
    let bare_payload = payload
        .strip_prefix(b"?")
        .and_then(|s| s.strip_suffix(b"$y"))
        .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
    let sep = bare_payload
        .iter()
        .position(|item| *item == b';')
        .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
    let mode = ByteParser::Decimal
        .to_u16(&bare_payload[..sep])
        .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
    if mode != expected_mode {
        return Err(Error::from(ErrorKind::InvalidData));
    }
    let status = ByteParser::Decimal
        .to_u16(&bare_payload[sep + 1..])
        .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
    Ok(match status {
        0 => ModeStatus::NotSupported,
        1 => ModeStatus::Enabled,
        2 => ModeStatus::Disabled,
        3 => ModeStatus::PermanentlyEnabled,
        4 => ModeStatus::PermanentlyDisabled,
        _ => return Err(Error::from(ErrorKind::InvalidData)),
    })
}

impl<const MODE: u16> Query for RequestMode<MODE> {
    type Response = ModeStatus;

    #[inline]
    fn control(&self) -> Control {
        Control::CSI
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        parse_mode_status(payload, MODE)
    }
}

impl Query for DynRequestMode {
    type Response = ModeStatus;

    #[inline]
    fn control(&self) -> Control {
        Control::CSI
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        parse_mode_status(payload, self.0)
    }
}

// --------------------------------- Style Management ----------------------------------

define_unit_command!(ResetStyle, "\x1b[m");

define_unit_sgr!(SetDefaultForeground, "39");
define_unit_sgr!(SetDefaultBackground, "49");
define_8bit_color!(
    SetForeground8,
    DynSetForeground8,
    30,
    (const { 90 - 8 }),
    "38;5;"
);
define_8bit_color!(
    SetBackground8,
    DynSetBackground8,
    40,
    (const { 100 - 8 }),
    "48;5;"
);
define_24bit_color!(SetForeground24, DynSetForeground24, "38;2;");
define_24bit_color!(SetBackground24, DynSetBackground24, "48;2;");

/// The enumeration of unit `Format` commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    #[must_use = "the only reason to invoke method is to access the returned value"]
    pub fn undo(&self) -> Self {
        use self::Format::*;

        match *self {
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
    #[inline]
    fn write_param(&self, f: &mut core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        <_ as core::fmt::Display>::fmt(&(*self as u8), f)
    }
}

impl Command for Format {}

impl core::fmt::Display for Format {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("\x1b[")?;
        self.write_param(f)?;
        f.write_str("m")
    }
}

define_unit_command!(RequestActiveStyle, "\x1bP$qm\x1b\\");

impl Query for RequestActiveStyle {
    type Response = Vec<u8>;

    #[inline]
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

/// The enumeration of unit `RequestColor` commands.
///
/// The discriminant ranges from 0 to 15 for the 16 ANSI colors. For the default
/// foreground, default background, cursor, or selection colors, it is 100 plus
/// the code used in the query. On Windows, this query is only supported by
/// Terminal 1.22 or later.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    fn successor(&self) -> Option<Self> {
        use self::RequestColor::*;

        Some(match *self {
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
    pub fn all() -> impl Iterator<Item = Self> {
        successors(Some(Self::Black), Self::successor)
    }
}

impl Command for RequestColor {}

impl core::fmt::Display for RequestColor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let code = *self as u32;
        if code < 16 {
            f.write_str("\x1b]4;")?;
            <_ as core::fmt::Display>::fmt(&code, f)?;
            f.write_str(";?\x1b\\")
        } else {
            f.write_str("\x1b]")?;
            <_ as core::fmt::Display>::fmt(&(code - 100), f)?;
            f.write_str(";?\x1b\\")
        }
    }
}

impl Query for RequestColor {
    /// An RGB color.
    ///
    /// The parsed response comprises one pair per RGB channel, with the first
    /// number the signal strength and the second number the signal width. The
    /// signal width is the number of hexadecimal digits, always between 1 and 4
    /// inclusive, and usually 4. Hence, to normalize (_s_, _w_) to a floating
    /// point number between 0 and 1, compute _s_/((16^_w_)-1).
    type Response = [(u16, u16); 3];

    #[inline]
    fn control(&self) -> Control {
        Control::OSC
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        use crate::err::ErrorKind;

        let code = *self as u8;
        let bytes = if code < 20 {
            let bytes = payload
                .strip_prefix(b"4;")
                .ok_or_else(|| Error::from(ErrorKind::BadSequence))?;
            if code < 10 {
                bytes.strip_prefix(&[b'0' + code])
            } else {
                bytes.strip_prefix(&[b'1', b'0' + code - 10])
            }
        } else {
            payload.strip_prefix(match *self {
                Self::Foreground => b"10",
                Self::Background => b"11",
                Self::Cursor => b"12",
                Self::Selection => b"17",
                _ => panic!("unknown theme color"),
            })
        }
        .and_then(|bytes| bytes.strip_prefix(b";rgb:"))
        .ok_or_else(|| Error::from(ErrorKind::BadSequence))?;

        fn parse(bytes: Option<&[u8]>) -> core::result::Result<(u16, u16), Error> {
            let bytes = bytes.ok_or_else(|| Error::from(ErrorKind::TooFewCoordinates))?;
            if bytes.is_empty() {
                return Err(ErrorKind::EmptyCoordinate.into());
            } else if 4 < bytes.len() {
                return Err(ErrorKind::OversizedCoordinate.into());
            }

            let n = ByteParser::Hexadecimal
                .to_u16(bytes)
                .ok_or_else(|| Error::from(ErrorKind::MalformedCoordinate))?;
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
    use super::*;
    use crate::Control;

    #[test]
    fn test_size_and_display() {
        assert_eq!(std::mem::size_of::<BeginBatch>(), 0);
        assert_eq!(std::mem::size_of::<MoveLeft::<2>>(), 0);
        assert_eq!(std::mem::size_of::<DynMoveLeft>(), 2);
        assert_eq!(std::mem::size_of::<MoveTo::<5, 7>>(), 0);
        assert_eq!(std::mem::size_of::<DynMoveTo>(), 4);
        assert_eq!(std::mem::size_of::<SetDefaultForeground>(), 0);
        assert_eq!(std::mem::size_of::<SetForeground8::<0>>(), 0);
        assert_eq!(std::mem::size_of::<SetForeground8::<15>>(), 0);
        assert_eq!(std::mem::size_of::<SetForeground8::<88>>(), 0);
        assert_eq!(std::mem::size_of::<SetBackground8::<7>>(), 0);
        assert_eq!(std::mem::size_of::<SetBackground8::<9>>(), 0);
        assert_eq!(std::mem::size_of::<SetBackground8::<226>>(), 0);
        assert_eq!(std::mem::size_of::<SetForeground24::<255, 103, 227>>(), 0);
        assert_eq!(std::mem::size_of::<SetBackground24::<134, 36, 161>>(), 0);

        assert_eq!(format!("{}", BeginBatch), "\x1b[?2026h");
        assert_eq!(format!("{}", MoveLeft::<2>), "\x1b[2C");
        assert_eq!(format!("{}", DynMoveLeft(2)), "\x1b[2C");
        assert_eq!(format!("{}", MoveTo::<5, 7>), "\x1b[5;7H");
        assert_eq!(format!("{}", DynMoveTo(5, 7)), "\x1b[5;7H");
        assert_eq!(format!("{}", SetDefaultForeground), "\x1b[39m");
        assert_eq!(format!("{}", SetForeground8::<0>), "\x1b[30m");
        assert_eq!(format!("{}", SetForeground8::<15>), "\x1b[97m");
        assert_eq!(format!("{}", SetForeground8::<88>), "\x1b[38;5;88m");
        assert_eq!(format!("{}", SetBackground8::<7>), "\x1b[47m");
        assert_eq!(format!("{}", SetBackground8::<9>), "\x1b[101m");
        assert_eq!(format!("{}", SetBackground8::<226>), "\x1b[48;5;226m");
        assert_eq!(
            format!("{}", SetForeground24::<255, 103, 227>),
            "\x1b[38;2;255;103;227m"
        );
        assert_eq!(
            format!("{}", SetBackground24::<134, 36, 161>),
            "\x1b[48;2;134;36;161m"
        );
    }

    #[test]
    fn test_parse_mode_status() -> std::io::Result<()> {
        let status = RequestMode::<2027>.parse(b"?2027;0$y")?;
        assert_eq!(status, ModeStatus::NotSupported);
        let status = RequestMode::<2027>.parse(b"?2027;3$y")?;
        assert_eq!(status, ModeStatus::PermanentlyEnabled);

        let status = DynRequestMode(2027).parse(b"?2027;2$y")?;
        assert_eq!(status, ModeStatus::Disabled);
        let status = DynRequestMode(2027).parse(b"?2027;4$y")?;
        assert_eq!(status, ModeStatus::PermanentlyDisabled);

        let status = RequestMode::<2002>.parse(b"?2027;3$y");
        assert!(status.is_err());
        assert_eq!(status.unwrap_err().kind(), ErrorKind::InvalidData);
        Ok(())
    }

    #[test]
    fn test_parse_terminal_id() -> std::io::Result<()> {
        assert_eq!(RequestTerminalId.control(), Control::DCS);

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
        assert_eq!(RequestCursorPosition.control(), Control::CSI);

        let position = RequestCursorPosition.parse(b"6;65R")?;
        assert_eq!(position, (6, 65));
        Ok(())
    }

    #[test]
    fn test_parse_theme_color() -> std::io::Result<()> {
        assert_eq!(RequestColor::Magenta.control(), Control::OSC);

        let color = RequestColor::Background.parse(b"11;rgb:a/b/cdef")?;
        assert_eq!(color, [(10, 1), (11, 1), (52_719, 4)]);
        let color = RequestColor::Magenta.parse(b"4;5;rgb:12/345/6789")?;
        assert_eq!(color, [(18, 2), (837, 3), (26_505, 4)]);
        let color = RequestColor::BrightMagenta.parse(b"4;13;rgb:ff/00/ff")?;
        assert_eq!(color, [(255, 2), (0, 2), (255, 2)]);
        Ok(())
    }
}
