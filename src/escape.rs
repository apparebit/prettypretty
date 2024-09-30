//! Helper functionality for consuming ANSI escape sequences.
//!
//! By using [`Scanner`], an application can more easily consume exactly one
//! escape sequence, independently of using sync or async terminal I/O. The
//! example code below illustrates the use of [`Scanner::process`] and
//! [`Scanner::consume`].
//!
//! ```
//! # use prettypretty::{Color, error::ColorFormatError};
//! # use prettypretty::escape::{Continuation, Scanner};
//! # use prettypretty::style::AnsiColor;
//! # use prettypretty::trans::ThemeEntry;
//! # fn the_trial() -> Result<Color, ColorFormatError> {
//! let entry = ThemeEntry::Ansi(AnsiColor::Red);
//! // print!("{}", entry); // does in fact query the terminal...
//! // 🧙 Hocus Pocus the Great makes a suitable response appear...
//! let mut terminal_input = b"\x1b]4;1;rgb:df/28/27\x07".iter();
//!
//! let mut scanner = Scanner::new();
//! loop {
//!     let byte = *terminal_input.next().unwrap();
//!     match scanner.process(byte) {
//!         // On abort, return error
//!         Continuation::Abort => return Err(ColorFormatError::MalformedThemeColor),
//!
//!         // On consume, parse payload as color
//!         Continuation::Consume => {
//!             let payload = scanner
//!                 .consume()
//!                 .or(Err(ColorFormatError::MalformedThemeColor))?;
//!             return entry.parse_response(payload);
//!         }
//!
//!         // Otherwise, keep on processing the input...
//!         Continuation::Continue => (),
//!     }
//! }
//! # }
//! # fn main() {
//! #     assert!(the_trial().is_ok());
//! # }
//! ```
//! <div class=color-swatch>
//! <div style="background-color: #df2827;"></div>
//! </div>
//! <br>
//!
//! After putting the terminal into cbreak or raw mode, a production version
//! might want to use
//! [`Translator::theme_entries`](crate::trans::Translator::theme_entries) to
//! iterate over all theme entries, writing out each entry's display to query
//! the terminal. It also might want to reuse the scanner instance. While the
//! internal buffer is sized for the use case and hence rather small, there is
//! no reason to recreate the scanner for every theme entry.

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

#[cfg_attr(feature = "pyffi", pymethods)]
impl Control {
    /// Determine whether this control is the plain escape function.
    pub fn is_escape(&self) -> bool {
        matches!(self, Self::ESC)
    }

    /// Determine whether this control terminates ANSI escape sequences.
    pub fn is_terminator(&self) -> bool {
        matches!(self, Self::BEL | Self::ST)
    }

    /// Determine whether this control starts an ANSI escape sequence.
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
    pub fn is_retained(&self) -> bool {
        matches!(self, Self::Retain | Self::DispatchCsi | Self::DispatchEsc)
    }

    /// Determine whether the action is the dispatch action.
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

        // FIXME: Recognize UTF-8 lead bytes instead and issue actions to
        // ignore/collect the full UTF-8 encoded code point instead.
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
    last_action: Action,
}

impl StateMachine {
    /// Create a new buffering state machine with default capacity.
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
            last_action: Action::Ignore,
        }
    }
}

