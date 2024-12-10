mod buffer;
mod machine;
mod utf8;

use buffer::Buffer;
use machine::{transition, Action, State};
use utf8::scan_utf8;

pub use machine::Control;

// ================================================================================================
// Tokens

/// A lexical token.
#[derive(Clone, Debug, PartialEq)]
pub enum Token<'t> {
    /// One or more UTF-8 characters excluding C0 and C1 controls.
    Text(&'t [u8]),
    /// A C0 or C1 control that doesn't start a sequence.
    Control(&'t [u8]),
    /// A control sequence.
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
        use Token::*;

        match self {
            Text(data) => data,
            Control(data) => data,
            Sequence(_, data) => data,
        }
    }
}

// ================================================================================================
// Errors

/// A scanner error.
///
/// Scanner errors provide detailed information about error conditions while
/// reading a token. [`Error::Unreadable`] wraps I/O errors.
#[derive(Debug)]
pub enum Error {
    /// No data is available when reading, most likely due to a timeout.
    NoData,
    /// A malformed UTF-8 character.
    BadUtf8,
    /// A malformed ANSI escape sequence.
    BadSequence,
    /// A well-formed ANSI escape sequence starting with the wrong control.
    BadSequenceStart,
    /// A token other than a sequence when a sequence is expected.
    NotASequence,
    /// An ANSI escape sequence longer than the available internal buffer space.
    OutOfMemory,
    /// An error reading from the reader providing data.
    Unreadable(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::NoData => "no data reading terminal",
            Self::BadUtf8 => "malformed UTF-8",
            Self::BadSequence => "malformed ANSI escape sequence",
            Self::BadSequenceStart => "unexpected control for ANSI escape sequence",
            Self::NotASequence => "token not a sequence",
            Self::OutOfMemory => "ANSI escape sequence too long for internal buffer",
            Self::Unreadable(_) => "error reading terminal",
        };

        f.write_str(s)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::Unreadable(error) = self {
            Some(error)
        } else {
            None
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Unreadable(value)
    }
}

impl From<Error> for std::io::Error {
    fn from(value: Error) -> Self {
        use Error::*;

        match value {
            BadUtf8 | BadSequence | BadSequenceStart | NotASequence => {
                Self::new(std::io::ErrorKind::InvalidData, value)
            }
            NoData => std::io::ErrorKind::Interrupted.into(),
            OutOfMemory => std::io::ErrorKind::OutOfMemory.into(),
            Unreadable(error) => error,
        }
    }
}

// ================================================================================================

/// The default capacity for a scanner's internal buffer.
///
/// At a minimum, the scanner's buffer should be large enough to hold all
/// possible responses to a query. Notably, when querying colors, that length is
/// 27 bytes. For example, a response for the color of the 16th ANSI color
/// *bright white* starts with `‹OSC›4;15;rgb:` and is followed by three
/// hexadecimal numbers that usually are four digits wide, e.g.,
/// `ffff/ffff/ffff`, and then `‹ST›`. Both OSC and ST require at most two
/// bytes, resulting in a sequence that is at most 27 bytes long.
pub const DEFAULT_CAPACITY: usize = 27;

