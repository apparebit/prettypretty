//! *Incomplete draft support* for keyboard, mouse, and query response events.
//!
//! Where [`Token`]s are an efficient syntactic representation, [`Event`]s are a
//! somewhat less efficient semantic representation.
//!
//! # Background
//!
//! Prettytty's [`crate::Scan`] trait reads so-called [`Token`]s from terminal
//! input, largely to distinguish between runs of UTF-8 encoded text and ANSI
//! escape sequences. To avoid the overhead of heap allocation for every single
//! token, their payloads draw on the scanner's internal buffer. While more
//! efficient, it also necessitates processing a token's payload before reading
//! the next.
//!
//! By contrast, this module's [`Event`]s are higher-level, heavier-weight
//! abstractions, which often stem from entire ANSI escape sequences. They also
//! are self-contained, i.e., not restricted by explicit lifetimes, yet
//! lightweight enough to implement [`Copy`]. They include key, mouse, window,
//! and response-to-query events. The [`TokenOrEvent`] struct is intended to
//! accommodate unknown ANSI escape sequences. Consistent with how hardware
//! terminals treat unknown escape sequences, an application may simply ignore
//! such tokens.
//!
//! Tokens should suffice for applications that only need to process key presses
//! for letters, numbers, and common symbols—or the occasional ANSI escape
//! sequence. Applications that need to process modifier keys, mouse input,
//! and ANSI escape sequences should prefer events.
//!
//! ## Keyboard Input
//!
//! Unfortunately, there are several conventions for encoding keyboard and mouse
//! input from terminals. Legacy encodings for keyboard events share some
//! patterns, such as
//! <code>CSI&nbsp;<em>P<sub>key</sub></em>&nbsp;;&nbsp;<em>P<sub>mod</sub></em>&nbsp;~</code>
//! or <code>SS3&nbsp;<em>key</em></code>. But they also differ for PC, VT-220,
//! VT-52, Sun, and HP keyboards, as covered in
//! [xterm](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Special-Keyboard-Keys)'s
//! documentation and highlighted by [this
//! table](https://invisible-island.net/xterm/xterm-function-keys.html). Worse,
//! legacy encodings are incomplete and ambiguous. For example, there is no way
//! to distinguish between <kbd>ctrl-i</kbd> and <kbd>shift-ctrl-i</kbd>.
//! <kbd>alt-c</kbd> may really be <kbd>esc</kbd> and <kbd>c</kbd> typed too
//! quickly after each other. <kbd>alt-c</kbd> also overlaps with the first byte
//! in the UTF-8 encoding of many extended Latin characters such as é.
//!
//! There are a number of attempts to fix these short-comings:
//!
//!   * [Fixterms](http://www.leonerd.org.uk/hacks/fixterms/)
//!   * [kitty's terminal protocol
//!     extensions](https://sw.kovidgoyal.net/kitty/protocol-extensions/#keyboard-handling),
//!     which build on fixterms
//!   * [xterm's
//!     modifyOtherKeys](https://invisible-island.net/xterm/modified-keys.html)
//!     enables a partially overlapping encoding
//!   * [`win32-input-mode`](https://github.com/microsoft/terminal/blob/main/doc/specs/%234999%20-%20Improved%20keyboard%20handling%20in%20Conpty.md),
//!     which encodes the Windows-specific `KEY_EVENT_RECORD` structure
//!
//! Currently, prettytty supports common legacy encodings as well as the
//! encoding based on <code>CSI⋯u</code> shared between fixterms, kitty, and
//! xterm. No configuration is necessary, all of them are recognized by default.
//! Prettytty does *not* yet support kitty's progressive enhancements. While
//! many of the enhancements are useful, their encoding is rather awkward,
//! distinguishing between semicolons and colons. While that may be consistent
//! with the letter of some ISO standards, it also is wilfully different from
//! all other ANSI escape sequences and well-established terminal practice.
//!
//!
//! # The `event` Feature
//!
//! This module is entirely optional and requires that the `event` feature is
//! enabled.

// Some background on encoding [mouse
// events](https://leonerds-code.blogspot.com/2012/04/wide-mouse-support-in-libvterm.html)

use crate::cmd::{ModeStatus, RequestColor};
use crate::Token;

