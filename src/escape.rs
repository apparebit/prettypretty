//! Processing terminal queries and responses.
//!
//! This module facilitates the integration of terminal I/O with [`VtScanner`].
//! It implements the state machine for recognizing ANSI escape sequences,
//! including DEC extensions. As illustrated in the code examples below, it is
//! best combined with the [`trans`](crate::trans) module's
//! [`ThemeEntry`](crate::trans::ThemeEntry) for querying a terminal for theme
//! colors and parsing its responses.
//!
//! To determine a terminal's current color theme, the application first puts
//! the terminal into cbreak or raw mode and then iterates over
//! [`ThemeEntry::all`](crate::trans::ThemeEntry::all), i.e., the default
//! foreground, default background, and 16 ANSI colors.
//!
//!
//! # Example #1: Byte by Byte
//!
//! The following example code sketches querying the terminal for a theme color
//! and turning the response into a color with help of a [`VtScanner`]:
//!
//! ```
//! # use prettypretty::{Color, error::ColorFormatError};
//! # use prettypretty::escape::VtScanner;
//! # use prettypretty::style::AnsiColor;
//! # use prettypretty::trans::ThemeEntry;
//! // Write `format!("{}", entry)` to terminal to issue the query.
//! let entry = ThemeEntry::Ansi(AnsiColor::Red);
//!
//! // The response should be an ANSI escape sequence like this one.
//! let mut terminal_input = b"\x1b]4;1;rgb:df/28/27\x07".iter();
//!
//! // Let's process the response with a scanner.
//! let mut scanner = VtScanner::new();
//! let color = loop {
//!     // Read byte and feed it to scanner's step() method.
//!     let byte = *terminal_input.next().unwrap();
//!     scanner.step(byte);
//!
//!     if scanner.did_abort() {
//!         // The escape sequence is malformed.
//!         break Err(ColorFormatError::MalformedThemeColor);
//!     } else if scanner.did_complete() {
//!         // Parse the escape sequence's payload as a color.
//!         break scanner
//!             .completed_str()
//!             .or(Err(ColorFormatError::MalformedThemeColor))
//!             .and_then(|payload| entry.parse_response(payload))
//!     }
//!
//!     // Keep on stepping...
//! };
//!
//! assert_eq!(color.unwrap(), Color::from_24bit(0xdf, 0x28, 0x27));
//! ```
//! <div class=color-swatch>
//! <div style="background-color: #df2827;"></div>
//! </div>
//! <br>
//!
//! As shown, consuming an escape sequence requires little more than stepping
//! through the input with [`VtScanner::step`] until either
//! [`VtScanner::did_abort`] or [`VtScanner::did_complete`]. Once complete,
//! [`VtScanner::completed_bytes`] and [`VtScanner::completed_str`] return the
//! escape sequence's payload. The example uses
//! [`ThemeEntry::parse_response`](crate::trans::ThemeEntry::parse_response) to
//! parse the payload into a color.
//!
//!
//! # Error Handling
//!
//! The above code is functional but not very robust. It fails to handle three
//! critical error conditions. Let's discuss each in turn.
//!
//! First, a terminal's input may contain content other than escape sequences.
//! While [`VtScanner`] knows how to handle that, the above example code does
//! not. Fixing that requires checking whether the result of [`VtScanner::step`]
//! for the first byte is [`Action::Start`] and otherwise leaving the input
//! untouched (which requires a look-ahead of one byte).
//!
//! Second, [`VtScanner`] buffers an escape sequence's payload. Since terminal
//! input cannot be trusted, the buffer capacity cannot be changed after
//! creation of a scanner. [`VtScanner::new`] allocates a buffer barely large
//! enough for processing terminal colors but no more. An application can check
//! whether the payload did fit into the buffer with [`VtScanner::did_overflow`].
//!
//! Third, [`VtScanner`] can detect the end of a well-formed escape sequence
//! without look-ahead. In that case, [`VtScanner::step`] returns one of the
//! dispatch actions and [`VtScanner::did_complete`] returns `true`. However, it
//! cannot detect all malformed escape sequences without looking at the next
//! byte. In particular, if a byte starts a new escape sequence, i.e.,
//! [`Control::is_sequence_start`] returns `true`, it also aborts the current
//! escape sequence. In that case, an application should effectively put the
//! byte back into input stream.
//!
//!
//! # Example #2: With a Buffered Reader
//!
//! One-byte look-ahead requires some form of buffering. Rust's
//! `std::io::BufRead` trait fits the bill quite nicely:
//!
//! ```
//! # use std::io::{BufRead, Error, ErrorKind};
//! # use prettypretty::Color;
//! # use prettypretty::escape::{Action, Control, VtScanner};
//! # use prettypretty::style::AnsiColor;
//! # use prettypretty::trans::ThemeEntry;
//! let entry = ThemeEntry::Ansi(AnsiColor::Red);
//! let mut terminal_input = b"\x1b]4;1;rgb:df/28/27\x07".as_slice();
//!
//! let mut scanner = VtScanner::new();
//! let mut first = true;
//!
//! let response = 'colorful: loop {
//!     // Track how many bytes to consume.
//!     let mut count = 0;
//!     let bytes = terminal_input.fill_buf()?;
//!
//!     for byte in bytes {
//!         let action = scanner.step(*byte);
//!
//!         if first {
//!             // Make sure the first byte starts escape sequence.
//!             first = false;
//!             if action != Action::Start {
//!                 return Err(ErrorKind::InvalidData.into());
//!             }
//!         } else if scanner.did_abort() {
//!             // Determine whether to consume last byte.
//!             if !Control::is_sequence_start(*byte) {
//!                 count += 1;
//!             }
//!             terminal_input.consume(count);
//!             return Err(ErrorKind::InvalidData.into());
//!
//!         } else if scanner.did_complete() {
//!             // Always consume last byte.
//!             terminal_input.consume(count + 1);
//!             if scanner.did_overflow() {
//!                 return Err(ErrorKind::OutOfMemory.into());
//!             } else {
//!                 break 'colorful scanner.completed_str()
//!                     .map_err(|e| Error::new(
//!                         ErrorKind::InvalidData, e
//!                     ))?;
//!             }
//!         }
//!
//!         // The byte is safe to consume.
//!         count += 1;
//!     }
//!
//!     // Consume buffer before trying to fill another.
//!     terminal_input.consume(count);
//! };
//!
//! // Parse payload and validate color.
//! let color = entry
//!     .parse_response(response)
//!     .map_err(|e| Error::new(ErrorKind::InvalidData, e));
//! assert_eq!(color.unwrap(), Color::from_24bit(0xdf, 0x28, 0x27));
//! # Ok::<(), Error>(())
//! ```
//! <div class=color-swatch>
//! <div style="background-color: #df2827;"></div>
//! </div>
//! <br>
//!
//! As so happens, the above code example pretty much covers the functionality
//! of [`VtScanner::scan_bytes`], except that the latter does not parse a color
//! from the payload and replaces the two conditional blocks gated by
//! [`VtScanner::did_abort`] and [`VtScanner::did_complete`] with the more
//! concise expression:
//!
//! ```ignore
//! if self.did_finish() {
//!     self.consume_on_finish(count, reader);
//!     break 'colorful self.str_on_finish()?;
//! }
//! ```
//!
//! [`VtScanner::did_finish`] checks whether an escape sequence was aborted or
//! completed. [`VtScanner::consume_on_finish`] invokes the reader's `consume`
//! method with the correct number of bytes, which does not include the current
//! byte if it starts a new escape sequence. [`VtScanner::bytes_on_finish`] and
//! [`VtScanner::str_on_finish`] return the escape sequence payload as a byte or
//! string slice, respectively, wrapped in a result—or an I/O error.

