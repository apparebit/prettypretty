#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use super::Fidelity;

/// A text attribute other than regular.
///
/// This enumeration models attributes that differ from the default appearance.
/// Discriminants are powers of two and hence can be combined into a bit vector.
/// Bold and thin are mutually exclusive attributes and cancel each other out
/// when both are enabled.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, eq_int, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attribute {
    Bold = 0x1,
    Thin = 0x2,
    Italic = 0x4,
    Underlined = 0x8,
    Blinking = 0x10,
    Reversed = 0x20,
    Hidden = 0x40,
    Stricken = 0x80,
}

impl Attribute {
    #[inline]
    const fn bits(&self) -> u8 {
        *self as u8
    }

    const fn successor(&self) -> Option<Self> {
        use self::Attribute::*;

        Some(match self {
            Bold => Thin,
            Thin => Italic,
            Italic => Underlined,
            Underlined => Blinking,
            Blinking => Reversed,
            Reversed => Hidden,
            Hidden => Stricken,
            Stricken => return None,
        })
    }
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Attribute {
    /// Get the SGR parameter for enabling this format.
    pub const fn enable_sgr(&self) -> u8 {
        use self::Attribute::*;

        match self {
            Bold => 1,
            Thin => 2,
            Italic => 3,
            Underlined => 4,
            Blinking => 5,
            Reversed => 7,
            Hidden => 8,
            Stricken => 9,
        }
    }

    /// Get the SGR parameter for disabling this format.
    pub const fn disable_sgr(&self) -> u8 {
        use self::Attribute::*;

        match self {
            Bold => 22,
            Thin => 22,
            Italic => 23,
            Underlined => 24,
            Blinking => 25,
            Reversed => 27,
            Hidden => 28,
            Stricken => 29,
        }
    }

    /// Add this text attribute to another attribute, format, or format update.
    /// <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __add__(&self, other: &Bound<'_, PyAny>) -> Result<FormatUpdate, PyErr> {
        other
            .extract::<Attribute>()
            .map(|o| (*self + o).into())
            .or_else(|_| other.extract::<Format>().map(|o| (*self + o).into()))
            .or_else(|_| other.extract::<FormatUpdate>().map(|o| *self + o))
    }

    /// Subtract another attribute, format, or format update from this text
    /// attribute. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __sub__(&self, other: &Bound<'_, PyAny>) -> Result<FormatUpdate, PyErr> {
        other
            .extract::<Attribute>()
            .map(|o| *self - o)
            .or_else(|_| other.extract::<Format>().map(|o| *self - o))
            .or_else(|_| other.extract::<FormatUpdate>().map(|o| *self - o))
    }

    /// Negate this format. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __neg__(&self) -> FormatUpdate {
        -(*self)
    }

    /// Get a debug representation.  <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

// ----------------------------------------------------------------------------------------------------------

/// A text format combining zero or more text attributes.
///
/// There are two fundamentally different representations of a terminal's text
/// formatting. The first representation captures the *formatting state*, i.e.,
/// models only attributes that differ from the terminal's default appearance.
/// The second representation captures *formatting changes*, i.e., models
/// instructions for changing a terminal's appearance. This struct implements
/// the former representation, i.e., a *formatting state*.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Format(u8);

impl Format {
    // The bitmask for the mutually exclusive bold and thin attributes.
    const WEIGHT: u8 = Attribute::Bold.bits() | Attribute::Thin.bits();

    #[inline]
    const fn empty() -> Self {
        Self(0)
    }

    #[inline]
    const fn new(bits: u8) -> Self {
        if bits & Self::WEIGHT == Self::WEIGHT {
            Self(bits & !Self::WEIGHT)
        } else {
            Self(bits)
        }
    }

    #[inline]
    const fn with_sum(bits1: u8, bits2: u8) -> Self {
        Self::new(bits1 | bits2)
    }

    #[inline]
    const fn bits(&self) -> u8 {
        self.0
    }

