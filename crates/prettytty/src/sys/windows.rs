use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::{Error, ErrorKind, Read, Result, Write};
use std::os::windows::io::{AsRawHandle, OwnedHandle};
use std::ptr::{from_mut, null};

use windows_sys::Win32::Foundation;
use windows_sys::Win32::Globalization;
use windows_sys::Win32::System::Console::{self, CONSOLE_MODE as ConsoleMode};
use windows_sys::Win32::System::Threading;

use super::RawHandle;
use super::util::{IdentList, IntoResult};
use crate::opt::{Mode, Options};

// ----------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct RawConnection {
    timeout: u32,
    input: OwnedHandle,
    output: OwnedHandle,
}

impl RawConnection {
    /// Open a new owned connection to the terminal device.
    pub fn open(options: &Options) -> Result<Self> {
        let timeout = 100 * (options.timeout() as u32);
        let input = OpenOptions::new()
            .read(true)
            .write(true)
            .open("CONIN$")?
            .into();
        let output = OpenOptions::new()
            .read(true)
            .write(true)
            .open("CONOUT$")?
            .into();

        Ok(Self {
            timeout,
            input,
            output,
        })
    }

    /// Get the process group ID.
    #[inline]
    pub fn group(&self) -> Result<u32> {
        Err(ErrorKind::Unsupported.into())
    }

    /// Get a handle for the terminal's input.
    #[inline]
    pub fn input(&self) -> RawInput {
        RawInput::new(self.input.as_raw_handle(), self.timeout)
    }

    /// Get a handle for the terminal's output.
    #[inline]
    pub fn output(&self) -> RawOutput {
        RawOutput::new(self.output.as_raw_handle())
    }
}

// ----------------------------------------------------------------------------------------------------------

/// A terminal configuration.
pub(crate) struct Config {
    input_mode: ConsoleMode,
    input_encoding: u32,
    output_mode: ConsoleMode,
    output_encoding: u32,
}

impl Config {
    pub fn read(input: RawInput) -> Result<Self> {
        let input_mode = Self::read_mode(&input)?;
        let input_encoding = unsafe { Console::GetConsoleCP() }.into_result()?;
        let output_mode = Self::read_mode(&input)?;
        let output_encoding = unsafe { Console::GetConsoleOutputCP() }.into_result()?;

        Ok(Self {
            input_mode,
            input_encoding,
            output_mode,
            output_encoding,
        })
    }

    fn read_mode(input: &RawInput) -> Result<ConsoleMode> {
        let mut mode = 0;
        unsafe { Console::GetConsoleMode(input.handle(), from_mut(&mut mode)) }.into_result()?;
        Ok(mode)
    }

    /// Apply the terminal mode to this configuration.
    ///
    /// For Unix, charred and cooked mode are the same; they make no changes.
    /// For Windows, charred mode makes no changes, but cooked mode switches
    /// to the UTF-8 encoding, `ENABLE_VIRTUAL_TERMINAL_INPUT`,
    /// `ENABLE_PROCESSED_OUTPUT`, amd `ENABLE_VIRTUAL_TERMINAL_PROCESSING`.
    /// These options ensure that the terminal actually processed ANSI
    /// escape sequences.
    pub fn apply(&self, options: &Options) -> Option<Self> {
        // Charred mode means "do not touch"
        if options.mode() == Mode::Charred {
            return None;
        }

        let mut input_mode = self.input_mode & Console::ENABLE_VIRTUAL_TERMINAL_INPUT;
        if options.mode() != Mode::Cooked {
            input_mode = input_mode & !Console::ENABLE_ECHO_INPUT & !Console::ENABLE_LINE_INPUT;
        }
        if options.mode() == Mode::Raw {
            input_mode = input_mode & !Console::ENABLE_PROCESSED_INPUT;
        }
        let input_encoding = Globalization::CP_UTF8;

        let output_mode = self.output_mode
            & Console::ENABLE_PROCESSED_OUTPUT
            & Console::ENABLE_VIRTUAL_TERMINAL_PROCESSING;
        let output_encoding = Globalization::CP_UTF8;

        Some(Self {
            input_mode,
            input_encoding,
            output_mode,
            output_encoding,
        })
    }

    pub fn write(&self, output: RawOutput) -> Result<()> {
        let result1 = Self::write_mode(&output, self.input_mode);
        let result2 = unsafe { Console::SetConsoleCP(self.input_encoding) }.into_result();
        let result3 = Self::write_mode(&output, self.output_mode);
        let result4 = unsafe { Console::SetConsoleOutputCP(self.output_encoding) }.into_result();

        result1.and(result2).and(result3).and(result4)?;
        Ok(())
    }

