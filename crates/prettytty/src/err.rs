//! Helper module with this crate's error type.
//!
//! Terminal errors complement I/O errors by providing additional information
//! about error conditions when scanning or parsing terminal input. They
//! seamlessly convert to and from I/O errors.

use super::cmd::{Format, ResetStyle, SetForeground8};

/// The enumeration of error kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// No data is available when reading, most likely due to a timeout.
    NoData,
    /// The state machine is in-flight and hence reading raw bytes is not safe.
    InFlight,
    /// A malformed UTF-8 character.
    MalformedUtf8,
    /// A malformed ANSI escape sequence.
    MalformedSequence,
    /// A pathological ANSI escape sequence is longer than a configurable threshold.
    PathologicalSequence,
    /// A well-formed ANSI escape sequence starting with the wrong control.
    BadControl,
    /// An unexpected but well-formed ANSI escape sequence.
    BadSequence,
    /// A token other than a sequence when a sequence is expected.
    NotASequence,
    /// An ANSI escape sequence longer than the available internal buffer space.
    OutOfMemory,
    /// Too few color components or coordinates.
    TooFewCoordinates,
    /// Too many color components or coordinates.
    TooManyCoordinates,
    /// Empty color coordinate.
    EmptyCoordinate,
    /// Oversized color coordinate.
    OversizedCoordinate,
    /// Malformed coordinate.
    MalformedCoordinate,
    /// An error reading from the reader providing data.
    Unreadable,
}

impl ErrorKind {
    /// Turn the error kind to an error message.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoData => "reading terminal input timed out without returning data",
            Self::InFlight => "token is in-flight, hence access to raw bytes is not safe",
            Self::MalformedUtf8 => "malformed UTF-8",
            Self::MalformedSequence => "malformed ANSI escape sequence",
            Self::PathologicalSequence => "pathologically long ANSI escape sequence",
            Self::BadControl => "unexpected control for ANSI escape sequence",
            Self::BadSequence => "unexpected ANSI escape sequence",
            Self::NotASequence => "token not an ANSI escape sequence",
            Self::OutOfMemory => "ANSI escape sequence too long for internal buffer",
            Self::Unreadable => "error reading terminal",
            Self::TooFewCoordinates => "too few color coordinates",
            Self::TooManyCoordinates => "too many color coordinates",
            Self::EmptyCoordinate => "empty color coordinate",
            Self::OversizedCoordinate => "oversized color coordinate",
            Self::MalformedCoordinate => "malformed color coordinate",
        }
    }
}

impl From<ErrorKind> for std::io::Error {
    fn from(value: ErrorKind) -> Self {
        Error::from(value).into()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error { kind, source: None }
    }
}

/// A terminal error.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    source: Option<std::io::Error>,
}

impl Error {
    /// Create a new unreadable error.
    pub fn unreadable(source: std::io::Error) -> Self {
        Self {
            kind: ErrorKind::Unreadable,
            source: Some(source),
        }
    }

    /// Get the error kind.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.kind.as_str())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self {
            kind: ErrorKind::Unreadable,
            source: Some(error),
        } = self
        {
            Some(error)
        } else {
            None
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::unreadable(value)
    }
}

impl From<Error> for std::io::Error {
    fn from(value: Error) -> Self {
        use self::ErrorKind::*;

        match value.kind {
            MalformedUtf8 | MalformedSequence | PathologicalSequence | BadControl | BadSequence
            | NotASequence | TooFewCoordinates | TooManyCoordinates | EmptyCoordinate
            | OversizedCoordinate | MalformedCoordinate => {
                Self::new(std::io::ErrorKind::InvalidData, value)
            }
            NoData => std::io::ErrorKind::TimedOut.into(),
            InFlight => std::io::ErrorKind::ResourceBusy.into(),
            OutOfMemory => std::io::ErrorKind::OutOfMemory.into(),
            Unreadable => {
                if let Some(error) = value.source {
                    error
                } else {
                    Self::new(std::io::ErrorKind::Other, value)
                }
            }
        }
    }
}

/// Determine whether an operation should be retried.
///
/// This function treats both interrupted and timed out operations as retryable.
pub fn should_retry<T, E>(result: std::result::Result<T, E>) -> bool
where
    E: Into<std::io::Error>,
{
    if let Err(err) = result {
        let kind = err.into().kind();
        kind == std::io::ErrorKind::Interrupted || kind == std::io::ErrorKind::TimedOut
    } else {
        false
    }
}

/// Report the error, including any sources.
pub fn report<E: std::error::Error>(error: &E) {
    println!(
        "{}{}ERROR: {}{}",
        Format::Bold,
        SetForeground8::<1>,
        error,
        ResetStyle
    );

    let mut error: &dyn std::error::Error = error;
    while let Some(inner) = error.source() {
        println!("    {}", inner);
        error = inner;
    }
}
