//! Recognizing ANSI escape sequences.

use std::io::{BufRead, Error, ErrorKind};

#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use super::utf8::{decode, UTF8_ACCEPT, UTF8_REJECT};

// ================================================================================================

/// A control when scanning ANSI escape sequences with [`VtScanner`].
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
    pyclass(eq, frozen, hash, module = "prettypretty.color.term")
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

/// An external action when scanning terminal I/O with [`VtScanner`].
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.term")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    /// Print current byte as an ASCII character.
    PrintAscii,

    /// Start a UTF-8 codepoint.
    StartUtf8,

    /// Continue a UTF-8 codepoint.
    ContinueUtf8,

    /// Complete a UTF-8 codepoint.
    FinishUtf8,

    /// Replace malformed UTF-8 sequence with � U+FFFD.
    ReplaceUtf8,

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
    /// Determine whether this action requires printing the current byte as
    /// ASCII.
    ///
    /// Upon this action, a terminal should always print the byte.
    #[inline]
    pub fn is_print(&self) -> bool {
        matches!(self, Self::PrintAscii)
    }

    /// Determine whether this action treats the current byte as part of a
    /// codepoint in UTF-8.
    ///
    /// Upon one of these actions, a terminal may do the same as for ASCII
    /// bytes: Just print it. That, however, preserves malformed UTF-8 in the
    /// stream.
    ///
    /// A more robust alternative requires a four byte intermediate buffer and
    /// implements a very simple state machine that uses actions as input
    /// symbols. Conveniently, the start state is called **Start**.
    ///
    /// <dl>
    /// <dt>Start</dt>
    /// <dd>On `StartUtf8`, clear the intermediate byte buffer, append the current
    /// byte, and transition to the **Loop** state.</dd>
    /// <dd>On `ReplaceUtf8`, emit the replacement character � U+FFFD and remain
    /// in this state.</dd>
    /// <dd>On all other actions, just stay in this state.</dd>
    ///
    /// <dt>Loop</dt>
    /// <dd>On `ContinueUtf8`, append the current byte to the intermediate byte
    /// buffer, and stay in this state.</dd>
    /// <dd>On `FinishUtf8`, append the current byte to the intermediate byte
    /// buffer, emit the buffer contents as a codepoint with valid UTF-8, and
    /// return to the **Start** state.</dd>
    /// <dd>On `ReplaceUtf8`, emit the replacement character � U+FFFD and return
    /// to the **Start**state.</dd>
    /// <dd>On all other actions, panic.</dd>
    /// </dl>
    ///
    /// That's it. State machines don't get much simpler...
    #[inline]
    pub fn is_utf8(&self) -> bool {
        use Action::*;

        matches!(self, StartUtf8 | ContinueUtf8 | FinishUtf8 | ReplaceUtf8)
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
    Utf8,
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

    // /// Determine whether this is the UTF-8 state.
    // pub fn is_utf8(&self) -> bool {
    //     matches!(self, Self::Utf8)
    // }

    /// Determine the minimum number of transitions from the ground to this
    /// state.
    ///
    /// This method counts transitions based on C0 controls. Like
    /// [`State::is_ground`], it treats the UTF-8 state as part of the larger
    /// ground state, i.e., its distance is 0.
    pub fn ground_distance(&self) -> usize {
        use State::*;

        match self {
            Ground | Utf8 => 0,
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

const fn otherwise(byte: u8, state: State) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
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

const fn ground(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x20..=0x7f => (Ground, PrintAscii),
        _ => otherwise(byte, Ground),
    }
}

// ------------------------------------------------------------------------------------------------
// Escape

const fn escape(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x20..=0x2f => (EscapeIntermediate, Retain),
        0x30..=0x4f | 0x51..=0x57 | 0x59 | 0x5a | 0x5c | 0x60..=0x7e => (Ground, DispatchEsc),
        0x50 => (DcsEntry, Start),
        0x58 => (SosString, Start),
        0x5b => (CsiEntry, Start),
        0x5d => (OscString, Start),
        0x5e => (PmString, Start),
        0x5f => (ApcString, Start),
        _ => otherwise(byte, Escape),
    }
}

const fn escape_intermediate(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x20..=0x2f => (EscapeIntermediate, Retain),
        0x30..=0x7e => (Ground, DispatchEsc),
        _ => otherwise(byte, EscapeIntermediate),
    }
}

