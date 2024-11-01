#[cfg(target_family = "unix")]
mod unix;

#[cfg(target_family = "unix")]
pub use unix::TerminalMode;
#[cfg(target_family = "unix")]
pub(crate) use unix::{TerminalConfig, TerminalReader, TerminalWriter};
