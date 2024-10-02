//! Processing terminal queries and responses.
//!
//! This module facilitates integration with terminal I/O without implementing
//! I/O. In particular, [`VtScanner`] represents the state machine for
//! recognizing ANSI escape sequences, including DEC's extensions. It is best
//! combined with the [`trans`](crate::trans) module's
//! [`ThemeEntry`](crate::trans::ThemeEntry) for representing the 18 colors in a
//! theme, issueing queries, and parsing responses.
//!
//!
//! # Prelude
//!
//! Before interrogating the terminal, an application must put the terminal into
//! cbreak or raw mode. It then iterates over
//! [`ThemeEntry::all`](crate::trans::ThemeEntry::all) and queries the terminal
//! for the corresponding color for each of the 18 theme colors, i.e., the
//! default foreground and background colors as well as the 16 ANSI colors.
//!
//!
//! # Example #1: Bytewise Steps
//!
//! The first example illustrates the use of
//! [`ThemeEntry`](crate::trans::ThemeEntry) and [`VtScanner`]
//! for determining a theme color with byte by byte steps.
//!
//! ```
//! # use prettypretty::{Color, error::ColorFormatError};
//! # use prettypretty::escape::VtScanner;
//! # use prettypretty::style::AnsiColor;
//! # use prettypretty::trans::ThemeEntry;
//! # fn the_trial() -> Result<Color, ColorFormatError> {
//! // Write `format!("{}", entry)` to terminal to issue the query.
//! let entry = ThemeEntry::Ansi(AnsiColor::Red);
//!
//! // The response should be an ANSI escape sequence like this one.
//! let mut terminal_input = b"\x1b]4;1;rgb:df/28/27\x07".iter();
//!
//! let mut scanner = VtScanner::new();
//! loop {
//!     // Read next byte and feed it to Scanner::step.
//!     let byte = *terminal_input.next().unwrap();
//!     scanner.step(byte);
//!
//!     if scanner.did_abort() {
//!         // The input is malformed.
//!         return Err(ColorFormatError::MalformedThemeColor);
//!     } else if scanner.did_complete() {
//!         // The input is a well-formed escape sequence.
//!         let payload = scanner
//!             .completed_string()
//!             .or(Err(ColorFormatError::MalformedThemeColor))?;
//!         return entry.parse_response(payload);
//!     }
//!     // Otherwise, keep on stepping bytes from the input.
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
//! Even though the code does not use it, [`VtScanner::step`] does return a
//! result, namely the external [`Action`] to perform. But [`VtScanner::step`]
//! also implements enough action processing itself, so that we don't need to
//! even look at the return value. Having said that, the calling code must check
//! for erroneous completion with [`VtScanner::did_abort`] and successful
//! completion with [`VtScanner::did_complete`], as illustrated in the example.
//!
//!
//! # Example #2: Processing the Entire Escape Sequence
//!
//! If that seems a bit boilerplaty, then that's because it is. The second
//! example illustrates how to replace most of the loop with a single method
//! invocation.
//!
//! ```
//! # use std::io::ErrorKind;
//! # use prettypretty::Color;
//! # use prettypretty::escape::VtScanner;
//! # use prettypretty::style::AnsiColor;
//! # use prettypretty::trans::ThemeEntry;
//! # fn the_trial() -> std::io::Result<Color> {
//! // Define theme entry, terminal input, and scanner as before.
//! let entry = ThemeEntry::Ansi(AnsiColor::Red);
//! let mut terminal_input = b"\x1b]4;1;rgb:df/28/27\x07".iter();
//! let mut scanner = VtScanner::new();
//!
//! // Step_until_string() does all necessary stepping.
//! let payload = scanner.step_until_string(
//!     // It requires callback for reading terminal input.
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
//! While certainly more concise, [`VtScanner::step_until_bytes`] and
//! [`VtScanner::step_until_string`] currently do have drawback: They hardcode
//! the shared error type to be `std::io::Error`. As it turns out, error
//! handling gets tricky in other ways as well.
//!
//!
//! # Error Handling #1: When Abort Consumes a Byte
//!
//! By precisely modelling a terminal's state machine and consuming the input
//! byte by byte, [`VtScanner`]'s implementation consumes just the bytes of an
//! ANSI escape sequence from the input, no more, no less. That, however, is
//! impossible for bytes that start a new escape sequence and thereby also abort
//! the current escape sequence. That doesn't pose a problem, as long as the
//! application keeps parsing escape sequences with the same scanner.
//!
//! But if the application consumes terminal input in some other way, it
//! effectively needs to put the extra byte back into the input stream. Plus, it
//! should reset the scanner before every use. Unfortunately, prettypretty can't
//! really help with putting the byte back into the input, since the best
//! strategy for doing so depends on the application. But it can help with
//! detecting such troublesome bytes. [`VtScanner::did_abort`] and
//! [`Control::is_sequence_start`] together spell trouble. When using
//! [`VtScanner::step_until_bytes`] or [`VtScanner::step_until_string`],
//! [`VtScanner::last_byte`] exposes the last byte passed to `step`.
//!
//!
//! # Error Handling #2: Buffer Overflow
//!
//! [`VtScanner`] buffers the payload of an ANSI escape sequence. Since its
//! input may come from untrusted sources, the implementation limits the
//! buffer's capacity and does *not* adjust it, even when filling all available
//! space. That is not a problem when querying a terminal for its colors because
//! the buffer is correctly dimensioned for this use case (with a capacity of
//! only 23 bytes). That may, however, pose a problem when parsing other ANSI
//! escape sequences. An application can detect this error condition with
//! [`VtScanner::did_overflow`]. It can also increase the capacity of a new
//! scanner with [`VtScanner::with_capacity`] in Rust or an explicit capacity
//! argument for the constructor in Python.

