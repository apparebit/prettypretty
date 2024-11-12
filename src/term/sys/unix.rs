//! Module to isolate unsafe libc operations.
//!
//! This module abstracts over the underlying libc invocations for managing the
//! terminal configuration, reading from the terminal, and writing to the
//! terminal. They are safe, as long as the file descriptors are valid. For that
//! same reason, [`TerminalConfig`], [`TerminalReader`], and [`TerminalWriter`]
//! must not be directly exposed to application code.

use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::{Read, Result, Write};
use std::os::fd::{AsRawFd, OwnedFd};
use std::ptr::{from_mut, from_ref};

use super::{into_result::IntoResult, RawHandle};
use crate::term::{Mode, Options};

// ------------------------------------------------------------------------------------------------

/// A connection to the terminal device.
///
/// [`Device::new`] opens a new connection to the terminal device and closes
/// that connection again when dropped. Since [`Device::read_handle`] and
/// [`Device::write_handle`] return raw handles, it is the caller's
/// responsibility to ensure that a raw handle is not used past the connection's
/// lifetime.
#[derive(Debug)]
pub(crate) struct Device {
    fd: OwnedFd,
}

impl Device {
    /// Open a new owned connection to the terminal device.
    pub fn new() -> Result<Self> {
        let fd = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")?
            .into();

        Ok(Self { fd })
    }

    /// Get process group ID.
    #[inline]
    pub fn pid(&self) -> Result<u32> {
        unsafe { libc::tcgetsid(self.fd.as_raw_fd()) }.into_result()
    }

    /// Get a handle for the device.
    #[inline]
    pub fn handle(&self) -> DeviceHandle {
        DeviceHandle(self.fd.as_raw_fd())
    }
}

/// A handle to the terminal device.
///
/// While strictly unnecessary on Unix, the Windows version is helpful because
/// it combines two raw handles.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DeviceHandle(RawHandle);

impl DeviceHandle {
    /// Access the raw handle for terminal input.
    #[inline]
    pub fn input(&self) -> RawHandle {
        self.0
    }

    /// Access the raw handle for terminal output.
    #[inline]
    pub fn output(&self) -> RawHandle {
        self.0
    }
}

// ------------------------------------------------------------------------------------------------

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

// ------------------------------------------------------------------------------------------------

/// The actual terminal attributes.
///
/// By wrapping the underlying libc type, this struct enables a meaningful
/// debug representation.
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
    #[inline]
    fn as_ref(&self) -> &libc::termios {
        &self.inner
    }
}

impl AsMut<libc::termios> for Termios {
    #[inline]
    fn as_mut(&mut self) -> &mut libc::termios {
        &mut self.inner
    }
}

impl std::fmt::Debug for Termios {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Determine enabled flags
        let mut flags = Vec::new();
        let mut append = |s| {
            flags.push(s);
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

        struct Flags(Vec<&'static str>);

        impl std::fmt::Debug for Flags {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_list().entries(self.0.iter()).finish()
            }
        }

        f.debug_struct("Termios")
            .field("flags", &Flags(flags))
            .field("vmin", &self.inner.c_cc[libc::VMIN])
            .field("vtime", &self.inner.c_cc[libc::VTIME])
            .finish()
    }
}

// ------------------------------------------------------------------------------------------------

/// A terminal configuration.
///
/// # Safety
///
/// The owner of a terminal configuration must ensure that the instance does not
/// outlive its file descriptor. As long as that invariant is preserved, this
/// struct ensures that calls to the underlying Posix API are safe and that a
/// configuration update is based on a previous configuration for the same
/// terminal.
#[derive(Debug)]
pub(crate) struct Config {
    handle: RawHandle,
    attributes: Termios,
}

impl Config {
    /// Configure the terminal with the given options.
    ///
    /// This method reads the current terminal configuration, updates a copy of
    /// the configuration, writes the updated copy, and returns the original.
    pub fn new(handle: DeviceHandle, options: &Options) -> Result<Self> {
        let handle = handle.input();
        let attributes = Self::read(handle)?;

        Self::write(
            handle,
            When::AfterFlush,
            &Self::update(&attributes, options),
        )?;

        Ok(Self { handle, attributes })
    }

    /// Reconfigure the terminal to use the given options.
    ///
    /// This method applies the options to a copy of the terminal's original
    /// configuration and then writes the updated copy to the terminal.
    #[allow(dead_code)]
    pub fn reconfigure(&mut self, options: &Options) -> Result<()> {
        Self::write(
            self.handle,
            When::AfterFlush,
            &Self::update(&self.attributes, options),
        )
    }

    /// Restore the original terminal configuration.
    pub fn restore(&self) -> Result<()> {
        Self::write(self.handle, When::Now, &self.attributes)
    }

    // ---------------------------------------------------------------------------------

    /// Read the configuration for the terminal with the given file descriptor.
    fn read(handle: RawHandle) -> Result<Termios> {
        let mut attributes = std::mem::MaybeUninit::uninit();
        unsafe { libc::tcgetattr(handle, attributes.as_mut_ptr()) }.into_result()?;
        Ok(Termios::new(unsafe { attributes.assume_init() }))
    }

    /// Create an updated configuration with the given mode and timeout.
    fn update(attributes: &Termios, options: &Options) -> Termios {
        let mut wrapper = attributes.clone();
        let inner = wrapper.as_mut();

        match options.mode() {
            Mode::Rare => {
                inner.c_lflag &= !(libc::ECHO | libc::ICANON);
            }
            Mode::Raw => {
                unsafe { libc::cfmakeraw(from_mut(inner)) };
            }
        }

        inner.c_cc[libc::VMIN] = 0;
        inner.c_cc[libc::VTIME] = options.timeout().get();
        wrapper
    }

    /// Write this configuration to the terminal with the file descriptor.
    fn write(handle: RawHandle, when: When, attributes: &Termios) -> Result<()> {
        unsafe { libc::tcsetattr(handle, when as i32, from_ref(attributes.as_ref())) }
            .into_result()?;
        Ok(())
    }
}

// ------------------------------------------------------------------------------------------------

/// A terminal reader.
///
/// # Safety
///
/// The owner of a terminal reader must ensure that the instance does not
/// outlive its file descriptor.
#[derive(Debug)]
pub(crate) struct Reader {
    handle: RawHandle,
}

impl Reader {
    /// Create a new reader with a raw file descriptor.
    ///
    /// The second parameter is only used on Windows. Adding it for Unix too
    /// simplifies the code instantiating the reader.
    pub fn new(handle: RawHandle, _: u32) -> Self {
        Self { handle }
    }
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            libc::read(
                self.handle,
                buf.as_mut_ptr() as *mut c_void,
                buf.len() as libc::size_t,
            )
        }
        .into_result()
    }
}

// ------------------------------------------------------------------------------------------------

/// A terminal writer.
///
/// # Safety
///
/// The owner of a terminal reader must ensure that the instance does not
/// outlive its file descriptor.
#[derive(Debug)]
pub(crate) struct Writer {
    handle: RawHandle,
}

impl Writer {
    /// Create a new writer with a raw file descriptor.
    pub fn new(handle: RawHandle) -> Self {
        Self { handle }
    }
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        unsafe {
            libc::write(
                self.handle,
                buf.as_ptr() as *const c_void,
                buf.len() as libc::size_t,
            )
        }
        .into_result()
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
