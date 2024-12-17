use std::io::{BufRead, BufWriter, Read, Result, Write};
use std::sync::{Mutex, MutexGuard};

use crate::opt::Options;
use crate::scan::Scanner;
use crate::sys::{Config, RawConnection, RawInput, RawOutput};
use crate::{Command, Scan};

/// A terminal connection providing [`Input`] and [`Output`].
///
/// This object owns the connection to the terminal. It provides independent,
/// mutually exclusive, and thread-safe access to [`Input`] as well as
/// [`Output`]. On Unix, the I/O types share the same underlying file
/// descriptor, whereas on Windows each I/O type uses a distinct handle.
///
/// Since a connection temporarily reconfigures the terminal, an application
/// should go out of its way to always execute this type's drop handler before
/// exit.
pub struct Connection {
    options: Options,
    stamp: u32,
    config: Config,
    scanner: Mutex<Scanner<RawInput>>,
    writer: Mutex<BufWriter<RawOutput>>,
    connection: RawConnection,
}

impl Connection {
    /// Open a terminal connection with the default options.
    pub fn open() -> Result<Self> {
        Self::with_options(Options::default())
    }

    /// Open a terminal connection with the given options.
    pub fn with_options(options: Options) -> Result<Self> {
        let connection = RawConnection::open(&options)?;
        let config = Config::read(connection.input())?;
        config.apply(&options).write(connection.output())?;
        let scanner = Mutex::new(Scanner::with_options(&options, connection.input()));
        let writer = Mutex::new(BufWriter::with_capacity(
            options.write_buffer_size(),
            connection.output(),
        ));
        let stamp = if options.verbose() {
            // macOS duration has microsecond resolution only, so that's our
            // least common denominator. If duration_since() fails, we use an
            // obviously wrong value as stamp.
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .map(|d| d.subsec_micros())
                .unwrap_or(0xff_ff_ff_ff)
        } else {
            0
        };

        let this = Self {
            options,
            stamp,
            config,
            scanner,
            writer,
            connection,
        };

        this.log("terminal::connect")?;
        Ok(this)
    }

    /// Get the options.
    pub fn options(&self) -> &Options {
        &self.options
    }

    /// Get both terminal input and output.
    pub fn io(&self) -> (Input, Output) {
        (self.input(), self.output())
    }

    /// Get the terminal input.
    pub fn input(&self) -> Input {
        Input {
            scanner: self.scanner.lock().expect("mutex is not poisoned"),
        }
    }

    /// Get the terminal output.
    pub fn output(&self) -> Output {
        Output {
            writer: self.writer.lock().expect("mutex is not poisoned"),
        }
    }

