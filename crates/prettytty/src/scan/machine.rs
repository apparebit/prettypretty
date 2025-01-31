// State Machine States, Actions, and Transitions
use crate::Control;

/// The current state when processing control (sequences).
#[derive(Clone, Copy, Debug)]
pub(super) enum State {
    Ground,
    Escape,
    EscapeIntermediate,
    SingleShift,
    StringBody,
    StringEnd,
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
}

/// The next action to take.
#[derive(Clone, Copy, Debug)]
pub(super) enum Action {
    /// Print the text character.
    Print,
    /// Start a new escape sequence. Unfortunately, a new sequence may also
    /// start with an AbortThenRetry action. Either way, the transition returns
    /// a control.
    StartSequence,
    /// Ignore the current byte, even though we are scanning an escape sequence.
    IgnoreByte,
    /// Retain the current byte as part of an escape sequence. For CSI, ESC, SS2,
    /// and SS3 escape sequences, the Dispatch action also requires retaining the
    /// current byte.
    RetainByte,
    /// Abort the current escape sequence. Also consume the current byte.
    AbortSequence,
    /// Abort the current escape sequence. Then try transitioning the current
    /// byte again. This can only work if the new state for AbortThenRetry is
    /// the same state as that for the intended retry action, i.e., typically
    /// ground. If it isn't ground, then special care must be taken so that the
    /// rest of the scanner's state is correctly maintained.
    AbortThenRetry,
    /// Dispatch the escape sequence.
    Dispatch,
    /// Handle the current byte as a control character (which may appear in the
    /// middle of an escape sequence).
    HandleControl,
}

// ------------------------------------------------------------------------------------------------

const fn otherwise(byte: u8, state: State) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (state, HandleControl, None),
        0x18 => (Ground, AbortThenRetry, None),
        0x1a => (Ground, AbortThenRetry, None),
        0x1b => (Ground, AbortThenRetry, None),
        0x20..=0x7e => (state, IgnoreByte, None),
        0x7f => (state, IgnoreByte, None),
        0x9c => (Ground, AbortSequence, None),
        0x80..0xa0 => (Ground, AbortThenRetry, None),
        _ => (state, IgnoreByte, None),
    }
}

// ------------------------------------------------------------------------------------------------
// Ground

const fn ground(byte: u8) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x18 => (Ground, HandleControl, None),
        0x1a => (Ground, HandleControl, None),
        0x1b => (Escape, StartSequence, Some(Control::ESC)),
        0x20..=0x7f => (Ground, Print, None),
        0x80..=0x8d | 0x91..=0x97 | 0x99 | 0x9a => (Ground, HandleControl, None),
        0x8e => (SingleShift, StartSequence, Some(Control::SS2)),
        0x8f => (SingleShift, StartSequence, Some(Control::SS3)),
        0x90 => (DcsEntry, StartSequence, Some(Control::DCS)),
        0x98 => (StringBody, StartSequence, Some(Control::SOS)),
        0x9b => (CsiEntry, StartSequence, Some(Control::CSI)),
        0x9c => (Ground, IgnoreByte, None),
        0x9d => (StringBody, StartSequence, Some(Control::OSC)),
        0x9e => (StringBody, StartSequence, Some(Control::PM)),
        0x9f => (StringBody, StartSequence, Some(Control::APC)),
        0xa0..=0xff => (Ground, Print, None),
        _ => otherwise(byte, Ground),
    }
}

// ------------------------------------------------------------------------------------------------
// Escape

const fn escape(byte: u8) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x20..=0x2f => (EscapeIntermediate, RetainByte, None),
        0x30..=0x4d | 0x51..=0x57 | 0x59 | 0x5a | 0x5c | 0x60..=0x7e => (Ground, Dispatch, None),
        0x4e => (SingleShift, IgnoreByte, Some(Control::SS2)),
        0x4f => (SingleShift, IgnoreByte, Some(Control::SS3)),
        0x50 => (DcsEntry, IgnoreByte, Some(Control::DCS)),
        0x58 => (StringBody, IgnoreByte, Some(Control::SOS)),
        0x5b => (CsiEntry, IgnoreByte, Some(Control::CSI)),
        0x5d => (StringBody, IgnoreByte, Some(Control::OSC)),
        0x5e => (StringBody, IgnoreByte, Some(Control::PM)),
        0x5f => (StringBody, IgnoreByte, Some(Control::APC)),
        _ => otherwise(byte, Escape),
    }
}

const fn escape_intermediate(byte: u8) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x20..=0x2f => (EscapeIntermediate, RetainByte, None),
        0x30..=0x7e => (Ground, Dispatch, None),
        _ => otherwise(byte, EscapeIntermediate),
    }
}

// ------------------------------------------------------------------------------------------------
// SS2, SS3

const fn single_shift(byte: u8) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x20..=0x7e => (Ground, Dispatch, None),
        _ => otherwise(byte, SingleShift),
    }
}

// ------------------------------------------------------------------------------------------------
// APC, PM, OSC, SOS

const fn string_body(byte: u8) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x00..=0x06 | 0x08..=0x17 | 0x19 | 0x1c..=0x1f => (StringBody, IgnoreByte, None),
        0x07 => (Ground, Dispatch, None),
        0x1b => (StringEnd, IgnoreByte, None),
        0x20..=0x7f => (StringBody, RetainByte, None),
        0x9c => (Ground, Dispatch, None),
        _ => otherwise(byte, StringBody),
    }
}

