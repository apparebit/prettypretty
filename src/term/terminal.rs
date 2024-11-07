use std::convert::AsRef;
use std::io::{BufRead, BufReader, BufWriter, ErrorKind, IoSlice, IoSliceMut, Read, Result, Write};
use std::num::{NonZeroU8, NonZeroUsize};
use std::sync::{Mutex, MutexGuard, OnceLock};

use super::sys::{Config, Device, Reader, Writer};

/// The non-canonical terminal mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// Rare mode, also known as cbreak mode, disables canonical line processing
    /// and echoing of characters, but continues processing end-of-line
    /// characters and signals. This is the default.
    #[default]
    Rare,
    /// Raw mode disables all terminal features beyond reading input and writing
    /// output including ctrl-c for signalling a process to terminate.
    Raw,
}

/// Options for configuring the terminal.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// The terminal mode.
    pub mode: Mode,

    /// The read timeout in 0.1s increments.
    pub timeout: NonZeroU8,

    /// The size of the read buffer.
    ///
    /// Parsing ANSI escape sequences requires a lookahead of one byte. Hence,
    /// this option must be positive.
    pub read_buffer: NonZeroUsize,

    /// The size of the write buffer.
    ///
    /// If this size is zero, writing to the terminal is effectively unbuffered.
    pub write_buffer: usize,
}

