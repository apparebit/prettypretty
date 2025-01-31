mod buffer;
mod machine;
mod utf8;

use self::buffer::Buffer;
use self::machine::{transition, Action, State};
use self::utf8::scan_utf8;

use super::err::{Error, ErrorKind};
use super::opt::Options;
use super::{Control, Token};

// ================================================================================================

/// A scanner for text and control tokens.
///
/// This struct builds Paul Flo Williams' [parser for DEC's ANSI-compatible
/// terminals](https://vt100.net/emu/dec_ansi_parser) to implement a state
/// machine for recognizing UTF-8 characters and ANSI control sequences alike.
/// Notably, [`Scanner::read_token`] produces the corresponding [`Token`]s. To
/// minimize[] overhead, the implementation turns subsequent UTF-8 characters
/// into text tokens. It is zero-copy as long as no control characters appear in
/// the middle of control sequences. As a result, tokens have the same lifetime
/// as the scanner itself, and each token must be processed before the next
/// invocation of `read_token`.
///
/// The implementation of the state machine has been carefully engineered to
/// return to the well-known start state if at all possible, including for
/// errors. Still, that is not always possible, notably for errors in the
/// underlying input and when recognizing a control code while already
/// processing a control sequence. Unless the underlying input keeps rejecting
/// read requests, reading more tokens is a viable strategy for eventually
/// returning to the start state.
pub struct Scanner<R> {
    /// The underlying reader.
    reader: R,
    /// The state machine state for the escape sequence being recognized.
    state: State,
    /// The control for the escape sequence being recognized.
    control: Option<Control>,
    // The byte data being scanned.
    buffer: Buffer,
    /// The flag for the current escape sequence being too long.
    did_overflow: bool,
    /// The actual length of the current escape sequence.
    sequence_length: usize,
    /// The maximum length for any escape sequence, which must be at least as
    /// large as the buffer size.
    max_sequence_length: usize,
    /// A single byte buffer for control characters in the middle of an escape
    /// sequence.
    extra: [u8; 1],
}

impl<R: std::io::Read> Scanner<R> {
    /// Create a new scanner with the given capacity.
    pub fn with_options(options: &Options, reader: R) -> Self {
        Self {
            reader,
            state: State::Ground,
            control: None,
            buffer: Buffer::with_options(options),
            did_overflow: false,
            sequence_length: 0,
            max_sequence_length: options.pathological_size(),
            extra: [0; 1],
        }
    }

    // ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~
    // Manage the internal buffer

    /// Ensure that the buffer has readable content.
    ///
    /// This method returns an option indicating how many bytes were read from
    /// the underlying input. If the internal buffer contains readable bytes,
    /// this method returns `None`. If it doesn't, this method makes space in
    /// the buffer and reads from the underlying input. If that read returns no
    /// bytes, this method returns `Some(0)`.
    fn ensure_readable(&mut self) -> Result<Option<usize>, Error> {
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
                self.buffer.defrag();
            }

