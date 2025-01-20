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
/// This struct implements the state machine for recognizing UTF-8 characters
/// and ANSI control sequences, with [`Scanner::read_token`] producing the
/// corresponding [`Token`]. To minimize overheads, the implementation turns
/// subsequent UTF-8 characters into text tokens and remains zero-copy as long
/// as there are no control characters in the middle of control sequences. As a
/// result, tokens have the same lifetime as the scanner itself, and each token
/// must be processed before the next invocation of `read_token`.
///
/// The implementation of the state machine has been carefully engineered to
/// return to the well-known start state if at all possible, including for
/// errors. Still, that is not always possible, notably for errors in the
/// underlying input and when recognizing a control code while already
/// processing a control sequence. Unless the underlying input keeps rejecting
/// read requests, reading more tokens is a viable strategy for eventually
/// returning to the start state.
#[derive(Debug)]
pub struct Scanner<R> {
    // The underlying reader.
    reader: R,
    // The state machine state while scanning tokens.
    state: State,
    control: Option<Control>,
    // The actual data buffer and a flag for it having overflowed.
    buffer: Buffer,
    did_overflow: bool,
    // The actual and maximum lengths for sequences. The limit must be at least
    // the buffer size but usually is larger.
    sequence_length: usize,
    sequence_limit: usize,
    // A single byte buffer for control characters while sequence is being read.
    extra: [u8; 1],
}

impl<R: std::io::Read> Scanner<R> {
    /// Create a new scanner with the given capacity.
    pub fn with_options(options: &Options, reader: R) -> Self {
        Self {
            reader,
            state: State::Ground,
            control: None,
            buffer: Buffer::with_capacity(options.read_buffer_size()),
            did_overflow: false,
            sequence_length: 0,
            sequence_limit: options.pathological_size(),
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

        match action {
            Print => unreachable!("printable characters are processed before control sequences"),
            AbortThenHandleControl | AbortThenStart => {
                // Since we don't consume the byte and restore the ground state,
                // the next invocation of read_token handles the control or
                // starts the sequence. That works only because, whereas
                // AbortThenDoSomething transitions out of arbitrary states,
                // DoSomething always transitions out of ground.
                self.state = State::Ground;
                return Err(ErrorKind::MalformedSequence.into());
            }
            StartSequence => {
                self.buffer.start_token();
                self.did_overflow = false;
                self.sequence_length = 1;
            }
            IgnoreByte | RetainByte => {
                if self.sequence_limit <= self.sequence_length {
                    // Hard reset scanner upon pathological control sequence.
                    // That includes discarding buffered bytes.
                    self.state = State::Ground;
                    self.buffer.reset();
                    return Err(ErrorKind::PathologicalSequence.into());
                }
                self.sequence_length += 1;
            }
            _ => {}
        }

        self.buffer.consume();
        if control.is_some() {
            self.control = control;
        }

        match action {
            AbortSequence => return Err(ErrorKind::MalformedSequence.into()),
            RetainByte => self.buffer.retain(),
            Dispatch
                if matches!(
                    self.control
                        .expect("dispatching a control sequence requires a control"),
                    CSI | ESC | SS2 | SS3
                ) =>
            {
                self.buffer.retain()
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

    /// Read the next token as a control sequence.
    #[allow(dead_code)]
    pub fn read_sequence(&mut self, control: Control) -> Result<&[u8], Error> {
        match self.read_token()? {
            Token::Sequence(actual, payload) => {
                if actual == control {
                    Ok(payload)
                } else {
                    Err(ErrorKind::BadControl.into())
                }
            }
            _ => Err(ErrorKind::NotASequence.into()),
        }
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
}