// ------------------------------------------------------------------------------------------------
// SOS, PM, APC

macro_rules! string_command {
    ($name1:ident($state1:ident) => $name2:ident($state2:ident) => $action:expr) => {
        const fn $name1(byte: u8) -> (State, Action) {
            use Action::*;
            use State::*;

            match byte {
                0x00..=0x06 | 0x08..=0x17 | 0x19 | 0x1c..=0x1f => ($state1, Ignore),
                0x07 => (Ground, $action),
                0x1b => ($state2, Ignore),
                0x20..=0x7f => ($state1, Ignore),
                0x9c => (Ground, $action),
                _ => otherwise(byte, $state1),
            }
        }

        const fn $name2(byte: u8) -> (State, Action) {
            use Action::*;
            use State::*;

            match byte {
                0x5c => (Ground, $action),
                _ => otherwise(byte, Ground),
            }
        }
    };
}

string_command!(sos_string(SosString) => sos_end(SosEnd) => DispatchSos);
string_command!(pm_string(PmString) => pm_end(PmEnd) => DispatchPm);
string_command!(apc_string(ApcString) => apc_end(ApcEnd) => DispatchApc);

// ------------------------------------------------------------------------------------------------
// OSC

const fn osc_string(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x06 | 0x08..=0x17 | 0x19 | 0x1c..=0x1f => (OscString, Ignore),
        0x07 => (Ground, DispatchOsc),
        0x1b => (OscEnd, Ignore),
        0x20..=0x7f => (OscString, Retain),
        0x9c => (Ground, DispatchOsc),
        _ => otherwise(byte, OscString),
    }
}

const fn osc_end(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x5c => (Ground, DispatchOsc),
        _ => otherwise(byte, Ground),
    }
}

// ------------------------------------------------------------------------------------------------
// CSI

const fn csi_entry(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x20..=0x2f => (CsiIntermediate, Retain),
        0x30..=0x39 | 0x3b..=0x3f => (CsiParam, Retain),
        0x3a => (CsiIgnore, Ignore),
        0x40..=0x7e => (Ground, DispatchCsi),
        _ => otherwise(byte, CsiEntry),
    }
}

const fn csi_param(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x20..=0x2f => (CsiIntermediate, Retain),
        0x30..=0x39 | 0x3b => (CsiParam, Retain),
        0x3a | 0x3c..=0x3f => (CsiIgnore, Ignore),
        0x40..=0x7e => (Ground, DispatchCsi),
        _ => otherwise(byte, CsiParam),
    }
}

const fn csi_intermediate(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x20..=0x2f => (CsiIntermediate, Retain),
        0x30..=0x3f => (CsiIgnore, Ignore),
        0x40..=0x7e => (Ground, DispatchCsi),
        _ => otherwise(byte, CsiIntermediate),
    }
}

const fn csi_ignore(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x20..=0x3f => (CsiIgnore, Ignore),
        0x40..=0x7e => (Ground, Ignore),
        _ => otherwise(byte, CsiIgnore),
    }
}

// ------------------------------------------------------------------------------------------------
// DCS

const fn dcs_entry(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (DcsEntry, Ignore),
        0x20..=0x2f => (DcsIntermediate, Retain),
        0x30..=0x39 | 0x3b..=0x3f => (DcsParam, Retain),
        0x3a => (DcsIgnore, Ignore),
        0x40..=0x7e => (DcsPassthrough, Retain),
        _ => otherwise(byte, DcsEntry),
    }
}

const fn dcs_param(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (DcsParam, Ignore),
        0x20..=0x2f => (DcsIntermediate, Retain),
        0x30..=0x39 | 0x3b => (DcsParam, Retain),
        0x3a | 0x3c..=0x3f => (DcsIgnore, Ignore),
        0x40..=0x7e => (DcsPassthrough, Retain),
        _ => otherwise(byte, DcsParam),
    }
}

const fn dcs_intermediate(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (DcsIntermediate, Ignore),
        0x20..=0x2f => (DcsIntermediate, Retain),
        0x30..=0x3f => (DcsIgnore, Ignore),
        0x40..=0x7e => (DcsPassthrough, Retain),
        _ => otherwise(byte, DcsIntermediate),
    }
}