/// A scanner for text and control tokens.
///
/// This struct implements the state machines for recognizing UTF-8 characters
/// and ANSI control sequences, with [`Scanner::read_token`] producing the
/// corresponding [`Token`]. To minimize overheads, it turns subsequent UTF-8
/// characters into text tokens, utilizes internal buffer, and requires no
/// copying to generate tokens as long as there are no control characters in the
/// middle of ANSI escape sequences. As a result, tokens have the same lifetime
/// as the scanner itself, and each token must be processed before the next
/// invocation of `read_token`.
#[derive(Debug)]
pub struct Scanner {
    state: State,
    control: Option<Control>,
    buffer: Buffer,
    did_overflow: bool,
    extra: [u8; 1],
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Scanner {
    /// Create a new scanner with the default capacity.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Create a new scanner with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            state: State::Ground,
            control: None,
            buffer: Buffer::with_capacity(capacity),
            did_overflow: false,
            extra: [0; 1],
        }
    }

    /// Determine if this scanner's state machine is in-flight.
    pub fn is_inflight(&self) -> bool {
        !matches!(self.state, State::Ground)
    }

    // ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~
    // Helper methods for read_token()

    /// Ensure that the buffer has readable content.
    fn ensure_readable<R: std::io::Read>(&mut self, reader: &mut R) -> Result<(), Error> {
        if !self.buffer.is_readable() {
            if matches!(self.state, State::Ground) {
                // No readable data or token to preserve. Just reset buffer.
                self.buffer.reset();
            } else if self.buffer.is_exhausted() {
                // To make progress again, reset buffer but remember error.
                self.buffer.reset();
                self.did_overflow = true;
            } else if !self.buffer.has_capacity() {
                // Some terminals require two reads for OSC/ST sequence.
                // Only backshift if there is no more capacity.
                self.buffer.backshift();
            }

            let count = self.buffer.fill(reader)?;
            if count == 0 {
                return Err(Error::NoData);
            }
        }

        Ok(())
    }

    /// Scan the buffer for one or more UTF-8 characters.
    ///
    /// This method returns a wrapped boolean indicating whether to return a
    /// text token. It also handles malformed UTF-8 errors.
    fn scan_text(&mut self) -> Result<bool, Error> {
        let mut bytes = self.buffer.peek_many();
        let mut index = 0;

        loop {
            if bytes.is_empty() || bytes[0] < 0x20 || (0x80..0xa0).contains(&bytes[0]) {
                break;
            }

            match scan_utf8(bytes) {
                Ok(size) => {
                    index += size;
                    bytes = &bytes[size..];
                }
                Err(size) => {
                    if index == 0 {
                        self.buffer.consume_many(size);
                        return Err(Error::BadUtf8);
                    } else {
                        break;
                    }
                }
            }
        }

        if 0 < index {
            self.buffer.start_token();
            self.buffer.consume_many(index);
            self.buffer.retain_many(index);
        }

        Ok(0 < index)
    }

    /// Step the state machine for ANSI escape sequences.
    ///
    /// Given a byte peeked from the buffer, this method transitions the state
    /// machine for ANSI escape sequences and updates the control and buffer. It
    /// also detects malformed sequences. The caller only needs to process
    /// complete controls and control sequences.
    fn step_sequence(&mut self, byte: u8) -> Result<Action, Error> {
        use Action::*;
        use Control::*;

        let (action, control);
        (self.state, action, control) = transition(self.state, byte);

        match action {
            Print => unreachable!("printable characters are processed before control sequences"),
            AbortThenHandleControl | AbortThenStart => {
                // Since we don't consume the byte and restore the ground
                // state, the next invocation of read_token handles the
                // control or starts the sequence. That works only because
                // AbortThenDoSomething transitions out of arbitrary states
                // are DoSomething transitions out of ground.
                self.state = State::Ground;
                return Err(Error::BadSequence);
            }
            StartSequence => {
                self.buffer.start_token();
                self.did_overflow = false;
            }
            _ => {}
        }

        self.buffer.consume();
        if control.is_some() {
            self.control = control;
        }

        match action {
            AbortSequence => return Err(Error::BadSequence),
            RetainByte => self.buffer.retain(),
            Dispatch if matches!(self.control.unwrap(), CSI | ESC | SS2 | SS3) => {
                self.buffer.retain()
            }
            _ => {}
        }

        Ok(action)
    }

    /// Create a control token for the byte.
    fn control_token(&mut self, byte: u8) -> Result<Token, Error> {
        self.extra[0] = byte;
        Ok(Token::Control(&self.extra))
    }

    /// Create a new sequence token.
    fn sequence_token(&self) -> Result<Token, Error> {
        if self.did_overflow {
            Err(Error::OutOfMemory)
        } else {
            Ok(Token::Sequence(self.control.unwrap(), self.buffer.token()))
        }
    }

    // ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~

    /// Read the next token.
    pub fn read_token<R: std::io::Read>(&mut self, reader: &mut R) -> Result<Token, Error> {
        loop {
            // Make sure that we have some bytes to process
            self.ensure_readable(reader)?;

            // Try fast path for text
            if matches!(self.state, State::Ground) && self.scan_text()? {
                return Ok(Token::Text(self.buffer.token()));
            }

            // Run the state machine for control sequences
            while let Some(byte) = self.buffer.peek() {
                use Action::*;

                match self.step_sequence(byte)? {
                    HandleControl => return self.control_token(byte),
                    Dispatch => return self.sequence_token(),
                    _ => continue,
                }
            }
        }
    }

    /// Read the next token as a control sequence.
    pub fn read_sequence<R: std::io::Read>(
        &mut self,
        reader: &mut R,
        control: Control,
    ) -> Result<&[u8], Error> {
        match self.read_token(reader)? {
            Token::Sequence(actual, payload) => {
                if actual == control {
                    Ok(payload)
                } else {
                    Err(Error::BadSequenceStart)
                }
            }
            _ => Err(Error::NotASequence),
        }
    }
}

