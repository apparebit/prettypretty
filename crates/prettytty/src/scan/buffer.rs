use std::cmp::min;
use std::io::Read;

use crate::opt::Options;

/// A scanner's buffer.
pub(super) struct Buffer {
    // Invariant: token_start <= token_end <= cursor <= filled <= N
    data: Vec<u8>,
    token_start: usize,
    token_end: usize,
    cursor: usize,
    filled: usize,
}

impl Buffer {
    /// Create a new buffer with the given options.
    pub fn with_options(options: &Options) -> Self {
        Self {
            data: vec![0; options.read_buffer_size()],
            token_start: 0,
            token_end: 0,
            cursor: 0,
            filled: 0,
        }
    }

    /// Reset this buffer.
    pub fn reset(&mut self) {
        self.token_start = 0;
        self.token_end = 0;
        self.cursor = 0;
        self.filled = 0;
    }

    /// Synchronize the token indices with the cursor.
    pub fn start_token(&mut self) {
        self.token_start = self.cursor;
        self.token_end = self.cursor;
    }

    /// Determine whether any bytes are available for reading.
    #[inline]
    pub fn is_readable(&self) -> bool {
        self.cursor < self.filled
    }

    /// Peek at the next byte.
    ///
    /// This method returns `None` if there are no more bytes to read.
    pub fn peek(&self) -> Option<u8> {
        if self.cursor < self.filled {
            Some(self.data[self.cursor])
        } else {
            None
        }
    }

    /// Consume the next byte.
    ///
    /// # Panics
    ///
    /// If there are no more bytes to read. This method should be invoked only
    /// after an immediately preceding invocation of [`Buffer::peek`] that
    /// produced a byte.
    pub fn consume(&mut self) {
        assert!(self.cursor < self.filled);
        self.cursor += 1;
    }

    /// Retain the most recently consumed byte for the current token.
    ///
    /// # Panics
    ///
    /// If the token doesn't end strictly *before* the cursor. This method
    /// should be invoked only after immediately preceding invocations of
    /// [`Buffer::peek`] and [`Buffer::consume`].
    pub fn retain(&mut self) {
        assert!(self.token_end < self.cursor);
        if self.token_start == self.token_end {
            // The token was empty, so start it with last read byte.
            self.token_start = self.cursor - 1;
            self.token_end = self.cursor;
        } else {
            // Add last read byte to token.
            if self.token_end + 1 < self.cursor {
                self.data[self.token_end] = self.data[self.cursor - 1];
            }
            self.token_end += 1;
        }
    }

    /// Get a slice with all unread bytes.
    pub fn peek_many(&self) -> &[u8] {
        self.data
            .get(self.cursor..self.filled)
            .expect("buffer's unread bytes should be in-bounds")
    }

    /// Consume up to `count` bytes.
    ///
    /// This method should be invoked only after an immediately preceding
    /// invocation of [`Buffer::peek_many`] that produced a slice with at least
    /// `count` bytes.
    pub fn consume_many(&mut self, count: usize) {
        self.cursor = min(self.filled, self.cursor.saturating_add(count));
    }

    /// Retain the given number of bytes.
    ///
    /// # Panics
    ///
    /// If the token doesn't end at least `count` bytes before the cursor. This
    /// method should be invoked only after immediately preceding invocations of
    /// [`Buffer::peek_many`] and [`Buffer::consume_many`] for a slice with at
    /// least `count` bytes.
    pub fn retain_many(&mut self, count: usize) {
        assert!(count <= self.cursor - self.token_end);
        let many_start = self.cursor - count;
        if self.token_start == self.token_end {
            self.token_start = many_start;
            self.token_end = self.cursor;
        } else {
            if self.token_end + count < self.cursor {
                self.data
                    .copy_within(many_start..self.cursor, self.token_end);
            }
            self.token_end += count;
        }
    }

    /// Get the token's value.
    pub fn token(&self) -> &[u8] {
        &self.data[self.token_start..self.token_end]
    }

    /// Determine whether this buffer is fragmented.
    ///
    /// The buffer is fragmented if it has space before the token or between
    /// token and cursor. In that case, [`Buffer::defrag`] can maximize
    /// continuous free space by shifting token and unread bytes towards the
    /// buffer start.
    #[inline]
    pub fn is_fragmented(&self) -> bool {
        0 < self.token_start || self.token_end < self.cursor
    }

    /// Determine whether this buffer has spare capacity.
    ///
    /// This method returns `true` if there is space to read in more data
    /// without defragmenting.
    #[inline]
    pub fn has_capacity(&self) -> bool {
        self.filled < self.data.capacity()
    }

    /// Determine whether this buffer has been exhausted.
    ///
    /// A buffer is exhausted if it is not readable, not fragmented, and without
    /// capacity.
    pub fn is_exhausted(&self) -> bool {
        !self.is_readable() && !self.is_fragmented() && !self.has_capacity()
    }

    /// Defragment the buffer contents.
    ///
    /// This method reclaims any space before the token and between the token
    /// and cursor by shifting token and unread bytes as far down as possible.
    /// In both cases, it is careful to copy bytes only when necessary.
    pub fn defrag(&mut self) {
        // Backshift token
        let token_length = self.token_end - self.token_start;
        if 0 < self.token_start && 0 < token_length {
            self.data.copy_within(self.token_start..self.token_end, 0);
        }

        // Backshift unread bytes
        let unread_length = self.filled - self.cursor;
        if token_length < self.cursor && 0 < unread_length {
            self.data
                .copy_within(self.cursor..self.filled, token_length);
        }

        // Update indices
        self.token_start = 0;
        self.token_end = token_length;
        self.cursor = token_length;
        self.filled = token_length + unread_length;
    }

    /// Fill the buffer and return the number of bytes read.
    ///
    /// # Panics
    ///
    /// If the number of bytes read is larger than the size of the read buffer.
    pub fn fill(&mut self, reader: &mut impl Read) -> std::io::Result<usize> {
        // SAFETY: .filled being in bounds is a critical invariant for this struct.
        let slice = unsafe { self.data.get_unchecked_mut(self.filled..) };
        let count = reader.read(slice)?;
        assert!(count <= slice.len(), "read count is at most buffer size");
        self.filled += count;
        Ok(count)
    }
}

impl std::fmt::Debug for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Buffer")
            .field("token_start", &self.token_start)
            .field("token_end", &self.token_end)
            .field("cursor", &self.cursor)
            .field("filled", &self.filled)
            .field("capacity", &self.data.capacity())
            .finish()
    }
}
