#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use super::{Fidelity, TerminalColor};

/// The enumeration of text format flags.
///
/// The first eight flags enable a format other than the terminal's default
/// appearance and the following eight flags, in the same order, disable the
/// corresponding format choices again. As a result, the flags to set and unset
/// a format are exactly eight bits apart. `FormatFlag` includes equivalent
/// `Regular` and `RegularToo` variants because `Bold` and `Thin` are not
/// independent from each other. Instead, they are mutually exclusive choices
/// for font weight.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum FormatFlag {
    Bold = 0x1,
    Thin = 0x2,
    Italic = 0x4,
    Underlined = 0x8,
    Blinking = 0x10,
    Reversed = 0x20,
    Hidden = 0x40,
    Stricken = 0x80,

    Regular = 0x100,
    RegularToo = 0x200,
    Upright = 0x400,
    NotUnderlined = 0x800,
    NotBlinking = 0x1000,
    NotReversed = 0x2000,
    NotHidden = 0x4000,
    NotStricken = 0x8000,
}

impl FormatFlag {
    /// Access the format flag's bits.
    const fn bits(&self) -> u16 {
        *self as u16
    }

    /// Set the format's flag's bits.
    const fn set(&self, value: u16) -> u16 {
        value | self.bits()
    }
}

use FormatFlag::*;
use GroupMask::*;

/// The bit masks for interrelated format flags.
#[derive(Copy, Clone, Debug)]
enum GroupMask {
    Weight = (Bold.bits() | Thin.bits() | Regular.bits() | RegularToo.bits()) as isize,
    Slant = (Italic.bits() | Upright.bits()) as isize,
    UnderlinedMask = (Underlined.bits() | NotUnderlined.bits()) as isize,
    BlinkingMask = (Blinking.bits() | NotBlinking.bits()) as isize,
    ReversedMask = (Reversed.bits() | NotReversed.bits()) as isize,
    HiddenMask = (Hidden.bits() | NotHidden.bits()) as isize,
    StrickenMask = (Stricken.bits() | NotStricken.bits()) as isize,
}

impl GroupMask {
    /// Access the format flag's bits.
    const fn bits(&self) -> u16 {
        *self as u16
    }

    /// Clear the value's group bits .
    const fn clear(&self, value: u16) -> u16 {
        value & !self.bits()
    }
}

/// Terminal text styles other than color.
///
/// This immutable struct efficiently encodes bold, thin, italic, underlined,
/// blinking, reversed, hidden, and stricken styles as well as the corresponding
/// defaults. It ensures that mutually exclusive styles, such as upright vs
/// italic text, are not present in the same format. You combine individual
/// styles by fluently invoking the corresponding methods.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Format(u16);

#[cfg_attr(feature = "pyffi", pymethods)]
impl Format {
    /// Create a new, empty format.
    #[cfg(feature = "pyffi")]
    #[new]
    pub fn new() -> Self {
        Self(0)
    }

    /// Create a new format like this one that also uses bold font weight.
    pub const fn bold(&self) -> Self {
        Self(Bold.set(Weight.clear(self.0)))
    }

    /// Create a new format like this one that also uses thin font weight.
    pub const fn thin(&self) -> Self {
        Self(Thin.set(Weight.clear(self.0)))
    }

    /// Create a new format like this one that also uses italic font slant.
    pub const fn italic(&self) -> Self {
        Self(Italic.set(Slant.clear(self.0)))
    }

    /// Create a new format like this one that also is underlined.
    pub const fn underlined(&self) -> Self {
        Self(Underlined.set(UnderlinedMask.clear(self.0)))
    }

    /// Create a new format like this one that also is blinking.
    pub const fn blinking(&self) -> Self {
        Self(Blinking.set(BlinkingMask.clear(self.0)))
    }

    /// Create a new format like this one that also is reversed.
    pub const fn reversed(&self) -> Self {
        Self(Reversed.set(ReversedMask.clear(self.0)))
    }

    /// Create a new format like this one that also is hidden.
    pub const fn hidden(&self) -> Self {
        Self(Hidden.set(HiddenMask.clear(self.0)))
    }

    /// Create a new format like this one that also is stricken.
    pub const fn stricken(&self) -> Self {
        Self(Stricken.set(StrickenMask.clear(self.0)))
    }

    /// Create a new format that restores the terminal's default appearance.
    pub const fn negate(&self) -> Self {
        Self(self.0 >> 8)
    }
}

#[cfg(not(feature = "pyffi"))]
impl Format {
    /// Create a new empty format.
    pub fn new() -> Self {
        Self(0)
    }
}

/// A style.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Style {
    Reset(),
    Format(Format),
    Foreground(TerminalColor),
    Background(TerminalColor),
    // ...
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
        let mut fidelity = Fidelity::Plain;

        for style in &self.styles {
            let f = match style {
                Style::Reset() => Fidelity::NoColor,
                Style::Format(_) => Fidelity::NoColor,
                Style::Foreground(color) => (*color).into(),
                Style::Background(color) => (*color).into(),
            };
            fidelity = fidelity.max(f)
        }

        fidelity
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}
