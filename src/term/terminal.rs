use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, BufWriter, ErrorKind, IoSlice, IoSliceMut, Read, Result, Write};
use std::num::NonZeroU8;
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::{Mutex, MutexGuard, OnceLock};

use super::sys::{TerminalConfig, TerminalMode, TerminalReader, TerminalWriter};

/// The terminal state.
///
/// This struct owns the file descriptor for the connection to the terminal
/// device as well as the configuration, reader, and writer objects. On drop, it
/// restores the original configuration and closes the connection.
#[derive(Debug)]
struct TerminalState {
    #[allow(dead_code)]
    fd: OwnedFd,
    config: TerminalConfig,
    reader: BufReader<TerminalReader>,
    writer: BufWriter<TerminalWriter>,
}

impl TerminalState {
    /// Create a new owned file descriptor for the controlling terminal.
    fn open_device() -> Result<OwnedFd> {
        Ok(OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")?
            .into())
    }

    /// Access the controlling terminal with the default attributes.
    ///
    /// This method opens a connection to the controlling terminal and
    /// configures the terminal to use cbreak mode and a 0.1s read timeout.
    pub fn new() -> Result<Self> {
        TerminalState::with_attributes(Terminal::MODE, Terminal::TIMEOUT)
    }

    /// Access the controlling terminal with the given attributes.
    ///
    /// This method opens a connection to the controlling terminal and
    /// configures the terminal to use the given mode and read timeout.
    pub fn with_attributes(mode: TerminalMode, timeout: NonZeroU8) -> Result<Self> {
        let fd = Self::open_device()?;
        let raw_fd = fd.as_raw_fd();
        let config = TerminalConfig::with_attributes(raw_fd, mode, timeout)?;

        Ok(Self {
            fd,
            config,
            reader: BufReader::with_capacity(Terminal::BUFFER_SIZE, TerminalReader::new(raw_fd)),
            writer: BufWriter::with_capacity(Terminal::BUFFER_SIZE, TerminalWriter::new(raw_fd)),
        })
    }

    /// Get the current terminal mode.
    pub fn mode(&self) -> TerminalMode {
        self.config.mode()
    }

    /// Get the current timeout.
    pub fn timeout(&self) -> NonZeroU8 {
        self.config.timeout()
    }
}

impl Drop for TerminalState {
    fn drop(&mut self) {
        // Make sure all output has been written.
        let _ = self.writer.flush();
        let _ = self.config.restore();
    }
}

// ------------------------------------------------------------------------------------------------

/// Access the terminal.
pub fn terminal() -> Terminal {
    static TERMINAL: OnceLock<Mutex<Option<TerminalState>>> = OnceLock::new();
    Terminal {
        inner: TERMINAL.get_or_init(|| Mutex::new(None)),
    }
}

/// The terminal.
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
    inner: &'static Mutex<Option<TerminalState>>,
}

impl Terminal {
    /// The buffer size.
    pub const BUFFER_SIZE: usize = 1024;

    /// The default mode.
    pub const MODE: TerminalMode = TerminalConfig::MODE;

    /// The default timeout.
    pub const TIMEOUT: NonZeroU8 = TerminalConfig::TIMEOUT;

    #[inline]
    fn lock(&mut self) -> MutexGuard<'static, Option<TerminalState>> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Connect to the controlling terminal.
    ///
    /// If the controlling terminal is not currently connected, this method
    /// connects to the terminal device and configures it to the default mode
    /// and read timeout. If the controlling terminal is already connected, this
    /// method does nothing.
    pub fn connect(mut self) -> Result<Self> {
        let mut inner = self.lock();
        if inner.is_none() {
            let state = TerminalState::new()?;
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
    pub fn connect_with(mut self, mode: TerminalMode, timeout: NonZeroU8) -> Result<Self> {
        let mut inner = self.lock();
        if let Some(state) = &*inner {
            if state.mode() == mode && state.timeout() == timeout {
                Ok(self)
            } else {
                Err(ErrorKind::InvalidInput.into())
            }
        } else {
            let state = TerminalState::with_attributes(mode, timeout)?;
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
            *inner = Some(TerminalState::new()?);
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
    pub fn access_with(
        mut self,
        mode: TerminalMode,
        timeout: NonZeroU8,
    ) -> Result<TerminalAccess<'static>> {
        let mut inner = self.lock();
        if let Some(state) = &*inner {
            if state.mode() == mode && state.timeout() == timeout {
                Ok(TerminalAccess::new(inner, OnDrop::DoNothing))
            } else {
                Err(ErrorKind::InvalidInput.into())
            }
        } else {
            *inner = Some(TerminalState::with_attributes(mode, timeout)?);
            Ok(TerminalAccess::new(inner, OnDrop::Disconnect))
        }
    }

    /// Disconnect the controlling terminal.
    ///
    /// If the controlling terminal is currently connected, this method restores
    /// its original configuration and disconnects from the terminal device.
    /// Otherwise, it does nothing.
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

/// An object providing exclusive access to terminal I/O.
///
/// This struct holds the mutex guaranteeing exclusive access for the duration
/// of its lifetime `'a`.
pub struct TerminalAccess<'a> {
    inner: MutexGuard<'a, Option<TerminalState>>,
    on_drop: OnDrop,
}

impl<'a> TerminalAccess<'a> {
    /// Grant terminal access without impacting the terminal connection.
    fn new(inner: MutexGuard<'a, Option<TerminalState>>, on_drop: OnDrop) -> Self {
        assert!(inner.is_some());
        Self { inner, on_drop }
    }
}

impl TerminalAccess<'_> {
    /// Get a mutable reference to the inner terminal state.
    #[inline]
    fn get_mut(&mut self) -> &mut TerminalState {
        // SAFETY: The option has a value as long as the terminal is connected.
        // The terminal is connected upon creation of this instance (see
        // assertion in new() above) and it can be disconnected only by dropping
        // this instance or by calling Terminal::disconnect, which needs to
        // reacquire the mutex.
        self.inner.as_mut().unwrap()
    }

    /// Write the entire buffer and flush thereafter.
    pub fn print_bytes(&mut self, buf: &[u8]) -> Result<()> {
        self.write_all(buf)?;
        self.flush()
    }

    /// Write the entire string slice and flush thereafter.
    pub fn print(&mut self, s: &str) -> Result<()> {
        self.write_all(s.as_bytes())?;
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
