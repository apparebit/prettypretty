use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::{stderr, stdin, stdout, IsTerminal, Read, Result, Write};
use std::os::fd::{AsRawFd, OwnedFd};
use std::ptr::{from_mut, from_ref};

use super::util::{IdentList, IntoResult};
use super::RawHandle;
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

/// A grouping of configuration flags.
enum ModeGroup {
    Input,
    Output,
    Control,
    Local,
}

impl ModeGroup {
    pub fn all() -> impl std::iter::Iterator<Item = ModeGroup> {
        use self::ModeGroup::*;

        std::iter::successors(Some(Input), |n| {
            Some(match n {
                Input => Output,
                Output => Control,
                Control => Local,
                Local => return None,
            })
        })
    }

    pub fn name(&self) -> &'static str {
        use self::ModeGroup::*;

        match self {
            Input => "input_modes",
            Output => "output_modes",
            Control => "control_modes",
            Local => "local_modes",
        }
    }
}

/// A raw terminal configuration.
pub(crate) struct RawConfig {
    state: libc::termios,
}

impl RawConfig {
    /// Read the configuration.
    pub fn read(connection: &RawConnection) -> Result<Self> {
        let mut state = std::mem::MaybeUninit::uninit();
        unsafe { libc::tcgetattr(connection.input().handle(), state.as_mut_ptr()) }
            .into_result()?;
        Ok(Self {
            state: unsafe { state.assume_init() },
        })
    }

    /// Apply the options to create a new configuration.
    pub fn apply(&self, options: &Options) -> Option<Self> {
        let mut state = self.state;

        match options.mode() {
            Mode::Charred | Mode::Cooked => return None,
            Mode::Rare => {
                state.c_lflag &= !(libc::ECHO | libc::ICANON);
            }
            Mode::Raw => {
                unsafe { libc::cfmakeraw(from_mut(&mut state)) };
            }
        }

        state.c_cc[libc::VMIN] = 0;
        state.c_cc[libc::VTIME] = options.timeout();
        Some(Self { state })
    }

    /// Write the configuration.
    pub fn write(&self, connection: &RawConnection) -> Result<()> {
        unsafe {
            libc::tcsetattr(
                connection.input().handle(),
                libc::TCSAFLUSH,
                from_ref(&self.state),
            )
        }
        .into_result()?;
        Ok(())
    }

    /// Get labels for active modes in given group.
    fn labels(&self, group: &ModeGroup) -> Vec<&'static str> {
        let mut labels = Vec::new();

        macro_rules! maybe_add {
            ($field:expr, $mask:expr, $label:expr) => {
                if $field & $mask != 0 {
                    labels.push($label);
                }
            };
        }

        match group {
            ModeGroup::Input => {
                for (label, mask) in [
                    ("BRKINT", libc::BRKINT),
                    ("ICRNL", libc::ICRNL),
                    ("IGNBRK", libc::IGNBRK),
                    ("IGNCR", libc::IGNCR),
                    ("IGNPAR", libc::IGNPAR),
                    ("INLCR", libc::INLCR),
                    ("INPCK", libc::INPCK),
                    ("ISTRIP", libc::ISTRIP),
                    ("IXANY", libc::IXANY),
                    ("IXOFF", libc::IXOFF),
                    ("IXON", libc::IXON),
                    ("PARMRK", libc::PARMRK),
                ] {
                    maybe_add!(self.state.c_iflag, mask, label);
                }
            }
            ModeGroup::Output => {
                for (label, mask) in [
                    ("OPOST", libc::OPOST),
                    ("OCRNL", libc::OCRNL),
                    ("ONOCR", libc::ONOCR),
                    ("ONLRET", libc::ONLRET),
                    ("OFILL", libc::OFILL),
                    ("OFDEL", libc::OFDEL),
                    // Missing: NLDLY, CRDLY, TABDLY, BSDLY, VTDLY, FFDLY
                ] {
                    maybe_add!(self.state.c_oflag, mask, label);
                }
            }
            ModeGroup::Control => {
                maybe_add!(self.state.c_cflag, libc::CLOCAL, "CLOCAL");
                maybe_add!(self.state.c_cflag, libc::CREAD, "CREAD");
                match self.state.c_cflag & libc::CSIZE {
                    libc::CS5 => labels.push("CS5"),
                    libc::CS6 => labels.push("CS6"),
                    libc::CS7 => labels.push("CS7"),
                    libc::CS8 => labels.push("CS8"),
                    _ => (),
                }
                for (label, mask) in [
                    ("CSTOPB", libc::CSTOPB),
                    ("HUPCL", libc::HUPCL),
                    ("PARENB", libc::PARENB),
                    ("PARODD", libc::PARODD),
                ] {
                    maybe_add!(self.state.c_cflag, mask, label);
                }
            }
            ModeGroup::Local => {
                for (label, mask) in [
                    ("ECHO", libc::ECHO),
                    ("ECHOE", libc::ECHOE),
                    ("ECHOK", libc::ECHOK),
                    ("ECHONL", libc::ECHONL),
                    ("ICANON", libc::ICANON),
                    ("IEXTEN", libc::IEXTEN),
                    ("ISIG", libc::ISIG),
                    ("NOFLSH", libc::NOFLSH),
                    ("TOSTOP", libc::TOSTOP),
                ] {
                    maybe_add!(self.state.c_lflag, mask, label);
                }
            }
        }

        labels
    }
}

impl std::fmt::Debug for RawConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debugger = f.debug_struct("RawConfig");
        for group in ModeGroup::all() {
            debugger.field(group.name(), &IdentList::new(self.labels(&group)));
        }

        debugger
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
    #[inline]
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
    #[inline]
    fn new(handle: RawHandle) -> Self {
        Self { handle }
    }

    #[allow(dead_code)]
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
