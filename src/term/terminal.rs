use std::cell::RefCell;
use std::io::{BufRead, BufReader, BufWriter, ErrorKind, IoSlice, IoSliceMut, Read, Result, Write};
use std::num::{NonZeroU8, NonZeroUsize};
use std::sync::{Mutex, MutexGuard, OnceLock};

#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use super::sys::{Config, Device, Reader, Writer};

/// The non-canonical terminal mode.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, eq_int, frozen, hash, module = "prettypretty.color.term")
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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

#[derive(Clone, Debug)]
struct OptionData {
    verbose: bool,
    mode: Mode,
    timeout: NonZeroU8,
    read_buffer_size: NonZeroUsize,
    write_buffer_size: usize,
}

impl OptionData {
    fn default_timeout() -> NonZeroU8 {
        use std::env::var_os;

        let mut timeout = 10;
        if var_os("SSH_CLIENT").is_some() || var_os("SSH_CONNECTION").is_some() {
            timeout *= 3;
        }
        NonZeroU8::new(timeout).unwrap()
    }

    #[inline]
    fn default_read_buffer_size() -> NonZeroUsize {
        NonZeroUsize::new(1_024).unwrap()
    }

    #[inline]
    fn default_write_buffer_size() -> usize {
        1_024
    }

    fn new() -> Self {
        Self {
            verbose: false,
            mode: Mode::Rare,
            timeout: OptionData::default_timeout(),
            read_buffer_size: OptionData::default_read_buffer_size(),
            write_buffer_size: OptionData::default_write_buffer_size(),
        }
    }
}

/// A builder for configuring [`Options`].
#[cfg_attr(
    feature = "pyffi",
    pyclass(module = "prettypretty.color.term", unsendable)
)]
#[derive(Debug)]
pub struct OptionBuilder {
    inner: RefCell<OptionData>,
}

