// State Machine States, Actions, and Transitions

/// Control codes that start ANSI escape sequences.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Control {
    /// Escape (C0)
    ESC = 0x1b,
    /// Device control string: `ESC P` (C0) or 0x90 (C1)
    DCS = 0x90,
    /// Start of String: `ESC X` (C0) or 0x98 (C1)
    SOS = 0x98,
    /// Single Shift 2: `ESC N` (C0) or 0x8e (C1)
    SS2 = 0x8e,
    /// Single Shift 3: `ESC O` (C0) or 0x8f (C1)
    SS3 = 0x8f,
    /// Control Sequence Introducer: `ESC [` (C0) or 0x9b (C1)
    CSI = 0x9b,
    /// Operating System Command: `ESC ]` (C0) or 0x9d (C1)
    OSC = 0x9d,
    /// Privacy Message: `ESC ^` (C0) or 0x9e (C1)
    PM = 0x9e,
    /// Application Program Command: `ESC _` (C0) or 0x9f (C1)
    APC = 0x9f,
}

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
    Print,
    StartSequence,
    IgnoreByte,
    RetainByte,
    AbortSequence,
    AbortThenHandleControl,
    AbortThenStart,
    Dispatch,
    HandleControl,
}

// ------------------------------------------------------------------------------------------------

const fn otherwise(byte: u8, state: State) -> (State, Action, Option<Control>) {
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (state, HandleControl, None),
        0x18 => (Ground, AbortThenHandleControl, None),
        0x1a => (Ground, AbortThenHandleControl, None),
        0x1b => (Escape, AbortThenStart, Some(Control::ESC)),
        0x20..=0x7e => (state, IgnoreByte, None),
        0x7f => (state, IgnoreByte, None),
        0x80..=0x8d | 0x91..=0x97 | 0x99 | 0x9a => (Ground, AbortThenHandleControl, None),
        0x8e => (SingleShift, AbortThenStart, Some(Control::SS2)),
        0x8f => (SingleShift, AbortThenStart, Some(Control::SS3)),
        0x90 => (DcsEntry, AbortThenStart, Some(Control::DCS)),
        0x98 => (StringBody, AbortThenStart, Some(Control::SOS)),
        0x9b => (CsiEntry, AbortThenStart, Some(Control::CSI)),
        0x9c => (Ground, AbortSequence, None),
        0x9d => (StringBody, AbortThenStart, Some(Control::OSC)),
        0x9e => (StringBody, AbortThenStart, Some(Control::PM)),
        0x9f => (StringBody, AbortThenStart, Some(Control::APC)),
        _ => (state, IgnoreByte, None),
    }
}

// ------------------------------------------------------------------------------------------------
// Ground

const fn ground(byte: u8) -> (State, Action, Option<Control>) {
    use Action::*;
    use State::*;

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
    use Action::*;
    use State::*;

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
    use Action::*;
    use State::*;

    match byte {
        0x20..=0x2f => (EscapeIntermediate, RetainByte, None),
        0x30..=0x7e => (Ground, Dispatch, None),
        _ => otherwise(byte, EscapeIntermediate),
    }
}

// ------------------------------------------------------------------------------------------------
// SS2, SS3

const fn single_shift(byte: u8) -> (State, Action, Option<Control>) {
    use Action::*;
    use State::*;

    match byte {
        0x20..=0x7e => (Ground, Dispatch, None),
        _ => otherwise(byte, SingleShift),
    }
}

// ------------------------------------------------------------------------------------------------
// APC, PM, OSC, SOS

const fn string_body(byte: u8) -> (State, Action, Option<Control>) {
    use Action::*;
    use State::*;

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
    use Action::*;
    use State::*;

    match byte {
        0x5c => (Ground, Dispatch, None),
        _ => otherwise(byte, Ground),
    }
}

// ------------------------------------------------------------------------------------------------
// CSI

const fn csi_entry(byte: u8) -> (State, Action, Option<Control>) {
    use Action::*;
    use State::*;

    match byte {
        0x20..=0x2f => (CsiIntermediate, RetainByte, None),
        0x30..=0x39 | 0x3b..=0x3f => (CsiParam, RetainByte, None),
        0x3a => (CsiIgnore, IgnoreByte, None),
        0x40..=0x7e => (Ground, Dispatch, None),
        _ => otherwise(byte, CsiEntry),
    }
}

const fn csi_param(byte: u8) -> (State, Action, Option<Control>) {
    use Action::*;
    use State::*;

    match byte {
        0x20..=0x2f => (CsiIntermediate, RetainByte, None),
        0x30..=0x39 | 0x3b => (CsiParam, RetainByte, None),
        0x3a | 0x3c..=0x3f => (CsiIgnore, IgnoreByte, None),
        0x40..=0x7e => (Ground, Dispatch, None),
        _ => otherwise(byte, CsiParam),
    }
}

const fn csi_intermediate(byte: u8) -> (State, Action, Option<Control>) {
    use Action::*;
    use State::*;

    match byte {
        0x20..=0x2f => (CsiIntermediate, RetainByte, None),
        0x30..=0x3f => (CsiIgnore, IgnoreByte, None),
        0x40..=0x7e => (Ground, Dispatch, None),
        _ => otherwise(byte, CsiIntermediate),
    }
}

const fn csi_ignore(byte: u8) -> (State, Action, Option<Control>) {
    use Action::*;
    use State::*;

    match byte {
        0x20..=0x3f => (CsiIgnore, IgnoreByte, None),
        0x40..=0x7e => (Ground, AbortSequence, None),
        _ => otherwise(byte, CsiIgnore),
    }
}

// ------------------------------------------------------------------------------------------------
// DCS

const fn dcs_entry(byte: u8) -> (State, Action, Option<Control>) {
    use Action::*;
    use State::*;

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
    use Action::*;
    use State::*;

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
    use Action::*;
    use State::*;

    match byte {
        0x00..=0x17 | 0x19 | 0x1c..=0x1f => (DcsIntermediate, IgnoreByte, None),
        0x20..=0x2f => (DcsIntermediate, RetainByte, None),
        0x30..=0x3f => (DcsIgnore, IgnoreByte, None),
        0x40..=0x7e => (DcsPassthrough, RetainByte, None),
        _ => otherwise(byte, DcsIntermediate),
    }
}

const fn dcs_passthrough(byte: u8) -> (State, Action, Option<Control>) {
    use Action::*;
    use State::*;

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
    use Action::*;
    use State::*;

    match byte {
        0x5c => (Ground, Dispatch, None),
        _ => otherwise(byte, Ground),
    }
}

const fn dcs_ignore(byte: u8) -> (State, Action, Option<Control>) {
    use Action::*;
    use State::*;

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
    use Action::*;
    use State::*;

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
    use State::*;

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