const fn dcs_passthrough(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x06 | 0x08..=0x17 | 0x19 | 0x1c..=0x1f => (DcsPassthrough, Retain),
        0x07 => (Ground, DispatchDcs),
        0x1b => (DcsPassthroughEnd, Ignore),
        0x20..=0x7e => (DcsPassthrough, Retain),
        0x9c => (Ground, DispatchDcs),
        _ => otherwise(byte, DcsPassthrough),
    }
}

const fn dcs_passthrough_end(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x5c => (Ground, DispatchDcs),
        _ => otherwise(byte, Ground),
    }
}

const fn dcs_ignore(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x06 | 0x08..=0x17 | 0x19 | 0x1c..=0x1f => (DcsIgnore, Ignore),
        0x07 => (Ground, Ignore),
        0x1b => (DcsIgnoreEnd, Ignore),
        0x20..=0x7f => (DcsIgnore, Ignore),
        0x9c => (Ground, Ignore),
        _ => otherwise(byte, DcsIgnore),
    }
}

const fn dcs_ignore_end(byte: u8) -> (State, Action) {
    use Action::*;
    use State::*;

    match byte {
        0x5c => (Ground, Ignore),
        _ => otherwise(byte, Ground),
    }
}

// ------------------------------------------------------------------------------------------------
// Complete transition function