/// A token or an event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenOrEvent<'a> {
    /// An event.
    Event(Event),
    /// A token, which is guaranteed not to be a text token.
    Token(Token<'a>),
}

impl TokenOrEvent<'_> {
    /// Determine whether this is a token.
    pub const fn is_token(&self) -> bool {
        matches!(*self, TokenOrEvent::Token(_))
    }

    /// Determine whether this is an event.
    pub const fn is_event(&self) -> bool {
        matches!(*self, TokenOrEvent::Event(_))
    }

    /// Drop any token, returning events only.
    pub fn as_event(&self) -> Option<Event> {
        match *self {
            TokenOrEvent::Event(event) => Some(event),
            _ => None,
        }
    }

    /// Map an event, dropping the token.
    pub fn map_event<R, F>(&self, op: F) -> Option<R>
    where
        F: FnOnce(Event) -> R,
    {
        match *self {
            TokenOrEvent::Event(event) => Some(op(event)),
            _ => None,
        }
    }

    /// Map either token or event to a common (result) type.
    ///
    /// This method consumes the token-or-event instance.
    pub fn map_either<R, Fe, Ft>(self, map_event: Fe, map_token: Ft) -> Option<R>
    where
        Fe: FnOnce(Event) -> Option<R>,
        Ft: FnOnce(Token<'_>) -> Option<R>,
    {
        match self {
            TokenOrEvent::Event(event) => map_event(event),
            TokenOrEvent::Token(token) => map_token(token),
        }
    }
}

/// An input event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// A key press, repeat, or release.
    Key(KeyEvent),
    /// A mouse move, button press, mouse drag, or button release.
    Mouse(MouseEvent),
    /// A window event.
    Window(WindowEvent),
    /// A cursor event.
    Cursor(CursorEvent),
    /// A color event.
    Color(ColorEvent),
    /// A terminal mode event.
    Mode(ModeStatus),
}

// -------------------------------------------------------------------------------------

/// A mouse event.
///
/// This struct captures mouse movements along with keyboard modifiers and
/// button presses. Since every mouse event includes a screen coordinate, this
/// struct includes both coordinates inline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub modifiers: Modifiers,
    pub column: u16,
    pub row: u16,
}

/// The kind of mouse event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseEventKind {
    /// Pressing a mouse button.
    Press(MouseButton),
    /// Keeping a mouse button pressed.
    Drag(MouseButton),
    /// Releasing a mouse button.
    Release(MouseButton),
    /// Moving the mouse.
    Move,
}

/// The mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

// -------------------------------------------------------------------------------------

/// A key event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub kind: KeyEventKind,
    pub modifiers: Modifiers,
    pub key: Key,
}

/// The kind of key event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyEventKind {
    /// Pressing a key.
    Press,
    /// Keeping a key pressed.
    Repeat,
    /// Releasing a key.
    Release,
}

