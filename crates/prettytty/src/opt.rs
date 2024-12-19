//! Helper module with the options for connecting to terminals.
//!
//! This module provides the options for a terminal connection and the
//! corresponding builder.
//!
//!
//! # Example
//!
//! ```
//! # use prettytty::opt::Options;
//! let options = Options::builder()
//!     .timeout(50)
//!     .build();
//!
//! assert_eq!(options.timeout(), 50);
//! ```

/// A terminal mode.
///
/// Terminals usually operate in canonical line-editing mode. As a result, input
/// usually only becomes available for reading after the user pressed the return
/// key. Rare or cbreak mode disables line-editing but leaves other features
/// such as control-c processing active. Raw mode also disables these additional
/// features, giving the application full control while also requiring that the
/// application implement everything itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// Rare or cbreak mode.
    #[default]
    Rare,
    /// Raw mode.
    Raw,
}

#[derive(Clone, Debug)]
struct OptionData {
    verbose: bool,
    mode: Mode,
    timeout: u8,
    pathological_size: usize,
    read_buffer_size: usize,
    write_buffer_size: usize,
}

impl OptionData {
    pub const fn new() -> Self {
        Self {
            verbose: false,
            mode: Mode::Rare,
            timeout: 1,
            pathological_size: 512,
            read_buffer_size: 256,
            write_buffer_size: 1_024,
        }
    }
}

/// A builder of options objects.
#[derive(Debug)]
pub struct OptionBuilder(OptionData);

impl OptionBuilder {
    /// Set verbose mode.
    pub fn verbose(&mut self, verbose: bool) -> &mut Self {
        self.0.verbose = verbose;
        self
    }

    /// Set rare or raw mode.
    pub fn mode(&mut self, mode: Mode) -> &mut Self {
        self.0.mode = mode;
        self
    }

    /// Set the timeout in 0.1s increments.
    pub fn timeout(&mut self, timeout: u8) -> &mut Self {
        self.0.timeout = timeout;
        self
    }

    /// Set the minimum length for pathological ANSI escape sequences.
    pub fn pathological_size(&mut self, size: usize) -> &mut Self {
        self.0.pathological_size = size;
        self
    }

    /// Set the read buffer size.
    ///
    /// This method also updates the pathological size to twice the given size.
    ///
    /// At a minimum, the this number should be large enough to hold possible
    /// responses to queries. When querying colors, that length is 27 bytes. For
    /// example, a response for the color of the 16th ANSI color *bright white*
    /// starts with `‹OSC›4;15;rgb:` and is followed by three hexadecimal
    /// numbers that usually are four digits wide, e.g., `ffff/ffff/ffff`, and
    /// then `‹ST›`. Both OSC and ST require at most two bytes, resulting in a
    /// sequence that is at most 27 bytes long.
    pub fn read_buffer_size(&mut self, size: usize) -> &mut Self {
        self.0.read_buffer_size = size;
        self.0.pathological_size = size.saturating_add(size);
        self
    }

    /// Set the write buffer size.
    pub fn write_buffer_size(&mut self, size: usize) -> &mut Self {
        self.0.write_buffer_size = size;
        self
    }

    /// Instantiate the options.
    pub fn build(&self) -> Options {
        Options(self.0.clone())
    }
}

/// An options object.
#[derive(Debug)]
pub struct Options(OptionData);

impl Default for Options {
    fn default() -> Self {
        Options(OptionData::new())
    }
}

impl Options {
    /// Create a new builder with the default option values.
    pub fn builder() -> OptionBuilder {
        OptionBuilder(OptionData::new())
    }

    /// Instantiate the default options but with verbose output enabled.
    pub fn verbose_default() -> Options {
        Self::builder().verbose(true).build()
    }

    /// Get the verbose flag.
    pub fn verbose(&self) -> bool {
        self.0.verbose
    }

    /// Get the terminal mode.
    pub fn mode(&self) -> Mode {
        self.0.mode
    }

    /// Get the timeout in 0.1s increments for blocking read operations.
    pub fn timeout(&self) -> u8 {
        self.0.timeout
    }

    /// Get the pathological size.
    pub fn pathological_size(&self) -> usize {
        self.0.pathological_size
    }

    /// Get the size of the read buffer.
    pub fn read_buffer_size(&self) -> usize {
        self.0.read_buffer_size
    }

    /// Get the size of the write buffer.
    pub fn write_buffer_size(&self) -> usize {
        self.0.write_buffer_size
    }
}