/// Determine the next state and action.
///
/// # Panics
///
/// If the current state is [`State::Utf8`], which is stateful and hence must
/// be handled by [`VtScanner`].
const fn transition(state: State, byte: u8) -> (State, Action) {
    use State::*;

    match state {
        Ground => ground(byte),
        Utf8 => panic!("cannot handle UTF-8 in transition function"),
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
/// crates and Williams' specification, this implementation features a
/// streamlined state machine model:
///
///  1. For each step, this version takes a byte and produces an action, while
///     internally transitioning from state to state as necessary. However,
///     neither states nor state transitions are externally accessible, only
///     higher-level properties of the state machine.
///  2. This version also treats all bytes that belong to an escape sequence's
///     payload the same by retaining them in its internal buffer. That way,
///     [`Action::Retain`] replaces the *collect*, *osc_put*, *param*, and *put*
///     actions of the original and obviates the *osc_start*, *osc_end*, *hook*,
///     and *unhook* actions entirely.
///
/// The disadvantage of thusly simplifying the state machine is that code using
/// `VtScanner` probably needs to reparse (parts of) escape sequences. As a
/// result, this version is better suited to applications that need to scan
/// terminal input for responses to queries than for the implementation of a
/// terminal.
///
/// Correct use of [`VtScanner`] requires handling the following corner cases:
///
///   * When trying to scan an escape sequence, make sure that the first byte
///     does indeed start one, i.e., [`VtScanner::step`] returns
///     [`Action::Start`], without consuming that byte.
///   * When trying to access the payload of an escape sequence, make sure that
///     the payload has not been cut off, i.e., [`VtScanner::did_overflow`]
///     returns `false`.
///   * When aborting an escape sequence, do not consume the final byte if it
///     starts another escape sequence as well, i.e.,
///     [`Control::is_sequence_start`] returns `true`.
///
/// Alas, before exploring how to address those corner cases, we might want to
/// start by parsing an escape sequence first. In particular, let's explore how
/// to query a terminal for its current color theme. Let's also assume that the
/// application already put the terminal into raw mode and is iterating over
/// [`ThemeEntry::all`](crate::trans::ThemeEntry::all), i.e., the default
/// foreground, default background, and 16 ANSI colors.
///
///
/// # Example #1: Byte by Byte
///
/// The following example code sketches one pass through that loop, which
/// queries the terminal for a particular theme color, ANSI red. We display the
/// theme entry on the terminal, which writes the corresponding query as an ANSI
/// escape sequence, and then turn the response into a color object with help of
/// [`VtScanner`].
///
/// ```
/// # use prettypretty::{Color, error::ColorFormatError};
/// # use prettypretty::term::VtScanner;
/// # use prettypretty::style::AnsiColor;
/// # use prettypretty::trans::ThemeEntry;
/// // Write `format!("{}", entry)` to the terminal to issue the query.
/// let entry = ThemeEntry::Ansi(AnsiColor::Red);
///
/// // The response should be an ANSI escape sequence like this one.
/// let mut input = b"\x1b]4;1;rgb:df/28/27\x07".iter();
///
/// // Let's process the escape sequence with a scanner.
/// let mut scanner = VtScanner::new();
/// let color = loop {
///     // Read byte and feed it to scanner's step() method.
///     let byte = *input.next().unwrap();
///     scanner.step(byte);
///
///     if scanner.did_abort() {
///         // The escape sequence is malformed.
///         break Err(ColorFormatError::MalformedThemeColor);
///     } else if scanner.did_complete() {
///         // Parse the escape sequence's payload as a color.
///         break scanner
///             .completed_str()
///             .or(Err(ColorFormatError::MalformedThemeColor))
///             .and_then(|payload| entry.parse_response(payload))
///     }
///
///     // Keep on stepping...
/// };
///
/// assert_eq!(color.unwrap(), Color::from_24bit(0xdf, 0x28, 0x27));
/// ```
/// <div class=color-swatch>
/// <div style="background-color: #df2827;"></div>
/// </div>
/// <br>
///
/// As shown, consuming an escape sequence requires little more than stepping
/// through the input with [`VtScanner::step`] until either
/// [`VtScanner::did_abort`] or [`VtScanner::did_complete`] returns `true`. Once
/// complete, [`VtScanner::completed_bytes`] and [`VtScanner::completed_str`]
/// return the escape sequence's payload. The example uses
/// [`ThemeEntry::parse_response`](crate::trans::ThemeEntry::parse_response) to
/// turn that payload into a color.
///
///
/// # Three Corner Cases
///
/// The above code is functional but not very robust. It fails to handle the
/// three corner cases introduced earlier. Let's consider each in turn.
///
/// First, a terminal's input may contain content other than escape sequences.
/// While [`VtScanner`]'s state machine knows how to handle other content as
/// well, the above example code does not. Fixing that requires checking whether
/// the result of [`VtScanner::step`] for the first byte is [`Action::Start`]
/// and otherwise leaving the input untouched. That effectively requires a
/// look-ahead of one byte.
///
/// Second, [`VtScanner`] buffers an escape sequence's payload. Since terminal
/// input cannot be trusted, the buffer capacity must not change in response to
/// the input. [`VtScanner::new`] allocates a buffer just large enough for
/// processing terminal colors but no more. If you plan on processing other
/// escape sequences, you should right-size the buffer with
/// [`VtScanner::with_capacity`]. In either case, your application should always
/// check whether the payload did fit into the buffer with
/// [`VtScanner::did_overflow`]. If the buffer did overflow, the bad news is
/// that either you got the capacity wrong or the terminal is under adversarial
/// control indeed. The good news is that, unlike for most buffer overflows,
/// memory safety wasn't compromised; the payload was truncated instead.
///
/// Third, [`VtScanner`] can detect the end of a well-formed escape sequence
/// without look-ahead. In that case, [`VtScanner::step`] returns an action,
/// whose [`Action::is_dispatch`] is `true`. Conveniently,
/// [`VtScanner::did_complete`] is `true` as well. However, `VtScanner` cannot
/// detect all malformed escape sequences without looking at the next byte. In
/// particular, if a byte starts a new escape sequence, i.e.,
/// [`Control::is_sequence_start`] returns `true`, it necessarily aborts the
/// current escape sequence as well. In short, an application finds out whether
/// it may consume a byte only after stepping `VtScanner` with that byte. That
/// effectively requires a look-ahead of one byte (again).
///
///
/// # Example #2: With a Buffered Reader
///
/// One-byte look-ahead implies some form of buffering. Rust's
/// `std::io::BufRead` trait fits the bill quite nicely:
///
/// ```
/// # use std::io::{BufRead, Error, ErrorKind};
/// # use prettypretty::Color;
/// # use prettypretty::term::{Action, Control, VtScanner};
/// # use prettypretty::style::AnsiColor;
/// # use prettypretty::trans::ThemeEntry;
/// let entry = ThemeEntry::Ansi(AnsiColor::Red);
/// let mut input = b"\x1b]4;1;rgb:df/28/27\x07".as_slice();
///
/// let mut scanner = VtScanner::new();
/// // Track the first byte.
/// let mut first = true;
///
/// // Use a loop label for double loop exits.
/// let response = 'colorful: loop {
///     // Track the number of consumed bytes.
///     let mut count = 0;
///     let bytes = input.fill_buf()?;
///     // Check for timeout (which looks just like EOF).
///     if bytes.is_empty() {
///         return Err(ErrorKind::TimedOut.into());
///     }
///
///     // Since bytes is the result of input.fill_buf(), it must borrow
///     // from input. Hence, we cannot use an expression with bytes as
///     // argument to input.consume() below. But we can precompute value.
///     let filled = bytes.len();
///
///     for byte in bytes {
///         let action = scanner.step(*byte);
///
///         if first {
///             // Make sure the first byte starts escape sequence.
///             first = false;
///             if action != Action::Start {
///                 return Err(ErrorKind::InvalidData.into());
///             }
///         } else if scanner.did_abort() {
///             // Determine whether to consume last byte.
///             if !Control::is_sequence_start(*byte) {
///                 count += 1;
///             }
///             input.consume(count);
///             return Err(ErrorKind::InvalidData.into());
///
///         } else if scanner.did_complete() {
///             // Always consume last byte.
///             input.consume(count + 1);
///             // Handle buffer overflow.
///             if scanner.did_overflow() {
///                 return Err(ErrorKind::OutOfMemory.into());
///             } else {
///                 break 'colorful scanner.completed_str()
///                     .map_err(|e| Error::new(
///                         ErrorKind::InvalidData, e
///                     ))?;
///             }
///         }
///
///         // The byte is safe to consume.
///         count += 1;
///     }
///
///     // Consume buffer before trying to fill another.
///     input.consume(filled);
/// };
///
/// // Parse payload and validate color.
/// let color = entry
///     .parse_response(response)
///     .map_err(|e| Error::new(ErrorKind::InvalidData, e));
/// assert_eq!(color.unwrap(), Color::from_24bit(0xdf, 0x28, 0x27));
/// # Ok::<(), Error>(())
/// ```
/// <div class=color-swatch>
/// <div style="background-color: #df2827;"></div>
/// </div>
/// <br>
///
/// Much better. I think. There is too much code now. And a lot of it seems to
/// belong to `VtScanner`'s implementation. After all, a parser of escape
/// sequences should be able to count the number of bytes belonging to one and
/// also tell the first byte apart from the following bytes. Similarly, it
/// should be able to map various predicates to errors.
///
///
/// # Example #3: Without Boilerplate
///
/// After integrating that functionality with `VtScanner` and adding a few more
/// methods, the example becomes:
///
/// ```
/// # use std::io::{BufRead, Error, ErrorKind};
/// # use prettypretty::Color;
/// # use prettypretty::term::{Action, Control, VtScanner};
/// # use prettypretty::style::AnsiColor;
/// # use prettypretty::trans::ThemeEntry;
/// let entry = ThemeEntry::Ansi(AnsiColor::Red);
/// let mut input = b"\x1b]4;1;rgb:df/28/27\x07".as_slice();
///
/// let mut scanner = VtScanner::new();
/// let response = 'colorful: loop {
///     let bytes = input.fill_buf()?;
///     if bytes.is_empty() {
///         return Err(ErrorKind::TimedOut.into());
///     }
///
///     let filled = bytes.len();
///     for byte in bytes {
///         let action = scanner.step(*byte);
///
///         if scanner.processed() == 0 && action != Action::Start {
///             return Err(ErrorKind::InvalidData.into());
///         } else if scanner.did_finish() {
///             input.consume(scanner.processed());
///             break 'colorful scanner.finished_str()?;
///         }
///     }
///     input.consume(filled);
/// };
///
/// // Parse payload and validate color.
/// let color = entry
///     .parse_response(response)
///     .map_err(|e| Error::new(ErrorKind::InvalidData, e));
/// assert_eq!(color.unwrap(), Color::from_24bit(0xdf, 0x28, 0x27));
/// # Ok::<(), Error>(())
/// ```
/// <div class=color-swatch>
/// <div style="background-color: #df2827;"></div>
/// </div>
/// <br>
///
/// [`VtScanner::processed`] returns the number of bytes processed so far while
/// recognizing an escape sequence. [`VtScanner::did_finish`] returns `true` if
/// either [`VtScanner::did_abort`] or [`VtScanner::did_complete`] returns
/// `true`. Finally, [`VtScanner::finished_bytes`] and
/// [`VtScanner::finished_str`] return an escape sequence's payload as a byte or
/// string slice—or an appropriate I/O error.
///
/// Prettypretty's Rust version can do one better: The generic
/// [`VtScanner::scan_bytes`] and [`VtScanner::scan_str`] methods encapsulate
/// the above double loop in its entirety. However, because they are generic
/// methods, they cannot be exposed to Python.
///
/// Still, concise and correct is good! 🎉
///
/// Ahem...
///
///
/// # One More Thing
///
/// As discussed in the documentation for the [`term`](crate::term) module,
/// reading terminal input requires that the terminal has been correctly
/// configured and that reads eventually time out. As is, that won't happen with
/// the `input.fill_buf()` expression just before the second loop—unless the
/// underlying terminal has been configured to time out.
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.term"))]
#[derive(Debug)]
pub struct VtScanner {
    previous_state: State,
    state: State,
    utf8_state: u8,
    utf8_codepoint: u32,
    buffer: Vec<u8>,
    did_overflow: bool,
    last_byte: u8,
    last_action: Action,
    processed: usize,
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
            utf8_state: 0,
            utf8_codepoint: 0,
            buffer: Vec::with_capacity(capacity),
            did_overflow: false,
            last_byte: 0,
            last_action: Action::Ignore,
            processed: 0,
        }
    }
}

