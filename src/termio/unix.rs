use std::fs::OpenOptions;
use std::io::{Error, Read, Result, Write};
use std::num::NonZeroU8;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};
use std::ptr::{from_mut, from_ref};

/// A trait for converting status codes to Rust results.
///
/// The implementations for pairs of signed and unsigned primitive integers of
/// the same size differ solely in the declared types for `Self` and `Unsigned`.
/// Hence, we delegate to a declarative macro.
trait IntoResult {
    /// The unsigned version of `Self`.
    type Unsigned;

    /// Actually convert a signed status code to a Rust result.
    ///
    /// If the status code is negative, this method returns the last OS error.
    /// Otherwise, it returns the wrapped unsigned status code.
    fn into_result(self) -> Result<Self::Unsigned>;
}

macro_rules! into_result {
    ($signed:ty, $unsigned:ty) => {
        impl IntoResult for $signed {
            type Unsigned = $unsigned;

            fn into_result(self) -> Result<Self::Unsigned> {
                if self < 0 {
                    Err(Error::last_os_error())
                } else {
                    Ok(self as Self::Unsigned)
                }
            }
        }
    };
}

into_result!(i32, u32);
into_result!(isize, usize);

// ================================================================================================

/// The timing of terminal configuration updates.
#[derive(Debug)]
#[repr(i32)]
enum When {
    /// Immediately apply the update (`TCSANOW`).
    Now = libc::TCSANOW,

    /// Apply the update after flushing the output and discarding the input
    /// (`TCSAFLUSH`).
    AfterFlush = libc::TCSAFLUSH,
}

/// A terminal configuration.
///
/// This type ensures that all calls to the underlying Posix API are safe. It
/// also ensures that a configuration update is based on a previous
/// configuration for the same terminal, as long as the application uses the
/// same file descriptor for all reads and writes. The application also needs to
/// restore the original configuration upon exit.
#[derive(Clone)]
struct TerminalConfig {
    inner: libc::termios,
}

impl TerminalConfig {
    /// Update the terminal to use cbreak mode as well as the given read timeout
    /// (with a unit of 0.1 seconds) and then return the original configuration.
    pub fn with_cbreak_mode(fd: BorrowedFd<'_>, timeout: NonZeroU8) -> Result<Self> {
        let config = Self::read(fd)?;
        config
            .clone()
            .set_cbreak_mode(timeout)
            .write(fd, When::AfterFlush)?;
        Ok(config)
    }

    /// Update the terminal to use raw mode as well as the given read timeout
    /// (with a unit of 0.1 seconds) and then return the original configuration.
    pub fn with_raw_mode(fd: BorrowedFd<'_>, timeout: NonZeroU8) -> Result<Self> {
        let config = Self::read(fd)?;
        config
            .clone()
            .set_raw_mode(timeout)
            .write(fd, When::AfterFlush)?;
        Ok(config)
    }

    // ---------------------------------------------------------------------------------

    /// Read the configuration for the terminal with the given file descriptor.
    fn read(fd: BorrowedFd<'_>) -> Result<Self> {
        let mut config = std::mem::MaybeUninit::uninit();

        unsafe { libc::tcgetattr(fd.as_raw_fd(), config.as_mut_ptr()) }.into_result()?;

        Ok(Self {
            inner: unsafe { config.assume_init() },
        })
    }

    /// Update the configuration to use cbreak mode with the given read timeout.
    fn set_cbreak_mode(&mut self, timeout: NonZeroU8) -> &Self {
        self.inner.c_lflag &= !(libc::ECHO | libc::ICANON);
        self.inner.c_cc[libc::VMIN] = 0;
        self.inner.c_cc[libc::VTIME] = timeout.get();
        self
    }

    /// Update the configuration to use raw mode with the given read timeout.
    fn set_raw_mode(&mut self, timeout: NonZeroU8) -> &Self {
        unsafe { libc::cfmakeraw(from_mut(&mut self.inner)) };
        self.inner.c_cc[libc::VMIN] = 0;
        self.inner.c_cc[libc::VTIME] = timeout.get();
        self
    }

    /// Write this configuration to the terminal with the file descriptor.
    fn write(&self, fd: BorrowedFd<'_>, when: When) -> Result<()> {
        unsafe { libc::tcsetattr(fd.as_raw_fd(), when as i32, from_ref(&self.inner)) }
            .into_result()?;
        Ok(())
    }
}

impl std::fmt::Debug for TerminalConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Determine enabled flags
        let mut flags = String::new();
        let mut append = |n| {
            if !flags.is_empty() {
                flags.push_str(", ");
            }
            flags.push_str(n);
        };

        for (name, value) in [
            ("BRKINT", libc::BRKINT),
            ("ICRNL", libc::ICRNL),
            ("IGNBRK", libc::IGNBRK),
            ("IGNCR", libc::IGNCR),
            ("INLCR", libc::INLCR),
            ("IXANY", libc::IXANY),
            ("IXOFF", libc::IXOFF),
            ("IXON", libc::IXON),
        ] {
            if self.inner.c_iflag & value != 0 {
                append(name);
            }
        }

        for (name, value) in [
            ("OPOST", libc::OPOST),
            ("OCRNL", libc::OCRNL),
            ("ONOCR", libc::ONOCR),
            ("ONLRET", libc::ONLRET),
        ] {
            if self.inner.c_oflag & value != 0 {
                append(name);
            }
        }

        for (name, value) in [
            ("ECHO", libc::ECHO),
            ("ECHOE", libc::ECHOE),
            ("ECHOK", libc::ECHOK),
            ("ECHONL", libc::ECHONL),
            ("ICANON", libc::ICANON),
            ("IEXTEN", libc::IEXTEN),
            ("ISIG", libc::ISIG),
            ("NOFLSH", libc::NOFLSH),
        ] {
            if self.inner.c_lflag & value != 0 {
                append(name);
            }
        }

        f.debug_struct("TerminalConfig")
            .field("flags", &flags)
            .field("vmin", &self.inner.c_cc[libc::VMIN])
            .field("vtime", &self.inner.c_cc[libc::VTIME])
            .finish()
    }
}

