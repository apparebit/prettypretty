#[cfg(target_family = "unix")]
pub(crate) type RawHandle = std::os::fd::RawFd;
#[cfg(target_family = "windows")]
pub(crate) type RawHandle = std::os::windows::io::RawHandle;

#[cfg(target_family = "unix")]
mod unix;
#[cfg(target_family = "windows")]
mod windows;

#[cfg(target_family = "unix")]
pub(crate) use unix::{Config, Device, Reader, Writer};
#[cfg(target_family = "windows")]
pub(crate) use windows::{Config, Device, Reader, Writer};