/// A logical modifier of keys and mouse button presses.
///
/// This enumeration abstracts over the concrete [modifier key](ModifierKey),
/// which may be on the left or right side of the keyboard, and instead includes
/// variants for the logical modifier only. Contemporary keyboard generally
/// include the following modifier keys:
///
///   * <kbd>shift</kbd>
///   * <kbd>alt</kbd> or<kbd>option</kbd>
///   * <kbd>control</kbd>
///   * <kbd>Windows</kbd>, <kbd>Linux</kbd>, or <kbd>command</kbd>
///   * <kbd>caps lock</kbd>
///   * possibly <kbd>num lock</kbd>
///
/// Influential past keyboards, such as the [space-cadet
/// keyboard](https://en.wikipedia.org/wiki/Space-cadet_keyboard), further
/// included the <kbd>super</kbd>, <kbd>hyper</kbd>, and <kbd>meta</kbd>
/// modifiers.
///
/// Terminal emulators usually agree only on the names of the first three
/// modifiers, (1) <kbd>shift</kbd>, (2) <kbd>alt</kbd>/<kbd>option</kbd>, and
/// (3) <kbd>control</kbd>. Amongst further modifiers, xterm labels the fourth
/// one <kbd>meta</kbd>, whereas kitty calls that modifier <kbd>super</kbd> and
/// the sixth modifier <kbd>meta</kbd>. Given these divergent names, prettytty
/// uses a neutral term, <kbd>command</kbd>, for the fourth modifier.
///
/// The [`Modifier::Keypad`] variant is a virtual modifier, i.e., it has no
/// physical key. It is used to distinguish keys, such as <kbd>/</kbd> or
/// <kbd>=</kbd>, that appear even on a 60% or 65% keyboard from equally
/// labelled keys abutting the numeric pad, which only appears on 96% or 100%
/// keyboards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Modifier {
    /// The <kbd>shift</kbd> key.
    Shift = 0x01,
    /// The <kbd>alt</kbd> or <kbd>option</kbd> key.
    Option = 0x02,
    /// The <kbd>control</kbd> key.
    Control = 0x04,
    /// The <kbd>Windows</kbd>, <kbd>Linux</kbd>, or <kbd>command</kbd> key.
    /// Xterm labels this modifier as <kbd>meta</kbd> and Kitty labels it as
    /// <kbd>super</kbd>
    Command = 0x08,
    /// A first extra modifier, labelled <kbd>hyper</kbd> by Kitty.
    Extra1 = 0x10,
    /// A second extra modifier, labelled <kbd>meta</kbd> by Kitty.
    Extra2 = 0x20,
    /// The <kbd>caps lock</kbd> key.
    CapsLock = 0x40,
    /// The <kbd>number lock</kbd> key.
    NumLock = 0x80,
    /// A virtual modifier indicating a keypad key.
    Keypad = 0x100,
}

impl Modifier {
    const fn successor(&self) -> Option<Self> {
        use Modifier::*;

        Some(match *self {
            Shift => Option,
            Option => Control,
            Control => Command,
            Command => Extra1,
            Extra1 => Extra2,
            Extra2 => CapsLock,
            CapsLock => NumLock,
            NumLock => Keypad,
            Keypad => return None,
        })
    }
}

impl From<Modifier> for Modifiers {
    fn from(value: Modifier) -> Self {
        Self(value as u16)
    }
}

impl<M: Into<Modifiers>> core::ops::Add<M> for Modifier {
    type Output = Modifiers;

    fn add(self, rhs: M) -> Self::Output {
        Modifiers::combine(self as u16, rhs.into().0)
    }
}

impl<M: Into<Modifiers>> core::ops::Add<M> for Modifiers {
    type Output = Modifiers;

    fn add(self, rhs: M) -> Self::Output {
        Self::combine(self.0, rhs.into().0)
    }
}

impl<M: Into<Modifiers>> core::ops::AddAssign<M> for Modifiers {
    fn add_assign(&mut self, rhs: M) {
        *self = Self::combine(self.0, rhs.into().0)
    }
}

impl<M: Into<Modifiers>> core::ops::Sub<M> for Modifier {
    type Output = Modifiers;

    fn sub(self, rhs: M) -> Self::Output {
        Modifiers(self as u16 & !rhs.into().0)
    }
}

impl<M: Into<Modifiers>> core::ops::Sub<M> for Modifiers {
    type Output = Modifiers;

    fn sub(self, rhs: M) -> Self::Output {
        Self(self.0 & !rhs.into().0)
    }
}

impl<M: Into<Modifiers>> core::ops::SubAssign<M> for Modifiers {
    fn sub_assign(&mut self, rhs: M) {
        *self = Self(self.0 & !rhs.into().0)
    }
}

/// A key's logical modifiers.
///
/// This struct combines zero or more logical modifiers. In fact, it defaults to
/// zero modifiers.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers(u16);

impl Modifiers {
    fn combine(left: u16, right: u16) -> Self {
        Self(left | right)
    }

    /// Decode an ANSI escape parameter.
    pub fn from_escape(code: u16) -> Option<Self> {
        let code = code.saturating_sub(1);
        if code <= (u8::MAX as u16) {
            Some(Self(code))
        } else {
            None
        }
    }

    /// Determine whether there are no active modifiers.
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Get the number of active modifiers.
    pub const fn len(&self) -> usize {
        self.0.count_ones() as usize
    }

    /// Determine whether the given modifier is enabled.
    pub fn has(&self, modifier: Modifier) -> bool {
        self.0 & modifier as u16 != 0
    }