// ================================================================================================

#[cfg(test)]
mod test {
    use super::{transition, Action, Control, Error, Scanner, State, Token};
    use std::mem::size_of;

    #[test]
    fn test_size() {
        assert_eq!(size_of::<(State, Action)>(), 2);
    }

    #[test]
    fn test_state_machine() {
        use Action::*;
        use Control::*;

        let mut state = State::Ground;

        let mut step = |byte| {
            let (action, control);
            (state, action, control) = transition(state, byte);
            (action, control)
        };

        let input = b"\x1b[31m";
        assert!(matches!(step(input[0]), (StartSequence, Some(ESC))));
        assert!(matches!(step(input[1]), (IgnoreByte, Some(CSI))));
        assert!(matches!(step(input[2]), (RetainByte, None)));
        assert!(matches!(step(input[3]), (RetainByte, None)));
        assert!(matches!(step(input[4]), (Dispatch, None)));
    }

    #[test]
    fn test_events() {
        let input = [
            b"a \xe2\x9c\xb6 \x1b[+=+@ \x1bOR \xe2x \x07 ".as_slice(),
            b"\x1b[31m \x1b[123$$4<=>m \xe2\x81\x82".as_slice(),
        ]
        .concat();
        let mut input = input.as_slice();

        let expected_output: &[Result<Token, Error>] = &[
            Ok(Token::Text(b"a \xe2\x9c\xb6 ".as_slice())),
            Err(Error::BadSequence),
            Ok(Token::Text(b" ".as_slice())),
            Ok(Token::Sequence(Control::SS3, &[b'R'])),
            Ok(Token::Text(b" ".as_slice())),
            Err(Error::BadUtf8),
            Ok(Token::Text(b"x ".as_slice())),
            Ok(Token::Control(&[0x07])),
            Ok(Token::Text(b" ".as_slice())),
            Ok(Token::Sequence(Control::CSI, b"31m".as_slice())),
            Ok(Token::Text(b" ".as_slice())),
            Err(Error::BadSequence),
            Ok(Token::Text(b" \xe2\x81\x82".as_slice())),
        ];

        let mut scanner = Scanner::new();
        for expected in expected_output {
            println!("\n{:#?}", scanner);
            let result = scanner.read_token(&mut input);
            println!("got {:?}, expected {:?}", result, expected);
            assert_eq!(result.is_ok(), expected.is_ok());
            if result.is_ok() {
                assert_eq!(&result.unwrap(), expected.as_ref().unwrap());
            } else {
                assert_eq!(
                    std::mem::discriminant(&result.unwrap_err()),
                    std::mem::discriminant(expected.as_ref().unwrap_err())
                );
            }
        }
    }
}
