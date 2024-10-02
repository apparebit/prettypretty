//! Helper functionality for consuming ANSI escape sequences.
//!
//! This module provides the low-level [`Scanner`] interface for reading ANSI
//! escape sequences from a terminal's input without leaving the input stream in
//! an ill-defined state upon errors.
//!
//!
//! # Prelude
//!
//! To query a terminal for its color theme, the terminal integration first puts
//! the terminal into cbreak or raw mode. It then iterates over
//! [`ThemeEntry::all`](crate::trans::ThemeEntry::all) to query the terminal for
//! all 18 theme colors, i.e., the default foreground and background colors
//! followed by the 16 ANSI colors.
//!
//!
//! # Example #1: Processing Byte by Byte
//!
//! The example code below illustrates the use of [`Scanner::process`] and
//! acting on its continuation result.
//!
//! ```
//! # use prettypretty::{Color, error::ColorFormatError};
//! # use prettypretty::escape::{Continuation, Scanner};
//! # use prettypretty::style::AnsiColor;
//! # use prettypretty::trans::ThemeEntry;
//! # fn the_trial() -> Result<Color, ColorFormatError> {
//! // Writing `format!("{}", entry)` to the terminal issues the query.
//! let entry = ThemeEntry::Ansi(AnsiColor::Red);
//!
//! // Here are the bytes of what might be the response.
//! let mut terminal_input = b"\x1b]4;1;rgb:df/28/27\x07".iter();
//!
//! let mut scanner = Scanner::new();
//! loop {
//!     // Read next byte and feed it to Scanner::process.
//!     let byte = *terminal_input.next().unwrap();
//!     match scanner.process(byte) {
//!
//!         // On continue, keep reading and feeding...
//!         Continuation::Continue => (),
//!
//!         // On abort, return error...
//!         Continuation::Abort => {
//!             return Err(ColorFormatError::MalformedThemeColor);
//!         }
//!
//!         // On complete, parse payload as color
//!         Continuation::Complete => {
//!             let payload = scanner
//!                 .completed_string()
//!                 .or(Err(ColorFormatError::MalformedThemeColor))?;
//!             return entry.parse_response(payload);
//!         }
//!     }
//! }
//! # }
//! # fn main() {
//! #     let result = the_trial().unwrap();
//! #     assert_eq!(result, Color::from_24bit(0xdf, 0x28, 0x27));
//! # }
//! ```
//! <div class=color-swatch>
//! <div style="background-color: #df2827;"></div>
//! </div>
//! <br>
//!
//! As shown in the example, the result of [`Scanner::process`] determines what
//! a caller should do next:
//!
//!   * As long as [`Scanner::process`] returns [`Continuation::Continue`], a
//!     terminal integration should keep reading individual bytes from terminal
//!     input and passing them to scanner by invoking `process` again.
//!   * If the result is [`Continuation::Abort`], the ANSI escape sequence was
//!     malformed. The caller may signal an error or immediately try to read
//!     another escape sequence.
//!   * If the result is [`Continuation::Complete`], the ANSI escape sequence is
//!     complete. The caller can use [`Scanner::completed_control`],
//!     [`Scanner::completed_bytes`], and [`Scanner::completed_string`] to
//!     access control and payload of the ANSI escape sequence.
//!
//! The continuation processing inside the loop is fairly boilerplaty, which
//! suggests an opportunity for further abstraction.
//!
//!
//! # Example #2: Processing the Entire Escape Sequence
//!
//! Indeed, as shown below, the entire loop can be replaced with an invocation
//! of [`Scanner::run_to_string`]. While more concise, this method does impose
//! use of `std::io::Error` as the error type. [`Scanner::run_to_bytes`]
//! provides the same functionality, except it returns the parsed bytes without
//! conversion to a string slice.
//!
//! ```
//! # use std::io::ErrorKind;
//! # use prettypretty::Color;
//! # use prettypretty::escape::{Continuation, Scanner};
//! # use prettypretty::style::AnsiColor;
//! # use prettypretty::trans::ThemeEntry;
//! # fn the_trial() -> std::io::Result<Color> {
//! // As before for theme entry, terminal input, and scanner:
//! let entry = ThemeEntry::Ansi(AnsiColor::Red);
//! let mut terminal_input = b"\x1b]4;1;rgb:df/28/27\x07".iter();
//! let mut scanner = Scanner::new();
//!
//! // Move the (faux) terminal input into the closure.
//! let payload = scanner.run_to_string(
//!     move || Ok(*terminal_input.next().unwrap()))?;
//!
//! // Use error payload to carry more specific error.
//! return entry
//!     .parse_response(payload)
//!     .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e));
//! # }
//! # fn main() {
//! #     let result = the_trial().unwrap();
//! #     assert_eq!(result, Color::from_24bit(0xdf, 0x28, 0x27));
//! # }
//! ```
//! <div class=color-swatch>
//! <div style="background-color: #df2827;"></div>
//! </div>
//! <br>
//!
//! In addition to parsing escape sequences as just illustrated, the terminal
//! integration also needs to correctly handle all error conditions, which may
//! add complexity.
//!
//!
//! # Error Handling #1: When Abort Consumes a Byte
//!
//! [`Scanner`] tries to read just the bytes of an ANSI escape sequence from the
//! input, no more, no less, and thereby avoid interaction with subsequent
//! content. By precisely modelling a terminal's state machine when consuming
//! the input byte by byte, the implementation achieves that for well-formed
//! ANSI escape sequences. However, that is impossible when the input byte
//! starts a new escape sequence and thereby aborts parsing of the current one.
//!
//! Still, that doesn't pose a problem—as long as the application keeps reading
//! ANSI escape sequences from the input with the same scanner.
//!
//! However, if an application also consumes terminal input in some other way,
//! it effectively needs to put the extra byte back into the input stream. It
//! also should reset the scanner before using it again. Unfortunately,
//! prettypretty can't really help with putting the byte back, since the best
//! strategy for doing so depends on the application. But it can help with
//! detection of such troublesome bytes—if [`Scanner::process`] returns
//! [`Continuation::Abort`] and [`Control::is_sequence_start`] returns `true`
//! for `process`' input. When using [`Scanner::run_to_bytes`] or
//! [`Scanner::run_to_string`], [`Scanner::last_byte`] exposes the most recent
//! byte passed to `process`.
//!
//!
//! # Error Handling #2: Buffer Overflow
//!
//! [`Scanner`] buffers the payload of an ANSI escape sequence. Since its input
//! may come from untrusted sources, the implementation limits the buffer's
//! capacity and does *not* adjust it, even when running out of space. That is
//! not a problem when querying a terminal for its colors because the buffer is
//! correctly dimensioned for this use case (with a capacity of only 23 bytes).
//! That may, however, pose a problem when parsing other ANSI escape sequences.
//! An application can detect this error condition with
//! [`Scanner::did_overflow`]. It can also increase the capacity of a new
//! scanner with [`Scanner::with_capacity`] in Rust or an explicit capacity
//! argument for the constructor in Python.

