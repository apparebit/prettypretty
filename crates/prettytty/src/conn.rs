use core::cell::RefCell;
use std::io::{BufRead, BufWriter, Error, ErrorKind, Read, Result, Write};
use std::sync::{Mutex, MutexGuard};

use crate::opt::{Options, Volume};
use crate::read::{DoggedReader, VerboseReader};
use crate::scan::Scanner;
use crate::sys::{RawConfig, RawConnection, RawOutput};
use crate::{Command, Scan};

// -----------------------------------------------------------------------------------------------

/// A writer with associated deferred commands.
#[derive(Debug)]
struct DeferredWriter {
    writer: BufWriter<RawOutput>,
    deferred: RefCell<Vec<Box<dyn Command + Send>>>,
}

impl DeferredWriter {
    /// Create a new list of deferred commands.
    pub fn new(writer: RawOutput, options: &Options) -> Self {
        Self {
            writer: BufWriter::with_capacity(options.write_buffer_size(), writer),
            deferred: RefCell::new(Vec::new()),
        }
    }

    /// Defer the execution of the given command.
    pub fn defer<C>(&self, cmd: C)
    where
        C: Command + Send + 'static,
    {
        self.deferred.borrow_mut().push(Box::new(cmd));
    }

    /// Take the list of commands and leave empty list behind.
    pub fn take(&self) -> Vec<Box<dyn Command + Send>> {
        self.deferred.take()
    }
}

impl Write for DeferredWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.writer.flush()
    }
}

// -----------------------------------------------------------------------------------------------

/// A terminal connection providing [`Input`] and [`Output`].
///
/// This object owns the connection to the terminal. It provides independent,
/// mutually exclusive, and thread-safe access to [`Input`] as well as
/// [`Output`]. On Unix, the I/O types share the same underlying file
/// descriptor, whereas on Windows each I/O type uses a distinct handle.
///
/// To facilitate reading from the terminal, this type reconfigures the
/// terminal, at a minimum by disabling the terminal's line editing mode. Since
/// its drop handler restores the original configuration, **an application
/// should go out of its way to always execute this type's drop handler** before
/// exit. Use [`drop`](std::mem::drop) to manually close a connection before it
/// goes out of scope.
///
/// An application may need to make further changes, such as using the alternate
/// screen or hiding the cursor, that also need to be undone before exit. In
/// that case, the application can use [`Output::exec_defer`]. The method takes
/// two [`Command`]s, executes the first command right away, and defers the
/// second command to just before the terminal connection is closed.
#[derive(Debug)]
pub struct Connection {
    options: Options,
    stamp: u32,
    config: Option<RawConfig>,
    scanner: Mutex<Scanner<Box<dyn Read + Send>>>,
    writer: Mutex<DeferredWriter>,
    connection: RawConnection,
}

fn _assert_connection_is_send_sync() {
    fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<Connection>();
}

impl Connection {
    /// Open a terminal connection with the default options.
    pub fn open() -> Result<Self> {
        Self::with_options(Options::default())
    }

