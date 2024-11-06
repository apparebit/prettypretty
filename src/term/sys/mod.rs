#[cfg(target_family = "unix")]
pub(crate) type RawHandle = std::os::fd::RawFd;

#[cfg(target_family = "unix")]
mod unix;

#[cfg(target_family = "unix")]
pub(crate) use unix::{Config, Device, Reader, Writer};