// ================================================================================================

/// A terminal reader.
///
/// The implementation safely wraps the libc API. It assumes that the terminal
/// is in cbreak or raw mode with a read timeout. Because of that timeout, a
/// zero result merely indicates a temporary lack of input.
#[derive(Debug)]
pub struct TerminalReader<'fd> {
    fd: BorrowedFd<'fd>,
}

impl<'fd> TerminalReader<'fd> {
    /// Create a new terminal reader with the given file descriptor.
    fn new(fd: BorrowedFd<'fd>) -> Self {
        Self { fd }
    }
}

impl Read for TerminalReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            libc::read(
                self.fd.as_raw_fd(),
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len() as libc::size_t,
            )
        }
        .into_result()
    }
}

// ================================================================================================

/// A terminal writer.
pub struct TerminalWriter<'fd> {
    fd: BorrowedFd<'fd>,
}

impl<'fd> TerminalWriter<'fd> {
    /// Create a new terminal writer with the given file descriptor.
    fn new(fd: BorrowedFd<'fd>) -> Self {
        Self { fd }
    }
}

impl Write for TerminalWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        unsafe {
            libc::write(
                self.fd.as_raw_fd(),
                buf.as_ptr() as *mut libc::c_void,
                buf.len() as libc::size_t,
            )
        }
        .into_result()
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

// ================================================================================================

/// The terminal.
///
/// The terminal struct owns the file descriptor for the `/dev/tty` device,
/// manages the terminal's configuration for using cbreak or raw mode, and
/// provides both reader and writer for terminal I/O. The generic parameter
/// identifies the current state and enforces proper usage, from [`Start`] to
/// [`Open`] to [`ReadWrite`] to dropped.
pub struct Terminal<S: TerminalState> {
    state: S,
}

/// The terminal's start state.
///
/// To transition from [`Start`] to [`Open`] state, simply [`Terminal::open`]
/// the terminal.
pub struct Start {}

/// The terminal's open state.
///
/// To transition from [`Open`] to [`ReadWrite`] state, configure the terminal
/// to use [`Terminal::cbreak_mode`] or [`Terminal::raw_mode`].
pub struct Open {
    fd: OwnedFd,
}

/// The terminal's read-write state.
///
/// In [`ReadWrite`] state, [`Terminal::reader`] and [`Terminal::writer`]
/// facilitate actual terminal I/O. When done, [`Terminal::restore`] the
/// terminal's original configuration and drop the terminal.
pub struct ReadWrite {
    fd: OwnedFd,
    config: TerminalConfig,
}

mod private {
    /// A seal for traits.
    pub trait Seal {}
}

/// The sealed marker trait for terminal states.
pub trait TerminalState: private::Seal {}

impl private::Seal for Start {}
impl private::Seal for Open {}
impl private::Seal for ReadWrite {}
impl TerminalState for Start {}
impl TerminalState for Open {}
impl TerminalState for ReadWrite {}

impl Terminal<Start> {
    //// Create a new terminal by opening `/dev/tty`.
    pub fn open() -> Result<Terminal<Open>> {
        let fd: OwnedFd = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")?
            .into();

        Ok(Terminal { state: Open { fd } })
    }
}

/// The default timeout, which is 0.1 seconds.
pub const TERMINAL_TIMEOUT: NonZeroU8 = unsafe { NonZeroU8::new_unchecked(1) };

impl Terminal<Open> {
    /// Configure the terminal to use cbreak mode and the given read timeout.
    ///
    /// The unit for timeouts is 0.1 seconds and so is the default
    /// [`TERMINAL_TIMEOUT`].
    pub fn cbreak_mode(self, timeout: NonZeroU8) -> Result<Terminal<ReadWrite>> {
        let fd = self.state.fd;
        let config = TerminalConfig::with_cbreak_mode(fd.as_fd(), timeout)?;

        Ok(Terminal {
            state: ReadWrite { fd, config },
        })
    }

    /// Configure the terminal to use raw mode and the given read timeout.
    ///
    /// The unit for timeouts is 0.1 seconds and so is the default
    /// [`TERMINAL_TIMEOUT`].
    pub fn raw_mode(self, timeout: NonZeroU8) -> Result<Terminal<ReadWrite>> {
        let fd = self.state.fd;
        let config = TerminalConfig::with_raw_mode(fd.as_fd(), timeout)?;

        Ok(Terminal {
            state: ReadWrite { fd, config },
        })
    }
}

impl Terminal<ReadWrite> {
    /// Get the reader for this terminal.
    pub fn reader(&self) -> TerminalReader<'_> {
        TerminalReader::new(self.state.fd.as_fd())
    }

    /// Get the writer for this terminal.
    pub fn writer(&self) -> TerminalWriter<'_> {
        TerminalWriter::new(self.state.fd.as_fd())
    }

    /// Restore the terminal's original configuration.
    pub fn restore(self) -> Result<()> {
        self.state.config.write(self.state.fd.as_fd(), When::Now)
    }
}