use std::io::{Error, ErrorKind};

#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

/// Controls used in ANSI escape sequences.
///
/// Variants other than [`Control::BEL`], [`Control::ESC`], and [`Control::ST`]
/// represent controls that start a ANSI escape sequence, with [`Control::CSI`]
/// the most common one.
///
/// [`Control::ESC`] is a plain C0 escape character by itself. If followed by a
/// suitable ASCII character, it may still turn into one of the other controls.
///
/// [`Control::BEL`] and [`Control::ST`] are used to terminate ANSI escape
/// sequences started with [`Control::APC`], [`Control::DCS`], [`Control::OSC`],
/// [`Control::PM`], and [`Control::SOS`]. Only [`Control::CSI`] is terminated
/// by regular ASCII characters.
///
/// Displaying an instance writes out the corresponding control.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.escape")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Control {
    /// Escape (C0).
    ESC,

    /// Application Program Command: ESC _ (C0) or 0x9f (C1).
    APC,

    /// Control Sequence Introducer: ESC [ (C0) or 0x9b (C1).
    CSI,

    /// Device control string: ESC P (C0) or 0x90 (C1).
    DCS,

    /// Operating System Command: ESC ] (C0) or 0x9d (C1).
    OSC,

    /// Privacy Message: ESC ^ (C0) or 0x9e (C1).
    PM,

    /// Start of String: ESC X (C0) or 0x98 (C1).
    SOS,

    /// Bell (C0).
    BEL,

    /// String Terminator: ESC \ (C0) or 0x9c (C1).
    ST,
}

impl Control {
    /// Determine whether the given byte starts an ANSI escape sequence.
    #[inline]
    pub fn is_sequence_start(byte: u8) -> bool {
        matches!(byte, 0x1b | 0x90 | 0x98 | 0x9b | 0x9d | 0x9e | 0x9f)
    }
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Control {
    /// Determine whether the given byte starts an ANSI escape sequence.
    #[cfg(feature = "pyffi")]
    #[pyo3(name = "is_sequence_start")]
    #[staticmethod]
    pub fn py_is_sequence_start(byte: u8) -> bool {
        Self::is_sequence_start(byte)
    }

    /// Determine whether this control is the plain escape function.
    #[inline]
    pub fn is_escape(&self) -> bool {
        matches!(self, Self::ESC)
    }

    /// Determine whether this control terminates ANSI escape sequences.
    #[inline]
    pub fn is_terminator(&self) -> bool {
        matches!(self, Self::BEL | Self::ST)
    }

    /// Determine whether this control starts an ANSI escape sequence.
    #[inline]
    pub fn is_function(&self) -> bool {
        !matches!(self, Self::BEL | Self::ESC | Self::ST)
    }

    /// Render a debug representation for this control. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    /// Render the corresponding 7-bit control. <i class=python-only>Python
    /// only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __str__(&self) -> String {
        format!("{}", self)
    }
}

