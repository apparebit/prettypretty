#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use super::{Fidelity, TerminalColor};

use Format::*;

/// Supported text formats.
///
/// This enum enables a compact and efficient encoding of text formats other
/// than colors. It includes two variants for each way terminal formats can
/// diverge from the default appearance, one to enable the format and one to
/// disable it again. Names for the disabling variants start with `Not`.
///
/// Enabling variants are sorted before disabling variants, with corresponding
/// enabling/disabling variants in the same order. `Bold` and `Thin` are
/// mutually exclusive and hence share the same disabling variant
/// `NotBoldOrThin`.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, eq_int, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Bold = 0x1,
    Thin = 0x2,
    Italic = 0x4,
    Underlined = 0x8,
    Blinking = 0x10,
    Reversed = 0x20,
    Hidden = 0x40,
    Stricken = 0x80,

    NotBoldOrThin = 0x100,
    // Reserved
    NotItalic = 0x400,
    NotUnderlined = 0x800,
    NotBlinking = 0x1000,
    NotReversed = 0x2000,
    NotHidden = 0x4000,
    NotStricken = 0x8000,
}

#[cfg(not(feature = "pyffi"))]
impl Format {
    /// Get an iterator over all formats.
    pub fn all() -> AllFormats {
        AllFormats(0)
    }
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Format {
    /// Get an iterator over all formats.
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn all() -> AllFormats {
        AllFormats(0)
    }

    /// Get the SGR parameter for this format.
    pub fn sgr_parameter(&self) -> u8 {
        match self {
            Bold => 1,
            Thin => 2,
            Italic => 3,
            Underlined => 4,
            Blinking => 5,
            Reversed => 7,
            Hidden => 8,
            Stricken => 9,
            NotBoldOrThin => 22,
            NotItalic => 23,
            NotUnderlined => 24,
            NotBlinking => 25,
            NotReversed => 27,
            NotHidden => 28,
            NotStricken => 29,
        }
    }

    /// Get the flag bit corresponding to this format.
    #[inline]
    const fn bits(&self) -> u16 {
        *self as u16
    }

    /// Test whether the value's flag bit for this format is set.
    #[inline]
    const fn test(&self, value: u16) -> bool {
        value & (*self as u16) != 0
    }

    /// Clear the value's flag bit for this format.
    #[inline]
    const fn clear(&self, value: u16) -> u16 {
        value & !(*self as u16)
    }

    /// Set the value's flag bit for this format.
    #[inline]
    const fn set(&self, value: u16) -> u16 {
        value | (*self as u16)
    }
}

// -------------------------------------------------------------------------------------

/// An iterator over all formats.
#[cfg_attr(feature = "pyffi", pyclass)]
#[derive(Debug)]
pub struct AllFormats(u8);

#[cfg_attr(feature = "pyffi", pymethods)]
impl AllFormats {
    /// Drain this iterator.
    pub fn drain(&mut self) {
        loop {
            if self.next().is_none() {
                return;
            }
        }
    }

    /// Access this iterator. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Get the next item. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Format> {
        slf.next()
    }
}

impl Iterator for AllFormats {
    type Item = Format;

    fn next(&mut self) -> Option<Self::Item> {
        let flag = match self.0 {
            0 => Bold,
            1 => Thin,
            2 => Italic,
            3 => Underlined,
            4 => Blinking,
            5 => Reversed,
            6 => Hidden,
            7 => Stricken,
            8 => NotBoldOrThin,
            9 => NotItalic,
            10 => NotUnderlined,
            11 => NotBlinking,
            12 => NotReversed,
            13 => NotHidden,
            14 => NotStricken,
            _ => return None,
        };

        self.0 += 1;

        Some(flag)
    }
}

impl std::iter::FusedIterator for AllFormats {}

// -------------------------------------------------------------------------------------

/// Masks for related format flags.
#[derive(Copy, Clone, Debug)]
enum Mask {
    Weight = (Bold.bits() | Thin.bits() | NotBoldOrThin.bits()) as isize,
    Slant = (Italic.bits() | NotItalic.bits()) as isize,
    Underlined = (Underlined.bits() | NotUnderlined.bits()) as isize,
    Blinking = (Blinking.bits() | NotBlinking.bits()) as isize,
    Reversed = (Reversed.bits() | NotReversed.bits()) as isize,
    Hidden = (Hidden.bits() | NotHidden.bits()) as isize,
    Stricken = (Stricken.bits() | NotStricken.bits()) as isize,
    NonDefaultFormats = 0xff,
}

impl Mask {
    /// Apply the mask, which clears all other bits.
    #[inline]
    const fn apply(&self, value: u16) -> u16 {
        value & (*self as u16)
    }

    /// Clear the mask's bits.
    #[inline]
    const fn clear(&self, value: u16) -> u16 {
        value & !(*self as u16)
    }
}

// -------------------------------------------------------------------------------------

/// Text formatting other than colors.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Formatting(u16);

#[cfg(not(feature = "pyffi"))]
impl Formatting {
    /// Create new, empty formatting.
    pub fn new() -> Self {
        Self(0)
    }
}