    /// Get an iterator over the active modifiers.
    pub fn modifiers(&self) -> ModifierIter {
        ModifierIter {
            modifiers: *self,
            cursor: None,
            remaining: self.len(),
        }
    }
}

impl core::fmt::Debug for Modifiers {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_set().entries(self.modifiers()).finish()
    }
}

/// An iterator over modifiers.
#[derive(Debug)]
pub struct ModifierIter {
    modifiers: Modifiers,
    cursor: Option<Modifier>,
    remaining: usize,
}

impl Iterator for ModifierIter {
    type Item = Modifier;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let modifier = match self.cursor {
                None => Modifier::Shift,
                Some(Modifier::NumLock) => return None,
                Some(modifier) => modifier
                    .successor()
                    .expect("with no-successor case already handled, successor must exist"),
            };
            self.cursor = Some(modifier);

            if self.modifiers.has(modifier) {
                self.remaining -= 1;
                return Some(modifier);
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for ModifierIter {
    fn len(&self) -> usize {
        self.remaining
    }
}

impl core::iter::FusedIterator for ModifierIter {}

// -------------------------------------------------------------------------------------

/// A key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Key {
    Escape,
    Enter,
    Tab,
    Backspace,
    Insert,
    Delete,
    Left,
    Right,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    CapsLock,
    ScrollLock,
    NumLock,
    PrintScreen,
    Pause,
    Menu,
    KeypadBegin,
    F(u8),
    Media(MediaKey),
    Mod(ModifierKey),
    Char(char),
}

impl Key {
    /// Get the key corresponding to SS3/CSI-1 and the given byte.
    pub fn with_ss3(byte: u8) -> Option<Self> {
        use Key::*;

        let key = match byte {
            b' ' => Char(' '),
            b'A' => Up,
            b'B' => Down,
            b'C' => Right,
            b'D' => Left,
            b'E' => KeypadBegin,
            b'F' => End,
            b'H' => Home,
            b'I' => Tab,
            b'M' => Enter,
            b'P' => F(1),
            b'Q' => F(2),
            b'R' => F(3),
            b'S' => F(4),
            b'X' => Char('='),
            b'j' => Char('*'),
            b'k' => Char('+'),
            b'l' => Char(','),
            b'm' => Char('-'),
            b'n' => Char('.'),
            b'o' => Char('/'),
            b'p' => Char('0'),
            b'q' => Char('1'),
            b'r' => Char('2'),
            b's' => Char('3'),
            b't' => Char('4'),
            b'u' => Char('5'),
            b'v' => Char('6'),
            b'w' => Char('7'),
            b'x' => Char('8'),
            b'y' => Char('9'),
            _ => return None,
        };

        Some(key)
    }

    /// Map a function key code to a key.
    ///
    /// For a DECFNK escape sequence, that is, CSI *Pcode* ; *Pmod* ~, this
    /// method maps the code parameter to a key.
    pub fn with_function_key(code: u8) -> Option<Self> {
        use Key::*;

        // https://www.xfree86.org/current/ctlseqs.html
        // https://vt100.net/docs/vt510-rm/DECFNK.html
        let key = match code {
            1 => Home,
            2 => Insert,
            3 => Delete,
            4 => End,
            5 => PageUp,
            6 => PageDown,
            7 => Left, // kitty incorrectly assigns Home
            8 => Down, // kitty incorrectly assigns End
            9 => Up,
            10 => Right,
            11..=15 => F(code - 10), // F1..=F5
            17..=21 => F(code - 11), // F6..=F10
            23..=26 => F(code - 12), // F11..=F14
            28..=29 => F(code - 13), // F15..=F16
            31..=34 => F(code - 14), // F17..=F20
            _ => return None,
        };

        Some(key)
    }