impl std::fmt::Display for Control {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Control::*;

        match self {
            ESC => f.write_str("\x1b"),
            APC => f.write_str("\x1b_"),
            CSI => f.write_str("\x1b["),
            DCS => f.write_str("\x1bP"),
            OSC => f.write_str("\x1b]"),
            PM => f.write_str("\x1b^"),
            SOS => f.write_str("\x1b_"),
            BEL => f.write_str("\x07"),
            ST => f.write_str("\x1b\\"),
        }
    }
}

/// The current state when processing terminal I/O.
#[derive(Copy, Clone, Debug)]
enum State {
    Ground,
    Escape,
    EscapeIntermediate,
    ApcString,
    ApcEnd,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    CsiIgnore,
    DcsEntry,
    DcsParam,
    DcsIntermediate,
    DcsPassthrough,
    DcsPassthroughEnd,
    DcsIgnore,
    DcsIgnoreEnd,
    OscString,
    OscEnd,
    PmString,
    PmEnd,
    SosString,
    SosEnd,
}

impl State {
    // /// Determine whether this is the ground state.
    // pub fn is_ground(&self) -> bool {
    //     matches!(self, Self::Ground)
    // }

    // /// Determine whether this is the escape state.
    // pub fn is_escape(&self) -> bool {
    //     matches!(self, Self::Escape)
    // }

    /// Determine the number of steps into parsing an escape sequence
    /// corresponding to this state.
    ///
    /// This method treats `Ground` as step 0, `Escape` as step 1, and all other
    /// states as step 2.
    pub fn step(&self) -> usize {
        match self {
            Self::Ground => 0,
            Self::Escape => 1,
            _ => 2,
        }
    }

    // /// Determine the kind of escape sequence currently being parsed.
    // ///
    // /// Unless this state is the ground state, this method returns a [`Control`]
    // /// other than `BEL` or `ST`.
    // pub fn control(&self) -> Option<Control> {
    //     use State::*;
    //     match self {
    //         Ground => None,
    //         Escape | EscapeIntermediate => Some(Control::ESC),
    //         ApcString | ApcEnd => Some(Control::APC),
    //         CsiEntry | CsiParam | CsiIntermediate | CsiIgnore => Some(Control::CSI),
    //         DcsEntry | DcsParam | DcsIntermediate | DcsPassthrough | DcsPassthroughEnd
    //         | DcsIgnore | DcsIgnoreEnd => Some(Control::DCS),
    //         OscString | OscEnd => Some(Control::OSC),
    //         PmString | PmEnd => Some(Control::PM),
    //         SosString | SosEnd => Some(Control::SOS),
    //     }
    // }
}

/// An action when processing terminal I/O.
#[derive(Copy, Clone, Debug)]
enum Action {
    /// Print current byte.
    ///
    /// By simply passing through bytes that are not ANSI escape sequences,
    /// UTF-8 encoded text is also passed through.
    Print,

    /// Start a new escape sequence.
    ///
    /// This action replaces `clear` in the original state machine
    /// specification.
    Start,

    /// Ignore the current byte, even if in the middle of an ANSI escape
    /// sequence.
    Ignore,

    /// Execute the byte, even in the middle of an escape sequence.
    ///
    /// Like [`Action::Ignore`], this action does *not* retain the current byte.
    Execute,

    /// Make the current byte part of the escape sequence being recognized.
    ///
    /// Instead of directly matching this action, use [`Action::is_retained`] to
    /// determine whether the current byte should be retained, since
    /// [`Action:DispatchCsi`] and [`Action::DispatchEsc`] are retained, too.
    ///
    /// This action replaces the `collect`, `osc_put`, `param`, and `put`
    /// actions in [Williams' state machine
    /// specification](https://vt100.net/emu/dec_ansi_parser).
    Retain,

    /// Dispatch an APC escape sequence.
    ///
    /// [Williams' state machine
    /// specification](https://vt100.net/emu/dec_ansi_parser) has no equivalent
    /// and silently ignores APC escape sequences.
    DispatchApc,

    /// Dispatch a CSI escape sequence.
    ///
    /// Like [`Action::Retain`], this action requires that the current byte be
    /// retained before dispatching the CSI sequence. Hence
    /// [`Action::is_retained`] returns `true` for this action, too.
    DispatchCsi,

    /// Dispatch a DCS escape sequence.
    ///
    /// Instead of dispatching DCS escape sequences upon completion, [Williams'
    /// state machine specification](https://vt100.net/emu/dec_ansi_parser) has
    /// a dynamic hook mechanism.
    DispatchDcs,

