//! Utility module for stylistic attributes of text other than color.
//!
//! Thanks to the fluent assembly of [`Style`](crate::style::Style)s with
//! [`stylist`](crate::style::stylist()), chances are that you don't need to
//! directly access this module's types.
//!
//! But in case that you do: A text [`Format`] is a collection of [`Attribute`]s
//! that represent the appearance of terminal output. A text format also is a
//! collection of changes to text attributes, which either enable or disable the
//! attribute. Likewise, an [`Attribute`] is an atomic unit for formatting a
//! terminal's text output and also an update to the formatting. Furthermore,
//! [`AllAttributes`] iterates over all possible attributes, whereas
//! [`AttributeIterator`] iterates over a format's attributes.
//!
//! To avoid a proliferation of formatting-related data structures, format and
//! attribute can be used as both state and state-change. However, the public
//! API for fluently constructing arbitrary formats is limited to

use std::iter::ExactSizeIterator;

#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use super::Fidelity;
use Attribute::*;

/// Text attributes other than colors.
///
/// This enum includes two variants for each text attribute that diverges from
/// the default appearance, one to enable the attribute and one to disable it
/// again.
///
/// Names for the disabling variants start with `Not`. Enabling variants are
/// sorted before disabling variants, with corresponding enabling/disabling
/// variants in the same order. `Bold` and `Thin` are mutually exclusive and
/// hence share the same disabling variant `NotBoldOrThin`.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, eq_int, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Attribute {
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
impl Attribute {
    /// Get an iterator over all text attributes.
    pub fn all() -> AllAttributes {
        AllAttributes(0)
    }
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Attribute {
    /// Get an iterator over all text attributes.
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn all() -> AllAttributes {
        AllAttributes(0)
    }

    /// Get the attribute's human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            Bold => "bold",
            Thin => "thin",
            Italic => "italic",
            Underlined => "underlined",
            Blinking => "blinking",
            Reversed => "reversed",
            Hidden => "hidden",
            Stricken => "stricken",
            NotBoldOrThin => "neither bold nor thin",
            NotItalic => "not italic",
            NotUnderlined => "not underlined",
            NotBlinking => "not blinking",
            NotReversed => "not reversed",
            NotHidden => "not hidden",
            NotStricken => "not stricken",
        }
    }

    /// Get the SGR parameter for this text attribute.
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

    /// Get the flag bit corresponding to this text attribute.
    #[inline]
    const fn bits(&self) -> u16 {
        *self as u16
    }

    /// Test whether the value's flag bit for this text attribute is set.
    #[inline]
    const fn test(&self, value: u16) -> bool {
        value & (*self as u16) != 0
    }

    /// Clear the value's flag bit for this text attribute.
    #[inline]
    const fn clear(&self, value: u16) -> u16 {
        value & !(*self as u16)
    }

    /// Set the value's flag bit for this text attribute.
    #[inline]
    const fn set(&self, value: u16) -> u16 {
        value | (*self as u16)
    }
}

// -------------------------------------------------------------------------------------

/// An iterator over all text attributes.
///
/// This iterator adheres to the canonical text attribute order, which places
/// enabling variants before disabling variants and orders each disabling
/// variant amongst other disabling variants the same as it orders the
/// corresponding enabling variant.
#[cfg_attr(feature = "pyffi", pyclass)]
#[derive(Debug)]
pub struct AllAttributes(u8);

#[cfg_attr(feature = "pyffi", pymethods)]
impl AllAttributes {
    /// Drain this iterator.
    pub fn drain(&mut self) {
        loop {
            if self.next().is_none() {
                return;
            }
        }
    }

