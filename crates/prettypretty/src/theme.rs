//! Utility module implementing terminal color themes.
#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use crate::error::OutOfBoundsError;
use crate::style::Layer;
use crate::termco::AnsiColor;
use crate::{rgb, Color};

#[cfg(feature = "tty")]
use crate::Float;
#[cfg(feature = "tty")]
use prettytty::{cmd::RequestColor, Command, Connection, Control, Query, Scan};
#[cfg(feature = "tty")]
use std::io::Write;

/// A color theme.
///
/// A color theme is a container with [`ThemeEntry::COUNT`] colors, one each for
/// the 16 ANSI colors as well as the default foreground and background colors
/// (in that order). The public interface is a compromise between struct and
/// array, a straurray if you will, to make the primary use case, processing the
/// colors in a theme, safer than when using numeric indices. Hence, you index a
/// color theme with semantic values, i.e., [`ThemeEntry`], [`Layer`], or
/// [`AnsiColor`]. At the same time, you can still access the underlying array
/// storage through [`AsRef<[Color]> for
/// Theme`](struct.Theme.html#impl-AsRef%3C%5BColor%5D%3E-for-Theme), albeit
/// Rust-only and read-only.
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.theme"))]
#[derive(Clone, PartialEq, Eq)]
pub struct Theme {
    inner: [Color; ThemeEntry::COUNT],
}

impl Theme {
    /// Create a new color theme with [`ThemeEntry::COUNT`] times the default color.
    pub fn new() -> Self {
        Self {
            inner: <[Color; ThemeEntry::COUNT]>::default(),
        }
    }

    /// Create a new color theme with the given colors.
    pub const fn with_array(colors: [Color; ThemeEntry::COUNT]) -> Self {
        Self { inner: colors }
    }

    /// Create a new color theme with the given colors.
    ///
    /// The given slice must have length [`ThemeEntry::COUNT`]. Otherwise, this
    /// method returns `None`.
    pub fn with_slice(colors: &[Color]) -> Option<Self> {
        if colors.len() != ThemeEntry::COUNT {
            None
        } else {
            let mut inner = <[Color; ThemeEntry::COUNT]>::default();
            inner.clone_from_slice(colors);
            Some(Self { inner })
        }
    }
}

#[cfg(feature = "pyffi")]
#[pymethods]
impl Theme {
    /// Create a new color theme with the given colors.
    #[new]
    pub const fn py_with_array(inner: [Color; ThemeEntry::COUNT]) -> Self {
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

#[cfg(feature = "tty")]
impl Theme {
    /// Query the terminal for the current theme colors using one loop.
    #[doc(hidden)]
    pub fn query1(connection: &Connection) -> std::io::Result<Self> {
        let (mut input, mut output) = connection.io();
        let mut theme = Self::new();

        for entry in ThemeEntry::all() {
            output.exec(entry)?;
            let payload = input.read_sequence(entry.control())?;
            theme[entry] = <ThemeEntry as Query>::parse(&entry, payload)?;
        }

        Ok(theme)
    }

    /// Query the terminal for the current theme colors using two loops.
    #[doc(hidden)]
    pub fn query2(connection: &Connection) -> std::io::Result<Self> {
        let (mut input, mut output) = connection.io();
        let mut theme = Self::new();

        for entry in ThemeEntry::all() {
            write!(output, "{}", entry)?;
        }
        output.flush()?;

        for entry in ThemeEntry::all() {
            let payload = input.read_sequence(entry.control())?;
            theme[entry] = <ThemeEntry as Query>::parse(&entry, payload)?;
        }

        Ok(theme)
    }

    /// Query the terminal for the current theme colors using three loops.
    #[doc(hidden)]
    pub fn query3(connection: &Connection) -> std::io::Result<Theme> {
        let (mut input, mut output) = connection.io();
        let mut theme = Self::new();

        for entry in ThemeEntry::all() {
            write!(output, "{}", entry)?;
        }
        output.flush()?;

        let mut payloads = Vec::with_capacity(18);
        for entry in ThemeEntry::all() {
            let payload = input.read_sequence(entry.control())?;
            payloads.push(payload.to_owned());
        }

        for (entry, payload) in ThemeEntry::all().zip(payloads) {
            theme[entry] = <ThemeEntry as Query>::parse(&entry, &payload)?;
        }

        Ok(theme)
    }

    /// Query the terminal for the current color theme. <i class=tty-only>TTY
    /// only!</i>
    pub fn query(connection: &Connection) -> std::io::Result<Self> {
        Self::query2(connection)
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
            ThemeEntry::Ansi(color) => &self.inner[color as usize],
            ThemeEntry::DefaultForeground() => &self.inner[16],
            ThemeEntry::DefaultBackground() => &self.inner[17],
        }
    }
}

impl std::ops::IndexMut<ThemeEntry> for Theme {
    fn index_mut(&mut self, index: ThemeEntry) -> &mut Self::Output {
        match index {
            ThemeEntry::Ansi(color) => &mut self.inner[color as usize],
            ThemeEntry::DefaultForeground() => &mut self.inner[16],
            ThemeEntry::DefaultBackground() => &mut self.inner[17],
        }
    }
}

impl std::ops::Index<AnsiColor> for Theme {
    type Output = Color;