    /// Map the CSI/u key code to a key.
    pub fn with_csi_u(code: u16) -> Option<(Self, Modifiers)> {
        use Key::*;

        let key = match code {
            9 => Tab,
            13 => Enter,
            27 => Escape,
            127 => Backspace,

            57_358 => CapsLock,
            57_359 => ScrollLock,
            57_360 => NumLock,
            57_361 => PrintScreen,
            57_362 => Pause,
            57_363 => Menu,
            57_376..=57_398 => F((code - 57_376) as u8 + 13),
            // Keypad keys
            57_399..=57_408 => Char((b'0' + (code - 57_399) as u8) as char),
            57_409 => Char('.'),
            57_410 => Char('/'),
            57_411 => Char('*'),
            57_412 => Char('-'),
            57_413 => Char('+'),
            57_414 => Enter,
            57_415 => Char('='),
            57_416 => Char(','),
            57_417 => Left,
            57_418 => Right,
            57_419 => Up,
            57_420 => Down,
            57_421 => PageUp,
            57_422 => PageDown,
            57_423 => Home,
            57_424 => End,
            57_425 => Insert,
            57_426 => Delete,
            57_427 => KeypadBegin,
            // Media and modifier keys
            57_428..=57_440 => Media(MediaKey::with_csi_u(code)?),
            57_441..=57_452 => Mod(ModifierKey::with_csi_u(code)?),
            _ => return None,
        };

        let modifiers = if (57_399..=57_427).contains(&code) {
            self::Modifier::Keypad.into()
        } else {
            Modifiers::default()
        };

        Some((key, modifiers))
    }
}

/// A media key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MediaKey {
    Play,
    Pause,
    PlayPause,
    Reverse,
    Stop,
    FastForward,
    Rewind,
    NextTrack,
    PreviousTrack,
    Record,
    LowerVolume,
    RaiseVolume,
    MuteVolume,
}

impl MediaKey {
    /// Map the CSI/u key code to a media key.
    pub fn with_csi_u(code: u16) -> Option<Self> {
        use MediaKey::*;

        let key = match code {
            57_428 => Play,
            57_429 => Pause,
            57_430 => PlayPause,
            57_431 => Reverse,
            57_432 => Stop,
            57_433 => FastForward,
            57_434 => Rewind,
            57_435 => NextTrack,
            57_436 => PreviousTrack,
            57_437 => Record,
            57_438 => LowerVolume,
            57_439 => RaiseVolume,
            57_440 => MuteVolume,
            _ => return None,
        };

        Some(key)
    }
}

/// A physical modifier key.
///
/// This enumeration comprises physical keys instead of logical [`Modifier`]s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModifierKey {
    LeftShift,
    LeftControl,
    LeftOption,
    LeftCommand,
    LeftExtra1,
    LeftExtra2,
    RightShift,
    RightControl,
    RightOption,
    RightCommand,
    RightExtra1,
    RightExtra2,
}

impl ModifierKey {
    /// Map the CSI/u key code to a modifier key.
    pub fn with_csi_u(code: u16) -> Option<Self> {
        use ModifierKey::*;

        let key = match code {
            57_441 => LeftShift,
            57_442 => LeftControl,
            57_443 => LeftOption,
            57_444 => LeftCommand,
            57_445 => LeftExtra1,
            57_446 => LeftExtra2,
            57_447 => RightShift,
            57_448 => RightControl,
            57_449 => RightOption,
            57_450 => RightCommand,
            57_451 => RightExtra1,
            57_452 => RightExtra2,
            _ => return None,
        };

        Some(key)
    }

    /// Convert this modifier key into a logical modifier flag.
    pub fn as_modifier(&self) -> Modifier {
        use ModifierKey::*;

        match *self {
            LeftShift => Modifier::Shift,
            LeftControl => Modifier::Control,
            LeftOption => Modifier::Option,
            LeftCommand => Modifier::Command,
            LeftExtra1 => Modifier::Extra1,
            LeftExtra2 => Modifier::Extra2,
            RightShift => Modifier::Shift,
            RightControl => Modifier::Control,
            RightOption => Modifier::Option,
            RightCommand => Modifier::Command,
            RightExtra1 => Modifier::Extra1,
            RightExtra2 => Modifier::Extra2,
        }
    }
}

/// An event to indicate a change in terminal size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowEvent {
    pub columns: u16,
    pub rows: u16,
}

/// An event to indicate a cursor position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CursorEvent {
    pub column: u16,
    pub row: u16,
}

/// An event to indicate a concrete color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorEvent {
    pub kind: RequestColor,
    /// The value has three components, with each component comprising a
    /// magnitude and a maximum value (which is either 0xff, 0xffff, 0xffffff,
    /// or 0xffff_ffff).
    pub value: [(u16, u16); 3],
}