impl Default for StateMachine {
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
        self.last_action = Action::Ignore;
    }

    // /// Determine whether the internal buffer is empty.
    // pub fn is_empty(&self) -> bool {
    //     self.buffer.is_empty()
    // }

    // /// Determine the number of bytes buffered for the current escape sequence.
    // pub fn len(&self) -> usize {
    //     self.buffer.len()
    // }

    // /// Determine this state machine's internal buffer capacity.
    // pub fn capacity(&self) -> usize {
    //     self.buffer.capacity()
    // }

    /// Determine whether the internal buffer did overflow.
    pub fn did_overflow(&self) -> bool {
        self.did_overflow
    }

    /// Process the given byte.
    pub fn process(&mut self, byte: u8) -> Action {
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
        self.last_action = action;

        action
    }

    /// Determine whether the last byte completed an ANSI escape sequence.
    pub fn is_complete(&self) -> bool {
        self.last_action.is_dispatch()
    }

    /// Determine the control for the complete escape sequence.
    ///
    /// If the escape sequence [`StateMachine::is_complete`], this method
    /// returns the corresponding control. Otherwise, it returns `None`.
    pub fn completed_control(&self) -> Option<Control> {
        self.last_action.control()
    }

    /// Determine whether the ANSI escape sequence is malformed.
    ///
    /// This method only works for ANSI escape sequences. In particular, it
    /// treats the state transition resulting from regular characters as
    /// malformed.
    pub fn is_malformed(&self) -> bool {
        let previous = self.previous_state.step();
        let current = self.state.step();

        if previous + 1 == current {
            // Getting started with sequence
            false
        } else if previous == 2 && current == 2 {
            // Inside the sequence
            false
        } else if 1 <= previous && 0 == current {
            // Just dispatched the sequence. Allow previous == 1 for transition
            // from Escape to Ground on most ASCII characters.
            !self.last_action.is_dispatch()
        } else {
            // Outside a sequence counts as malformed
            true
        }
    }

    /// Access the bytes retained for the just completed ANSI escape sequence.
    ///
    /// If this state machine [did just complete](StateMachine::is_complete)
    /// parsing an ANSI escape sequence, this method returns the retained bytes.
    /// Otherwise, it returns an empty slice.
    pub fn retained(&self) -> &[u8] {
        if self.is_complete() {
            &self.buffer
        } else {
            &[]
        }
    }

    /// Get the payload for the just completed ANSI escape sequence as a string.
    pub fn retained_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.retained())
    }
}

/// An enumeration of continuations.
///
/// [`Scanner::process`] returns one of this enum's three variants to indicate
/// the caller's next action.
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

    /// Consume the scanned escape sequence.
    ///
    /// [`Scanner::process`] returns this continuation when it has successfully
    /// scanned an entire escape sequence. The caller should invoke
    /// [`Scanner::consume`] to consume the payload of the escape sequence. It
    /// can use [`Scanner::control`] to inquire about the kind of escape
    /// sequence.
    Consume,
}

/// A scanner for escape sequences.
///
/// This struct exposes a much simplified interface to a fully general state
/// machine for parsing terminal I/O. The interface has four methods:
///
///   * [`Scanner::reset`] clears any residual state before trying to scan a new
///     escape sequence.
///   * [`Scanner::process`] pushes a byte into the state machine and returns a
///     [`Continuation`] to indicate the next step, which is either to *abort*
///     parsing the current escape sequence, *continue* processing with another
///     byte, or *consume* the complete escape sequence.
///   * [`Scanner::consume`] returns the payload of the escape sequence without
///     the leading control and, for sequences other than CSI or ESC, also the
///     trailing control.
///   * [`Scanner::control`] returns the control for the escape sequence being
///     consumed. It returns `None` when the current continuation is not
///     [`Continuation::Consume`].
///
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.escape"))]
#[derive(Debug)]
pub struct Scanner {
    machine: StateMachine,
}

impl Scanner {
    /// Create a new escape sequence scanner.
    pub fn new() -> Self {
        // "4;15;rgb:ffff/ffff/ffff".len() == 23
        Self {
            machine: StateMachine::with_capacity(23),
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
    #[cfg(feature = "pyffi")]
    #[new]
    pub fn py_new() -> Self {
        Self::new()
    }

    /// Reset this escape sequence scanner.
    pub fn reset(&mut self) {
        self.machine.reset()
    }

    /// Process the given byte and return the continuation.
    pub fn process(&mut self, byte: u8) -> Continuation {
        self.machine.process(byte);

        if self.machine.is_complete() && !self.machine.did_overflow() {
            Continuation::Consume
        } else if self.machine.is_malformed() || self.machine.did_overflow() {
            Continuation::Abort
        } else {
            Continuation::Continue
        }
    }

    /// Determine the control leading the scanned escape sequence.
    ///
    /// If the continuation is [`Continuation::Consume`], this method returns
    /// the control that started the scanned escape sequence.
    pub fn control(&self) -> Option<Control> {
        self.machine.completed_control()
    }

    /// Consume a scanned escape sequence.
    ///
    /// If [`Scanner::process`] returned [`Continuation::Consume`], this method
    /// returns the payload for a successfully scanned escape sequence.
    /// Otherwise, this method returns an empty slice. The payload does not
    /// include the leading control nor, for escape sequences other than CSI or
    /// ESC, the trailing control. [`Scanner::control`] returns the leading
    /// control.
    pub fn consume(&self) -> Result<&str, std::str::Utf8Error> {
        self.machine.retained_str()
    }

    /// Get a debug representation for this scanner. <i class=python-only>Python
    /// only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("Scanner({:?})", self.machine)
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