    fn index(&self, index: AnsiColor) -> &Self::Output {
        &self.inner[index as usize]
    }
}

impl std::ops::IndexMut<AnsiColor> for Theme {
    fn index_mut(&mut self, index: AnsiColor) -> &mut Self::Output {
        &mut self.inner[index as usize]
    }
}

impl std::ops::Index<Layer> for Theme {
    type Output = Color;

    fn index(&self, index: Layer) -> &Self::Output {
        match index {
            Layer::Foreground => &self.inner[16],
            Layer::Background => &self.inner[17],
        }
    }
}

impl std::fmt::Debug for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debugger = f.debug_struct("Theme");
        for entry in ThemeEntry::all() {
            debugger.field(&entry.name().replace(" ", "_"), &self[entry]);
        }
        debugger.finish()
    }
}

// --------------------------------------------------------------------------------------------------------------------

/// A color theme entry.
///
/// This enumeration combines a variant wrapping an [`AnsiColor`] with two more
/// variants for the default foreground and background colors to identify the
/// [`ThemeEntry::COUNT`] entries of a color theme. Displaying a theme entry
/// produces the ANSI escape sequence used to query a terminal for the
/// corresponding color.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, ord, module = "prettypretty.color.theme")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ThemeEntry {
    Ansi(AnsiColor),
    DefaultForeground(),
    DefaultBackground(),
}

impl ThemeEntry {
    /// The total number of theme entries.
    pub const COUNT: usize = 18;

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
            Self::Ansi(color) => color.name(),
            Self::DefaultForeground() => "default foreground",
            Self::DefaultBackground() => "default background",
        }
    }

    /// Get an abbreviation for this theme entry's name.
    ///
    /// This method returns a two-letter abbreviations for this theme entry. See
    /// [`AnsiColor::abbr`] for a description of the abbreviations for ANSI
    /// colors.
    pub fn abbr(&self) -> &'static str {
        match self {
            Self::Ansi(color) => color.abbr(),
            Self::DefaultForeground() => "fg",
            Self::DefaultBackground() => "bg",
        }
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

