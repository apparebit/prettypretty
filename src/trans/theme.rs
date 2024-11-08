#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use crate::core::parse_x;
use crate::error::{ColorFormatError, OutOfBoundsError, ThemeError, ThemeErrorKind};
use crate::style::{AnsiColor, Layer};
use crate::{rgb, Color, ColorSpace};

/// A color theme.
///
/// A color theme is a container with 18 colors, one each for the default
/// foreground and background colors as well as the 16 ANSI colors. Not
/// surprisingly, the internal representation is an array with 18 colors. It
/// even is accessible through [`AsRef<[Color]> for
/// Theme`](struct.Theme.html#impl-AsRef%3C%5BColor%5D%3E-for-Theme), albeit
/// Rust-only and read-only. The primary reason for encapsulating the array
/// thusly is to force the use of semantic index values, i.e., [`ThemeEntry`],
/// [`Layer`](crate::style::Layer), or [`AnsiColor`](crate::style::AnsiColor).
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.trans"))]
#[derive(Clone, Debug)]
pub struct Theme {
    inner: [Color; 18],
}

impl Theme {
    /// Create a new color theme with 18 times the default color.
    pub fn new() -> Self {
        Self {
            inner: <[Color; 18]>::default(),
        }
    }

    /// Create a new color theme with the given colors.
    pub const fn with_array(colors: [Color; 18]) -> Self {
        Self { inner: colors }
    }

    /// Create a new color theme with the given colors.
    ///
    /// The given slice must have length 18. Otherwise, this method returns
    /// `None`.
    pub fn with_slice(colors: &[Color]) -> Option<Self> {
        if colors.len() != 18 {
            None
        } else {
            let mut inner = <[Color; 18]>::default();
            inner.clone_from_slice(colors);
            Some(Self { inner })
        }
    }

    /// Query the terminal for the current color theme.
    #[cfg(all(feature = "term", target_family = "unix"))]
    pub fn query_terminal() -> Result<Theme, ThemeError> {
        use crate::term::{terminal, VtScanner};
        use std::io::Write;

        // Set up terminal, scanner, and empty theme
        let mut tty = terminal()
            .access()
            .map_err(|e| ThemeError::new(ThemeErrorKind::AccessDevice, e.into()))?;
        let mut scanner = VtScanner::new();
        let mut theme = Theme::default();

        for entry in ThemeEntry::all() {
            // Write query to terminal
            write!(tty, "{}", entry)
                .and_then(|()| tty.flush())
                .map_err(|e| ThemeError::new(ThemeErrorKind::WriteQuery(entry), e.into()))?;

            // Parse terminal's response
            let response = scanner
                .scan_str(&mut tty)
                .map_err(|e| ThemeError::new(ThemeErrorKind::ScanEscape(entry), e.into()))?;
            let color = entry
                .parse_response(response)
                .map_err(|e| ThemeError::new(ThemeErrorKind::ParseColor(entry), e.into()))?;

            // Update theme
            theme[entry] = color;
        }

        Ok(theme)
    }
}