use std::io::{Error, ErrorKind};

#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

// ================================================================================================

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

// ------------------------------------------------------------------------------------------------

/// An external action when processing terminal I/O.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.escape")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Action {
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
    /// [`Action::DispatchCsi`] and [`Action::DispatchEsc`] are retained, too.
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

#[cfg_attr(feature = "pyffi", pymethods)]
impl Action {
    /// Determine whether this action is [`Action::Print`].
    pub fn is_print(&self) -> bool {
        matches!(self, Self::Print)
    }

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
// State Machine States and Transitions

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

    /// Determine the number of transitions from the ground state.
    ///
    /// This method assumes C0 controls only. Hence, controls such as CSI
    /// require two characters and result in states two transitions from the
    /// ground state.
    ///
    /// This method treats `Ground` as zero states away from itself, `Escape` as
    /// one state away from the ground
    /// This method treats `Ground` as step 0, `Escape` as step 1, and all other
    /// states as step 2.
    pub fn ground_transitions(&self) -> usize {
        use State::*;

        match self {
            Ground => 0,
            Escape => 1,
            EscapeIntermediate | ApcString | CsiEntry | DcsEntry | OscString | PmString
            | SosString => 2,
            ApcEnd | CsiParam | CsiIntermediate | CsiIgnore | DcsParam | DcsIntermediate
            | DcsPassthrough | DcsIgnore | OscEnd | PmEnd | SosEnd => 3,
            DcsPassthroughEnd | DcsIgnoreEnd => 4,
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

// ------------------------------------------------------------------------------------------------

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

/// A state machine for scanning terminal input or output.
///
/// Like Alacritty's [vte](https://github.com/alacritty/vte) and Wezterm's
/// [vtparse](https://github.com/wez/wezterm) crates, this type leverages Paul
/// Flo Williams' [parser for DEC's ANSI-compatible video
/// terminals](https://vt100.net/emu/dec_ansi_parser). However, unlike these two
/// crates and Williams' original, this version features a streamlined state
/// machine model. For each step, the state machine simply consumes a byte as
/// input and produces an [`Action`] as output. Internally, it also transitions
/// from one state to another.
///
/// Since the state machine primarily targets applications that interrogate a
/// terminal with ANSI escape sequences, processing escape sequence payloads as
/// bytes arrive provides little benefit while also imposing explicit state
/// management on considerably more code. Hence, this type buffers incoming
/// bytes and provides the complete payload upon dispatch of an escape sequence.
/// That way, it replaces the original's *collect*, *osc_put*, *param*, and
/// *put* actions with just a *retain* action.
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.escape"))]
#[derive(Debug)]
pub struct VtScanner {
    previous_state: State,
    state: State,
    buffer: Vec<u8>,
    did_overflow: bool,
    last_byte: u8,
    last_action: Action,
}

impl VtScanner {
    /// The scanner's default capacity, which is optimized for recognizing
    /// terminal responses with theme colors.
    pub const DEFAULT_CAPACITY: usize = 23;

    /// Create a new scanner with the default capacity.
    #[inline]
    pub fn new() -> Self {
        Self::with_capacity(Self::DEFAULT_CAPACITY)
    }

    /// Create a new scanner with the given capacity.
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

impl Default for VtScanner {
    #[inline]
    fn default() -> Self {
        Self::with_capacity(Self::DEFAULT_CAPACITY)
    }
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl VtScanner {
    /// Create a new scanner. <i class=python-only>Python only!</i>
    ///
    /// The default capacity is just enough to parse terminal responses with
    /// theme colors.
    #[cfg(feature = "pyffi")]
    #[new]
    #[pyo3(signature = (capacity=VtScanner::DEFAULT_CAPACITY))]
    pub fn py_new(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }

    /// Reset this scanner.
    pub fn reset(&mut self) {
        self.previous_state = State::Ground;
        self.state = State::Ground;
        self.buffer.clear();
        self.did_overflow = false;
        self.last_byte = 0;
        self.last_action = Action::Ignore;
    }

    /// Determine this scanner's internal buffer capacity.
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Process the given input byte.
    ///
    /// This method performs a state machine step, transitioning the internal
    /// state and producing an external action. For [`Action::Start`],
    /// [`Action::Retain`], [`Action::DispatchCsi`], and
    /// [`Action::DispatchEsc`], it also updates the internal buffer, clearing
    /// it for `Start` and adding the input for the other three actions.
    pub fn step(&mut self, byte: u8) -> Action {
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

    /// Determine the input byte for the last step.
    #[inline]
    pub fn last_byte(&self) -> u8 {
        self.last_byte
    }

    /// Determine whether the internal buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Determine the current number of buffered bytes.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Determine whether the internal buffer did overflow since the last start
    /// action.
    #[inline]
    pub fn did_overflow(&self) -> bool {
        self.did_overflow
    }

    /// Determine whether the last step aborted an ANSI escape sequence.
    pub fn did_abort(&self) -> bool {
        // We only need to distinguish between 0, 1, and 2+ transitions
        let previous = self.previous_state.ground_transitions().min(2);
        let current = self.state.ground_transitions().min(2);

        if current < previous {
            // Abort is transitions from ground shrinking without dispatch action
            !self.last_action.is_dispatch()
        } else if current == 2 && previous == 2 {
            // Abort is transitions from ground equally high with start action
            self.last_action.is_start()
        } else {
            false
        }
    }

    /// Determine whether the last step completed an ANSI escape sequence.
    #[inline]
    pub fn did_complete(&self) -> bool {
        self.last_action.is_dispatch()
    }

    /// Determine the control for the just completed ANSI escape sequence.
    ///
    /// If [`VtScanner::did_complete`], this method returns the corresponding
    /// control. Otherwise, it returns `None`.
    #[inline]
    pub fn completed_control(&self) -> Option<Control> {
        self.last_action.control()
    }

    /// Access the payload for the just completed ANSI escape sequence as a byte
    /// slice.
    ///
    /// If [`VtScanner::did_complete`], this method returns the payload of the
    /// corresponding ANSI escape sequence as a byte slice. Otherwise, it
    /// returns an empty slice. [`VtScanner::completed_string`] does the same,
    /// except it returns a string slice.
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
    /// If [`VtScanner::did_complete`], this method returns the payload of the
    /// corresponding ANSI escape sequence as a string slice. Otherwise, it
    /// returns an empty slice. [`VtScanner::completed_bytes`] does the same,
    /// except it returns a byte slice.
    #[inline]
    pub fn completed_string(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.completed_bytes())
    }

    /// Get a debug representation for this scanner. <i class=python-only>Python
    /// only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("Scanner({:?})", self)
    }
}

impl VtScanner {
    /// Scan an escape sequence and return its payload as a byte slice. <i
    /// class=rust-only>Rust only!</i>
    ///
    /// This method repeatedly calls [`VtScanner::step`] on the bytes returned
    /// by the `read` closure until this scanner [`VtScanner::did_abort`] or
    /// [`VtScanner::did_complete`], returning an error in the former case and
    /// the payload's bytes in the latter case. This method also returns an
    /// error, if the `read` closure fails at reading a byte from terminal
    /// input.
    ///
    /// It is safe to call [`VtScanner::completed_control`] after this method
    /// has returned with a byte slice. The last consumed input byte can be
    /// accessed through [`VtScanner::last_byte`].
    ///
    /// [`VtScanner::step_until_string`] does the same, except it returns a
    /// string slice.
    pub fn step_until_bytes<F>(&mut self, mut read: F) -> std::io::Result<&[u8]>
    where
        F: FnMut() -> std::io::Result<u8>,
    {
        loop {
            let byte = read()?;
            self.step(byte);
            if self.did_abort() {
                return Err(ErrorKind::InvalidData.into());
            } else if self.did_complete() {
                return Ok(self.completed_bytes());
            }
        }
    }

    /// Scan an escape sequence and return its payload as a string slice. <i
    /// class=rust-only>Rust only!</i>
    ///
    /// This method repeatedly calls [`VtScanner::step`] on the bytes returned
    /// by the `read` closure until this scanner [`VtScanner::did_abort`] or
    /// [`VtScanner::did_complete`], returning an error in the former case and
    /// the payload's string in the latter case. This method also returns an
    /// error, if the `read` closure fails at reading a byte from terminal
    /// input.
    ///
    /// It is safe to call [`VtScanner::completed_control`] after this method
    /// has returned with a string slice. The last consumed input byte can be
    /// accessed through [`VtScanner::last_byte`].
    ///
    /// [`VtScanner::step_until_bytes`] does the same, except it returns a byte
    /// slice.
    pub fn step_until_string<F>(&mut self, read: F) -> std::io::Result<&str>
    where
        F: FnMut() -> std::io::Result<u8>,
    {
        let bytes = self.step_until_bytes(read)?;
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
