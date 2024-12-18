use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::{stderr, stdin, stdout, IsTerminal, Read, Result, Write};
use std::os::fd::{AsRawFd, OwnedFd};
use std::ptr::{from_mut, from_ref};

use super::{into_result::IntoResult, RawHandle};
use crate::opt::{Mode, Options};

// ----------------------------------------------------------------------------------------------------------

#[derive(Debug)]
enum RawConnectionHandle {
    Owned(OwnedFd),
    #[allow(dead_code)]
    StdIo(RawHandle, RawHandle),
}

impl RawConnectionHandle {
    fn input(&self) -> RawHandle {
        match self {
            Self::Owned(handle) => handle.as_raw_fd(),
            Self::StdIo(handle, _) => *handle,
        }
    }

    fn output(&self) -> RawHandle {
        match self {
            Self::Owned(handle) => handle.as_raw_fd(),
            Self::StdIo(_, handle) => *handle,
        }
    }
}

/// A connection to a terminal device.
#[derive(Debug)]
pub(crate) struct RawConnection {
    handle: RawConnectionHandle,
}

impl RawConnection {
    /// Open a new terminal connection.
    pub fn open(_: &Options) -> Result<Self> {
        let fd = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")?
            .into();

        Ok(Self {
            handle: RawConnectionHandle::Owned(fd),
        })
    }

    /// Simulate a terminal connection with standard I/O.
    ///
    /// This method returns a connection as long as standard input and either
    /// standard output or standard error have not been redirected and are
    /// connected to a terminal.
    ///
    /// Such a simulated connection is *not* equivalent to an actual terminal
    /// connection because any I/O through Rust's standard library can interfere
    /// with the connection's operation. That applies even to I/O that happened
    /// before the connection was created, since Rust's standard library
    /// performs its own buffering of standard I/O. In other words, a simulated
    /// connection is only safe to use as long as the standard library
    /// facilities are only used after the last connection has been dropped.
    #[allow(dead_code)]
    pub fn with_stdio() -> Option<Self> {
        if stdin().is_terminal() {
            let output = if stdout().is_terminal() {
                stdout().as_raw_fd()
            } else if stderr().is_terminal() {
                stderr().as_raw_fd()
            } else {
                return None;
            };

            Some(Self {
                handle: RawConnectionHandle::StdIo(stdin().as_raw_fd(), output),
            })
        } else {
            None
        }
    }

    /// Get process group ID.
    #[inline]
    pub fn group(&self) -> Result<u32> {
        unsafe { libc::tcgetsid(self.handle.input()) }.into_result()
    }

    /// Get a handle for reading from the connection.
    #[inline]
    pub fn input(&self) -> RawInput {
        RawInput::new(self.handle.input())
    }

    /// Get a handle for writing to the connection.
    pub fn output(&self) -> RawOutput {
        RawOutput::new(self.handle.output())
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