impl Options {
    /// Create a new terminal options object with the default values.
    pub const fn new() -> Self {
        Self {
            mode: Mode::Rare,
            timeout: unsafe { NonZeroU8::new_unchecked(1) },
            read_buffer: unsafe { NonZeroUsize::new_unchecked(1_024) },
            write_buffer: 1_024,
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}

// ------------------------------------------------------------------------------------------------

/// The terminal state.
///
/// This struct owns the connection to the terminal device as well as the
/// configuration, reader, and writer objects. On drop, it restores the original
/// configuration and closes the connection.
#[derive(Debug)]
struct State {
    options: Options,
    #[allow(dead_code)]
    device: Device,
    config: Config,
    reader: BufReader<Reader>,
    writer: BufWriter<Writer>,
}

impl State {
    /// Access the controlling terminal with the default options.
    ///
    /// This method opens a connection to the controlling terminal and
    /// configures the terminal with the default options for mode and read
    /// timeout.
    pub fn new() -> Result<Self> {
        State::with_options(Options::new())
    }

    /// Access the controlling terminal with the given attributes.
    ///
    /// This method opens a connection to the controlling terminal and
    /// configures the terminal to use the given mode and read timeout.
    pub fn with_options(options: Options) -> Result<Self> {
        let device = Device::new()?;
        let handle = device.handle();
        let config = Config::new(handle, &options)?;
        let reader =
            BufReader::with_capacity(options.read_buffer.get(), Reader::new(handle.input()));
        let writer = BufWriter::with_capacity(options.write_buffer, Writer::new(handle.output()));

        Ok(Self {
            options,
            device,
            config,
            reader,
            writer,
        })
    }

    /// Get the current terminal mode.
    pub fn mode(&self) -> Mode {
        self.options.mode
    }

    /// Get the current timeout.
    pub fn timeout(&self) -> NonZeroU8 {
        self.options.timeout
    }

    /// Get the size of the read buffer.
    pub fn read_buffer(&self) -> NonZeroUsize {
        self.options.read_buffer
    }

    /// Get the size of the write buffer.
    pub fn write_buffer(&self) -> usize {
        self.options.write_buffer
    }
}

// State is Send on macOS because raw file descriptors are just numbers. However
// HANDLE on Windows is a *mut T, which is an invitation for trouble in Rust.
#[cfg(target_family = "windows")]
unsafe impl Send for State {}

impl Drop for State {
    fn drop(&mut self) {
        // Make sure all output has been written.
        let _ = self.writer.flush();
        let _ = self.config.restore();
    }
}

// ------------------------------------------------------------------------------------------------

/// Access the controlling terminal.
pub fn terminal() -> Terminal {
    static TERMINAL: OnceLock<Mutex<Option<State>>> = OnceLock::new();
    Terminal {
        inner: TERMINAL.get_or_init(|| Mutex::new(None)),
    }
}

/// The controlling terminal.
///
/// This struct multiplexes access to I/O with the terminal device. The main use
/// cases for direct terminal I/O are reading key presses as they are typed and
/// querying the terminal by exchanging ANSI escape sequences. For all other
/// uses, including styling text, applications should continue using the
/// standard output and error streams.
///
/// An application has two options for interacting with the terminal device:
///
///  1. If the application needs to perform terminal I/O throughout its
///     lifetime, it should [`Terminal::connect`] to the terminal device at
///     startup and [`Terminal::disconnect`] during shutdown again. When needed,
///     it should [`Terminal::try_access`] terminal I/O, treating `None` as
///     indicator for termination.
///  2. If the application performs only occasional terminal I/O, it can just
///     [`Terminal::access`] terminal I/O. That method establishes a temporary
///     connection, if the terminal device is not currently connected, and
///     otherwise just reuses the existing connection.
///
/// [`Terminal::access`] is also suitable for interactive usage of prettypretty,
/// e.g., from within an interactive Python interpreter.
pub struct Terminal {
    inner: &'static Mutex<Option<State>>,
}

impl Terminal {
    #[inline]
    fn lock(&mut self) -> MutexGuard<'static, Option<State>> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Connect to the controlling terminal.
    ///
    /// If the controlling terminal is not currently connected, this method
    /// connects to the terminal device and configures it to the default mode
    /// and read timeout. If the controlling terminal is already connected, this
    /// method does nothing.
    ///
    /// # Safety
    ///
    /// When manually managing the terminal connection, the application should
    /// invoke [`Terminal::disconnect`] before exiting to restore the terminal's
    /// original configuration. Hence, manually connecting to the terminal is an
    /// inherently unsafe operation.
    pub unsafe fn connect(mut self) -> Result<Self> {
        let mut inner = self.lock();
        if inner.is_none() {
            let state = State::new()?;
            *inner = Some(state);
        }
        Ok(self)
    }

    /// Connect to the controlling terminal with the given attributes.
    ///
    /// If the controlling terminal is not currently connected, this method
    /// connects to the terminal device and configures it to use the given mode
    /// and read timeout. If the controlling terminal is already connected, this
    /// method returns an `InvalidInput` error, unless both mode and read
    /// timeout are the same as the terminal's.
    ///
    /// # Safety
    ///
    /// When manually managing the terminal connection, the application should
    /// invoke [`Terminal::disconnect`] before exiting to restore the terminal's
    /// original configuration. Hence, manually connecting to the terminal is an
    /// inherently unsafe operation.
    pub unsafe fn connect_with(mut self, options: Options) -> Result<Self> {
        let mut inner = self.lock();
        if let Some(state) = &*inner {
            if state.mode() == options.mode && state.timeout() == options.timeout {
                Ok(self)
            } else {
                Err(ErrorKind::InvalidInput.into())
            }
        } else {
            let state = State::with_options(options)?;
            *inner = Some(state);
            Ok(self)
        }
    }

    /// Try accessing terminal I/O.
    ///
    /// If the controlling terminal is currently connected, this method returns
    /// an object providing exclusive access to terminal I/O. Dropping the
    /// object relinquishes access again. If the controlling terminal is not
    /// currently connected, this method returns `None`.
    pub fn try_access(mut self) -> Option<TerminalAccess<'static>> {
        let inner = self.lock();
        if inner.is_some() {
            Some(TerminalAccess::new(inner, OnDrop::DoNothing))
        } else {
            None
        }
    }

    /// Access terminal I/O.
    ///
    /// If the controlling terminal is currently connected, this method returns
    /// an object providing exclusive access to terminal I/O. Dropping the
    /// object relinquishes access again.
    ///
    /// If the controlling terminal is not currently connected, this method
    /// connects to the terminal, reconfigures it to use the default mode and
    /// read timeout, and then returns an object providing exclusive access to
    /// terminal I/O. Dropping the object not only relinquishes access again but
    /// also disconnects the terminal.
    pub fn access(mut self) -> Result<TerminalAccess<'static>> {
        let mut inner = self.lock();
        if inner.is_some() {
            Ok(TerminalAccess::new(inner, OnDrop::DoNothing))
        } else {
            *inner = Some(State::new()?);
            Ok(TerminalAccess::new(inner, OnDrop::Disconnect))
        }
    }