    #[inline]
    const fn and_not(&self, other: Self) -> Self {
        Self::new(self.bits() & !other.bits())
    }
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Format {
    /// Get the empty, default format. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    #[new]
    pub fn py_new() -> Self {
        Self::default()
    }

    /// Create a new format from the formatting entity. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    #[pyo3(name = "of")]
    #[staticmethod]
    pub fn py_of(formatting: &Bound<'_, PyAny>) -> Result<Format, PyErr> {
        formatting
            .extract::<Attribute>()
            .map(|f| f.into())
            .or_else(|_| formatting.extract::<Format>())
    }

    /// Determine whether this format is the default format.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Get the number of format attributes that diverge from the default
    /// formatting. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __len__(&self) -> usize {
        self.len()
    }

    /// Get the number of format attributes that diverge from the default
    /// formatting.
    #[inline]
    pub const fn len(&self) -> usize {
        self.0.count_ones() as usize
    }

    /// Get an iterator over the non-default text attributes.
    #[inline]
    pub const fn attributes(&self) -> AttributeIter {
        AttributeIter {
            format: *self,
            cursor: None,
            remaining: self.len(),
        }
    }

    /// Add this formatting to the other value. <i class=python-only>Python
    /// only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __add__(&self, other: &Bound<'_, PyAny>) -> Result<FormatUpdate, PyErr> {
        other
            .extract::<Attribute>()
            .map(|o| (*self + o).into())
            .or_else(|_| other.extract::<Format>().map(|o| (*self + o).into()))
            .or_else(|_| other.extract::<FormatUpdate>().map(|o| *self + o))
    }

    /// Subtract the other value from this formatting. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __sub__(&self, other: &Bound<'_, PyAny>) -> Result<FormatUpdate, PyErr> {
        other
            .extract::<Attribute>()
            .map(|o| *self - o)
            .or_else(|_| other.extract::<Format>().map(|o| *self - o))
            .or_else(|_| other.extract::<FormatUpdate>().map(|o| *self - o))
    }

    /// Negate this formatting. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __neg__(&self) -> FormatUpdate {
        -(*self)
    }

    /// Generate a debug representation for this text format. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl std::fmt::Debug for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.attributes()).finish()
    }
}

// ----------------------------------------------------------------------------------------------------------

/// An iterator over text attributes.
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.style"))]
#[derive(Debug)]
pub struct AttributeIter {
    format: Format,
    cursor: Option<Attribute>,
    remaining: usize,
}

impl std::iter::Iterator for AttributeIter {
    type Item = Attribute;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let format = match self.cursor {
                None => Attribute::Bold,
                Some(Attribute::Stricken) => return None,
                Some(format) => format.successor().unwrap(),
            };
            self.cursor = Some(format);

            if self.format.bits() & format.bits() != 0 {
                self.remaining -= 1;
                return Some(format);
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for AttributeIter {
    fn len(&self) -> usize {
        self.remaining
    }
}

impl std::iter::FusedIterator for AttributeIter {}

#[cfg(feature = "pyffi")]
#[pymethods]
impl AttributeIter {
    /// Get the number of outstanding formats. <i class=python-only>Python
    /// only!</i>
    pub fn __len__(&self) -> usize {
        self.len()
    }

    /// Access this iterator. <i class=python-only>Python only!</i>
    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Get the next format. <i class=python-only>Python only!</i>
    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Attribute> {
        slf.next()
    }
}

// ----------------------------------------------------------------------------------------------------------

/// A format update comprising disabling and enabling formats.
///
/// There are two fundamentally different representations of a terminal's text
/// formatting. The first representation captures the *formatting state*, i.e.,
/// models only attributes that differ from the terminal's default appearance.
/// The second representation captures *formatting changes*, i.e., models
/// instructions for changing a terminal's appearance. This struct implements
/// the latter representation, i.e., a *formatting change*.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FormatUpdate {
    disable: Format,
    enable: Format,
}

impl FormatUpdate {
    /// Create a new empty format update in a const context.
    const fn empty() -> Self {
        Self {
            disable: Format::empty(),
            enable: Format::empty(),
        }
    }