#[cfg(feature = "pyffi")]
#[pymethods]
impl Theme {
    /// Query the terminal for the current color theme.
    #[cfg(all(feature = "term", target_family = "unix"))]
    #[pyo3(name = "query_terminal")]
    #[staticmethod]
    pub fn py_query_terminal() -> PyResult<Self> {
        Theme::query_terminal().map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    /// Create a new color theme with the given colors.
    #[new]
    pub const fn py_with_array(inner: [Color; 18]) -> Self {
        Self::with_array(inner)
    }

    /// Get the color for the given theme entry.
    pub fn __getitem__(
        &self,
        #[pyo3(from_py_with = "into_theme_entry")] index: ThemeEntry,
    ) -> Color {
        self[index].clone()
    }

    /// Set the color for the given theme entry.
    pub fn __setitem__(
        &mut self,
        #[pyo3(from_py_with = "into_theme_entry")] index: ThemeEntry,
        value: Color,
    ) {
        self[index] = value;
    }

    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<[Color]> for Theme {
    fn as_ref(&self) -> &[Color] {
        &self.inner
    }
}

impl std::ops::Index<ThemeEntry> for Theme {
    type Output = Color;

    fn index(&self, index: ThemeEntry) -> &Self::Output {
        match index {
            ThemeEntry::DefaultForeground() => &self.inner[0],
            ThemeEntry::DefaultBackground() => &self.inner[1],
            ThemeEntry::Ansi(color) => &self.inner[2 + color as usize],
        }
    }
}

impl std::ops::IndexMut<ThemeEntry> for Theme {
    fn index_mut(&mut self, index: ThemeEntry) -> &mut Self::Output {
        match index {
            ThemeEntry::DefaultForeground() => &mut self.inner[0],
            ThemeEntry::DefaultBackground() => &mut self.inner[1],
            ThemeEntry::Ansi(color) => &mut self.inner[2 + color as usize],
        }
    }
}

impl std::ops::Index<AnsiColor> for Theme {
    type Output = Color;

    fn index(&self, index: AnsiColor) -> &Self::Output {
        &self.inner[2 + index as usize]
    }
}

impl std::ops::IndexMut<AnsiColor> for Theme {
    fn index_mut(&mut self, index: AnsiColor) -> &mut Self::Output {
        &mut self.inner[2 + index as usize]
    }
}

impl std::ops::Index<Layer> for Theme {
    type Output = Color;

    fn index(&self, index: Layer) -> &Self::Output {
        match index {
            Layer::Foreground => &self.inner[0],
            Layer::Background => &self.inner[1],
        }
    }
}

// --------------------------------------------------------------------------------------------------------------------

/// The entries of a color theme.
///
/// This enumeration combines two variants for the default foreground and
/// background color with another variant that wraps an [`AnsiColor`], in that
/// order, to identify the 18 entries of a color theme. Displaying a theme entry
/// produces the ANSI escape sequence used to query a terminal for the
/// corresponding color.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, ord, module = "prettypretty.color.trans")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ThemeEntry {
    DefaultForeground(),
    DefaultBackground(),
    Ansi(AnsiColor),
}

impl ThemeEntry {
    /// Create a new iterator over all theme entries in canonical order.
    pub fn all() -> ThemeEntryIterator {
        ThemeEntryIterator::new()
    }
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl ThemeEntry {
    /// Create a new iterator over all theme entries in canonical order.
    #[cfg(feature = "pyffi")]
    #[pyo3(name = "all")]
    #[staticmethod]
    pub fn py_all() -> ThemeEntryIterator {
        ThemeEntryIterator::new()
    }

    /// Try getting the theme entry for the given index.
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn try_from_index(value: usize) -> Result<ThemeEntry, OutOfBoundsError> {
        ThemeEntry::try_from(value)
    }

    /// Get this theme entry's human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::DefaultForeground() => "default foreground",
            Self::DefaultBackground() => "default background",
            Self::Ansi(color) => color.name(),
        }
    }

    /// Get an abbreviation for this theme entry's name.
    ///
    /// This method returns a two-letter abbreviations for this theme entry. See
    /// [`AnsiColor::abbr`] for a description of the abbreviations for ANSI
    /// colors.
    pub fn abbr(&self) -> &'static str {
        match self {
            Self::DefaultForeground() => "fg",
            Self::DefaultBackground() => "bg",
            Self::Ansi(color) => color.abbr(),
        }
    }

    /// Parse the response to a theme color query.
    ///
    /// The string should contain the escape sequence *without* the leading OSC
    /// and trailing ST or BEL controls. This method validates that the response
    /// is for this theme entry indeed.
    pub fn parse_response(&self, s: &str) -> Result<Color, ColorFormatError> {
        let s = match self {
            Self::DefaultForeground() => s.strip_prefix("10;"),
            Self::DefaultBackground() => s.strip_prefix("11;"),
            Self::Ansi(color) => {
                // Consume parameter for ANSI colors
                let s = s
                    .strip_prefix("4;")
                    .ok_or(ColorFormatError::MalformedThemeColor)?;

                // Consume color code
                let code = *color as u8;
                if code < 10 {
                    s.strip_prefix(char::from(b'0' + code))
                } else {
                    s.strip_prefix('1')
                        .and_then(|s| s.strip_prefix(char::from(b'0' + code - 10)))
                }
                .and_then(|s| s.strip_prefix(';'))
            }
        }
        .ok_or(ColorFormatError::WrongThemeColor)?;

        Ok(Color::new(ColorSpace::Srgb, parse_x(s)?))
    }

    /// Render a debug representation for this theme entry. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    /// Render an ANSI escape sequence to query a terminal for this theme
    /// entry's current color. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __str__(&self) -> String {
        format!("{}", self)
    }
}