    /// Access terminal I/O.
    ///
    /// If the controlling terminal is currently connected and uses the same
    /// mode and read timeout, this method returns an object providing exclusive
    /// access to terminal I/O. Dropping the object relinquishes access again.
    /// If the controlling terminal is currently connected and used a different
    /// mode or read timeout, this method returns an `InvalidInput` error.
    ///
    /// If the controlling terminal is not currently connected, this method
    /// connects to the terminal, reconfigures it to use the given mode and read
    /// timeout, and then returns an object providing exclusive access to
    /// terminal I/O. Dropping the object not only relinquishes access again but
    /// also disconnects the terminal.
    pub fn access_with(mut self, options: Options) -> Result<TerminalAccess<'static>> {
        let mut inner = self.lock();
        if let Some(state) = &*inner {
            if state.mode() == options.mode && state.timeout() == options.timeout {
                Ok(TerminalAccess::new(inner, OnDrop::DoNothing))
            } else {
                Err(ErrorKind::InvalidInput.into())
            }
        } else {
            *inner = Some(State::with_options(options)?);
            Ok(TerminalAccess::new(inner, OnDrop::Disconnect))
        }
    }

    /// Disconnect the controlling terminal.
    ///
    /// If the controlling terminal is currently connected, this method restores
    /// its original configuration and disconnects from the terminal device.
    /// Otherwise, it does nothing.
    ///
    /// Unlike connecting to the terminal device, disconnecting again is a safe
    /// operation. First, `disconnect` itself restores the terminal's original
    /// configuration. Second, [`Terminal::try_access`], [`Terminal::access`],
    /// and [`Terminal::access_with`] have well-defined semantics when the
    /// terminal device is not connected, with `try_access` returning `None` and
    /// the other two methods opening temporary connections that are
    /// automatically closed.
    pub fn disconnect(mut self) {
        let mut inner = self.lock();
        drop(inner.take());
    }
}

// ------------------------------------------------------------------------------------------------

/// Flag for drop action.
#[derive(Debug)]
enum OnDrop {
    DoNothing,
    Disconnect,
}

/// Exclusive access to terminal I/O.
///
/// This struct holds the mutex guaranteeing exclusive access for the duration
/// of its lifetime `'a`.
pub struct TerminalAccess<'a> {
    inner: MutexGuard<'a, Option<State>>,
    on_drop: OnDrop,
}

impl<'a> TerminalAccess<'a> {
    /// Grant terminal access without impacting the terminal connection.
    fn new(inner: MutexGuard<'a, Option<State>>, on_drop: OnDrop) -> Self {
        assert!(inner.is_some());
        Self { inner, on_drop }
    }
}

impl TerminalAccess<'_> {
    /// Get a mutable reference to the inner terminal state.
    #[inline]
    fn get_mut(&mut self) -> &mut State {
        // SAFETY: The option has a value as long as the terminal is connected.
        // The terminal is connected upon creation of this instance (see
        // assertion in new() above) and it can be disconnected only by dropping
        // this instance or by calling Terminal::disconnect, which needs to
        // reacquire the mutex.
        self.inner.as_mut().unwrap()
    }

    /// Get the terminal's current mode.
    pub fn mode(&self) -> Mode {
        self.inner.as_ref().unwrap().mode()
    }

    /// Get the terminal's current read timeout.
    pub fn timeout(&self) -> u8 {
        self.inner.as_ref().unwrap().timeout().get()
    }

    /// Get the size of the read buffer.
    pub fn read_buffer(&self) -> usize {
        self.inner.as_ref().unwrap().read_buffer().get()
    }

    /// Get the size of the write buffer.
    pub fn write_buffer(&self) -> usize {
        self.inner.as_ref().unwrap().write_buffer()
    }

    /// Write the entire buffer and flush thereafter.
    pub fn print_bytes(&mut self, buf: &[u8]) -> Result<()> {
        self.write_all(buf)?;
        self.flush()
    }

    /// Write the entire string slice and flush thereafter.
    pub fn print(&mut self, s: impl AsRef<str>) -> Result<()> {
        self.write_all(s.as_ref().as_bytes())?;
        self.flush()
    }
}

impl Read for TerminalAccess<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.get_mut().reader.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        self.get_mut().reader.read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.get_mut().reader.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        self.get_mut().reader.read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.get_mut().reader.read_exact(buf)
    }
}

impl BufRead for TerminalAccess<'_> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.get_mut().reader.fill_buf()
    }

    fn consume(&mut self, n: usize) {
        self.get_mut().reader.consume(n)
    }

    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
        self.get_mut().reader.read_until(byte, buf)
    }

    fn read_line(&mut self, buf: &mut String) -> Result<usize> {
        self.get_mut().reader.read_line(buf)
    }
}

impl Write for TerminalAccess<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.get_mut().writer.write(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        self.get_mut().writer.write_vectored(bufs)
    }

    fn flush(&mut self) -> Result<()> {
        self.get_mut().writer.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.get_mut().writer.write_all(buf)
    }

    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> Result<()> {
        self.get_mut().writer.write_fmt(args)
    }
}

impl std::fmt::Debug for TerminalAccess<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::ops::Deref;

        // If we have a TerminalAccess instance, we must be holding the mutex.
        f.debug_struct("TerminalAccess")
            .field("inner.lock.data", self.inner.deref())
            .field("on_drop", &self.on_drop)
            .finish()
    }
}

impl Drop for TerminalAccess<'_> {
    fn drop(&mut self) {
        if let OnDrop::Disconnect = self.on_drop {
            drop(self.inner.take())
        }
    }
}