impl Default for VtScanner {
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
        self.utf8_state = 0;
        self.utf8_codepoint = 0;
        self.buffer.clear();
        self.did_overflow = false;
        self.last_byte = 0;
        self.last_action = Action::Ignore;
        self.processed = 0;
    }

    /// Determine this scanner's internal buffer capacity.
    ///
    /// Unlike `Vec<T>`, a scanner's capacity does not change after creation.
    /// This is a security precaution, since the bytes processed by this type
    /// may originate from untrusted users.
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
        // Update the number of processed bytes with one byte delay.
        if matches!(self.state, State::Ground | State::Utf8) {
            self.processed = 0;
        } else if Control::is_sequence_start(self.last_byte) {
            self.processed = 1;
        } else {
            self.processed += 1;
        }

        // Determine next state and action.
        let (state, action) = match self.state {
            // Process UTF-8.
            State::Ground if 0x80 <= byte => self.start_utf8(byte),
            State::Utf8 => self.continue_utf8(byte),

            // Process ASCII and ANSI escape sequences.
            _ => transition(self.state, byte),
        };

        // Handle buffering for escape sequences.
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

        // Perform bookkeeping.
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

    /// Determine the Unicode character for the last step.
    ///
    /// If the last action was `PrintAscii`, `FinishUtf8`, or `ReplaceUtf8`,
    /// this method returns the corresponding Unicode code point. Note that a
    /// code point may or may not represent what we informally call a
    /// "character" or "letter". In other words, code points implement a
    /// variable-length encoding of "characters" and UTF-8 lets bytes implement
    /// a variable-length encoding of code points. It's turtles all the way
    /// down...
    pub fn codepoint(&self) -> Option<char> {
        match self.last_action {
            Action::PrintAscii => Some(self.last_byte as char),
            Action::FinishUtf8 => Some(unsafe { char::from_u32_unchecked(self.utf8_codepoint) }),
            Action::ReplaceUtf8 => Some('\u{FFFD}'),
            _ => None,
        }
    }

    /// Determine the number of processed bytes.
    ///
    /// This method returns the number of bytes processed for the current
    /// character or escape sequence. It only counts bytes that have been
    /// consumed and cannot be part of another escape sequence. As a result, the
    /// value returned by this method usually is one less than the number of
    /// bytes processed by [`VtScanner::step`]. However, when
    /// [`VtScanner::did_finish`] returns `true`, this method catches up with
    /// stepping and returns the accurate number of bytes consumed for that
    /// escape sequence.
    pub fn processed(&self) -> usize {
        if self.did_complete() {
            self.processed + 1
        } else if self.did_abort() {
            if Control::is_sequence_start(self.last_byte) {
                self.processed
            } else {
                self.processed + 1
            }
        } else {
            self.processed
        }
    }

    /// Determine whether the internal buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Determine the current number of buffered bytes.
    ///
    /// This method returns a number between zero and the
    /// [`VtScanner::capacity`].
    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Determine whether the internal buffer did overflow since the last start
    /// action.
    ///
    /// If this method returns `true`, this scanner's internal buffer had
    /// insufficient capacity and cut off the escape sequence's payload. As a
    /// security measure, a scanner's capacity can only be set during creation.
    /// To do so, invoke [`VtScanner::with_capacity`] from Rust or
    /// [`VtScanner`]'s constructor from Python with an appropriate quantity.
    /// The default capacity, i.e., 23 bytes, is barely large enough for
    /// scanning escape sequences with theme colors. Many escape sequences may
    /// be significantly longer.
    #[inline]
    pub fn did_overflow(&self) -> bool {
        self.did_overflow
    }

    /// Determine whether the last step aborted an ANSI escape sequence.
    pub fn did_abort(&self) -> bool {
        // We only need to distinguish between 0, 1, and 2+ transitions.
        let previous = self.previous_state.ground_distance().min(2);
        let current = self.state.ground_distance().min(2);

        if current < previous {
            // If the current state is closer to ground than the previous state,
            // an abort happened if the state machine did not dispatch.
            !self.last_action.is_dispatch()
        } else if current == previous && 0 < current {
            // If neither state is ground and both are equally far from ground,
            // an abort happened if the last action was a start action.
            self.last_action.is_start()
        } else {
            // If both states are ground, there is no escape sequence. If the
            // current state is further from ground than the previous state, an
            // escape sequence is starting. In either case, there was no abort.
            false
        }
    }

    /// Determine whether the last step completed an ANSI escape sequence.
    ///
    /// If this method returns `true`, the control and payload of the escape
    /// sequence are accessible with [`VtScanner::completed_control`],
    /// [`VtScanner::completed_bytes`], and [`VtScanner::completed_str`].
    /// However, the payload is cut off if [`VtScanner::did_overflow`] also
    /// returns `true`.
    #[inline]
    pub fn did_complete(&self) -> bool {
        self.last_action.is_dispatch()
    }

    /// Determine the control for the just completed ANSI escape sequence.
    ///
    /// If this scanner did just complete an ANSI escape sequence, this method
    /// returns the escape sequence's control. Otherwise, it returns `None`.
    #[inline]
    pub fn completed_control(&self) -> Option<Control> {
        self.last_action.control()
    }

    /// Access the payload for the just completed ANSI escape sequence as a byte
    /// slice.
    ///
    /// If [`VtScanner::did_complete`] returns `true`, this method returns the
    /// payload of the escape sequence as a byte slice. However, if
    /// [`VtScanner::did_overflow`] also returns `true`, the payload is
    /// incomplete. If this scanner did not complete an escape sequence, this
    /// method returns an empty byte slice.
    ///
    /// [`VtScanner::completed_str`] does the same, except it returns a string
    /// slice. By contrast, [`VtScanner::finished_bytes`] and
    /// [`VtScanner::finished_str`] return a wrapped byte or string slice and
    /// automatically account for error conditions, too.
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
    /// If [`VtScanner::did_complete`] returns `true`, this method returns the
    /// payload of the escape sequence as a string slice. However, if
    /// [`VtScanner::did_overflow`] also returns `true`, the payload is
    /// incomplete. If this scanner did not complete an escape sequence, this
    /// method returns an empty string slice.
    ///
    /// [`VtScanner::completed_bytes`] does the same, except it returns a byte
    /// slice. By contrast, [`VtScanner::finished_bytes`] and
    /// [`VtScanner::finished_str`] return a wrapped byte or string slice and
    /// automatically account for error conditions, too.
    #[inline]
    pub fn completed_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.completed_bytes())
    }

    /// Determine whether the last step aborted or completed an ANSI escape
    /// sequence.
    #[inline]
    pub fn did_finish(&self) -> bool {
        self.did_abort() || self.did_complete()
    }

    /// Determine the result that finishes the escape sequence.
    ///
    /// If scanning the escape sequence did complete without overflow, this
    /// method returns the payload as a byte slice. Otherwise, it returns an
    /// appropriate I/O error.
    ///
    /// [`VtScanner::finished_str`] does the same, except it returns a string
    /// slice. [`VtScanner::completed_bytes`] and [`VtScanner::completed_str`]
    /// return the byte or string slice without error checking and hence are far
    /// less general.
    pub fn finished_bytes(&self) -> std::io::Result<&[u8]> {
        if self.did_complete() {
            if self.did_overflow() {
                Err(ErrorKind::OutOfMemory.into())
            } else {
                Ok(self.completed_bytes())
            }
        } else if self.did_abort() {
            Err(ErrorKind::InvalidData.into())
        } else {
            Err(ErrorKind::Other.into())
        }
    }

    /// Determine the result that finishes the escape sequence.
    ///
    /// If scanning the escape sequence did complete without overflow, this
    /// method returns the payload as a string slice. Otherwise, it returns an
    /// appropriate I/O error.
    ///
    /// [`VtScanner::finished_bytes`] does the same, except it returns a byte
    /// slice. [`VtScanner::completed_bytes`] and [`VtScanner::completed_str`]
    /// return the byte or string slice without error checking and hence are far
    /// less general.
    pub fn finished_str(&self) -> std::io::Result<&str> {
        let bytes = self.finished_bytes()?;
        std::str::from_utf8(bytes).map_err(|e| Error::new(ErrorKind::InvalidData, e))
    }

    /// Get a debug representation for this scanner. <i class=python-only>Python
    /// only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("Scanner({:?})", self)
    }
}