    fn write_mode(output: &RawOutput, mode: ConsoleMode) -> Result<()> {
        unsafe { Console::SetConsoleMode(output.handle(), mode) }.into_result()?;
        Ok(())
    }
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut input_labels = Vec::new();
        for (label, mask) in [
            ("ENABLE_ECHO_INPUT", Console::ENABLE_ECHO_INPUT),
            ("ENABLE_INSERT_MODE", Console::ENABLE_INSERT_MODE),
            ("ENABLE_LINE_INPUT", Console::ENABLE_LINE_INPUT),
            ("ENABLE_MOUSE_INPUT", Console::ENABLE_MOUSE_INPUT),
            ("ENABLE_PROCESSED_INPUT", Console::ENABLE_PROCESSED_INPUT),
            ("ENABLE_QUICK_EDIT_MODE", Console::ENABLE_QUICK_EDIT_MODE),
            ("ENABLE_WINDOW_INPUT", Console::ENABLE_WINDOW_INPUT),
            ("ENABLE_VIRTUAL_TERMINAL_INPUT", Console::ENABLE_VIRTUAL_TERMINAL_INPUT),
        ] {
            if self.input_mode & mask != 0 {
                input_labels.push(label);
            }
        }

        let mut output_labels = Vec::new();
        for (label, mask) in [
            ("ENABLE_PROCESSED_OUTPUT", Console::ENABLE_PROCESSED_OUTPUT),
            ("ENABLE_WRAP_AT_EOL_OUTPUT", Console::ENABLE_WRAP_AT_EOL_OUTPUT),
            ("ENABLE_VIRTUAL_TERMINAL_PROCESSING", Console::ENABLE_VIRTUAL_TERMINAL_PROCESSING),
            ("DISABLE_NEWLINE_AUTO_RETURN", Console::DISABLE_NEWLINE_AUTO_RETURN),
            ("ENABLE_LVB_GRID_WORLDWIDE", Console::ENABLE_LVB_GRID_WORLDWIDE),
        ] {
            if self.output_mode & mask != 0 {
                output_labels.push(label);
            }
        }

        f.debug_struct("Config")
            .field("input_mode", &IdentList::new(input_labels))
            .field("input_encoding", &self.input_encoding)
            .field("output_mode", &IdentList::new(output_labels))
            .field("output_encoding", &self.output_encoding)
            .finish()
    }
}

// ----------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct RawInput {
    timeout: u32,
    handle: RawHandle,
}

impl RawInput {
    #[inline]
    fn new(handle: RawHandle, timeout: u32) -> Self {
        Self { handle, timeout }
    }

    #[inline]
    fn handle(&self) -> RawHandle {
        self.handle
    }
}

// SAFETY: Windows HANDLE is defined as a *mut c_void but most instances are
// thread-safe. In fact, Rust's standard library [implements `Send` and
// `Sync`](https://github.com/rust-lang/rust/blob/8e37e151835d96d6a7415e93e6876561485a3354/library/std/src/os/windows/io/handle.rs#L111),
// for wrapped handles, too. Also, access to raw input is gated by a mutex.
unsafe impl Send for RawInput {}

impl Read for RawInput {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let status = unsafe { Threading::WaitForSingleObject(self.handle, self.timeout) };
        if status == Foundation::WAIT_OBJECT_0 {
            let mut did_read: u32 = 0;
            unsafe {
                Console::ReadConsoleA(
                    self.handle,
                    buf.as_mut_ptr() as *mut c_void,
                    buf.len() as u32,
                    from_mut(&mut did_read),
                    null(),
                )
            }
            .into_result()?;
            Ok(did_read as usize)
        } else if status == Foundation::WAIT_TIMEOUT {
            Ok(0)
        } else if status == Foundation::WAIT_FAILED {
            Err(Error::last_os_error())
        } else {
            Err(ErrorKind::Other.into())
        }
    }
}

// ------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct RawOutput {
    //#[allow(dead_code)]
    handle: RawHandle,
}

impl RawOutput {
    /// Create a new writer with a raw file descriptor.
    #[inline]
    pub fn new(handle: RawHandle) -> Self {
        Self { handle }
    }

    #[inline]
    pub fn handle(&self) -> RawHandle {
        self.handle
    }
}

// SAFETY: Windows HANDLE is defined as a *mut c_void but most instances are
// thread-safe. In fact, Rust's standard library [implements `Send` and
// `Sync`](https://github.com/rust-lang/rust/blob/8e37e151835d96d6a7415e93e6876561485a3354/library/std/src/os/windows/io/handle.rs#L111),
// for wrapped handles, too. Also, access to raw input is gated by a mutex.
unsafe impl Send for RawOutput {}

impl Write for RawOutput {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut did_write: u32 = 0;
        unsafe {
            Console::WriteConsoleA(
                self.handle,
                // https://learn.microsoft.com/en-us/windows/console/writeconsole
                // says this pointer is *const c_void. That would be consistent
                // with ReadConsoleA (see above) as well. Alas, windows-sys
                // insists on the pointer being *const u8.
                buf.as_ptr(),
                buf.len() as u32,
                from_mut(&mut did_write),
                null(),
            )
        }
        .into_result()?;
        Ok(did_write as usize)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
