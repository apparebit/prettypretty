//! Module to isolate unsafe Windows operations.
//!
//! This module abstracts over the underlying Windows API invocations for
//! managing the terminal configuration, reading from the terminal, and writing
//! to the terminal. They are safe, as long as the file descriptors are valid.
//! For that same reason, [`Config`], [`Reader`], and [`Writer`] must not be
//! directly exposed to application code.

use std::fs::OpenOptions;
use std::io::{Error, ErrorKind, Read, Result, Write};
use std::os::windows::io::{AsRawHandle, OwnedHandle};
use std::ptr::from_mut;

use windows_sys::Win32::System::Console::{
    GetConsoleMode, SetConsoleMode, ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT,
    ENABLE_PROCESSED_OUTPUT, ENABLE_VIRTUAL_TERMINAL_INPUT, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
};

use super::RawHandle;
use crate::term::{Mode, Options};

/// A trait for converting Windows status BOOL to Rust std::io results.
trait IntoResult {
    /// Convert the return type into an error.
    fn into_result(self) -> Result<()>;
}

impl IntoResult for i32 {
    fn into_result(self) -> Result<()> {
        if self != 0 {
            Ok(())
        } else {
            Err(Error::last_os_error())
        }
    }
}

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

    /// Get a raw handle for reading from the connection.
    pub fn input(&self) -> RawHandle {
        self.input.as_raw_handle()
    }

    /// Get a raw handle for writing to the connection.
    pub fn output(&self) -> RawHandle {
        self.output.as_raw_handle()
    }
}

// ----------------------------------------------------------------------------------------------------------

/// A terminal configuration.
#[derive(Debug)]
pub(crate) struct Config {
    input_handle: RawHandle,
    input_mode: u32,
    output_handle: RawHandle,
    output_mode: u32,
}

impl Config {
    /// Create a new terminal configuration.
    pub fn new(
        input_handle: RawHandle,
        output_handle: RawHandle,
        options: &Options,
    ) -> Result<Self> {
        // It's safe to exit early because for now we are just reading modes.
        let input_mode = Self::read(input_handle)?;
        let output_mode = Self::read(output_handle)?;

        let mut new_input_mode = input_mode
            & !ENABLE_ECHO_INPUT
            & !ENABLE_LINE_INPUT
            & ENABLE_VIRTUAL_TERMINAL_INPUT;

        if options.mode == Mode::Raw {
            new_input_mode &= !ENABLE_PROCESSED_INPUT;
        }

        let new_output_mode =
            output_mode & ENABLE_PROCESSED_OUTPUT & ENABLE_VIRTUAL_TERMINAL_PROCESSING;

        // If first update fails, nothing was changed. If second update fails,
        // we probably should reset first update.
        Self::write(input_handle, new_input_mode)?;
        Self::write(output_handle, new_output_mode)?;

        Ok(Self {
            input_handle,
            input_mode,
            output_handle,
            output_mode,
        })
    }

    /// Restore the original terminal configuration.
    pub fn restore(&self) -> Result<()> {
        // Since we are trying to restore the original terminal modes, we should
        // always try to apply both updates, even if one of them fails.
        let result1 = Self::write(self.input_handle, self.input_mode);
        let result2 = Self::write(self.output_handle, self.output_mode);

        result1.and(result2)
    }

    // ------------------------------------------------------------------------------------------------------

    fn read(handle: RawHandle) -> Result<u32> {
        let mut mode = 0;
        unsafe { GetConsoleMode(handle, from_mut(&mut mode)) }.into_result()?;
        Ok(mode)
    }

    fn write(handle: RawHandle, mode: u32) -> Result<()> {
        unsafe { SetConsoleMode(handle, mode) }.into_result()
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
    #[allow(dead_code)]
    handle: RawHandle,
}

impl Reader {
    /// Create a new reader with a raw handle.
    pub fn new(handle: RawHandle) -> Self {
        Self { handle }
    }
}

impl Read for Reader {
    fn read(&mut self, _: &mut [u8]) -> Result<usize> {
        Err(ErrorKind::Unsupported.into()) // FIXME!
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
    fn write(&mut self, _: &[u8]) -> Result<usize> {
        Err(ErrorKind::Unsupported.into()) // FIXME!
    }

    fn flush(&mut self) -> Result<()> {
        Err(ErrorKind::Unsupported.into()) // FIXME!
    }
}
