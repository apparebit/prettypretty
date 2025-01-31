// Platform-dependent types are prefixed with `Raw`.

#[cfg(target_family = "unix")]
pub(crate) type RawHandle = std::os::fd::RawFd;
#[cfg(target_family = "windows")]
pub(crate) type RawHandle = std::os::windows::io::RawHandle;

mod util;

#[cfg(target_family = "unix")]
mod unix;
#[cfg(target_family = "windows")]
mod windows;

#[cfg(target_family = "unix")]
pub(crate) use self::unix::{RawConfig, RawConnection, RawOutput};
#[cfg(target_family = "windows")]
pub(crate) use windows::{RawConfig, RawConnection, RawOutput};