use std::io::{BufRead, Error, ErrorKind};

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

    /// Determine the minimum number of transitions required to reach this
    /// state from the ground state using C0 controls only.
    pub fn ground_distance(&self) -> usize {
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
        0x20..=0x7f => (Ground, Print),
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
/// machine model:
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
/// this version probably needs to reparse (parts of) escape sequences. As a
/// result, this version is better suited to applications that need to scan
/// terminal input for responses to queries than the implementation of a
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
/// The documentation for the [`escape`](crate::escape) module provides more
/// detail and an example illustrating how to use synchronous I/O with `BufRead`
/// to correctly handle all three corner cases.
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
    ///
    /// Unlike `Vec<T>`, a scanner's capacity does not change after creation.
    /// This is a security precaution, since the bytes processed by this type
    /// may originate from untrusted users.
    #[inline]
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
            // an abort happened if the last action as a start action.
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
    /// slice.
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
    /// slice.
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
    /// [`VtScanner::str_on_finish`] does the same, except it returns a string
    /// slice.
    pub fn bytes_on_finish(&self) -> std::io::Result<&[u8]> {
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
    /// [`VtScanner::bytes_on_finish`] does the same, except it returns a byte
    /// slice.
    pub fn str_on_finish(&self) -> std::io::Result<&str> {
        let bytes = self.bytes_on_finish()?;
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
    /// Consume processed bytes from reader, while also accounting for the
    /// current byte.
    ///
    /// If [`VtScanner::did_finish`] returns `true`, this method consumes at
    /// least processed bytes from the reader. It also consumes the most recent
    /// byte—as long as that byte did not abort the escape sequence while also
    /// starting a new one. If this scanner did not finish an escape sequence,
    /// this method does nothing.
    pub fn consume_on_finish<R: BufRead>(&self, processed: usize, reader: &mut R) {
        let processed = if self.did_complete() {
            processed + 1
        } else if self.did_abort() {
            if Control::is_sequence_start(self.last_byte) {
                processed
            } else {
                processed + 1
            }
        } else {
            return;
        };

        reader.consume(processed);
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
        let mut first = true;

        loop {
            let mut count = 0;
            let bytes = reader.fill_buf()?;

            for byte in bytes {
                let action = self.step(*byte);

                // For an escape sequence, first byte triggers Start
                if first {
                    first = false;
                    if action != Action::Start {
                        return Err(ErrorKind::InvalidData.into());
                    }
                } else if self.did_finish() {
                    self.consume_on_finish(count, reader);
                    return self.bytes_on_finish();
                }

                count += 1;
            }

            reader.consume(count);
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
