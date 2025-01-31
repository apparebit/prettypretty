use std::io::{ErrorKind, Read};
use std::time::Instant;

use crate::util::nicely_str;

/// A reader that tolerates interruptions.
pub(crate) struct DoggedReader<R> {
    inner: R,
}

impl<R> DoggedReader<R> {
    /// Create a new dogged reader.
    pub fn new(inner: R) -> Self {
        Self { inner }
    }
}

impl<R: Read> Read for DoggedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            match self.inner.read(buf) {
                Ok(n) => return Ok(n),
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
    }
}

/// A reader that prints helpful information for debugging.
#[derive(Debug)]
pub(crate) struct VerboseReader<R> {
    inner: R,
    timeout: f32,
}

impl<R> VerboseReader<R> {
    /// Create a new reader.
    pub fn new(inner: R, timeout: u8) -> Self {
        Self {
            inner,
            timeout: timeout as f32 * 0.1,
        }
    }
}

impl<R: Read> Read for VerboseReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let start_time = Instant::now();
        let mut retries = 0;
        let mut interrupts = 0;

        loop {
            match self.inner.read(buf) {
                Ok(0) => {
                    let duration = start_time.elapsed().as_secs_f32();
                    if duration < self.timeout {
                        retries += 1;
                        continue;
                    }

                    print!(
                        "read:  0 bytes, {:.1}s timeout, {} retries, {} interrupts\r\n",
                        duration, retries, interrupts
                    );
                    return Ok(0);
                }
                Ok(n) => {
                    print!(
                        "read: {:2} bytes, {} retries, {} interrupts, {}\r\n",
                        n,
                        retries,
                        interrupts,
                        nicely_str(&buf[..n])
                    );
                    return Ok(n);
                }
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {
                    interrupts += 1;
                    continue;
                }
                Err(e) => {
                    print!(
                        "read: {:?}, {} retries, {} interrupts\r\n",
                        &e, retries, interrupts
                    );
                    return Err(e);
                }
            }
        }
    }
}