#[cfg(feature = "pyffi")]
#[pymethods]
impl OptionBuilder {
    /// Set verbose mode for debugging.
    #[pyo3(name = "verbose")]
    pub fn py_verbose(slf: PyRef<'_, Self>, verbose: bool) -> PyRef<'_, Self> {
        slf.verbose(verbose);
        slf
    }

    /// Set rare or raw mode.
    #[pyo3(name = "mode")]
    pub fn py_mode(slf: PyRef<'_, Self>, mode: Mode) -> PyRef<'_, Self> {
        slf.mode(mode);
        slf
    }

    /// Set the timeout in 0.1s increments.
    #[pyo3(name = "timeout")]
    pub fn py_timeout(slf: PyRef<'_, Self>, timeout: u8) -> PyRef<'_, Self> {
        slf.timeout(timeout);
        slf
    }

    /// Set the read buffer size.
    #[pyo3(name = "read_buffer_size")]
    pub fn py_read_buffer_size(slf: PyRef<'_, Self>, size: usize) -> PyRef<'_, Self> {
        slf.read_buffer_size(size);
        slf
    }

    /// Set the write buffer size.
    #[pyo3(name = "write_buffer_size")]
    pub fn py_write_buffer_size(slf: PyRef<'_, Self>, size: usize) -> PyRef<'_, Self> {
        slf.write_buffer_size(size);
        slf
    }

    /// Build the options.
    #[pyo3(name = "build")]
    pub fn py_build(&self) -> Options {
        self.build()
    }

    /// Get a debug representation for the options.
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl OptionBuilder {
    /// Enable/disable verbose mode for debugging.
    ///
    /// In verbose mode, the opening and closing of a connection is written to
    /// the console through the connection. That implies that opening a
    /// connection is only recorded *after* it has been opened and closing a
    /// connection is recorded *before* it has been closed.
    ///
    /// The default is quiet operation.
    #[inline]
    pub fn verbose(&self, verbose: bool) -> &Self {
        self.inner.borrow_mut().verbose = verbose;
        self
    }

    /// Set rare or raw mode.
    ///
    /// The default is rare mode because it disables only those features that
    /// must be disabled for receiving input from the terminal without delay,
    /// i.e., line editing and character echoing. Meanwhile, raw mode disables
    /// features, including end-of-line and ctrl-c processing, that are
    /// generally useful.
    #[inline]
    pub fn mode(&self, mode: Mode) -> &Self {
        self.inner.borrow_mut().mode = mode;
        self
    }

    /// Set the timeout.
    ///
    /// The default timeout depends on the current runtime context. Notably, it
    /// is three times larger for SSH connections than for local terminals.
    #[inline]
    pub fn timeout(&self, timeout: u8) -> &Self {
        self.inner.borrow_mut().timeout = NonZeroU8::new(timeout).expect("timeout is positive");
        self
    }

    /// Set the read buffer size.
    #[inline]
    pub fn read_buffer_size(&self, size: usize) -> &Self {
        self.inner.borrow_mut().read_buffer_size =
            NonZeroUsize::new(size).expect("read buffer size is positive");
        self
    }

    /// Set the write buffer size.
    #[inline]
    pub fn write_buffer_size(&self, size: usize) -> &Self {
        self.inner.borrow_mut().write_buffer_size = size;
        self
    }

    /// Build the options.
    #[inline]
    pub fn build(&self) -> Options {
        Options {
            inner: self.inner.clone().into_inner(),
        }
    }
}

/// Options for configuring the terminal.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "pyffi", pyclass(frozen, module = "prettypretty.color.term"))]
pub struct Options {
    inner: OptionData,
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Options {
    /// Get a new option builder with the defaults.
    #[cfg(feature = "pyffi")]
    #[pyo3(name = "builder")]
    #[staticmethod]
    pub fn py_builder() -> OptionBuilder {
        Options::builder()
    }

    /// Get the default options, except that verbose mode is enabled.
    #[cfg(feature = "pyffi")]
    #[pyo3(name = "in_verbose")]
    #[staticmethod]
    pub fn py_in_verbose() -> Self {
        Options::in_verbose()
    }

    /// Determine whether verbose mode is enabled.
    #[inline]
    pub fn verbose(&self) -> bool {
        self.inner.verbose
    }

    /// Get the mode.
    #[inline]
    pub fn mode(&self) -> Mode {
        self.inner.mode
    }

    /// Get the timeout.
    #[inline]
    pub fn timeout(&self) -> NonZeroU8 {
        self.inner.timeout
    }

    /// Get the read buffer size.
    #[inline]
    pub fn read_buffer_size(&self) -> NonZeroUsize {
        self.inner.read_buffer_size
    }

    /// Get the write buffer size.
    #[inline]
    pub fn write_buffer_size(&self) -> usize {
        self.inner.write_buffer_size
    }

    /// Get a debug representation for the options.
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl Options {
    /// Get a new option builder with the defaults.
    #[inline]
    pub fn builder() -> OptionBuilder {
        OptionBuilder {
            inner: RefCell::new(OptionData::new()),
        }
    }

    /// Get the default options, except that verbose mode is enabled.
    pub fn in_verbose() -> Self {
        Self::builder().verbose(true).build()
    }
}

impl Default for Options {
    fn default() -> Self {
        Options {
            inner: OptionData::new(),
        }
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
    stamp: u32,

    // Drop the following four fields in that order and last. Reading needs to
    // stop before configuration is restored, though we can still write
    // thereafter. All three must stop before the device is disconnected.
    reader: BufReader<Reader>,
    config: Config,
    writer: BufWriter<Writer>,
    device: Device,
}

impl State {
    /// Access the controlling terminal with the default options.
    ///
    /// This method opens a connection to the controlling terminal and
    /// configures the terminal with the default options for mode and read
    /// timeout.
    pub fn new() -> Result<Self> {
        State::with_options(Options::default())
    }

    /// Access the controlling terminal with the given attributes.
    ///
    /// This method opens a connection to the controlling terminal and
    /// configures the terminal to use the given mode and read timeout.
    pub fn with_options(options: Options) -> Result<Self> {
        let device = Device::new()?;
        let handle = device.handle();
        let config = Config::new(handle, &options)?;
        let reader = BufReader::with_capacity(
            options.read_buffer_size().get(),
            Reader::new(handle.input(), 100 * (options.timeout().get() as u32)),
        );
        let writer =
            BufWriter::with_capacity(options.write_buffer_size(), Writer::new(handle.output()));
        let stamp = if options.verbose() {
            // macOS duration has microsecond resolution only.
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .subsec_micros()
        } else {
            0
        };

        let mut this = Self {
            options,
            device,
            config,
            reader,
            writer,
            stamp,
        };

        if this.options.verbose() {
            write!(
                this.writer,
                // The extra space aligns the close tag
                "\r\ntty::connect    pid={:<5} tid={:<5} in={:?} out={:?} stamp={:>6}\r\n",
                std::process::id(),
                this.device.pid().unwrap_or(0),
                handle.input(),
                handle.output(),
                stamp,
            )?;
            this.writer.flush()?;
        }

        Ok(this)
    }

    /// Get the process group ID.
    #[inline]
    pub fn pid(&self) -> Result<u32> {
        self.device.pid()
    }

    /// Get the options.
    #[inline]
    pub fn options(&self) -> &Options {
        &self.options
    }
}

// State is Send on macOS because raw file descriptors are just numbers. However
// HANDLE on Windows is a *mut T, which is an invitation for trouble in Rust.
#[cfg(target_family = "windows")]
unsafe impl Send for State {}

impl Drop for State {
    fn drop(&mut self) {
        // No need to flush, see below.
        if 0 < self.stamp {
            let _ = write!(
                self.writer,
                "tty::disconnect pid={:<5} tid={:<5} in={:?} out={:?} stamp={:>6}\r\n",
                std::process::id(),
                self.device.pid().unwrap_or(0),
                self.device.handle().input(),
                self.device.handle().output(),
                self.stamp,
            );
        }

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
            if state.options().mode() == options.mode()
                && state.options().timeout() == options.timeout()
            {
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
            if state.options().mode() == options.mode()
                && state.options().timeout() == options.timeout()
            {
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

    /// Get the terminal's process group ID.
    #[inline]
    pub fn pid(&self) -> Result<u32> {
        self.inner.as_ref().unwrap().pid()
    }

    /// Get the terminal's current options.
    #[inline]
    pub fn options(&self) -> &Options {
        self.inner.as_ref().unwrap().options()
    }

    /// Write the entire buffer and flush thereafter.
    pub fn print_bytes(&mut self, buf: &[u8]) -> Result<()> {
        self.write_all(buf)?;
        self.flush()
    }

    /// Write the display of the value and flush thereafter.
    pub fn print<T: std::fmt::Display>(&mut self, value: T) -> Result<()> {
        self.write_fmt(format_args!("{}", value))?;
        self.flush()
    }
}

impl Read for TerminalAccess<'_> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.get_mut().reader.read(buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        self.get_mut().reader.read_vectored(bufs)
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.get_mut().reader.read_to_end(buf)
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        self.get_mut().reader.read_to_string(buf)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.get_mut().reader.read_exact(buf)
    }
}

impl BufRead for TerminalAccess<'_> {
    #[inline]
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.get_mut().reader.fill_buf()
    }

    #[inline]
    fn consume(&mut self, n: usize) {
        self.get_mut().reader.consume(n)
    }

    #[inline]
    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
        self.get_mut().reader.read_until(byte, buf)
    }

    #[inline]
    fn read_line(&mut self, buf: &mut String) -> Result<usize> {
        self.get_mut().reader.read_line(buf)
    }
}

impl Write for TerminalAccess<'_> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.get_mut().writer.write(buf)
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        self.get_mut().writer.write_vectored(bufs)
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        self.get_mut().writer.flush()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.get_mut().writer.write_all(buf)
    }

    #[inline]
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
