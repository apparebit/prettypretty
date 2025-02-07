use std::io::Result;

use crate::util::nicely;
use crate::{Input, Output};

/// A command for the terminal.
///
/// Commands provide instructions to the terminal and are communicated in-band
/// by writing ANSI escape codes. Doing so is the responsibility of the
/// [`std::fmt::Display`] implementation, whereas the [`std::fmt::Debug`]
/// implementation should simply identify the command.
///
/// This trait is object-safe.
pub trait Command: std::fmt::Debug + std::fmt::Display {}

/// A borrowed command is a command.
impl<C: Command + ?Sized> Command for &C {}

/// A boxed command is a command.
impl<C: Command + ?Sized> Command for Box<C> {}

/// Combine several commands into a single new command.
///
/// The new command preserves the order of its component commands. Upon display,
/// it emits as many ANSI escape sequence as it has component commands. Upon
/// debug, it reveals the macro's source arguments.
///
/// Since commands in the [`cmd`](crate::cmd) module generally implement
/// [`Clone`], [`Copy`], [`Debug`](std::fmt::Debug), [`PartialEq`], and [`Eq`],
/// fused commands do so, too. However, since [`DynLink`](crate::cmd::DynLink)
/// and [`DynSetWindowTitle`](crate::cmd::DynSetWindowTitle) have string-valued
/// fields and hence cannot implement [`Copy`], these two commands *cannot* be
/// fused.
///
/// When fusing only SGR commands, prefer [`fuse_sgr!`](crate::fuse_sgr), which
/// generates commands that emit a single ANSI escape sequence only.
///
/// # Example
///
/// ```
/// # use prettytty::{cmd::{MoveDown, MoveRight}, fuse};
/// let move_down_right_twice = fuse!(MoveDown::<2>, MoveRight::<2>);
/// assert_eq!(format!("{}", move_down_right_twice), "\x1b[2B\x1b[2D");
/// ```
#[macro_export]
macro_rules! fuse {
    ($($command:expr),+ $(,)?) => {{
        /// One or more combined commands.
        #[derive(Copy, Clone, PartialEq, Eq)]
        struct Fused;

        impl $crate::Command for Fused {}

        impl ::std::fmt::Debug for Fused {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.write_str(concat!(stringify!(fuse!), "(", stringify!($($command),+), ")"))
            }
        }

        impl ::std::fmt::Display for Fused {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                $($command.fmt(f)?;)*
                Ok(())
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
/// display, it emits only one ANSI escape sequence. Upon debug, it reveals the
/// macro's source arguments.
///
/// Since commands in the [`cmd`](crate::cmd) module generally implement
/// [`Clone`], [`Copy`], [`Debug`](std::fmt::Debug), [`PartialEq`], and [`Eq`],
/// fused SGR commands do so, too.
///
/// To fuse commands other than SGR commands, use [`fuse!`].
#[macro_export]
macro_rules! fuse_sgr {
    ( $sgr:expr, $( $sgr2:expr ),* $(,)? ) => {{
        /// One or more SGR commands fused into one.
        #[derive(Copy, Clone, PartialEq, Eq)]
        struct FusedSgr;

        impl ::std::fmt::Debug for FusedSgr {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.write_str(concat!(stringify!(fuse_sgr!), "(", stringify!($sgr, $($sgr2),*), ")"))
            }
        }

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
/// optionally followed by another control (usually `BEL` or `ST`). The trait
/// uses a method, and not a constant, for the control, so as to remain
/// object-safe.
///
///
/// # Example
///
/// ## The Elaborate Version
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
/// # let (mut input, mut output) = tty.io();
/// # output.exec(MoveToColumn::<17>)?;
/// // Write the command
/// output.exec(RequestCursorPosition)?;
///
/// // Read the token
/// let token = input.read_token()?;
///
/// // Extract and parse payload
/// let pos;
/// if let Token::Sequence(control, payload) = token {
///     if control == RequestCursorPosition.control() {
///         pos = RequestCursorPosition.parse(payload)?
///     } else {
///         return Err(ErrorKind::BadControl.into());
///     }
/// } else {
///     return Err(ErrorKind::NotASequence.into());
/// }
/// # assert_eq!(pos.1, 17);
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// ## Using `Scan::read_sequence`
///
/// In the above example, generating precise errors requires about as much code
/// as extracting and parsing the payload. The [`Scan::read_sequence`] method
/// abstracts over this boilerplate. It certainly helps:
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
/// # let (mut input, mut output) = tty.io();
/// # output.exec(MoveToColumn::<17>)?;
/// // Write the command
/// output.exec(RequestCursorPosition)?;
///
/// // Read the ANSI escape sequence and extract the payload
/// let payload = input.read_sequence(RequestCursorPosition.control())?;
///
/// // Parse the payload
/// let pos = RequestCursorPosition.parse(payload)?;
/// # assert_eq!(pos.1, 17);
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// ## Using `Query::run`
///
/// While much cleaner, the previous example still is boilerplate. After all,
/// every query needs to write the request, scan the response for the payload,
/// and parse the payload. [`Query::run`] abstracts over that:
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
/// # let (mut input, mut output) = tty.io();
/// # output.exec(MoveToColumn::<17>)?;
/// let pos = RequestCursorPosition.run(&mut input, &mut output)?;
/// # assert_eq!(pos.1, 17);
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// Nice, right? Alas, [`Query::run`] may be slower than needs be when
/// processing a batch of queries. The method's documentation addresses this and
/// other performance considerations.
pub trait Query: Command {
    /// The type of the response data.
    type Response;

    /// Get the response's control.
    fn control(&self) -> Control;

    /// Parse the payload into a response object.
    fn parse(&self, payload: &[u8]) -> Result<Self::Response>;

    /// Run this query.
    ///
    /// This method writes the request to the given output, reads the response
    /// from the given input, parses the response payload, returning the result.
    ///
    ///
    /// # Performance Considerations
    ///
    /// Since accessing a connection's input and output entails acquiring a
    /// mutex each, this method takes the input and output objects as arguments.
    /// That way, the caller controls when to acquire the two objects and incur
    /// the corresponding overhead. As a result, the caller also incurs the
    /// notational overhead of passing two arguments prefixed with `&mut`
    /// instead of passing one argument prefixed with `&` (as the connection
    /// object uses interior mutability). While not ideal, favoring flexibility
    /// and performance over concision seems the right trade-off.
    ///
    /// This method is well-suited to running the occasional query. However,
    /// when executing several queries in a row, e.g., when querying a terminal
    /// for its color theme, this method may not be performant, especially when
    /// running in a remote shell. Instead, an application should write all
    /// requests to output before flushing (once) and then process all
    /// responses. Prettypretty's
    /// [`Theme::query`](https://apparebit.github.io/prettypretty/prettypretty/theme/struct.Theme.html#method.query)
    /// does just that. If you [check the
    /// source](https://github.com/apparebit/prettypretty/blob/f25d2215d0747ca86ac8bcb5a48426dd7a496eb4/crates/prettypretty/src/theme.rs#L95),
    /// it actually implements versions with one, two, and three processing
    /// loops; the [query
    /// benchmark](https://github.com/apparebit/prettypretty/blob/main/crates/prettypretty/benches/query.rs)
    /// compares their performance.
    fn run(&self, input: &mut Input, output: &mut Output) -> Result<Self::Response> {
        output.exec(self)?;
        let payload = input.read_sequence(self.control())?;
        self.parse(payload)
    }
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
    /// A C0 or C1 control that doesn't start or end a sequence. This token
    /// always has one byte of character data.
    Control(&'t [u8]),
    /// A control sequence with its initial control and subsequent payload.
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
    ///
    /// The length of the returned byte slice varies for text and sequence
    /// tokens. It always is 1, however, for the control token.
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
        debug.field(&nicely(self.data())).finish()
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::cmd::{Format, SetBackground8, SetForeground8};

    #[test]
    fn test_fuse() {
        let s = format!(
            "{}",
            fuse!(Format::Bold, SetForeground8::<0>, SetBackground8::<15>)
        );
        assert_eq!(s, "\x1b[1m\x1b[30m\x1b[107m");

        let cmd = fuse!(Format::Blinking, SetBackground8::<219>);
        assert_eq!(format!("{}", cmd), "\x1b[5m\x1b[48;5;219m");
        assert_eq!(
            format!("{:?}", cmd),
            "fuse!(Format::Blinking, SetBackground8::<219>)"
        );

        let double = format!("{}{}", cmd, cmd);

        let copy = cmd;
        assert_eq!(format!("{}{}", cmd, copy), double);
        assert_eq!(
            format!("{:?}", cmd),
            "fuse!(Format::Blinking, SetBackground8::<219>)"
        );

        let clone = cmd.clone();
        assert_eq!(format!("{}{}", cmd, clone), double);
        assert_eq!(
            format!("{:?}", cmd),
            "fuse!(Format::Blinking, SetBackground8::<219>)"
        );

        assert_eq!(cmd, copy);
        assert_eq!(cmd, clone);
    }

    #[test]
    fn test_fuse_sgr() {
        let s = format!(
            "{}",
            fuse_sgr!(Format::Bold, SetForeground8::<0>, SetBackground8::<15>)
        );
        assert_eq!(s, "\x1b[1;30;107m");

        let cmd = fuse_sgr!(Format::Bold, SetForeground8::<0>, SetBackground8::<15>);
        assert_eq!(format!("{}", cmd), "\x1b[1;30;107m");
        assert_eq!(
            format!("{:?}", cmd),
            "fuse_sgr!(Format::Bold, SetForeground8::<0>, SetBackground8::<15>)"
        );

        let double = format!("{}{}", cmd, cmd);

        let copy = cmd;
        assert_eq!(format!("{}{}", cmd, copy), double);
        assert_eq!(
            format!("{:?}", cmd),
            "fuse_sgr!(Format::Bold, SetForeground8::<0>, SetBackground8::<15>)"
        );

        let clone = cmd.clone();
        assert_eq!(format!("{}{}", cmd, clone), double);
        assert_eq!(
            format!("{:?}", cmd),
            "fuse_sgr!(Format::Bold, SetForeground8::<0>, SetBackground8::<15>)"
        );

        assert_eq!(cmd, copy);
        assert_eq!(cmd, clone);
    }
}