const fn string_end(byte: u8) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x5c => (Ground, Dispatch, None),
        _ => (Escape, AbortThenRetry, Some(Control::ESC)),
    }
}

// ------------------------------------------------------------------------------------------------
// CSI

const fn csi_entry(byte: u8) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x20..=0x2f => (CsiIntermediate, RetainByte, None),
        0x30..=0x39 | 0x3b..=0x3f => (CsiParam, RetainByte, None),
        0x3a => (CsiIgnore, IgnoreByte, None),
        0x40..=0x7e => (Ground, Dispatch, None),
        _ => otherwise(byte, CsiEntry),
    }
}

const fn csi_param(byte: u8) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x20..=0x2f => (CsiIntermediate, RetainByte, None),
        0x30..=0x39 | 0x3b => (CsiParam, RetainByte, None),
        0x3a | 0x3c..=0x3f => (CsiIgnore, IgnoreByte, None),
        0x40..=0x7e => (Ground, Dispatch, None),
        _ => otherwise(byte, CsiParam),
    }
}

const fn csi_intermediate(byte: u8) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x20..=0x2f => (CsiIntermediate, RetainByte, None),
        0x30..=0x3f => (CsiIgnore, IgnoreByte, None),
        0x40..=0x7e => (Ground, Dispatch, None),
        _ => otherwise(byte, CsiIntermediate),
    }
}

const fn csi_ignore(byte: u8) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x20..=0x3f => (CsiIgnore, IgnoreByte, None),
        0x40..=0x7e => (Ground, AbortSequence, None),
        _ => otherwise(byte, CsiIgnore),
    }
}

// ------------------------------------------------------------------------------------------------
// DCS

const fn dcs_entry(byte: u8) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (DcsEntry, IgnoreByte, None),
        0x20..=0x2f => (DcsIntermediate, RetainByte, None),
        0x30..=0x39 | 0x3b..=0x3f => (DcsParam, RetainByte, None),
        0x3a => (DcsIgnore, IgnoreByte, None),
        0x40..=0x7e => (DcsPassthrough, RetainByte, None),
        _ => otherwise(byte, DcsEntry),
    }
}

const fn dcs_param(byte: u8) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (DcsParam, IgnoreByte, None),
        0x20..=0x2f => (DcsIntermediate, RetainByte, None),
        0x30..=0x39 | 0x3b => (DcsParam, RetainByte, None),
        0x3a | 0x3c..=0x3f => (DcsIgnore, IgnoreByte, None),
        0x40..=0x7e => (DcsPassthrough, RetainByte, None),
        _ => otherwise(byte, DcsParam),
    }
}

const fn dcs_intermediate(byte: u8) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (DcsIntermediate, IgnoreByte, None),
        0x20..=0x2f => (DcsIntermediate, RetainByte, None),
        0x30..=0x3f => (DcsIgnore, IgnoreByte, None),
        0x40..=0x7e => (DcsPassthrough, RetainByte, None),
        _ => otherwise(byte, DcsIntermediate),
    }
}

const fn dcs_passthrough(byte: u8) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x00..=0x06 | 0x08..=0x17 | 0x19 | 0x1c..=0x1f => (DcsPassthrough, RetainByte, None),
        0x07 => (Ground, Dispatch, None),
        0x1b => (DcsPassthroughEnd, IgnoreByte, None),
        0x20..=0x7e => (DcsPassthrough, RetainByte, None),
        0x9c => (Ground, Dispatch, None),
        _ => otherwise(byte, DcsPassthrough),
    }
}

const fn dcs_passthrough_end(byte: u8) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x5c => (Ground, Dispatch, None),
        _ => (Escape, AbortThenRetry, Some(Control::ESC)),
    }
}

const fn dcs_ignore(byte: u8) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x00..=0x06 | 0x08..=0x17 | 0x19 | 0x1c..=0x1f => (DcsIgnore, IgnoreByte, None),
        0x07 => (Ground, AbortSequence, None),
        0x1b => (DcsIgnoreEnd, IgnoreByte, None),
        0x20..=0x7f => (DcsIgnore, IgnoreByte, None),
        0x9c => (Ground, AbortSequence, None),
        _ => otherwise(byte, DcsIgnore),
    }
}

const fn dcs_ignore_end(byte: u8) -> (State, Action, Option<Control>) {
    use self::Action::*;
    use self::State::*;

    match byte {
        0x5c => (Ground, AbortSequence, None),
        _ => otherwise(byte, Ground),
    }
}

// ------------------------------------------------------------------------------------------------
// Complete transition function

/// Determine the next state and action.
///
/// # Panics
///
/// If the current byte is a visible ASCII character or starts a UTF-8 sequence
/// and the current state is ground.
pub(super) const fn transition(state: State, byte: u8) -> (State, Action, Option<Control>) {
    use self::State::*;

    match state {
        Ground => ground(byte),
        Escape => escape(byte),
        EscapeIntermediate => escape_intermediate(byte),
        SingleShift => single_shift(byte),
        StringBody => string_body(byte),
        StringEnd => string_end(byte),
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