    const fn new(disable: Format, enable: Format) -> Self {
        let (disable0, enable0) = (disable, enable);
        let mut disable = disable0.and_not(enable0);
        let enable = enable0.and_not(disable0);
        if enable.bits() & Format::WEIGHT != 0 {
            disable = Format::new(disable.bits() & !Format::WEIGHT);
        }

        Self { disable, enable }
    }
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl FormatUpdate {
    /// Create a new empty format update. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    #[new]
    pub fn py_new() -> Self {
        Self::default()
    }

    /// Create a new format update from the formatting entity. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    #[pyo3(name = "of")]
    #[staticmethod]
    pub fn py_of(formatting: &Bound<'_, PyAny>) -> Result<FormatUpdate, PyErr> {
        formatting
            .extract::<Attribute>()
            .map(|f| f.into())
            .or_else(|_| formatting.extract::<Format>().map(|f| f.into()))
            .or_else(|_| formatting.extract::<FormatUpdate>())
    }

    /// Determine whether this format update is empty, i.e., changes nothing.
    pub const fn is_empty(&self) -> bool {
        self.disable.is_empty() && self.enable.is_empty()
    }

    /// Get the formatting to be disabled.
    pub const fn disable(&self) -> Format {
        self.disable
    }

    /// Get the formatting to be enabled.
    pub const fn enable(&self) -> Format {
        self.enable
    }

    /// Cap this format by the given fidelity.
    ///
    /// This method returns this format, unless the fidelity is plain, in which
    /// case it returns an empty format.
    pub const fn cap(&self, fidelity: Fidelity) -> Self {
        match fidelity {
            Fidelity::Plain => Self::empty(),
            _ => *self,
        }
    }

    /// Add this format to the other value. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __add__(&self, other: &Bound<'_, PyAny>) -> Result<FormatUpdate, PyErr> {
        other
            .extract::<Attribute>()
            .map(|o| *self + o)
            .or_else(|_| other.extract::<Format>().map(|o| *self + o))
            .or_else(|_| other.extract::<FormatUpdate>().map(|o| *self + o))
    }

    /// Subtract the other value from this format update. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __sub__(&self, other: &Bound<'_, PyAny>) -> Result<FormatUpdate, PyErr> {
        other
            .extract::<Attribute>()
            .map(|o| *self - o)
            .or_else(|_| other.extract::<Format>().map(|o| *self - o))
            .or_else(|_| other.extract::<FormatUpdate>().map(|o| *self - o))
    }

    /// Negate this format. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __neg__(&self) -> FormatUpdate {
        -(*self)
    }

    /// Get a debug representation.  <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

// ----------------------------------------------------------------------------------------------------------
// From

impl From<Attribute> for Format {
    fn from(value: Attribute) -> Self {
        Self(value.bits())
    }
}

impl From<Attribute> for FormatUpdate {
    fn from(value: Attribute) -> Self {
        Self {
            disable: Format::default(),
            enable: value.into(),
        }
    }
}