/// Negate a formatting's bits.
#[inline]
const fn negate_bits(value: u16) -> u16 {
    // Turn Thin into NotBoldOrThin, which is Bold << 8.
    if Thin.test(value) {
        Bold.set(Thin.clear(value)) << 8
    } else {
        value << 8
    }
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Formatting {
    /// Create new, empty formatting.
    #[cfg(feature = "pyffi")]
    #[new]
    pub fn new() -> Self {
        Self(0)
    }

    /// Create new formatting like this one that also uses bold font weight.
    pub const fn bold(&self) -> Self {
        Self(Bold.set(Mask::Weight.clear(self.0)))
    }

    /// Create new formatting like this one that also uses thin font weight.
    pub const fn thin(&self) -> Self {
        Self(Thin.set(Mask::Weight.clear(self.0)))
    }

    /// Create new formatting like this one that also uses italic font slant.
    pub const fn italic(&self) -> Self {
        Self(Italic.set(Mask::Slant.clear(self.0)))
    }

    /// Create new formatting like this one that also is underlined.
    pub const fn underlined(&self) -> Self {
        Self(Underlined.set(Mask::Underlined.clear(self.0)))
    }

    /// Create new formatting like this one that also is blinking.
    pub const fn blinking(&self) -> Self {
        Self(Blinking.set(Mask::Blinking.clear(self.0)))
    }

    /// Create new formatting like this one that also is reversed.
    pub const fn reversed(&self) -> Self {
        Self(Reversed.set(Mask::Reversed.clear(self.0)))
    }

    /// Create new formatting like this one that also is hidden.
    pub const fn hidden(&self) -> Self {
        Self(Hidden.set(Mask::Hidden.clear(self.0)))
    }

    /// Create new formatting like this one that also is stricken.
    pub const fn stricken(&self) -> Self {
        Self(Stricken.set(Mask::Stricken.clear(self.0)))
    }

    /// Determine whether this formatting includes the given format.
    pub const fn has(&self, format: Format) -> bool {
        format.test(self.0)
    }

    /// Get an iterator over the constituent formats.
    pub fn formats(&self) -> FormatIterator {
        FormatIterator {
            formatting: *self,
            all_formats: Format::all(),
        }
    }

    /// Negate this formatting. <i class=python-only>Python only!</i>
    ///
    /// If a terminal uses this formatting, the negated formatting restores the
    /// terminal's default appearance again.
    #[cfg(feature = "pyffi")]
    pub fn __invert__(&self) -> Self {
        !*self
    }

    /// Determine the difference between this and another formatting. <i
    /// class=python-only>Python only!</i>
    ///
    /// If a terminal uses the other formatting, the returned difference changes
    /// the terminal's formatting to this one. The returned difference is
    /// minimal.
    #[cfg(feature = "pyffi")]
    pub fn __sub__(&self, other: &Self) -> Self {
        *self - *other
    }
}

impl std::ops::Not for Formatting {
    type Output = Self;

    /// Negate this formatting.
    ///
    /// If a terminal uses this formatting, the negated formatting restores the
    /// terminal's default appearance.
    fn not(self) -> Self::Output {
        Self(negate_bits(self.0))
    }
}

impl std::ops::Sub for Formatting {
    type Output = Self;

    /// Determine the difference between this and another formatting. <i
    /// class=python-only>Python only!</i>
    ///
    /// If a terminal uses the other formatting, the returned difference changes
    /// the terminal's formatting to this one. The returned difference is
    /// minimal.
    fn sub(self, rhs: Self) -> Self::Output {
        let enable = Mask::NonDefaultFormats.apply(self.0 & !rhs.0);
        let mut disable = negate_bits(rhs.0 & !self.0);
        if Mask::Weight.apply(enable) != 0 {
            disable = NotBoldOrThin.clear(disable);
        }
        Self(enable | disable)
    }
}

// -------------------------------------------------------------------------------------

/// An iterator over formattings individual formats.
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.style"))]
#[derive(Debug)]
pub struct FormatIterator {
    formatting: Formatting,
    all_formats: AllFormats,
}

#[cfg(feature = "pyffi")]
impl FormatIterator {
    /// Access this iterator. <i class=python-only>Python only!</i>
    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Get the next item. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Format> {
        slf.next()
    }
}

impl Iterator for FormatIterator {
    type Item = Format;

    fn next(&mut self) -> Option<Self::Item> {
        // Keep iterating until we hit a format that is part of this formatting
        // or we run out of formats.
        loop {
            match self.all_formats.next() {
                None => return None,
                Some(format) => {
                    if format.test(self.formatting.0) {
                        return Some(format);
                    }
                }
            }
        }
    }
}

impl std::iter::FusedIterator for FormatIterator {}

// =====================================================================================

/// A style.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Style {
    Reset(),
    Text(Formatting),
    Foreground(TerminalColor),
    Background(TerminalColor),
    // ...
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Style {
    /// Determine the terminal fidelity necessary for rendering this style as
    /// is.
    ///
    /// This method necessarily return a fidelity higher than
    /// [`Fidelity::Plain`].
    pub fn fidelity(&self) -> Fidelity {
        match self {
            Style::Reset() => Fidelity::NoColor,
            Style::Text(_) => Fidelity::NoColor,
            Style::Foreground(color) => (*color).into(),
            Style::Background(color) => (*color).into(),
        }
    }
}

/// The definition of rich text.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RichText {
    styles: Vec<Style>,
    text: String,
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl RichText {
    pub fn fidelity(&self) -> Fidelity {
        self.styles
            .iter()
            .map(Style::fidelity)
            .max()
            .unwrap_or(Fidelity::Plain)
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}