    /// Dispatch a plain escape sequence.
    ///
    /// Like [`Action::Retain`], this action requires that the current byte be
    /// retained before dispatching the CSI sequence. Hence
    /// [`Action::is_retained`] returns `true` for this action, too.
    DispatchEsc,

    /// Dispatch an OSC escape sequence.
    ///
    /// Instead of dispatching OSC escape sequences upon completion, [Williams'
    /// state machine specification](https://vt100.net/emu/dec_ansi_parser)
    /// simply passed on the payload.
    DispatchOsc,

    /// Dispatch a PM escape sequence.
    ///
    /// [Williams' state machine
    /// specification](https://vt100.net/emu/dec_ansi_parser) has no equivalent
    /// and silently ignores PM escape sequences.
    DispatchPm,

    /// Dispatch an SOS escape sequence.
    ///
    /// [Williams' state machine
    /// specification](https://vt100.net/emu/dec_ansi_parser) has no equivalent
    /// and silently ignores SOS escape sequences.
    DispatchSos,
}

impl Action {
    // /// Determine whether this action is [`Action::Print`].
    // pub fn is_print(&self) -> bool {
    //     matches!(self, Self::Print)
    // }

    /// Determine whether the action is [`Action::Start`].
    #[inline]
    pub fn is_start(&self) -> bool {
        matches!(self, Self::Start)
    }

    /// Determine whether this action requires retaining the current byte.
    ///
    /// This method returns `true` if the current byte belongs to the ANSI
    /// escape sequence currently being recognized. That is the case for
    /// [`Action::Retain`], [`Action::DispatchCsi`], and
    /// [`Action::DispatchEsc`]. When buffering retained bytes, the current byte
    /// must be added to the buffer before dispatching the latter two actions.
    #[inline]
    pub fn is_retained(&self) -> bool {
        matches!(self, Self::Retain | Self::DispatchCsi | Self::DispatchEsc)
    }

    /// Determine whether the action is the dispatch action.
    #[inline]
    pub fn is_dispatch(&self) -> bool {
        use Action::*;

        matches!(
            self,
            DispatchApc
                | DispatchCsi
                | DispatchDcs
                | DispatchEsc
                | DispatchOsc
                | DispatchPm
                | DispatchSos
        )
    }

    /// Determine the kind of escape sequence dispatched by this action.
    ///
    /// If this action is not a dispatch, this method returns `None`. Otherwise,
    /// it returns a control other than [`Control::BEL`] and [`Control::ST`].
    #[inline]
    pub fn control(&self) -> Option<Control> {
        use Action::*;
        use Control::*;

        match self {
            DispatchApc => Some(APC),
            DispatchCsi => Some(CSI),
            DispatchDcs => Some(DCS),
            DispatchEsc => Some(ESC),
            DispatchOsc => Some(OSC),
            DispatchPm => Some(PM),
            DispatchSos => Some(SOS),
            _ => None,
        }
    }
}

// ================================================================================================

const fn otherwise(b: u8, state: State) -> (State, Action) {
    use Action::*;
    use State::*;

    match b {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (state, Execute),
        0x18 | 0x1a => (Ground, Execute),
        0x1b => (Escape, Start),
        0x20..=0x7e => (state, Ignore),
        0x7f => (state, Ignore),
        0x80..=0x8f | 0x91..=0x97 | 0x99 | 0x9a => (Ground, Execute),
        0x90 => (DcsEntry, Start),
        0x98 => (SosString, Start),
        0x9b => (CsiEntry, Start),
        0x9c => (Ground, Ignore),
        0x9d => (OscString, Start),
        0x9e => (PmString, Start),
        0x9f => (ApcString, Start),
        _ => (state, Ignore),
    }
}

// ------------------------------------------------------------------------------------------------
// Ground

const fn ground(b: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match b {
        0x20..=0x7f => (Ground, Print),
        _ => otherwise(b, Ground),
    }
}

// ------------------------------------------------------------------------------------------------
// Escape

const fn escape(b: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match b {
        0x20..=0x2f => (EscapeIntermediate, Retain),
        0x30..=0x4f | 0x51..=0x57 | 0x59 | 0x5a | 0x5c | 0x60..=0x7e => (Ground, DispatchEsc),
        0x50 => (DcsEntry, Start),
        0x58 => (SosString, Start),
        0x5b => (CsiEntry, Start),
        0x5d => (OscString, Start),
        0x5e => (PmString, Start),
        0x5f => (ApcString, Start),
        _ => otherwise(b, Escape),
    }
}

const fn escape_intermediate(b: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match b {
        0x20..=0x2f => (EscapeIntermediate, Retain),
        0x30..=0x7e => (Ground, DispatchEsc),
        _ => otherwise(b, EscapeIntermediate),
    }
}