    /// Open a terminal connection with the given options.
    ///
    /// If this method cannot establish a connection to the controlling
    /// terminal, it fails with a [`ErrorKind::ConnectionRefused`] error.
    #[allow(clippy::print_stdout)]
    pub fn with_options(options: Options) -> Result<Self> {
        let connection = RawConnection::open(&options)
            .map_err(|e| Error::new(ErrorKind::ConnectionRefused, e))?;

        let config = RawConfig::read(&connection)?;
        let verbose = !matches!(options.volume(), Volume::Silent);
        if verbose {
            println!("terminal::config {:?}", &config);
        }
        let config = config.apply(&options).map_or_else(
            || Ok::<Option<RawConfig>, Error>(None),
            |reconfig| {
                if verbose {
                    println!("terminal::reconfig {:?}", &reconfig);
                }
                reconfig.write(&connection)?;
                if verbose {
                    // We need explicit carriage-return and line-feed characters
                    // because the reconfiguration just took effect.
                    print!("terminal::reconfigured\r\n")
                }
                Ok(Some(config))
            },
        )?;

        let reader: Box<dyn Read + Send> = if matches!(options.volume(), Volume::Detailed) {
            Box::new(VerboseReader::new(connection.input(), options.timeout()))
        } else {
            Box::new(DoggedReader::new(connection.input()))
        };
        let scanner = Mutex::new(Scanner::with_options(&options, reader));
        let writer = Mutex::new(DeferredWriter::new(connection.output(), &options));
        let stamp = if verbose {
            // macOS duration has microsecond resolution only, so that's our
            // least common denominator. If duration_since() fails, we use an
            // obviously wrong value as stamp.
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .map(|d| d.subsec_micros())
                .unwrap_or(0)
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

    /// Get the options used when opening this connection.
    #[inline]
    pub fn options(&self) -> &Options {
        &self.options
    }

    /// Get both terminal input and output.
    ///
    /// The returned input and output objects ensure mutually exclusive access
    /// to the terminal's input and output, respectively. Dropping them releases
    /// access again.
    #[inline]
    pub fn io(&self) -> (Input, Output) {
        (self.input(), self.output())
    }

    /// Get the terminal input.
    ///
    /// The returned input object ensures mutually exclusive access to the
    /// terminal's input. Dropping the input object releases access again.
    ///
    /// # Panics
    ///
    /// If the underlying mutex has been poisoned.
    #[inline]
    pub fn input(&self) -> Input {
        Input {
            scanner: self.scanner.lock().expect("can't lock poisoned mutex"),
        }
    }

    /// Get the terminal output.
    ///
    /// The returned output object ensures mutually exclusive access to the
    /// terminal's output. Dropping the output object releases access again.
    ///
    /// # Panics
    ///
    /// If the underlying mutex has been poisoned.
    #[inline]
    pub fn output(&self) -> Output {
        Output {
            writer: self.writer.lock().expect("can't lock poisoned mutex"),
        }
    }

    fn log(&self, message: impl AsRef<str>) -> Result<()> {
        if !matches!(self.options.volume(), Volume::Silent) {
            // Don't wait for output.
            let mut writer = self
                .writer
                .try_lock()
                .map_err(|_| Error::from(ErrorKind::WouldBlock))?;

            write!(
                writer,
                "{} pid={} group={} stamp={}\r\n",
                message.as_ref(),
                std::process::id(),
                self.connection.group().unwrap_or(0),
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

        // Execute deferred commands and flush output.
        let _ = self.writer.lock().map(|mut writer| {
            for cmd in writer.take().into_iter().rev() {
                let _ = write!(writer, "{}", cmd);
            }
            let _ = writer.flush();
        });

        // Restore terminal configuration
        if let Some(ref cfg) = self.config {
            let _ = cfg.write(&self.connection);
        }
    }
}

// -----------------------------------------------------------------------------------------------

/// A terminal [`Connection`]'s input.
///
/// In addition to [`Read`] and [`BufRead`], terminal input also implements
/// [`Scan`] for ingesting text and ANSI escape sequences. The implementation of
/// all three traits uses the same, shared buffer. At the same time, it does not
/// share any state (nor implementation) with standard I/O in Rust's standard
/// library.
///
/// Reads from the terminal connection time out after a duration configurable in
/// 0.1s increments. In that case, [`Read::read`] returns a count of 0,
/// [`BufRead::fill_buf`] an empty slice, and [`Scan::read_token`] an error with
/// kind [`ErrorKind::TimedOut`](std::io::ErrorKind::TimedOut). On Unix, the
/// timeout is implemented with the terminal's [`MIN` and `TIME`
/// parameters](https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap11.html#tag_11_01_07_03)
/// On Windows, the timeout is implemented with
/// [`WaitForSingleObject`](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject).
///
///
/// # Scanning Tokens vs Reading Bytes
///
/// Despite requiring a fairly elaborate state machine, the implementation of
/// [`read_token()`](crate::Scan::read_token) has been carefully engineered to
/// return to the start state whenever possible. However, that is not possible
/// when reading from the terminal connection results in an error or when a
/// [`Token::Control`](crate::Token::Control) appears in the middle of a
/// [`Token::Sequence`](crate::Token::Sequence). In these cases,
/// [`in_flight()`](crate::Scan::in_flight) returns `true`.
///
/// It is possible to interleave reading bytes through [`Read`] and [`BufRead`]
/// as well as tokens through [`Scan`], as long as byte-reads consume data at
/// token granularity as well. For that reason,
/// [`fill_buf()`](BufRead::fill_buf) and [`consume()`](BufRead::consume) are
/// much preferred over [`read()`](Read::read) because the former two methods
/// provide exact control over consumed bytes, whereas the latter method does
/// not. For the same reason, byte-reads fail with
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
/// # Pathological Input
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
    pub scanner: MutexGuard<'a, Scanner<Box<dyn Read + Send>>>,
}

impl Input<'_> {
    /// Determine whether the input has bytes buffered.
    #[must_use = "the only reason to invoke method is to access the returned value"]
    pub fn is_readable(&self) -> bool {
        self.scanner.is_readable()
    }
}

impl Scan for Input<'_> {
    #[inline]
    fn in_flight(&self) -> bool {
        self.scanner.in_flight()
    }

    #[inline]
    fn read_token(&mut self) -> Result<crate::Token> {
        self.scanner.read_token().map_err(core::convert::Into::into)
    }
}

impl Read for Input<'_> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut source = self.scanner.fill_buf()?;
        let count = source.read(buf)?;
        self.scanner.consume(count)?;
        Ok(count)
    }
}

