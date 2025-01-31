use std::io::{BufRead, BufWriter, Error, ErrorKind, Read, Result, Write};
use std::sync::{Mutex, MutexGuard};

use crate::opt::{Options, Volume};
use crate::read::{DoggedReader, VerboseReader};
use crate::scan::Scanner;
use crate::sys::{RawConfig, RawConnection, RawOutput};
use crate::{Command, Scan};

/// A terminal connection providing [`Input`] and [`Output`].
///
/// This object owns the connection to the terminal. It provides independent,
/// mutually exclusive, and thread-safe access to [`Input`] as well as
/// [`Output`]. On Unix, the I/O types share the same underlying file
/// descriptor, whereas on Windows each I/O type uses a distinct handle.
///
/// Since a connection temporarily reconfigures the terminal, **an application
/// should go out of its way to always execute this type's drop handler** before
/// exit.
pub struct Connection {
    options: Options,
    stamp: u32,
    config: Option<RawConfig>,
    scanner: Mutex<Scanner<Box<dyn Read + Send>>>,
    writer: Mutex<BufWriter<RawOutput>>,
    connection: RawConnection,
}

fn _assert_connection_is_sync_send() {
    fn is_sync_send<T: Sync + Send>() {}
    is_sync_send::<Connection>();
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
        let writer = Mutex::new(BufWriter::with_capacity(
            options.write_buffer_size(),
            connection.output(),
        ));
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
    #[inline]
    pub fn input(&self) -> Input {
        Input {
            scanner: self.scanner.lock().expect("mutex is not poisoned"),
        }
    }

    /// Get the terminal output.
    ///
    /// The returned output object ensures mutually exclusive access to the
    /// terminal's output. Dropping the output object releases access again.
    #[inline]
    pub fn output(&self) -> Output {
        Output {
            writer: self.writer.lock().expect("mutex is not poisoned"),
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

        // map() avoids panic for poisoned mutex
        let _ = self.writer.lock().map(|mut writer| {
            let _ = writer.flush();
        });

        // Restore terminal configuration
        if let Some(cfg) = &self.config {
            let _ = cfg.write(&self.connection);
        }
    }
}

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
    scanner: MutexGuard<'a, Scanner<Box<dyn Read + Send>>>,
}

impl Scan for Input<'_> {
    #[inline]
    fn in_flight(&self) -> bool {
        self.scanner.in_flight()
    }

    #[inline]
    fn read_token(&mut self) -> Result<crate::Token> {
        self.scanner.read_token().map_err(|e| e.into())
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
        self.scanner.fill_buf().map_err(|e| e.into())
    }

    #[inline]
    fn consume(&mut self, amt: usize) {
        // Don't panic...
        let _ = self.scanner.consume(amt);
    }
}

/// A terminal [`Connection`]'s output.
///
/// Since terminal output is buffered, actually executing commands requires
/// flushing the output. As a convenience, [`Output::print`] and
/// [`Output::println`] write strings and [`Output::exec`] writes individual
/// commands, while also flushing the output on every invocation.
#[derive(Debug)]
pub struct Output<'a> {
    writer: MutexGuard<'a, BufWriter<RawOutput>>,
}

impl Output<'_> {
    /// Write and flush the text.
    #[inline]
    pub fn print(&mut self, text: impl AsRef<str>) -> Result<()> {
        self.writer.write_all(text.as_ref().as_bytes())?;
        self.writer.flush()
    }

    /// Write and flush the text followed by carriage return and line feed.
    #[inline]
    pub fn println(&mut self, text: impl AsRef<str>) -> Result<()> {
        self.writer.write_all(text.as_ref().as_bytes())?;
        self.writer.write_all(b"\r\n")?;
        self.writer.flush()
    }

    /// Write and flush the command.
    #[inline]
    pub fn exec(&mut self, cmd: impl Command) -> Result<()> {
        write!(self.writer, "{}", cmd)?;
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