impl From<Format> for FormatUpdate {
    fn from(value: Format) -> Self {
        Self {
            disable: Format::default(),
            enable: value,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------
// Add

impl std::ops::Add for Attribute {
    type Output = Format;

    fn add(self, other: Self) -> Self::Output {
        Format::with_sum(self.bits(), other.bits())
    }
}

impl std::ops::Add<Format> for Attribute {
    type Output = Format;

    fn add(self, other: Format) -> Self::Output {
        Format::with_sum(self.bits(), other.bits())
    }
}

impl std::ops::Add<FormatUpdate> for Attribute {
    type Output = FormatUpdate;

    fn add(self, other: FormatUpdate) -> Self::Output {
        FormatUpdate::new(other.disable, self + other.enable)
    }
}

impl std::ops::Add<Attribute> for Format {
    type Output = Format;

    fn add(self, other: Attribute) -> Self::Output {
        Format::with_sum(self.bits(), other.bits())
    }
}

impl std::ops::Add for Format {
    type Output = Format;

    fn add(self, other: Self) -> Self::Output {
        Format::with_sum(self.bits(), other.bits())
    }
}

impl std::ops::Add<FormatUpdate> for Format {
    type Output = FormatUpdate;

    fn add(self, other: FormatUpdate) -> Self::Output {
        FormatUpdate::new(other.disable, self + other.enable)
    }
}

impl std::ops::Add<Attribute> for FormatUpdate {
    type Output = FormatUpdate;

    fn add(self, other: Attribute) -> Self::Output {
        FormatUpdate::new(self.disable, self.enable + other)
    }
}

impl std::ops::Add<Format> for FormatUpdate {
    type Output = FormatUpdate;

    fn add(self, other: Format) -> Self::Output {
        FormatUpdate::new(self.disable, self.enable + other)
    }
}

impl std::ops::Add for FormatUpdate {
    type Output = FormatUpdate;

    fn add(self, other: Self) -> Self::Output {
        FormatUpdate::new(self.disable + other.disable, self.enable + other.enable)
    }
}

// ----------------------------------------------------------------------------------------------------------
// Neg

impl std::ops::Neg for Attribute {
    type Output = FormatUpdate;

    fn neg(self) -> Self::Output {
        FormatUpdate::new(self.into(), Format::default())
    }
}

impl std::ops::Neg for Format {
    type Output = FormatUpdate;

    fn neg(self) -> Self::Output {
        FormatUpdate::new(self, Format::default())
    }
}

impl std::ops::Neg for FormatUpdate {
    type Output = FormatUpdate;

    fn neg(self) -> Self::Output {
        FormatUpdate::new(self.enable, self.disable)
    }
}

// ----------------------------------------------------------------------------------------------------------
// Sub

impl std::ops::Sub for Attribute {
    type Output = FormatUpdate;

    fn sub(self, other: Self) -> Self::Output {
        FormatUpdate::new(other.into(), self.into())
    }
}

impl std::ops::Sub<Format> for Attribute {
    type Output = FormatUpdate;

    fn sub(self, other: Format) -> Self::Output {
        FormatUpdate::new(other, self.into())
    }
}

impl std::ops::Sub<FormatUpdate> for Attribute {
    type Output = FormatUpdate;

    fn sub(self, other: FormatUpdate) -> Self::Output {
        self + (-other)
    }
}

impl std::ops::Sub<Attribute> for Format {
    type Output = FormatUpdate;

    fn sub(self, other: Attribute) -> Self::Output {
        FormatUpdate::new(other.into(), self)
    }
}

impl std::ops::Sub for Format {
    type Output = FormatUpdate;

    fn sub(self, other: Self) -> Self::Output {
        FormatUpdate::new(other, self)
    }
}

impl std::ops::Sub<FormatUpdate> for Format {
    type Output = FormatUpdate;

    fn sub(self, other: FormatUpdate) -> Self::Output {
        self + (-other)
    }
}

impl std::ops::Sub<Attribute> for FormatUpdate {
    type Output = FormatUpdate;

    fn sub(self, other: Attribute) -> Self::Output {
        self + (-other)
    }
}

impl std::ops::Sub<Format> for FormatUpdate {
    type Output = FormatUpdate;

    fn sub(self, other: Format) -> Self::Output {
        self + (-other)
    }
}

impl std::ops::Sub for FormatUpdate {
    type Output = FormatUpdate;

    fn sub(self, other: Self) -> Self::Output {
        self + (-other)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_format_arithmetic() {
        use super::Attribute::*;

        let bold_underlined = Bold + Underlined;
        assert_eq!(bold_underlined.bits(), Bold.bits() | Underlined.bits());

        let thin_italic = Thin + Italic;
        assert_eq!(thin_italic, Italic + Thin);
        assert_eq!(thin_italic.bits(), Thin.bits() | Italic.bits());

        assert_eq!(bold_underlined + thin_italic, Underlined + Italic);
        assert_eq!(thin_italic + bold_underlined, Italic + Underlined);
        assert_eq!(Format::new(0), Format::default());
        assert_eq!(Bold + Thin, Format::default());
        assert_eq!(Thin + Bold, Format::default());
        assert_eq!(thin_italic + Bold, Format::from(Italic));
        assert_eq!(Bold + thin_italic, Format::from(Italic));

        let update1 = -Thin;
        assert_eq!(update1.disable(), Format::from(Thin));
        assert_eq!(update1.enable(), Format::default());

        let update2 = update1 + Bold;
        assert_eq!(update2.disable(), Format::default());
        assert_eq!(update2.enable(), Format::from(Bold));

        let update3 = Thin + Italic - update2;
        assert_eq!(update3.disable(), Format::default());
        assert_eq!(update3.enable(), Thin + Italic);

        assert_eq!(format!("{:?}", Bold + Underlined), "{Bold, Underlined}");
        assert_eq!(
            format!("{:?}", update3),
            "FormatUpdate { disable: {}, enable: {Thin, Italic} }"
        )
    }
}
