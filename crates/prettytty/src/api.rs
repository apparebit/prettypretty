use std::io::Result;

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

/// A command for the terminal.
///
/// Commands provide instructions to the terminal and are communicated in-band
/// by writing ANSI escape codes. The actual writing is performed by the display
/// trait's `fmt` method.
pub trait Command: std::fmt::Display {}

/// A borrowed command is a command.
impl<C: Command + ?Sized> Command for &C {}

/// A boxed command is a command.
impl<C: Command + ?Sized> Command for Box<C> {}

/// A command using select-graphic-rendition ANSI escape sequences.
///
/// To facilitate composition, SGR commands implement [`Sgr::write_param`],
/// which write the parameter(s) with the leading `CSI` and the trailing `m`.
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

/// A macro to compose several SGR commands into a composite one.
#[macro_export]
macro_rules! sgr {
    ( $sgr:expr, $( $sgr2:expr ),* $(,)? ) => {{
        struct SgrSeq;

        impl std::fmt::Display for SgrSeq {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("\x1b[")?;
                $sgr.write_param(f)?;
                $(
                    f.write_str(";")?;
                    $sgr2.write_param(f)?;
                )*
                f.write_str("m")
            }
        }

        impl $crate::Command for SgrSeq {}
        impl $crate::Sgr for SgrSeq {
            fn write_param(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt:Result {
                $sgr.write_param(f)?;
                $(
                    f.write_str(";")?;
                    $sgr2.write_param(f)?;
                )*
                Ok(())
            }
        }

        SgrSeq
    }};
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
/// # let rcp = RequestCursorPosition;
/// # let tty = Connection::open()?;
/// # let pos = {
/// # let (mut output, mut input) = (tty.output(), tty.input());
/// # output.exec(MoveToColumn(17))?;
/// # output.exec(rcp)?;
/// match input.read_token()? {
///     Token::Sequence(ctrl, payload) if ctrl == rcp.control() => {
///         rcp.parse(payload)?
///     }
///     Token::Sequence(_, _) => return Err(ErrorKind::BadControl.into()),
///     _ => return Err(ErrorKind::NotASequence.into()),
/// }
/// # };
/// # drop(tty);
/// # assert_eq!(pos.1, 17);
/// # Ok::<(), std::io::Error>(())
/// ```
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

/// A text or control sequence token.
#[derive(Clone, Debug, PartialEq)]
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

/// A scanner for UTF-8 characters and control sequences.
///
/// An implementation of this trait implements the state machine necessary for
/// scanning UTF-8 characters and control sequences. It also buffers the data it
/// reads from the terminal.
pub trait Scan: std::io::BufRead {
    /// Determine if the state machine currently is in-flight.
    ///
    /// Using a scanner as a reader is only safe if this method returns `false`.
    fn in_flight(&self) -> bool;

    /// Read the next token.
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
    fn in_flight(&self) -> bool {
        (**self).in_flight()
    }

    fn read_token(&mut self) -> Result<Token> {
        (**self).read_token()
    }
}

/// A boxed scanner is a scanner.
impl<S: Scan + ?Sized> Scan for Box<S> {
    fn in_flight(&self) -> bool {
        (**self).in_flight()
    }

    fn read_token(&mut self) -> Result<Token> {
        (**self).read_token()
    }
}