    fn log(&self, message: impl AsRef<str>) -> Result<()> {
        if self.options.verbose() {
            let mut writer = self.writer.lock().expect("mutex is not poisoned");
            write!(
                writer,
                "{} pid={} group={} stamp={}\r\n",
                message.as_ref(),
                std::process::id(),
                self.connection.group()?,
                self.stamp
            )?;
            writer.flush()
        } else {
            Ok(())
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.log("terminal::disconnect");
        // Map instead of unwrap so that we don't panic in drop.
        let _ = self.writer.lock().map(|mut w| {
            let _ = w.flush();
        });
        let _ = self.config.write(self.connection.output());
    }
}

/// A terminal [`Connection`]'s input.
///
/// Token scanning internally buffers input data, whereas the readers perform
/// unbuffered reads from the terminal. Reads always time out after a duration
/// configurable in 0.1s increments and return a zero-length slice or zero byte
/// count. [`read_token()`](crate::Scan::read_token) returns an
/// [`ErrorKind::Interrupted`](std::io::ErrorKind::Interrupted) instead. On
/// Unix, the timeout is implemented with the terminal's [`MIN` and `TIME`
/// parameters](https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap11.html#tag_11_01_07_03)
/// On Windows, the timeout is implemented with
/// [`WaitForSingleObject`](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject).
///
///
/// # Scanning Tokens vs Reading Bytes
///
/// Despite requiring a fairly elaborate state machine, the implementation of
/// [`read_token()`](crate::Scan::read_token) has been carefully engineered to
/// return to the start state before returning whenever possible. The two
/// exceptions are (1) errors when reading from the terminal connection and (2)
/// a [`Token::Control`](crate::Token::Control) result when in the middle of
/// recognizing a [`Token::Sequence`](crate::Token::Sequence). In these cases,
/// [`in_flight()`](crate::Scan::in_flight) returns `true`.
///
/// Correctly consuming tokens through [`Scan`] and bytes through [`BufRead`]
/// and [`Read`] requires that byte-reads consume data at token granularity as
/// well. For that reason, [`fill_buf()`](BufRead::fill_buf) and
/// [`consume()`](BufRead::consume) are much preferred over
/// [`read()`](Read::read), since the former two methods provide exact control
/// over consumed bytes, whereas the latter method does not. For the same
/// reason, byte-reads fail with
/// [`ErrorKind::InFlight`](crate::err::ErrorKind::InFlight), if the state
/// machine currently is in-flight.
///
///
/// # Error Recovery
///
/// Unless the terminal connection keeps erroring, a viable error recovery
/// strategy is to keep reading tokens. The state machine usually returns to the
/// start state after the first error. Doing so requires reading at most 3 bytes
/// for UTF-8 characters and, in theory, an unlimited number of bytes for
/// pathological ANSI escape sequences.
///
///
/// # Pathological Inputs
///
/// To protect against such pathological inputs, the implementation gracefully
/// handles out-of-memory conditions, i.e., when a sequence is longer than the
/// internal buffer size. It does *not* dynamically grow the buffer size, but
/// instead keeps processing bytes until the sequence is complete and then
/// returns [`ErrorKind::OutOfMemory`](crate::err::ErrorKind::OutOfMemory).
/// However, if a sequence is much longer than the buffer size, continuing to
/// scan it makes little sense. Hence, upon reaching a configurable limit, the
/// state machine forcibly resets and discards any unread bytes before returning
/// [`ErrorKind::PathologicalSequence`](crate::err::ErrorKind::PathologicalSequence).
/// In that case, it probably is advisable to terminate the terminal connection,
/// since a denial-of-service attack appears to be under way.
#[derive(Debug)]
pub struct Input<'a> {
    scanner: MutexGuard<'a, Scanner<RawInput>>,
}

impl Scan for Input<'_> {
    fn in_flight(&self) -> bool {
        self.scanner.in_flight()
    }

    fn read_token(&mut self) -> Result<crate::Token> {
        self.scanner.read_token().map_err(|e| e.into())
    }
}

impl Read for Input<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut source = self.scanner.fill_buf()?;
        let count = source.read(buf)?;
        self.scanner.consume(count)?;
        Ok(count)
    }
}

impl BufRead for Input<'_> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.scanner.fill_buf().map_err(|e| e.into())
    }

    fn consume(&mut self, amt: usize) {
        // Don't panic...
        let _ = self.scanner.consume(amt);
    }
}

/// A terminal [`Connection`]'s output.
///
/// Since terminal output is buffered, actually executing commands requires
/// flushing the output. [`Output::print`] helps write and flush simple strings,
/// and [`Output::exec`] helps write and flush individual commands.
#[derive(Debug)]
pub struct Output<'a> {
    writer: MutexGuard<'a, BufWriter<RawOutput>>,
}

impl Output<'_> {
    /// Write and flush the text.
    pub fn print(&mut self, text: impl AsRef<str>) -> Result<()> {
        self.writer.write_all(text.as_ref().as_bytes())?;
        self.writer.flush()
    }

    /// Write and flush the command.
    pub fn exec(&mut self, cmd: impl Command) -> Result<()> {
        write!(self.writer, "{}", cmd)?;
        self.writer.flush()
    }
}

impl Write for Output<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.writer.flush()
    }
}