impl BufRead for Input<'_> {
    #[inline]
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.scanner.fill_buf().map_err(core::convert::Into::into)
    }

    #[inline]
    fn consume(&mut self, amt: usize) {
        // Don't panic...
        let _ = self.scanner.consume(amt);
    }
}

// -----------------------------------------------------------------------------------------------

/// A terminal [`Connection`]'s output.
///
/// Since terminal output is buffered, actually executing commands requires
/// flushing the output. As a convenience, [`Output::print`] and
/// [`Output::println`] write strings and [`Output::exec`] writes individual
/// commands, while also flushing the output on every invocation.
#[derive(Debug)]
pub struct Output<'a> {
    writer: MutexGuard<'a, DeferredWriter>,
}

impl Output<'_> {
    /// Write and flush the text.
    #[inline]
    #[must_use = "method returns result that may indicate an error"]
    pub fn print<T: AsRef<str>>(&mut self, text: T) -> Result<()> {
        self.writer.write_all(text.as_ref().as_bytes())?;
        self.writer.flush()
    }

    /// Write and flush the text followed by carriage return and line feed.
    #[inline]
    #[must_use = "method returns result that may indicate an error"]
    pub fn println<T: AsRef<str>>(&mut self, text: T) -> Result<()> {
        self.writer.write_all(text.as_ref().as_bytes())?;
        self.writer.write_all(b"\r\n")?;
        self.writer.flush()
    }

    /// Execute the command.
    ///
    /// This method writes the display for the given command and then flushes
    /// the terminal output.
    #[inline]
    #[must_use = "method returns result that may indicate an error"]
    pub fn exec<C: Command>(&mut self, cmd: C) -> Result<()> {
        write!(self.writer, "{}", cmd)?;
        self.writer.flush()
    }

    /// Execute one command and defer the other.
    ///
    /// This method tries to write the first command to the terminal's output.
    /// If that succeeds, it enqueues the second command for execution when the
    /// connection is being closed and then flushes the output.
    ///
    /// The second command must be `'static`, so that it is alive for the
    /// lifetime of the connection. It must be `Send`, so that connection
    /// objects can be moved across threads. Since most commands are zero-sized
    /// types, they trivially fulfill both requirements.
    #[must_use = "method returns result that may indicate an error"]
    pub fn exec_defer<C1, C2>(&mut self, cmd1: C1, cmd2: C2) -> Result<()>
    where
        C1: Command,
        C2: Command + Send + 'static,
    {
        write!(self.writer, "{}", cmd1)?;
        self.writer.defer(cmd2);
        self.writer.flush()
    }
}

impl Write for Output<'_> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.writer.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        self.writer.flush()
    }
}