    /// Get the number of attributes still to be yielded. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __len__(&self) -> usize {
        self.len()
    }

    /// Access this iterator. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Get the next text attribute. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Attribute> {
        slf.next()
    }

    /// Get a debug representation for this iterator. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl Iterator for AllAttributes {
    type Item = Attribute;

    fn next(&mut self) -> Option<Self::Item> {
        let attr = match self.0 {
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

        Some(attr)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = 15 - self.0 as usize;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for AllAttributes {
    #[inline]
    fn len(&self) -> usize {
        15 - self.0 as usize
    }
}

impl std::iter::FusedIterator for AllAttributes {}

// -------------------------------------------------------------------------------------

/// Masks for related text attributes.
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
///
/// There are two fundamentally different representations of a terminal's text
/// formatting. The first representation captures the *formatting state*, i.e.,
/// models only attributes that differ from the terminal's default appearance.
/// The second representation captures *formatting changes*, i.e., models
/// instructions for changing a terminal's appearance. Both representations are
/// closely related, since the difference between two formatting states is a
/// formatting change and applying a formatting change to a terminal replaces
/// its old formatting state with a new one. Finally, for every text attribute
/// of a formatting state, a formatting change has options to enable or disable
/// the attribute, with the same option possibly disabling more than one
/// attribute.
///
/// Exposing the formatting state to users is very much preferable because they
/// are concerned with the results, i.e., the terminal appearance, and not the
/// commands required for configuring the terminal accordingly. But
/// prettypretty's implementation necessarily makes formatting changes and ANSI
/// escape codes also embody formatting changes. To avoid a proliferation of
/// formatting-related data structures, this struct reflects a hybrid approach.
/// While it is based on format changes, its public interface only supports the
/// fluent enabling of text attributes that differ from the default appearance.
/// Yet negation and subtraction may very well result in formats that also
/// disable text attributes. The implementation does ensure that at most one out
/// of a group of mutually exclusive attributes is set. Hence, any format can
/// have at most seven attributes.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Format(u16);

impl Format {
    /// Create a new, empty format.
    #[inline]
    pub const fn new() -> Self {
        Self(0)
    }
}

/// Negate a format's bits.
#[inline]
pub(crate) const fn negate_bits(value: u16) -> u16 {
    // Turn Thin into NotBoldOrThin, which is Bold << 8.
    if Thin.test(value) {
        Bold.set(Thin.clear(value)) << 8
    } else {
        value << 8
    }
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Format {
    /// Create a new, empty format.
    #[cfg(feature = "pyffi")]
    #[new]
    #[inline]
    pub const fn py_new() -> Self {
        Self(0)
    }

    /// Create a new format like this one that also uses bold font weight.
    #[inline]
    pub const fn bold(&self) -> Self {
        Self(Bold.set(Mask::Weight.clear(self.0)))
    }

    /// Create a new format like this one that also uses thin font weight.
    #[inline]
    pub const fn thin(&self) -> Self {
        Self(Thin.set(Mask::Weight.clear(self.0)))
    }

    /// Create a new format like this one that also uses italic font slant.
    #[inline]
    pub const fn italic(&self) -> Self {
        Self(Italic.set(Mask::Slant.clear(self.0)))
    }

    /// Create a new format like this one that also is underlined.
    #[inline]
    pub const fn underlined(&self) -> Self {
        Self(Underlined.set(Mask::Underlined.clear(self.0)))
    }

    /// Create a new format like this one that also is blinking.
    #[inline]
    pub const fn blinking(&self) -> Self {
        Self(Blinking.set(Mask::Blinking.clear(self.0)))
    }

    /// Create a new format like this one that also is reversed.
    #[inline]
    pub const fn reversed(&self) -> Self {
        Self(Reversed.set(Mask::Reversed.clear(self.0)))
    }

    /// Create a new format like this one that also is hidden.
    #[inline]
    pub const fn hidden(&self) -> Self {
        Self(Hidden.set(Mask::Hidden.clear(self.0)))
    }

    /// Create a new format like this one that also is stricken.
    #[inline]
    pub const fn stricken(&self) -> Self {
        Self(Stricken.set(Mask::Stricken.clear(self.0)))
    }

    /// Determine whether this format includes the given text attribute.
    #[inline]
    pub const fn has(&self, attr: Attribute) -> bool {
        attr.test(self.0)
    }

    /// Get the number of attributes in this format, which is at most seven.
    #[inline]
    pub fn attribute_count(&self) -> u8 {
        self.0.count_ones() as u8
    }

    /// Get an iterator over the constituent text attributes.
    ///
    /// [`Format::attribute_count`] returns the number of attributes yielded by
    /// this iterator, which is between 0 and seven (inclusive).
    pub fn attributes(&self) -> AttributeIterator {
        AttributeIterator {
            format: *self,
            remaining: self.attribute_count(),
            all_attributes: Attribute::all(),
        }
    }

    /// Cap this format by the given terminal fidelity.
    ///
    /// If the terminal supports ANSI escape codes, i.e., has a fidelity other
    /// than [`Fidelity::Plain`], this method returns the format wrapped in
    /// `Some`. Otherwise, it returns `None`.
    pub fn cap(&self, fidelity: Fidelity) -> Option<Self> {
        match fidelity {
            Fidelity::Plain => None,
            _ => Some(*self),
        }
    }

    /// Get the SGR parameters for this format.
    ///
    /// Note that the [`Format::attribute_count`] also is the number of SGR
    /// parameters required for this format.
    pub fn sgr_parameters(&self) -> Vec<u8> {
        self.attributes().map(|a| a.sgr_parameter()).collect()
    }

    /// Negate this format. <i class=python-only>Python only!</i>
    ///
    /// If a terminal uses this format, the negated format restores the
    /// terminal's default appearance again.
    #[cfg(feature = "pyffi")]
    pub fn __invert__(&self) -> Self {
        !*self
    }

    /// Determine the difference between this and another format. <i
    /// class=python-only>Python only!</i>
    ///
    /// If a terminal uses the other format, the returned difference changes the
    /// terminal's format to this one. The returned difference is minimal.
    #[cfg(feature = "pyffi")]
    pub fn __sub__(&self, other: &Self) -> Self {
        *self - *other
    }

    /// Convert this format to its debug representation. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl std::ops::Not for Format {
    type Output = Self;

    /// Negate this format.
    ///
    /// If a terminal uses this format, the negated format restores the
    /// terminal's default appearance.
    #[inline]
    fn not(self) -> Self::Output {
        Self(negate_bits(self.0))
    }
}

impl std::ops::Sub for Format {
    type Output = Self;

    /// Determine the difference between this and another format.
    ///
    /// If a terminal uses the other format, the returned difference changes the
    /// terminal's format from the appearance of the other format to this one.
    /// The returned difference is minimal.
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

/// An iterator over the attributes contributing to a format.
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.style"))]
#[derive(Debug)]
pub struct AttributeIterator {
    format: Format,
    remaining: u8,
    all_attributes: AllAttributes,
}

#[cfg(feature = "pyffi")]
#[pymethods]
impl AttributeIterator {
    /// Get the number of remaining attributes. <i class=python-only>Python
    /// only!</i>
    pub fn __len__(&self) -> usize {
        self.len()
    }

    /// Access this iterator. <i class=python-only>Python only!</i>
    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Get the next attribute. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Attribute> {
        slf.next()
    }
}

impl Iterator for AttributeIterator {
    type Item = Attribute;

    fn next(&mut self) -> Option<Self::Item> {
        // Keep iterating until we hit an attribute that is part of the format
        // or we run out of attributes.
        loop {
            match self.all_attributes.next() {
                None => return None,
                Some(attr) => {
                    if self.format.has(attr) {
                        self.remaining -= 1;
                        return Some(attr);
                    }
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining as usize, Some(self.remaining as usize))
    }
}

impl ExactSizeIterator for AttributeIterator {
    #[inline]
    fn len(&self) -> usize {
        self.remaining as usize
    }
}

impl std::iter::FusedIterator for AttributeIterator {}

// =====================================================================================

#[cfg(test)]
mod test {
    use super::{
        negate_bits, Blinking, Bold, Format, NotBlinking, NotBoldOrThin, NotUnderlined, Thin,
        Underlined,
    };

    #[test]
    fn test_attribute() {
        let mut value: u16 = 0;

        assert!(!Underlined.test(value));
        value = Underlined.set(value);
        assert!(Underlined.test(value));
        value = negate_bits(value);
        assert_eq!(value, NotUnderlined.bits());
        value = Underlined.clear(0xffff);
        assert_eq!(value, 0xffff & !Underlined.bits());
    }

    #[test]
    fn test_format() {
        // Format #1: {Bold, Underlined}
        let format = Format::new().thin().bold().underlined();
        assert_eq!(format.attribute_count(), 2);
        assert!(!format.has(Thin));
        assert!(format.has(Bold));
        assert!(format.has(Underlined));

        let attributes = format.attributes();
        assert_eq!(attributes.len(), 2);

        for attr in attributes {
            assert!(matches!(attr, Bold | Underlined));
        }

        // Format 2: Not{Bold, Underlined} + Blinking
        let format = (!format).blinking();
        assert_eq!(format.attribute_count(), 3);
        assert!(format.has(Blinking));
        assert!(format.has(NotBoldOrThin));
        assert!(format.has(NotUnderlined));

        let attributes = format.attributes();
        assert_eq!(attributes.len(), 3);

        for attr in attributes {
            assert!(matches!(attr, Blinking | NotBoldOrThin | NotUnderlined));
        }

        // Format 3: Not{Not{Bold, Underlined} + Blinking}
        let format = !format;
        assert_eq!(format.attribute_count(), 1);
        assert!(format.has(NotBlinking));

        let attributes = format.attributes();
        assert_eq!(attributes.len(), 1);

        for attr in attributes {
            assert!(matches!(attr, NotBlinking));
        }
    }
}
