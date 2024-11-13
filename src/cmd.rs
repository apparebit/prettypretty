//! Controlling the terminal with ANSI escape sequences.
//!
//! This module defines [`Command`] as the common interface for sending commands
//! to terminals. The implementation supports only ANSI escape sequence, since
//! that is the preferred interface for controlling the terminal even on
//! Windows. The operating system has been supporting ANSI escape sequence since
//! Windows 10 TH2 (v1511).
//!
//! This module also defines a basic library of such commands. Each command
//! implements the `Debug` and `Display` traits as well. The `Debug`
//! representation is the usual datatype representation, whereas the `Display`
//! representation is the ANSI escape sequence. As a result, all commands
//! defined by this module can be directly written to output, just like
//! [`Style`](crate::style::Style) and [`ThemeEntry`](crate::theme::ThemeEntry).
//!
//! The core library includes the following commands:
//!
//!   * For terminal management, [`RequestTerminalId`].
//!   * For window management, [`SaveWindowTitleOnStack`],
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

#![allow(dead_code)]

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

// -------------------------------- Content Management ---------------------------------

define_simple_suite!(RequestBatchMode, "\x1b[?2026$p");
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

pub fn link(text: impl AsRef<str>, href: impl AsRef<str>, id: Option<&str>) -> Link {
    Link::new(text, href, id)
}

define_command!(Link, self, f { f.write_str(&self.0) } );
define_display!(Link);

// =====================================================================================

#[cfg(test)]
mod test {
    use super::{BeginBatchedOutput, MoveLeft, MoveTo};

    #[test]
    fn test_size_and_display() {
        assert_eq!(std::mem::size_of::<BeginBatchedOutput>(), 0);
        assert_eq!(std::mem::size_of::<MoveLeft>(), 2);
        assert_eq!(std::mem::size_of::<MoveTo>(), 4);

        assert_eq!(format!("{}", BeginBatchedOutput), "\x1b[?2026h");
        assert_eq!(format!("{}", MoveLeft(2)), "\x1b[2C");
        assert_eq!(format!("{}", MoveTo(5, 7)), "\x1b[5;7H")
    }
}