// ------------------------------------------------------------------------------------------------
// SOS, PM, APC

macro_rules! string_command {
    ($name1:ident($state1:ident) => $name2:ident($state2:ident) => $action:expr) => {
        const fn $name1(b: u8) -> (State, Action) {
            use Action::*;
            use State::*;

            match b {
                0x00..=0x06 | 0x08..=0x17 | 0x19 | 0x1c..=0x1f => ($state1, Ignore),
                0x07 => (Ground, $action),
                0x1b => ($state2, Ignore),
                0x20..=0x7f => ($state1, Ignore),
                0x9c => (Ground, $action),
                _ => otherwise(b, $state1),
            }
        }

        const fn $name2(b: u8) -> (State, Action) {
            use Action::*;
            use State::*;

            match b {
                0x5c => (Ground, $action),
                _ => otherwise(b, Ground),
            }
        }
    };
}

string_command!(sos_string(SosString) => sos_end(SosEnd) => DispatchSos);
string_command!(pm_string(PmString) => pm_end(PmEnd) => DispatchPm);
string_command!(apc_string(ApcString) => apc_end(ApcEnd) => DispatchApc);

// ------------------------------------------------------------------------------------------------
// OSC

const fn osc_string(b: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match b {
        0x00..=0x06 | 0x08..=0x17 | 0x19 | 0x1c..=0x1f => (OscString, Ignore),
        0x07 => (Ground, DispatchOsc),
        0x1b => (OscEnd, Ignore),
        0x20..=0x7f => (OscString, Retain),
        0x9c => (Ground, DispatchOsc),
        _ => otherwise(b, OscString),
    }
}

const fn osc_end(b: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match b {
        0x5c => (Ground, DispatchOsc),
        _ => otherwise(b, Ground),
    }
}

// ------------------------------------------------------------------------------------------------
// CSI

const fn csi_entry(b: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match b {
        0x20..=0x2f => (CsiIntermediate, Retain),
        0x30..=0x39 | 0x3b..=0x3f => (CsiParam, Retain),
        0x3a => (CsiIgnore, Ignore),
        0x40..=0x7e => (Ground, DispatchCsi),
        _ => otherwise(b, CsiEntry),
    }
}

const fn csi_param(b: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match b {
        0x20..=0x2f => (CsiIntermediate, Retain),
        0x30..=0x39 | 0x3b => (CsiParam, Retain),
        0x3a | 0x3c..=0x3f => (CsiIgnore, Ignore),
        0x40..=0x7e => (Ground, DispatchCsi),
        _ => otherwise(b, CsiParam),
    }
}

const fn csi_intermediate(b: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match b {
        0x20..=0x2f => (CsiIntermediate, Retain),
        0x30..=0x3f => (CsiIgnore, Ignore),
        0x40..=0x7e => (Ground, DispatchCsi),
        _ => otherwise(b, CsiIntermediate),
    }
}

const fn csi_ignore(b: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match b {
        0x20..=0x3f => (CsiIgnore, Ignore),
        0x40..=0x7e => (Ground, Ignore),
        _ => otherwise(b, CsiIgnore),
    }
}

// ------------------------------------------------------------------------------------------------
// DCS

const fn dcs_entry(b: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match b {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (DcsEntry, Ignore),
        0x20..=0x2f => (DcsIntermediate, Retain),
        0x30..=0x39 | 0x3b..=0x3f => (DcsParam, Retain),
        0x3a => (DcsIgnore, Ignore),
        0x40..=0x7e => (DcsPassthrough, Retain),
        _ => otherwise(b, DcsEntry),
    }
}

const fn dcs_param(b: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match b {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (DcsParam, Ignore),
        0x20..=0x2f => (DcsIntermediate, Retain),
        0x30..=0x39 | 0x3b => (DcsParam, Retain),
        0x3a | 0x3c..=0x3f => (DcsIgnore, Ignore),
        0x40..=0x7e => (DcsPassthrough, Retain),
        _ => otherwise(b, DcsParam),
    }
}

const fn dcs_intermediate(b: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match b {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (DcsIntermediate, Ignore),
        0x20..=0x2f => (DcsIntermediate, Retain),
        0x30..=0x3f => (DcsIgnore, Ignore),
        0x40..=0x7e => (DcsPassthrough, Retain),
        _ => otherwise(b, DcsIntermediate),
    }
}

const fn dcs_passthrough(b: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match b {
        0x00..=0x06 | 0x08..=0x17 | 0x19 | 0x1c..=0x1f => (DcsPassthrough, Retain),
        0x07 => (Ground, DispatchDcs),
        0x1b => (DcsPassthroughEnd, Ignore),
        0x20..=0x7e => (DcsPassthrough, Retain),
        0x9c => (Ground, DispatchDcs),
        _ => otherwise(b, DcsPassthrough),
    }
}

