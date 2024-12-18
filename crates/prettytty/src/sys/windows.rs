use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::{Error, ErrorKind, Read, Result, Write};
use std::os::windows::io::{AsRawHandle, OwnedHandle};
use std::ptr::{from_mut, null};

use windows_sys::Win32::Foundation;
use windows_sys::Win32::Globalization;
use windows_sys::Win32::System::Console::{self, CONSOLE_MODE as ConsoleMode};
use windows_sys::Win32::System::Threading;

use super::{into_result::IntoResult, RawHandle};
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
        RawOutput::new(self.input.as_raw_handle())
    }
}

// ----------------------------------------------------------------------------------------------------------

/// A terminal configuration.
#[derive(Debug)]
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

    pub fn apply(&self, options: &Options) -> Self {
        // Determine new input and output modes.
        let mut input_mode = self.input_mode
            & !Console::ENABLE_ECHO_INPUT
            & !Console::ENABLE_LINE_INPUT
            & Console::ENABLE_VIRTUAL_TERMINAL_INPUT;
        if options.mode() == Mode::Raw {
            input_mode &= !Console::ENABLE_PROCESSED_INPUT;
        }
        let input_encoding = Globalization::CP_UTF8;
        let output_mode = self.output_mode
            & Console::ENABLE_PROCESSED_OUTPUT
            & Console::ENABLE_VIRTUAL_TERMINAL_PROCESSING;
        let output_encoding = Globalization::CP_UTF8;

        Self {
            input_mode,
            input_encoding,
            output_mode,
            output_encoding,
        }
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
