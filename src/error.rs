#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

/// An out-of-bounds error.
///
/// This error indicates an index value that is out of bounds for some range.
/// The ranges used by this crate include:
///
///   * `0..=5` for coordinates of [`EmbeddedRgb`](crate::EmbeddedRgb);
///   * `0..=15` for index values of [`AnsiColor`](crate::AnsiColor);
///   * `0..=23` for the gay levels of [`GrayGradient`](crate::GrayGradient);
///   * `16..=231` for index values of [`EmbeddedRgb`](crate::EmbeddedRgb);
///   * `232..=255` for index values of [`GrayGradient`](crate::GrayGradient).
///
#[derive(Clone, Debug)]
pub struct OutOfBoundsError {
    pub value: usize,
    pub expected: std::ops::RangeInclusive<usize>,
}

impl OutOfBoundsError {
    /// Create a new out-of-bounds error.
    pub fn new(value: impl Into<usize>, expected: std::ops::RangeInclusive<usize>) -> Self {
        Self {
            value: value.into(),
            expected,
        }
    }
}

impl std::fmt::Display for OutOfBoundsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} should fit into range {}..={}",
            self.value,
            self.expected.start(),
            self.expected.end()
        )
    }
}

#[cfg(feature = "pyffi")]
impl From<OutOfBoundsError> for PyErr {
    fn from(value: OutOfBoundsError) -> Self {
        pyo3::exceptions::PyIndexError::new_err(value.to_string())
    }
}