            let count = self.buffer.fill(&mut self.reader)?;
            return Ok(Some(count));
        }

        Ok(None)
    }

    // ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~
    // Support for reading bytes

    /// Determine whether this scanner's state machine is in-flight.
    pub fn in_flight(&self) -> bool {
        !matches!(self.state, State::Ground)
    }

    /// Get a buffer with unread bytes.
    ///
    /// This method only reads from the underlying input, if there are no unread
    /// bytes already buffered.
    pub fn fill_buf(&mut self) -> Result<&[u8], Error> {
        if self.in_flight() {
            return Err(ErrorKind::InFlight.into());
        }

        Ok(if let Some(0) = self.ensure_readable()? {
            &[]
        } else {
            self.buffer.peek_many()
        })
    }

    /// Consume unread bytes.
    ///
    /// Unless the state machine is in-flight, this method consumes at most the
    /// given number of bytes.
    pub fn consume(&mut self, count: usize) -> Result<(), Error> {
        if self.in_flight() {
            return Err(ErrorKind::InFlight.into());
        }

        self.buffer.consume_many(count);
        Ok(())
    }

    // ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~
    // Support for reading tokens

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
                        return Err(ErrorKind::MalformedUtf8.into());
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
        use self::Action::*;
        use self::Control::*;

        let (action, control);
        (self.state, action, control) = transition(self.state, byte);

        // Handle control. Handle sequence start and length.
        if control.is_some() {
            self.control = control;

            // Setting the control implies the start of a sequence.
            self.buffer.start_token();
            self.did_overflow = false;
            self.sequence_length = 1;
        } else if !matches!(self.state, State::Ground) {
            self.sequence_length += 1;

            if self.max_sequence_length <= self.sequence_length {
                // Hard reset scanner upon pathological control sequence.
                // That includes discarding buffered bytes.
                self.state = State::Ground;
                self.buffer.reset();
                return Err(ErrorKind::PathologicalSequence.into());
            }
        }

        // Handle bug and early return.
        if matches!(action, Print) {
            panic!("printable characters should not appear within control sequence");
        } else if matches!(action, AbortThenRetry) {
            return Err(ErrorKind::MalformedSequence.into());
        }

        // Handle buffer.
        self.buffer.consume();

        match action {
            AbortSequence => return Err(ErrorKind::MalformedSequence.into()),
            RetainByte => self.buffer.retain(),
            Dispatch => {
                let control = self
                    .control
                    .expect("dispatching a control sequence requires a control");
                if matches!(control, CSI | ESC | SS2 | SS3) {
                    self.buffer.retain()
                }
            }
            _ => {}
        }

        Ok(action)
    }

    /// Create a control token for the byte.
    fn new_control_token(&mut self, byte: u8) -> Result<Token, Error> {
        self.extra[0] = byte;
        Ok(Token::Control(&self.extra))
    }

    /// Create a new sequence token.
    fn new_sequence_token(&self) -> Result<Token, Error> {
        if self.did_overflow {
            Err(ErrorKind::OutOfMemory.into())
        } else {
            Ok(Token::Sequence(
                self.control.expect("a control sequence has a control"),
                self.buffer.token(),
            ))
        }
    }

    // ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~

    /// Read the next token.
    pub fn read_token(&mut self) -> Result<Token, Error> {
        loop {
            // Make sure that we have some bytes to process
            if let Some(0) = self.ensure_readable()? {
                return Err(ErrorKind::NoData.into());
            }

            // Try fast path for text
            if matches!(self.state, State::Ground) && self.scan_text()? {
                return Ok(Token::Text(self.buffer.token()));
            }

            // Run the state machine for control sequences
            while let Some(byte) = self.buffer.peek() {
                use self::Action::*;

                match self.step_sequence(byte)? {
                    HandleControl => return self.new_control_token(byte),
                    Dispatch => return self.new_sequence_token(),
                    _ => continue,
                }
            }
        }
    }
}

impl<R> std::fmt::Debug for Scanner<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scanner")
            .field("state", &self.state)
            .field("control", &self.control)
            .field("buffer", &self.buffer)
            .field("did_overflow", &self.did_overflow)
            .field("sequence_length", &self.sequence_length)
            .field("max_sequence_length", &self.max_sequence_length)
            .finish_non_exhaustive()
    }
}

// ================================================================================================

#[cfg(test)]
mod test {
    use super::{transition, Action, Control, Error, ErrorKind, Scanner, State, Token};
    use crate::opt::Options;
    use std::mem::size_of;

    #[test]
    fn test_size() {
        assert_eq!(size_of::<(State, Action)>(), 2);
    }

    #[test]
    fn test_state_machine() {
        use self::Action::*;
        use self::Control::*;

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
        let input = input.as_slice();

        let expected_output: &[Result<Token, Error>] = &[
            Ok(Token::Text(b"a \xe2\x9c\xb6 ".as_slice())),
            Err(ErrorKind::MalformedSequence.into()),
            Ok(Token::Text(b" ".as_slice())),
            Ok(Token::Sequence(Control::SS3, &[b'R'])),
            Ok(Token::Text(b" ".as_slice())),
            Err(ErrorKind::MalformedUtf8.into()),
            Ok(Token::Text(b"x ".as_slice())),
            Ok(Token::Control(&[0x07])),
            Ok(Token::Text(b" ".as_slice())),
            Ok(Token::Sequence(Control::CSI, b"31m".as_slice())),
            Ok(Token::Text(b" ".as_slice())),
            Err(ErrorKind::MalformedSequence.into()),
            Ok(Token::Text(b" \xe2\x81\x82".as_slice())),
        ];

        let mut scanner = Scanner::with_options(&Options::default(), input);
        for expected in expected_output {
            println!("\n{:#?}", scanner);
            let result = scanner.read_token();
            println!("got {:?}, expected {:?}", result, expected);
            assert_eq!(result.is_ok(), expected.is_ok());
            if result.is_ok() {
                assert_eq!(&result.unwrap(), expected.as_ref().unwrap());
            } else {
                assert_eq!(
                    result.unwrap_err().kind(),
                    expected.as_ref().unwrap_err().kind()
                );
            }
        }
    }

    #[test]
    fn test_bad_osc() {
        let input = b"\x1b]junk\x1b]text\x1b\\".as_slice();
        let mut scanner = Scanner::with_options(&Options::default(), input);
        let t = scanner.read_token();
        assert!(t.is_err());
        assert_eq!(t.unwrap_err().kind(), ErrorKind::MalformedSequence);
        let t = scanner.read_token();
        assert!(t.is_ok());
        assert_eq!(t.unwrap(), Token::Sequence(Control::OSC, b"text"));
    }
}
