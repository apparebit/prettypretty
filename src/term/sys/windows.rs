//! Module to isolate unsafe Windows operations.
//!
//! This module abstracts over the underlying Windows API invocations for
//! managing the terminal configuration, reading from the terminal, and writing
//! to the terminal. They are safe, as long as the file descriptors are valid.
//! For that same reason, [`Config`], [`Reader`], and [`Writer`] must not be
//! directly exposed to application code.

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
use crate::term::{Mode, Options};

// ----------------------------------------------------------------------------------------------------------

/// The connections to the terminal device.
///
/// [`Device::new`] opens connections to the console input buffer and console
/// screen buffer and closes them again when dropped. Since
/// [`Device::read_handle`] and [`Device::write_handle`] return raw handles, it
/// is the caller's responsibility to ensure that the raw handle is not used
/// past the connectiona' lifetimes.
#[derive(Debug)]
pub(crate) struct Device {
    input: OwnedHandle,
    output: OwnedHandle,
}

impl Device {
    /// Open a new owned connection to the terminal device.
    pub fn new() -> Result<Self> {
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

        Ok(Self { input, output })
    }

    /// Get the process group ID.
    #[inline]
    pub fn pid(&self) -> Result<u32> {
        Err(ErrorKind::Unsupported.into())
    }

    /// Get a handle for the device.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the returned device handle does not outlive
    /// this device.
    #[inline]
    pub fn handle(&self) -> DeviceHandle {
        DeviceHandle {
            input: self.input.as_raw_handle(),
            output: self.output.as_raw_handle(),
        }
    }
}

/// A handle for a terminal device.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DeviceHandle {
    input: RawHandle,
    output: RawHandle,
}

impl DeviceHandle {
    /// Access the raw handle for terminal input.
    #[inline]
    pub fn input(&self) -> RawHandle {
        self.input
    }

    /// Access the raw handle for terminal output.
    #[inline]
    pub fn output(&self) -> RawHandle {
        self.output
    }
}

// ----------------------------------------------------------------------------------------------------------

/// A terminal configuration.
#[derive(Debug)]
pub(crate) struct Config {
    handle: DeviceHandle,
    input_mode: ConsoleMode,
    input_encoding: u32,
    output_mode: ConsoleMode,
    output_encoding: u32,
}

impl Config {
    /// Create a new terminal configuration.
    pub fn new(handle: DeviceHandle, options: &Options) -> Result<Self> {
        // Early exit is safe because we are only reading.
        let input_mode = Self::read_mode(handle.input())?;
        let input_encoding = unsafe { Console::GetConsoleCP() }.into_result()?;
        let output_mode = Self::read_mode(handle.output())?;
        let output_encoding = unsafe { Console::GetConsoleOutputCP() }.into_result()?;

        let this = Self {
            handle,
            input_mode,
            input_encoding,
            output_mode,
            output_encoding,
        };

        // Determine new input and output modes.
        let mut new_input_mode = input_mode
            & !Console::ENABLE_ECHO_INPUT
            & !Console::ENABLE_LINE_INPUT
            & Console::ENABLE_VIRTUAL_TERMINAL_INPUT;

        if options.mode() == Mode::Raw {
            new_input_mode &= !Console::ENABLE_PROCESSED_INPUT;
        }

        let new_output_mode = output_mode
            & Console::ENABLE_PROCESSED_OUTPUT
            & Console::ENABLE_VIRTUAL_TERMINAL_PROCESSING;

        // If the update fails, try to restore old configuration.
        this.update(new_input_mode, new_output_mode).or_else(|e| {
            this.restore();
            Err(e)
        })?;

        Ok(this)
    }

    fn update(&self, input_mode: ConsoleMode, output_mode: ConsoleMode) -> Result<()> {
        // Fail early to limit damage.
        Self::write_mode(self.handle.input(), input_mode)?;
        unsafe { Console::SetConsoleCP(Globalization::CP_UTF8) }.into_result()?;
        Self::write_mode(self.handle.output(), output_mode)?;
        unsafe { Console::SetConsoleOutputCP(Globalization::CP_UTF8) }.into_result()?;
        Ok(())
    }

    /// Restore the original terminal configuration.
    pub fn restore(&self) -> Result<()> {
        // Since we are trying to restore the original terminal modes, we should
        // always try to apply all four updates, even if one of them fails.
        let result1 = Self::write_mode(self.handle.input(), self.input_mode);
        let result2 = unsafe { Console::SetConsoleCP(self.input_encoding) }.into_result();
        let result3 = Self::write_mode(self.handle.output(), self.output_mode);
        let result4 = unsafe { Console::SetConsoleOutputCP(self.output_encoding) }.into_result();

        result1.and(result2).and(result3).and(result4)?;
        Ok(())
    }

    // ------------------------------------------------------------------------------------------------------

    fn read_mode(handle: RawHandle) -> Result<ConsoleMode> {
        let mut mode = 0;
        unsafe { Console::GetConsoleMode(handle, from_mut(&mut mode)) }.into_result()?;
        Ok(mode)
    }

    fn write_mode(handle: RawHandle, mode: ConsoleMode) -> Result<()> {
        unsafe { Console::SetConsoleMode(handle, mode) }.into_result()?;
        Ok(())
    }
}

unsafe impl Send for Config {}

// ----------------------------------------------------------------------------------------------------------

/// A terminal reader.
///
/// # Safety
///
/// The owner of a terminal reader must ensure that the instance does not
/// outlive its handle.
#[derive(Debug)]
pub(crate) struct Reader {
    handle: RawHandle,
    timeout: u32,
}

impl Reader {
    /// Create a new reader with a raw handle.
    pub fn new(handle: RawHandle, timeout: u32) -> Self {
        Self { handle, timeout }
    }
}

impl Read for Reader {
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

/// A terminal writer.
///
/// # Safety
///
/// The owner of a terminal reader must ensure that the instance does not
/// outlive its handle.
#[derive(Debug)]
pub(crate) struct Writer {
    #[allow(dead_code)]
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
        let mut did_write: u32 = 0;
        unsafe {
            Console::WriteConsoleA(
                self.handle,
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

// ------------------------------------------------------------------------------------------------
