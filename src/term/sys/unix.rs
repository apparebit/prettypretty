//! Module to isolate unsafe libc operations.
//!
//! This module abstracts over the underlying libc invocations for managing the
//! terminal configuration, reading from the terminal, and writing to the
//! terminal. They are safe, as long as the file descriptors are valid. For that
//! same reason, [`TerminalConfig`], [`TerminalReader`], and [`TerminalWriter`]
//! must not be directly exposed to application code.

use std::io::{Read, Result, Write};
use std::num::NonZeroU8;
use std::os::fd::{AsRawFd, RawFd};
use std::ptr::{from_mut, from_ref};

/// A trait for converting libc status codes to Rust std::io results.
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
    /// Otherwise, it returns the now unsigned status code wrapped as a result.
    fn into_result(self) -> Result<Self::Unsigned>;
}

macro_rules! into_result {
    ($signed:ty, $unsigned:ty) => {
        impl IntoResult for $signed {
            type Unsigned = $unsigned;

            fn into_result(self) -> Result<Self::Unsigned> {
                if self < 0 {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(self as Self::Unsigned)
                }
            }
        }
    };
}

into_result!(i32, u32);
into_result!(isize, usize);

// ------------------------------------------------------------------------------------------------

/// The non-canonical terminal mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalMode {
    /// Cbreak mode disables canonical line processing and echoing of
    /// characters, but continues processing end-of-line characters and signals.
    Cbreak,
    /// Raw mode disables all terminal features beyond reading input and writing
    /// output, notably including ctrl-c for signalling a process to terminate.
    Raw,
}

/// The timing of terminal configuration updates.
#[allow(dead_code)]
#[derive(Debug)]
#[repr(i32)]
enum When {
    /// Immediately apply the update (`TCSANOW`).
    Now = libc::TCSANOW,

    /// Apply the update after flushing the output (`TCSADRAIN`).
    AfterOutputFlush = libc::TCSADRAIN,

    /// Apply the update after flushing the output and discarding the input
    /// (`TCSAFLUSH`).
    AfterFlush = libc::TCSAFLUSH,
}

/// The actual terminal attributes.
#[derive(Clone)]
struct Termios {
    inner: libc::termios,
}

impl Termios {
    /// Create a new instance
    fn new(inner: libc::termios) -> Self {
        Self { inner }
    }
}

impl AsRef<libc::termios> for Termios {
    fn as_ref(&self) -> &libc::termios {
        &self.inner
    }
}

impl AsMut<libc::termios> for Termios {
    fn as_mut(&mut self) -> &mut libc::termios {
        &mut self.inner
    }
}

impl std::fmt::Debug for Termios {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Determine enabled flags
        let mut flags = String::new();
        let mut append = |s| {
            if !flags.is_empty() {
                flags.push_str(", ");
            }
            flags.push_str(s);
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

        f.debug_struct("Termios")
            .field("flags", &flags)
            .field("vmin", &self.inner.c_cc[libc::VMIN])
            .field("vtime", &self.inner.c_cc[libc::VTIME])
            .finish()
    }
}

// ------------------------------------------------------------------------------------------------

/// A terminal configuration.
///
/// This struct ensures that calls to the underlying Posix API are safe and that
/// a configuration update is based on a previous configuration for the same
/// terminal.
#[derive(Debug)]
pub(crate) struct TerminalConfig {
    fd: RawFd,
    mode: TerminalMode,
    timeout: NonZeroU8,
    attributes: Termios,
}

impl TerminalConfig {
    /// The default mode.
    pub const MODE: TerminalMode = TerminalMode::Cbreak;

    /// The default timeout.
    pub const TIMEOUT: NonZeroU8 = unsafe { NonZeroU8::new_unchecked(1) };

    /// Configure the terminal to use the default mode and read timeout.
    ///
    /// This method reads the current terminal configuration, updates a copy of
    /// the configuration, writes the updated copy, and returns the original.
    pub fn configure(fd: impl AsRawFd) -> Result<Self> {
        TerminalConfig::with_attributes(fd, Self::MODE, Self::TIMEOUT)
    }