const fn dcs_passthrough_end(b: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match b {
        0x5c => (Ground, DispatchDcs),
        _ => otherwise(b, Ground),
    }
}

const fn dcs_ignore(b: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match b {
        0x00..=0x06 | 0x08..=0x17 | 0x19 | 0x1c..=0x1f => (DcsIgnore, Ignore),
        0x07 => (Ground, Ignore),
        0x1b => (DcsIgnoreEnd, Ignore),
        0x20..=0x7f => (DcsIgnore, Ignore),
        0x9c => (Ground, Ignore),
        _ => otherwise(b, DcsIgnore),
    }
}

const fn dcs_ignore_end(b: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match b {
        0x5c => (Ground, Ignore),
        _ => otherwise(b, Ground),
    }
}

// ------------------------------------------------------------------------------------------------
// Complete transition function

const fn transition(state: State, byte: u8) -> (State, Action) {
    use State::*;

    match state {
        Ground => ground(byte),
        Escape => escape(byte),
        EscapeIntermediate => escape_intermediate(byte),
        SosString => sos_string(byte),
        SosEnd => sos_end(byte),
        PmString => pm_string(byte),
        PmEnd => pm_end(byte),
        ApcString => apc_string(byte),
        ApcEnd => apc_end(byte),
        OscString => osc_string(byte),
        OscEnd => osc_end(byte),
        CsiEntry => csi_entry(byte),
        CsiParam => csi_param(byte),
        CsiIntermediate => csi_intermediate(byte),
        CsiIgnore => csi_ignore(byte),
        DcsEntry => dcs_entry(byte),
        DcsParam => dcs_param(byte),
        DcsIntermediate => dcs_intermediate(byte),
        DcsPassthrough => dcs_passthrough(byte),
        DcsPassthroughEnd => dcs_passthrough_end(byte),
        DcsIgnore => dcs_ignore(byte),
        DcsIgnoreEnd => dcs_ignore_end(byte),
    }
}

// ================================================================================================

/// A state machine for parsing ANSI escape sequences.
///
/// This struct implements a state machine for recognizing ANSI escape
/// sequences. Like the Alacritty's [vte](https://github.com/alacritty/vte) and
/// Wezterm's [vtparse](https://github.com/wez/wezterm) crates, the
/// implementation is based on Paul Flo Williams' [parser for DEC's
/// ANSI-compatible video terminals](https://vt100.net/emu/dec_ansi_parser).
/// Unlike these two crates and Williams' original, this version has been
/// streamlined to be simpler and more uniform. Hence, it does not distinguish
/// between entering, remaining in, and exiting a state, instead only featuring
/// transitions from state to state (which may be the same). Furthermore, it
/// replace the original's `collect`, `osc_put`, `param`, and `put` actions for
/// handling the current byte with [`Action::Retain`].
#[derive(Debug)]
struct StateMachine {
    previous_state: State,
    state: State,
    buffer: Vec<u8>,
    did_overflow: bool,
    last_byte: u8,
    last_action: Action,
}

impl StateMachine {
    /// Create a new buffering state machine with default capacity.
    #[inline]
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Create a new buffering state machine with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            previous_state: State::Ground,
            state: State::Ground,
            buffer: Vec::with_capacity(capacity),
            did_overflow: false,
            last_byte: 0,
            last_action: Action::Ignore,
        }
    }
}

impl Default for StateMachine {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachine {
    /// Reset this state machine.
    pub fn reset(&mut self) {
        self.previous_state = State::Ground;
        self.state = State::Ground;
        self.buffer.clear();
        self.did_overflow = false;
        self.last_byte = 0;
        self.last_action = Action::Ignore;
    }

    // /// Determine this state machine's internal buffer capacity.
    // pub fn capacity(&self) -> usize {
    //     self.buffer.capacity()
    // }

    /// Process the given byte.
    pub fn process(&mut self, byte: u8) -> Action {
        let byte = if 0xa0 <= byte { byte - 0x80 } else { byte };
        let (state, action) = transition(self.state, byte);

        if action.is_start() {
            self.buffer.clear();
            self.did_overflow = false;
        } else if action.is_retained() {
            if self.buffer.len() < self.buffer.capacity() {
                self.buffer.push(byte);
            } else {
                self.did_overflow = true;
            }
        }

        self.previous_state = self.state;
        self.state = state;
        self.last_byte = byte;
        self.last_action = action;

        action
    }

    /// Determine the most recently processed byte.
    #[inline]
    pub fn last_byte(&self) -> u8 {
        self.last_byte
    }

    // /// Determine whether the internal buffer is empty.
    // pub fn is_empty(&self) -> bool {
    //     self.buffer.is_empty()
    // }

    // /// Determine the number of bytes buffered for the current escape sequence.
    // pub fn len(&self) -> usize {
    //     self.buffer.len()
    // }

