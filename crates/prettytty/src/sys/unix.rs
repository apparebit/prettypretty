use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::{Read, Result, Write};
use std::os::fd::{AsRawFd, OwnedFd};
use std::ptr::{from_mut, from_ref};

use super::{into_result::IntoResult, RawHandle};
use crate::opt::{Mode, Options};

// ----------------------------------------------------------------------------------------------------------

/// A connection to a terminal device.
#[derive(Debug)]
pub(crate) struct RawConnection {
    fd: OwnedFd,
}

impl RawConnection {
    /// Open a new terminal connection.
    pub fn open(_: &Options) -> Result<Self> {
        let fd = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")?
            .into();

        Ok(Self { fd })
    }

    /// Get process group ID.
    #[inline]
    pub fn group(&self) -> Result<u32> {
        unsafe { libc::tcgetsid(self.fd.as_raw_fd()) }.into_result()
    }

    /// Get a handle for reading from the connection.
    #[inline]
    pub fn input(&self) -> RawInput {
        RawInput::new(self.fd.as_raw_fd())
    }

    /// Get a handle for writing to the connection.
    pub fn output(&self) -> RawOutput {
        RawOutput::new(self.fd.as_raw_fd())
    }
}

// ----------------------------------------------------------------------------------------------------------

/// A terminal configuration.
pub(crate) struct Config {
    state: libc::termios,
}

impl Config {
    /// Read the configuration.
    pub fn read(input: RawInput) -> Result<Self> {
        let mut state = std::mem::MaybeUninit::uninit();
        unsafe { libc::tcgetattr(input.handle(), state.as_mut_ptr()) }.into_result()?;
        Ok(Self {
            state: unsafe { state.assume_init() },
        })
    }

    /// Apply the options to create a new configuration.
    pub fn apply(&self, options: &Options) -> Self {
        let mut state = self.state;

        match options.mode() {
            Mode::Rare => {
                state.c_lflag &= !(libc::ECHO | libc::ICANON);
            }
            Mode::Raw => {
                unsafe { libc::cfmakeraw(from_mut(&mut state)) };
            }
        }

        state.c_cc[libc::VMIN] = 0;
        state.c_cc[libc::VTIME] = options.timeout();
        Self { state }
    }

    /// Write the configuration.
    pub fn write(&self, output: RawOutput) -> Result<()> {
        unsafe { libc::tcsetattr(output.handle(), libc::TCSAFLUSH, from_ref(&self.state)) }
            .into_result()?;
        Ok(())
    }
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Determine enabled flags
        let mut flags = Vec::new();

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
            if self.state.c_iflag & value != 0 {
                flags.push(name);
            }
        }

        for (name, value) in [
            ("OPOST", libc::OPOST),
            ("OCRNL", libc::OCRNL),
            ("ONOCR", libc::ONOCR),
            ("ONLRET", libc::ONLRET),
        ] {
            if self.state.c_oflag & value != 0 {
                flags.push(name);
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
            if self.state.c_lflag & value != 0 {
                flags.push(name);
            }
        }

        struct Flags<'a>(Vec<&'a str>);

        impl std::fmt::Debug for Flags<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_list().entries(self.0.iter()).finish()
            }
        }

        f.debug_struct("Termios")
            .field("flags", &Flags(flags))
            .field("vmin", &self.state.c_cc[libc::VMIN])
            .field("vtime", &self.state.c_cc[libc::VTIME])
            .finish()
    }
}

// ----------------------------------------------------------------------------------------------------------

/// Raw unbuffered terminal input.
#[derive(Debug)]
pub(crate) struct RawInput {
    handle: RawHandle,
}

impl RawInput {
    fn new(handle: RawHandle) -> Self {
        Self { handle }
    }

    #[inline]
    fn handle(&self) -> RawHandle {
        self.handle
    }
}

impl Read for RawInput {
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

// ----------------------------------------------------------------------------------------------------------

/// A terminal's output.
#[derive(Debug)]
pub(crate) struct RawOutput {
    handle: RawHandle,
}

impl RawOutput {
    fn new(handle: RawHandle) -> Self {
        Self { handle }
    }

    #[inline]
    fn handle(&self) -> RawHandle {
        self.handle
    }
}

impl Write for RawOutput {
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
