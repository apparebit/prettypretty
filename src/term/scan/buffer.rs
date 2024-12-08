use std::io::Read;

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
    /// Create a new buffer with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: vec![0; capacity],
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
    /// This method panics if there are no more bytes to read. It should be
    /// invoked at most once right after `peek`.
    pub fn consume(&mut self) {
        assert!(self.cursor < self.filled);
        self.cursor += 1;
    }

    /// Retain the most recently consumed byte for the current token.
    ///
    /// This method panics if the token doesn't end before the cursor. It should
    /// be invoked at most once right after `consume`.
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
        self.data.get(self.cursor..self.filled).unwrap()
    }

    /// Consume the given number of bytes.
    ///
    /// This method panics if the count is larger than the number of unread
    /// bytes.
    pub fn consume_many(&mut self, count: usize) {
        let new_cursor = self.cursor.saturating_add(count);
        assert!(new_cursor <= self.filled);
        self.cursor = new_cursor;
    }

    /// Retain the given number of bytes.
    ///
    /// This method panics if the token doesn't end at least count bytes before
    /// the cursor.
    pub fn retain_many(&mut self, count: usize) {
        assert!(self.token_end + count <= self.cursor);
        let many_start = self.cursor - count;
        if self.token_start == self.token_end {
            self.token_start = many_start;
            self.token_end = self.cursor;
        } else {
            if self.token_end + count < self.cursor {
                self.data.copy_within(many_start..self.cursor, self.token_end);
            }
            self.token_end += count;
        }
    }

    /// Get the token's value.
    pub fn token(&self) -> &[u8] {
        &self.data[self.token_start..self.token_end]
    }

    /// Determine whether this buffer can create space by backshifting.
    ///
    /// This method returns `true` if the there is space before the token or
    /// between token and cursor.
    #[inline]
    pub fn is_backshiftable(&self) -> bool {
        0 < self.token_start || self.token_end < self.cursor
    }

    /// Determine whether this buffer has spare capacity.
    ///
    /// This method returns `true` if there is space to read in more data
    /// without backshifting.
    #[inline]
    pub fn has_capacity(&self) -> bool {
        self.filled < self.data.capacity()
    }

    /// Determine whether this buffer has been exhausted.
    ///
    /// A buffer is exhausted if it is not readable, not backshiftable, and
    /// without capacity.
    pub fn is_exhausted(&self) -> bool {
        !self.is_readable() && !self.is_backshiftable() && !self.has_capacity()
    }

    /// Backshift the buffer contents.
    ///
    /// This method reclaims space before token and between token and cursor.
    pub fn backshift(&mut self) {
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

        self.token_start = 0;
        self.token_end = token_length;
        self.cursor = token_length;
        self.filled = token_length + unread_length;
    }

    /// Fill the buffer and return the number of bytes read.
    pub fn fill(&mut self, reader: &mut impl Read) -> std::io::Result<usize> {
        let slice = self.data.get_mut(self.filled..).unwrap();
        let count = reader.read(slice)?;
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