    /// Determine whether the internal buffer did overflow.
    #[inline]
    pub fn did_overflow(&self) -> bool {
        self.did_overflow
    }

    /// Determine whether the last byte aborted an ANSI escape sequence.
    pub fn did_abort(&self) -> bool {
        let previous = self.previous_state.step();
        let current = self.state.step();

        if current < previous {
            // Stepping back without dispatch implies an abort.
            !self.last_action.is_dispatch()
        } else if current == 2 && previous == 2 {
            // With both states at level 2, the start action implies an abort.
            self.last_action.is_start()
        } else {
            false
        }
    }

    /// Determine whether the last byte completed an ANSI escape sequence.
    #[inline]
    pub fn did_complete(&self) -> bool {
        self.last_action.is_dispatch()
    }

    /// Determine the control for the just completed ANSI escape sequence.
    ///
    /// If [`StateMachine::did_complete`], this method returns the corresponding
    /// control. Otherwise, it returns `None`.
    #[inline]
    pub fn completed_control(&self) -> Option<Control> {
        self.last_action.control()
    }

    /// Access the payload for the just completed ANSI escape sequence.
    ///
    /// If [`StateMachine::did_complete`], this method returns the payload of
    /// the corresponding ANSI escape sequence.
    #[inline]
    pub fn completed_bytes(&self) -> &[u8] {
        if self.did_complete() {
            &self.buffer
        } else {
            &[]
        }
    }

    /// Access the payload for the just completed ANSI escape sequence as a
    /// string slice.
    ///
    /// If [`StateMachine::did_complete`], this method returns the payload of
    /// the corresponding ANSI escape sequence as a string slice.
    #[inline]
    pub fn completed_string(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.completed_bytes())
    }
}

// ------------------------------------------------------------------------------------------------

/// An enumeration of continuation options.
///
/// [`Scanner::process`] returns this variant to instruct the caller of how to
/// progress.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.escape")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Continuation {
    /// Abort scanning a malformed escape sequence.
    ///
    /// [`Scanner::process`] returns this continuation only after consuming all
    /// bytes belonging to the malformed escape sequence. Hence, it is possible
    /// to try reading another escape sequence from the same source right away.
    Abort,

    /// Continue scanning the escape sequence.
    ///
    /// [`Scanner::process`] returns this continuation as long as it requires
    /// another byte to scan the escape sequence. The caller should retrieve
    /// another byte from the source and invoke the `process` method with it.
    Continue,

    /// Complete the scanned escape sequence.
    ///
    /// [`Scanner::process`] returns this continuation when it has successfully
    /// scanned an entire escape sequence. The caller should invoke
    /// [`Scanner::completed_bytes`] or [`Scanner::completed_string`] to consume
    /// the payload of the escape sequence. It can also use
    /// [`Scanner::completed_control`] to inquire about the kind of escape
    /// sequence.
    Complete,
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Continuation {
    /// Determine whether this continuation is abort.
    #[inline]
    pub fn is_abort(&self) -> bool {
        matches!(self, Self::Abort)
    }

    /// Determine whether this continuation is continue.
    #[inline]
    pub fn is_continue(&self) -> bool {
        matches!(self, Self::Continue)
    }

    /// Determine whether this continuation is consume.
    #[inline]
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }
}

/// A scanner for escape sequences.
///
/// The module documentation for [`escape`](crate::escape) explains the use of
/// this type, providing code examples and elaborating on some of the finer
/// points of error handling.
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.escape"))]
#[derive(Debug)]
pub struct Scanner {
    machine: StateMachine,
}

impl Scanner {
    /// The scanner's default capacity.
    pub const DEFAULT_CAPACITY: usize = 23;

    /// Create a new escape sequence scanner. The scanner's capacity is set just
    /// large enough for parsing responses to color queries.
    pub fn new() -> Self {
        Self::with_capacity(Self::DEFAULT_CAPACITY)
    }