impl VtScanner {
    fn start_utf8(&mut self, byte: u8) -> (State, Action) {
        self.utf8_state = UTF8_ACCEPT;
        self.utf8_codepoint = 0;
        decode(byte, &mut self.utf8_state, &mut self.utf8_codepoint);

        if self.utf8_state == UTF8_REJECT {
            (State::Ground, Action::ReplaceUtf8)
        } else if self.utf8_state == UTF8_ACCEPT {
            unreachable!("ASCII characters are processed separately.")
        } else {
            (State::Utf8, Action::StartUtf8)
        }
    }

    fn continue_utf8(&mut self, byte: u8) -> (State, Action) {
        decode(byte, &mut self.utf8_state, &mut self.utf8_codepoint);
        if self.utf8_state == UTF8_REJECT {
            (State::Ground, Action::ReplaceUtf8)
        } else if self.utf8_state == UTF8_ACCEPT {
            (State::Ground, Action::FinishUtf8)
        } else {
            (State::Utf8, Action::ContinueUtf8)
        }
    }

    /// Scan an escape sequence and returns its payload as a byte slice. <i
    /// class=rust-only>Rust only!</i>
    ///
    /// This method [`VtScanner::step`]s through the given reader's bytes.
    ///
    /// If the first byte triggers [`Action::Start`], i.e., is the first byte of
    /// an escape sequence, this method consumes the reader's bytes until the
    /// escape sequence is aborted or completed. In the latter case, this method
    /// returns the payload as a byte slice. In either case, this method does
    /// not consume the last byte, if it also starts a new escape sequence.
    ///
    /// If the first byte does not trigger `Start`, this method returns an error
    /// without consuming the byte.
    ///
    /// [`VtScanner::scan_str`] does the same except it returns a string slice.
    pub fn scan_bytes<R: BufRead>(&mut self, reader: &mut R) -> std::io::Result<&[u8]> {
        loop {
            // Make sure lifetime of bytes ends before consume().
            let bytes = reader.fill_buf()?;
            if bytes.is_empty() {
                return Err(ErrorKind::TimedOut.into());
            }

            let filled = bytes.len();
            for byte in bytes {
                let action = self.step(*byte);

                // For an escape sequence, first byte triggers Start
                if self.processed() == 0 && action != Action::Start {
                    return Err(ErrorKind::InvalidData.into());
                } else if self.did_finish() {
                    reader.consume(self.processed());
                    return self.finished_bytes();
                }
            }

            reader.consume(filled);
        }
    }

    /// Scan an escape sequence and returns its payload as a string slice. <i
    /// class=rust-only>Rust only!</i>
    ///
    /// This method [`VtScanner::step`]s through the given reader's bytes.
    ///
    /// If the first byte triggers [`Action::Start`], i.e., is the first byte of
    /// an escape sequence, this method consumes the reader's bytes until the
    /// escape sequence is aborted or completed. In the latter case, this method
    /// returns the payload as a string slice. In either case, this method does
    /// not consume the last byte, if it also starts a new escape sequence.
    ///
    /// If the first byte does not trigger `Start`, this method returns an error
    /// without consuming the byte.
    ///
    /// [`VtScanner::scan_bytes`] does the same, except it returns a byte slice.
    pub fn scan_str<R: BufRead>(&mut self, reader: &mut R) -> std::io::Result<&str> {
        let bytes = self.scan_bytes(reader)?;
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
