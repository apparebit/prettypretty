use std::io::Result;

use crate::util::nicely_str;

/// A command for the terminal.
///
/// Commands provide instructions to the terminal and are communicated in-band
/// by writing ANSI escape codes. The actual writing is performed by the display
/// trait's `fmt` method.
///
/// This trait is object-safe.
pub trait Command: std::fmt::Display {}

/// A borrowed command is a command.
impl<C: Command + ?Sized> Command for &C {}

/// A boxed command is a command.
impl<C: Command + ?Sized> Command for Box<C> {}

/// Combine several commands into a single new command.
///
/// The new command preserves the order of its component commands. Upon display,
/// it emits as many ANSI escape sequence as it has component commands.
#[macro_export]
macro_rules! fuse {
    ($($command:expr),+ $(,)?) => {{
        /// One or more combined commands.
        struct Fused;

        impl $crate::Command for Fused {}
        impl ::std::fmt::Display for Fused {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                $($command.fmt(f)?;)*
            }
        }

        Fused
    }}
}

// ------------------------------------------------------------------------------------------------

/// A command using select-graphic-rendition ANSI escape sequences.
///
/// To facilitate composition, SGR commands implement [`Sgr::write_param`],
/// which writes the parameter(s) with the leading `CSI` and the trailing `m`.
///
/// Technically, an `impl &mut std::fmt::Write` would suffice for `out`, but
/// that would make the method generic and hence also prevent the trait from
/// being object-safe. Declaring `out` to be a formatter instead doesn't
/// restrict the trait by much, since `write_param()` is most likely invoked
/// inside an implementation of `Display::fmt` anyways, while also ensuring that
/// the trait is object-safe.
pub trait Sgr: Command {
    /// Write the parameter(s) for this SGR command.
    fn write_param(&self, out: &mut std::fmt::Formatter<'_>) -> ::std::fmt::Result;
}

/// A borrowed SGR is an SGR.
impl<S: Sgr + ?Sized> Sgr for &S {
    fn write_param(&self, out: &mut std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        (**self).write_param(out)
    }
}

/// A boxed SGR is an SGR.
impl<S: Sgr + ?Sized> Sgr for Box<S> {
    fn write_param(&self, out: &mut std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        (**self).write_param(out)
    }
}

/// Combine several SGR commands into a single new SGR command.
///
/// The new SGR command preserves the order of its component commands. Upon
/// display, it emits only one ANSI escape sequence.
#[macro_export]
macro_rules! fuse_sgr {
    ( $sgr:expr, $( $sgr2:expr ),* $(,)? ) => {{
        /// One or more SGR commands fused into one.
        struct FusedSgr;

        impl ::std::fmt::Display for FusedSgr {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str("\x1b[")?;
                self.write_param(f)?;
                f.write_str("m")
            }
        }

        impl $crate::Command for FusedSgr {}
        impl $crate::Sgr for FusedSgr {
            fn write_param(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                $sgr.write_param(f)?;
                $(
                    f.write_str(";")?;
                    $sgr2.write_param(f)?;
                )*
                Ok(())
            }
        }

        FusedSgr
    }};
}

// ------------------------------------------------------------------------------------------------

/// Control codes that start or end ANSI escape sequences.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Control {
    /// Bell (C0)
    BEL = 0x07,
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
    /// String Terminator: `ESC \\` (C0) or 0x9c (C1)
    ST = 0x9c,
    /// Operating System Command: `ESC ]` (C0) or 0x9d (C1)
    OSC = 0x9d,
    /// Privacy Message: `ESC ^` (C0) or 0x9e (C1)
    PM = 0x9e,
    /// Application Program Command: `ESC _` (C0) or 0x9f (C1)
    APC = 0x9f,
}

impl std::fmt::Display for Control {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use self::Control::*;

        f.write_str(match self {
            BEL => "\x07",
            ESC => "\x1b",
            DCS => "\x1bP",
            SOS => "\x1bX",
            SS2 => "\x1bN",
            SS3 => "\x1bO",
            CSI => "\x1b[",
            ST => "\x1b\\",
            OSC => "\x1b]",
            PM => "\x1b^",
            APC => "\x1b_",
        })
    }
}

/// A command that receives a response.
///
/// Queries are request/response interactions with the terminal. For purposes of
/// this trait, the response consists of a control followed by the payload
/// optionally followed by another control (usually `BEL` or `ST`). The control
/// is not represented by a constant but rather returned by a method, so that
/// the trait remains object-safe.
///
///
/// # Example
///
/// To process a query's response, [`Scan::read_token()`] and ensure that the
/// [`Token`] is a sequence with a [`Query::control()`]. Then [`Query::parse`]
/// the payload.
///
/// ```
/// # use prettytty::{Connection, Query, Scan, Token};
/// # use prettytty::cmd::{MoveToColumn, RequestCursorPosition};
/// # use prettytty::err::ErrorKind;
/// # use prettytty::opt::Options;
/// # let options = Options::default();
/// # let tty = match Connection::with_options(options) {
/// #     Ok(tty) => tty,
/// #     Err(err) if err.kind() == std::io::ErrorKind::ConnectionRefused => return Ok(()),
/// #     Err(err) => return Err(err),
/// # };
/// # let pos = {
/// # let (mut input, mut output) = tty.io();
/// # output.exec(MoveToColumn::<17>)?;
/// // Write the command
/// output.exec(RequestCursorPosition)?;
///
/// // Read the token
/// let token = input.read_token()?;
///
/// // Extract and parse payload
/// if let Token::Sequence(control, payload) = token {
///     if control == RequestCursorPosition.control() {
///         RequestCursorPosition.parse(payload)?
///     } else {
///         return Err(ErrorKind::BadControl.into());
///     }
/// } else {
///     return Err(ErrorKind::NotASequence.into());
/// }
/// # };
/// # drop(tty);
/// # assert_eq!(pos.1, 17);
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// In the above example, generating precise errors requires about as much code
/// as extracting and parsing the payload. The [`Scan::read_sequence`] method
/// abstracts over this boilerplate.
pub trait Query: Command {
    /// The type of the response data.
    type Response;