impl From<AnsiColor> for ThemeEntry {
    fn from(value: AnsiColor) -> Self {
        ThemeEntry::Ansi(value)
    }
}

impl TryFrom<usize> for ThemeEntry {
    type Error = OutOfBoundsError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value == 0 {
            Ok(ThemeEntry::DefaultForeground())
        } else if value == 1 {
            Ok(ThemeEntry::DefaultBackground())
        } else if value <= 17 {
            Ok(ThemeEntry::Ansi(AnsiColor::try_from(value as u8 - 2)?))
        } else {
            Err(OutOfBoundsError::new(value, 0..=17))
        }
    }
}

/// Convert ANSI colors and layers into theme entries.
#[cfg(feature = "pyffi")]
pub(crate) fn into_theme_entry(obj: &Bound<'_, PyAny>) -> PyResult<ThemeEntry> {
    obj.extract::<ThemeEntry>()
        .or_else(|_| obj.extract::<AnsiColor>().map(ThemeEntry::Ansi))
        .or_else(|_| {
            obj.extract::<Layer>().map(|l| match l {
                Layer::Foreground => ThemeEntry::DefaultForeground(),
                Layer::Background => ThemeEntry::DefaultBackground(),
            })
        })
}

impl std::fmt::Display for ThemeEntry {
    /// Get an ANSI escape sequence to query a terminal for this theme entry's
    /// current color.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DefaultForeground() => f.write_str("\x1b]10;?\x1b\\"),
            Self::DefaultBackground() => f.write_str("\x1b]11;?\x1b\\"),
            Self::Ansi(color) => f.write_fmt(format_args!("\x1b]4;{};?\x1b\\", *color as u8)),
        }
    }
}

/// A helper for iterating over theme entries.
///
/// This iterator is fused, i.e., after returning `None` once, it will keep
/// returning `None`. This iterator also is exact, i.e., its `size_hint()`
/// returns the exact number of remaining items.
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.trans"))]
#[derive(Debug)]
pub struct ThemeEntryIterator {
    index: usize,
}

impl ThemeEntryIterator {
    fn new() -> Self {
        Self { index: 0 }
    }
}

impl Iterator for ThemeEntryIterator {
    type Item = ThemeEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if 18 <= self.index {
            None
        } else {
            let item = ThemeEntry::try_from(self.index).unwrap();
            self.index += 1;
            Some(item)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = 18 - self.index;
        (remaining, Some(remaining))
    }
}

impl std::iter::ExactSizeIterator for ThemeEntryIterator {
    fn len(&self) -> usize {
        18 - self.index
    }
}

impl std::iter::FusedIterator for ThemeEntryIterator {}

#[cfg(feature = "pyffi")]
#[pymethods]
impl ThemeEntryIterator {
    /// Get the number of remaining theme entries. <i class=python-only>Python
    /// only!</i>
    pub fn __len__(&self) -> usize {
        self.len()
    }

    /// Return this iterator. <i class=python-only>Python only!</i>
    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Return the next theme entry. <i class=python-only>Python only!</i>
    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<ThemeEntry> {
        slf.next()
    }
}

// --------------------------------------------------------------------------------------------------------------------

/// The color theme with the 2+16 colors of [VGA text
/// mode](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit).
pub const VGA_COLORS: Theme = Theme::with_array([
    rgb!(0, 0, 0),
    rgb!(255, 255, 255),
    rgb!(0, 0, 0),
    rgb!(170, 0, 0),
    rgb!(0, 170, 0),
    rgb!(170, 85, 0),
    rgb!(0, 0, 170),
    rgb!(170, 0, 170),
    rgb!(0, 170, 170),
    rgb!(170, 170, 170),
    rgb!(85, 85, 85),
    rgb!(255, 85, 85),
    rgb!(85, 255, 85),
    rgb!(255, 255, 85),
    rgb!(85, 85, 255),
    rgb!(255, 85, 255),
    rgb!(85, 255, 255),
    rgb!(255, 255, 255),
]);

#[cfg(test)]
mod test {
    use super::ThemeEntry;
    use crate::style::AnsiColor;

    #[test]
    fn test_theme_entry() {
        assert_eq!(
            format!("{}", ThemeEntry::DefaultForeground()),
            "\x1b]10;?\x1b\\"
        );

        assert_eq!(
            ThemeEntry::Ansi(AnsiColor::BrightGreen).to_string(),
            "\x1b]4;10;?\x1b\\".to_string()
        )
    }
}