    /// Create a new escape sequence scanner with the given capacity. Since the
    /// input to a scanner cannot be trusted, the scanner's capacity is fixed
    /// and does not grow. But it also determines what escape sequences can be
    /// parsed with the scanner, since buffer overflows result in abort
    /// continuations.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            machine: StateMachine::with_capacity(capacity),
        }
    }
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Scanner {
    /// Create a new escape sequence scanner. <i class=python-only>Python
    /// only!</i>
    ///
    /// The default capacity is just enough to parse OSC sequences with theme
    /// colors.
    #[cfg(feature = "pyffi")]
    #[new]
    #[pyo3(signature = (capacity=Scanner::DEFAULT_CAPACITY))]
    pub fn py_new(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }

    /// Reset this escape sequence scanner.
    #[inline]
    pub fn reset(&mut self) {
        self.machine.reset()
    }

    /// Process the given byte and return the continuation.
    pub fn process(&mut self, byte: u8) -> Continuation {
        self.machine.process(byte);

        if self.machine.did_complete() {
            Continuation::Complete
        } else if self.machine.did_abort() {
            Continuation::Abort
        } else {
            Continuation::Continue
        }
    }

    /// Determine the most recently processed byte.
    #[inline]
    pub fn last_byte(&self) -> u8 {
        self.machine.last_byte()
    }

    /// Determine whether the internal buffer did overflow.
    #[inline]
    pub fn did_overflow(&self) -> bool {
        self.machine.did_overflow()
    }

    /// Determine the control leading the scanned escape sequence.
    ///
    /// If the continuation is [`Continuation::Complete`], this method returns
    /// the control that started the scanned escape sequence.
    #[inline]
    pub fn completed_control(&self) -> Option<Control> {
        self.machine.completed_control()
    }

    /// Access the scanned ANSI escape sequence.
    ///
    /// If [`Scanner::process`] returned [`Continuation::Complete`], this method
    /// returns the payload without leading control and, for escape sequences
    /// other than CSI or ESC, trailing control. Otherwise, this method returns
    /// an empty slice.
    ///
    /// If [`Scanner::did_overflow`] returns `true`, the internal buffer did not
    /// have sufficient capacity to store all bytes of the ANSI escape sequence.
    ///
    /// [`Scanner::completed_string`] returns the same data as string slice.
    /// [`Scanner::completed_control`] returns the leading control.
    #[inline]
    pub fn completed_bytes(&self) -> &[u8] {
        self.machine.completed_bytes()
    }

    /// Access the scanned ANSI escape sequence.
    ///
    /// If [`Scanner::process`] returned [`Continuation::Complete`], this method
    /// returns the payload without leading control and, for escape sequences
    /// other than CSI or ESC, trailing control. Otherwise, this method returns
    /// an empty slice.
    ///
    /// If [`Scanner::did_overflow`] returns `true`, the internal buffer did not
    /// have sufficient capacity to store all bytes of the ANSI escape sequence.
    ///
    /// [`Scanner::completed_bytes`] returns the same data as a byte slice.
    /// [`Scanner::completed_control`] returns the leading control.
    #[inline]
    pub fn completed_string(&self) -> Result<&str, std::str::Utf8Error> {
        self.machine.completed_string()
    }

    /// Get a debug representation for this scanner. <i class=python-only>Python
    /// only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("Scanner({:?})", self.machine)
    }
}

impl Scanner {
    /// Scan an ANSI escape sequence to completion and return the payload's
    /// bytes. <i class=rust-only>Rust only!</i>
    ///
    /// This method repeatedly calls [`Scanner::process`] on the result of
    /// `read` until the returned continuation is not [`Continuation::Continue`]
    /// anymore and, unless scanning has been aborted, returns the result of
    /// [`Scanner::completed_bytes`]. While convenient, this method does force
    /// the error type to `std::io::Error`.
    ///
    /// [`Scanner::did_overflow`] and [`Scanner::completed_control`] can still
    /// be called after this method has returned. Furthermore, the last
    /// processed byte can be accessed through [`Scanner::last_byte`].
    pub fn run_to_bytes<F>(&mut self, mut read: F) -> std::io::Result<&[u8]>
    where
        F: FnMut() -> std::io::Result<u8>,
    {
        loop {
            let byte = read()?;
            match self.process(byte) {
                Continuation::Continue => (),
                Continuation::Abort => return Err(ErrorKind::InvalidData.into()),
                Continuation::Complete => return Ok(self.completed_bytes()),
            }
        }
    }

    /// Scan an ANSI escape sequence to completion and return the payload's
    /// string slice. <i class=rust-only>Rust only!</i>
    ///
    /// This method repeatedly calls [`Scanner::process`] on the result of
    /// `read` until the returned continuation is not [`Continuation::Continue`]
    /// anymore and, unless scanning has been aborted, returns the result of
    /// [`Scanner::completed_string`]. While convenient, this method does force
    /// the error type to `std::io::Error`, including for the conversion from a
    /// byte slice to a UTF-8 string.
    ///
    /// [`Scanner::did_overflow`] and [`Scanner::completed_control`] can still
    /// be called after this method has returned. Furthermore, the last
    /// processed byte can be accessed through [`Scanner::last_byte`].
    pub fn run_to_string<F>(&mut self, read: F) -> std::io::Result<&str>
    where
        F: FnMut() -> std::io::Result<u8>,
    {
        let bytes = self.run_to_bytes(read)?;
        std::str::from_utf8(bytes).map_err(|e| Error::new(ErrorKind::InvalidData, e))
    }
}

// ================================================================================================

#[cfg(test)]
mod test {
    use super::{Action, State};
    use std::mem::size_of;

    #[test]
    fn test() {
        assert_eq!(size_of::<(State, Action)>(), 2);
    }
}