    /// Get the response's control.
    fn control(&self) -> Control;

    /// Parse the payload into a response object.
    fn parse(&self, payload: &[u8]) -> Result<Self::Response>;
}

/// A borrowed query is a query.
impl<Q: Query + ?Sized> Query for &Q {
    type Response = Q::Response;

    fn control(&self) -> Control {
        (**self).control()
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        (**self).parse(payload)
    }
}

/// A boxed query is a query.
impl<Q: Query + ?Sized> Query for Box<Q> {
    type Response = Q::Response;

    fn control(&self) -> Control {
        (**self).control()
    }

    fn parse(&self, payload: &[u8]) -> Result<Self::Response> {
        (**self).parse(payload)
    }
}

// ------------------------------------------------------------------------------------------------

/// A text or control sequence token.
#[derive(Clone, PartialEq)]
pub enum Token<'t> {
    /// One or more UTF-8 characters excluding C0 and C1 controls.
    Text(&'t [u8]),
    /// A C0 or C1 control that doesn't start or end a sequence.
    Control(&'t [u8]),
    /// A control sequence with its initial control and payload.
    Sequence(Control, &'t [u8]),
}

impl Token<'_> {
    /// Get this token's control.
    pub fn control(&self) -> Option<Control> {
        match self {
            Token::Sequence(control, _) => Some(*control),
            _ => None,
        }
    }

    /// Get this token's character data.
    pub fn data(&self) -> &[u8] {
        use self::Token::*;

        match self {
            Text(data) => data,
            Control(data) => data,
            Sequence(_, data) => data,
        }
    }
}

impl std::fmt::Debug for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Text(_) => "Text",
            Self::Control(_) => "Control",
            Self::Sequence(_, _) => "Sequence",
        };

        let mut debug = f.debug_tuple(name);
        if let Some(control) = self.control() {
            debug.field(&control);
        }
        debug.field(&nicely_str(self.data())).finish()
    }
}

/// A scanner for UTF-8 characters and control sequences.
///
/// An implementation of this trait provides the state machine necessary for
/// scanning UTF-8 characters and control sequences. Since scanning control
/// sequences requires one byte lookahead, an implementation also buffers the
/// data it reads from the terminal.
///
/// Some terminal emulators may require more than one read from terminal input
/// to consume a complete ANSI escape sequence serving as query response. Hence
/// [`Scan::read_token`] may perform an arbitrary number of reads from the
/// underlying input, including none, to recognize a complete control sequence.
/// However, an implementation must not issue reads after it has started
/// recognizing a text token. In other words, when reading a text token, the end
/// of buffered data also is the end of the text token.
///
/// This trait is object-safe.
pub trait Scan: std::io::BufRead {
    /// Determine if the state machine currently is in-flight.
    ///
    /// Using a scanner as a reader is only safe if this method returns `false`.
    fn in_flight(&self) -> bool;

    /// Read the next token.
    ///
    /// If the internal buffer has been exhausted, this method may read from the
    /// connection upon invocation. For text tokens, it performs no further
    /// reads. That is, a text token always ends with the currently buffered
    /// data.
    fn read_token(&mut self) -> Result<Token>;

    /// Read the next token as a control sequence.
    ///
    /// This method reads the next token and, after making sure it is a control
    /// sequence starting with the given control, returns the payload.
    fn read_sequence(&mut self, control: Control) -> Result<&[u8]> {
        match self.read_token()? {
            Token::Sequence(actual, payload) => {
                if actual == control {
                    Ok(payload)
                } else {
                    Err(crate::err::ErrorKind::BadControl.into())
                }
            }
            _ => Err(crate::err::ErrorKind::NotASequence.into()),
        }
    }
}

/// A mutably borrowed scanner is a scanner.
impl<S: Scan + ?Sized> Scan for &mut S {
    #[inline]
    fn in_flight(&self) -> bool {
        (**self).in_flight()
    }

    #[inline]
    fn read_token(&mut self) -> Result<Token> {
        (**self).read_token()
    }
}

/// A boxed scanner is a scanner.
impl<S: Scan + ?Sized> Scan for Box<S> {
    #[inline]
    fn in_flight(&self) -> bool {
        (**self).in_flight()
    }

    #[inline]
    fn read_token(&mut self) -> Result<Token> {
        (**self).read_token()
    }
}

fn _assert_traits_are_object_safe<T>() {
    fn is_object_safe<T: ?Sized>() {}

    is_object_safe::<dyn Command>();
    is_object_safe::<dyn Sgr>();
    is_object_safe::<dyn Query<Response = T>>();
    is_object_safe::<dyn Scan>();
}