    /// Configure the terminal with the given mode and read timeout.
    ///
    /// This method reads the current terminal configuration, updates a copy of
    /// the configuration, writes the updated copy, and returns the original.
    pub fn with_attributes(
        fd: impl AsRawFd,
        mode: TerminalMode,
        timeout: NonZeroU8,
    ) -> Result<Self> {
        let fd = fd.as_raw_fd();
        let attributes = Self::read(fd)?;

        Self::write(
            fd,
            When::AfterFlush,
            &Self::update(&attributes, mode, timeout),
        )?;

        Ok(Self {
            fd,
            mode,
            timeout,
            attributes,
        })
    }

    /// Get the current terminal mode.
    pub fn mode(&self) -> TerminalMode {
        self.mode
    }

    /// Get the current read timeout.
    pub fn timeout(&self) -> NonZeroU8 {
        self.timeout
    }

    /// Reconfigure the terminal to use the given mode and read timeout.
    ///
    /// If the terminal is using a different mode or timeout, this method
    /// updates a copy of the original configuration and writes the updated copy
    /// to the terminal.
    #[allow(dead_code)]
    pub fn reconfigure(&mut self, mode: TerminalMode, timeout: NonZeroU8) -> Result<()> {
        if self.mode != mode || self.timeout != timeout {
            Self::write(
                self.fd,
                When::AfterFlush,
                &Self::update(&self.attributes, mode, timeout),
            )?;
            self.mode = mode;
            self.timeout = timeout;
        }
        Ok(())
    }

    /// Restore the original terminal configuration.
    pub fn restore(&self) -> Result<()> {
        Self::write(self.fd, When::AfterOutputFlush, &self.attributes)
    }

    // ---------------------------------------------------------------------------------

    /// Read the configuration for the terminal with the given file descriptor.
    fn read(fd: RawFd) -> Result<Termios> {
        let mut attributes = std::mem::MaybeUninit::uninit();
        unsafe { libc::tcgetattr(fd, attributes.as_mut_ptr()) }.into_result()?;
        Ok(Termios::new(unsafe { attributes.assume_init() }))
    }

    /// Create an updated configuration with the given mode and timeout.
    fn update(attributes: &Termios, mode: TerminalMode, timeout: NonZeroU8) -> Termios {
        let mut wrapper = attributes.clone();
        let inner = wrapper.as_mut();

        match mode {
            TerminalMode::Cbreak => {
                inner.c_lflag &= !(libc::ECHO | libc::ICANON);
            }
            TerminalMode::Raw => {
                unsafe { libc::cfmakeraw(from_mut(inner)) };
            }
        }

        inner.c_cc[libc::VMIN] = 0;
        inner.c_cc[libc::VTIME] = timeout.get();
        wrapper
    }

    /// Write this configuration to the terminal with the file descriptor.
    fn write(fd: RawFd, when: When, attributes: &Termios) -> Result<()> {
        unsafe { libc::tcsetattr(fd, when as i32, from_ref(attributes.as_ref())) }.into_result()?;
        Ok(())
    }
}

// ------------------------------------------------------------------------------------------------

/// A terminal reader.
#[derive(Debug)]
pub(crate) struct TerminalReader {
    fd: RawFd,
}

impl TerminalReader {
    /// Create a new reader with a raw file descriptor.
    pub fn new(fd: impl AsRawFd) -> Self {
        Self { fd: fd.as_raw_fd() }
    }
}

impl Read for TerminalReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            libc::read(
                self.fd,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len() as libc::size_t,
            )
        }
        .into_result()
    }
}

// ------------------------------------------------------------------------------------------------

/// A terminal writer.
#[derive(Debug)]
pub(crate) struct TerminalWriter {
    fd: RawFd,
}

impl TerminalWriter {
    /// Create a new writer with a raw file descriptor.
    pub fn new(fd: impl AsRawFd) -> Self {
        Self { fd: fd.as_raw_fd() }
    }
}

impl Write for TerminalWriter {
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