#[cfg(feature = "tty")]
impl ThemeEntry {
    /// Convert the theme entry to a color request. <i class=tty-only>TTY
    /// only!</i>
    pub fn request(&self) -> RequestColor {
        if let ThemeEntry::Ansi(color) = self {
            match color {
                AnsiColor::Black => RequestColor::Black,
                AnsiColor::Red => RequestColor::Red,
                AnsiColor::Green => RequestColor::Green,
                AnsiColor::Yellow => RequestColor::Yellow,
                AnsiColor::Blue => RequestColor::Blue,
                AnsiColor::Magenta => RequestColor::Magenta,
                AnsiColor::Cyan => RequestColor::Cyan,
                AnsiColor::White => RequestColor::White,
                AnsiColor::BrightBlack => RequestColor::BrightBlack,
                AnsiColor::BrightRed => RequestColor::BrightRed,
                AnsiColor::BrightGreen => RequestColor::BrightGreen,
                AnsiColor::BrightYellow => RequestColor::BrightYellow,
                AnsiColor::BrightBlue => RequestColor::BrightBlue,
                AnsiColor::BrightMagenta => RequestColor::BrightMagenta,
                AnsiColor::BrightCyan => RequestColor::BrightCyan,
                AnsiColor::BrightWhite => RequestColor::BrightWhite,
            }
        } else {
            match self {
                ThemeEntry::DefaultForeground() => RequestColor::Foreground,
                ThemeEntry::DefaultBackground() => RequestColor::Background,
                _ => unreachable!(),
            }
        }
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
        if value <= 15 {
            Ok(ThemeEntry::Ansi(AnsiColor::try_from(value as u8)?))
        } else if value == 16 {
            Ok(ThemeEntry::DefaultForeground())
        } else if value == 17 {
            Ok(ThemeEntry::DefaultBackground())
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

#[cfg(feature = "tty")]
/// Theme entry as a command. <i class=tty-only>TTY only!</i>
impl Command for ThemeEntry {}

#[cfg(feature = "tty")]
/// Theme entry as a query. <i class=tty-only>TTY only!</i>
impl Query for ThemeEntry {
    type Response = Color;

    fn control(&self) -> prettytty::Control {
        Control::OSC
    }

    fn parse(&self, payload: &[u8]) -> std::io::Result<Self::Response> {
        let [r, g, b] = self.request().parse(payload)?;
        fn as_float((numerator, denominator): (u16, u16)) -> Float {
            // 1, 2, 3, 4 --> 4, 8, 12, 16 --> 0x10, 0x100, 0x1000, 0x10000
            numerator as Float / ((1 << (denominator << 2)) - 1) as Float
        }

        Ok(Color::srgb(as_float(r), as_float(g), as_float(b)))
    }
}

impl std::fmt::Display for ThemeEntry {
    /// Get an ANSI escape sequence to query a terminal for this theme entry's
    /// current color.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ansi(color) => f.write_fmt(format_args!("\x1b]4;{};?\x1b\\", *color as u8)),
            Self::DefaultForeground() => f.write_str("\x1b]10;?\x1b\\"),
            Self::DefaultBackground() => f.write_str("\x1b]11;?\x1b\\"),
        }
    }
}

/// An iterator over theme entries.
///
/// [`ThemeEntry::all`] returns this iterator, which produces all theme entries
/// in the canonical order. It is fused, i.e., after returning `None` once, it
/// will keep returning `None`. It also is exact, i.e., its `size_hint()`
/// returns the exact number of remaining items.
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.theme"))]
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
        if ThemeEntry::COUNT <= self.index {
            None
        } else {
            let item =
                ThemeEntry::try_from(self.index).expect("index should be smaller than count");
            self.index += 1;
            Some(item)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = ThemeEntry::COUNT - self.index;
        (remaining, Some(remaining))
    }
}

impl std::iter::ExactSizeIterator for ThemeEntryIterator {
    fn len(&self) -> usize {
        ThemeEntry::COUNT - self.index
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
    rgb!(0, 0, 0),       // Black
    rgb!(170, 0, 0),     // Red
    rgb!(0, 170, 0),     // Green
    rgb!(170, 85, 0),    // Yellow(ish)
    rgb!(0, 0, 170),     // Blue
    rgb!(170, 0, 170),   // Magenta
    rgb!(0, 170, 170),   // Cyan
    rgb!(170, 170, 170), // White
    rgb!(85, 85, 85),    // Bright Black
    rgb!(255, 85, 85),   // Bright Red
    rgb!(85, 255, 85),   // Bright Green
    rgb!(255, 255, 85),  // Bright Yellow
    rgb!(85, 85, 255),   // Bright Blue
    rgb!(255, 85, 255),  // Bright Magenta
    rgb!(85, 255, 255),  // Bright Cyan
    rgb!(255, 255, 255), // Bright White
    rgb!(0, 0, 0),       // Default Foreground
    rgb!(255, 255, 255), // Default Background
]);

#[cfg(test)]
mod test {
    use super::ThemeEntry;
    use crate::termco::AnsiColor;

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
