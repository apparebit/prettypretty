#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use super::Colorant;

/// A text format other than regular.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, eq_int, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Bold = 0x1,
    Thin = 0x2,
    Italic = 0x4,
    Underlined = 0x8,
    Blinking = 0x10,
    Reversed = 0x20,
    Hidden = 0x40,
    Stricken = 0x80,
}

impl Format {
    #[inline]
    const fn bits(&self) -> u8 {
        *self as u8
    }

    const fn successor(&self) -> Option<Self> {
        use self::Format::*;

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
impl Format {
    /// Get the SGR parameter for enabling this format.
    pub const fn enable(&self) -> u8 {
        use self::Format::*;

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
    pub const fn disable(&self) -> u8 {
        use self::Format::*;

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

    /// Add this format to the other value. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __add__(&self, other: &Bound<'_, PyAny>) -> Result<FormatUpdate, PyErr> {
        other
            .extract::<Format>()
            .map(|o| (*self + o).into())
            .or_else(|_| other.extract::<Formatting>().map(|o| (*self + o).into()))
            .or_else(|_| other.extract::<FormatUpdate>().map(|o| *self + o))
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

/// The combination of zero or more formats.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Formatting(u8);

impl Formatting {
    const WEIGHT: u8 = Format::Bold.bits() | Format::Thin.bits();

    /// The empty default formatting.
    pub const NONE: Formatting = Formatting(0);

    #[inline]
    const fn new(bits: u8) -> Self {
        if bits & Self::WEIGHT == Self::WEIGHT {
            Self(bits & !Self::WEIGHT)
        } else {
            Self(bits)
        }
    }

    #[inline]
    const fn sum(bits1: u8, bits2: u8) -> Self {
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
impl Formatting {
    /// Get the empty, default formatting.
    #[cfg(feature = "pyffi")]
    #[pyo3(name = "default")]
    #[staticmethod]
    pub fn py_default() -> Self {
        Self::default()
    }

    /// Determine whether this formatting is just regular formatting.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Get the number of formats that diverge from regular formatting.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.count_ones() as usize
    }

    /// Get the only format.
    #[inline]
    pub fn as_format(&self) -> Option<Format> {
        if self.0.count_ones() == 1 {
            self.formats().next()
        } else {
            None
        }
    }

    /// Get an iterator over the formats.
    #[inline]
    pub fn formats(&self) -> FormatIter {
        FormatIter {
            format: *self,
            cursor: None,
            remaining: self.len(),
        }
    }

    /// Add this format to the other value. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __add__(&self, other: &Bound<'_, PyAny>) -> Result<FormatUpdate, PyErr> {
        other
            .extract::<Format>()
            .map(|o| (*self + o).into())
            .or_else(|_| other.extract::<Formatting>().map(|o| (*self + o).into()))
            .or_else(|_| other.extract::<FormatUpdate>().map(|o| *self + o))
    }

    /// Negate this formatting. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __neg__(&self) -> FormatUpdate {
        -(*self)
    }
}

impl std::fmt::Debug for Formatting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.formats()).finish()
    }
}

// ----------------------------------------------------------------------------------------------------------

/// An iterator over formats.
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.style"))]
#[derive(Debug)]
pub struct FormatIter {
    format: Formatting,
    cursor: Option<Format>,
    remaining: usize,
}

impl std::iter::Iterator for FormatIter {
    type Item = Format;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let format = match self.cursor {
                None => Format::Bold,
                Some(Format::Stricken) => return None,
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

impl ExactSizeIterator for FormatIter {
    fn len(&self) -> usize {
        self.remaining
    }
}

impl std::iter::FusedIterator for FormatIter {}

#[cfg(feature = "pyffi")]
#[pymethods]
impl FormatIter {
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
    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Format> {
        slf.next()
    }
}

// ----------------------------------------------------------------------------------------------------------

/// A format update comprising disabling and enabling formats.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.style")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FormatUpdate {
    disable: Formatting,
    enable: Formatting,
}

impl FormatUpdate {
    fn new(disable: Formatting, enable: Formatting) -> Self {
        let (disable0, enable0) = (disable, enable);
        let mut disable = disable0.and_not(enable0);
        let enable = enable0.and_not(disable0);
        if enable.bits() & Formatting::WEIGHT != 0 {
            disable = Formatting::new(disable.bits() & !Formatting::WEIGHT);
        }

        Self { disable, enable }
    }
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl FormatUpdate {
    /// Get the disabled formatting.
    pub fn disable(&self) -> Formatting {
        self.disable
    }

    /// Get the enabled formatting.
    pub fn enable(&self) -> Formatting {
        self.enable
    }

    /// Get as disabled formatting.
    pub fn as_disable(&self) -> Option<Formatting> {
        if self.enable.is_empty() {
            Some(self.disable)
        } else {
            None
        }
    }

    /// Get as enabled formatting.
    pub fn as_enable(&self) -> Option<Formatting> {
        if self.disable.is_empty() {
            Some(self.enable)
        } else {
            None
        }
    }

    /// Add this format to the other value. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __add__(&self, other: &Bound<'_, PyAny>) -> Result<FormatUpdate, PyErr> {
        other
            .extract::<Format>()
            .map(|o| *self + o)
            .or_else(|_| other.extract::<Formatting>().map(|o| *self + o))
            .or_else(|_| other.extract::<FormatUpdate>().map(|o| *self + o))
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

impl From<Format> for Formatting {
    fn from(value: Format) -> Self {
        Self(value.bits())
    }
}

impl From<Format> for FormatUpdate {
    fn from(value: Format) -> Self {
        Self {
            disable: Formatting::default(),
            enable: value.into(),
        }
    }
}

impl From<Formatting> for FormatUpdate {
    fn from(value: Formatting) -> Self {
        Self {
            disable: Formatting::default(),
            enable: value,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------
// Add

impl std::ops::Add for Format {
    type Output = Formatting;

    fn add(self, other: Self) -> Self::Output {
        Formatting::sum(self.bits(), other.bits())
    }
}

impl std::ops::Add<Formatting> for Format {
    type Output = Formatting;

    fn add(self, other: Formatting) -> Self::Output {
        Formatting::sum(self.bits(), other.bits())
    }
}

impl std::ops::Add<FormatUpdate> for Format {
    type Output = FormatUpdate;

    fn add(self, other: FormatUpdate) -> Self::Output {
        FormatUpdate::new(other.disable, self + other.enable)
    }
}

impl std::ops::Add<Format> for Formatting {
    type Output = Formatting;

    fn add(self, other: Format) -> Self::Output {
        Formatting::sum(self.bits(), other.bits())
    }
}

impl std::ops::Add for Formatting {
    type Output = Formatting;

    fn add(self, other: Self) -> Self::Output {
        Formatting::sum(self.bits(), other.bits())
    }
}

impl std::ops::Add<FormatUpdate> for Formatting {
    type Output = FormatUpdate;

    fn add(self, other: FormatUpdate) -> Self::Output {
        FormatUpdate::new(other.disable, self + other.enable)
    }
}

impl std::ops::Add<Format> for FormatUpdate {
    type Output = FormatUpdate;

    fn add(self, other: Format) -> Self::Output {
        FormatUpdate::new(self.disable, self.enable + other)
    }
}

impl std::ops::Add<Formatting> for FormatUpdate {
    type Output = FormatUpdate;

    fn add(self, other: Formatting) -> Self::Output {
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

impl std::ops::Neg for Format {
    type Output = FormatUpdate;

    fn neg(self) -> Self::Output {
        FormatUpdate::new(self.into(), Formatting::default())
    }
}

impl std::ops::Neg for Formatting {
    type Output = FormatUpdate;

    fn neg(self) -> Self::Output {
        FormatUpdate::new(self, Formatting::default())
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

impl std::ops::Sub for Format {
    type Output = FormatUpdate;

    fn sub(self, other: Self) -> Self::Output {
        FormatUpdate::new(other.into(), self.into())
    }
}

impl std::ops::Sub<Formatting> for Format {
    type Output = FormatUpdate;

    fn sub(self, other: Formatting) -> Self::Output {
        FormatUpdate::new(other, self.into())
    }
}

impl std::ops::Sub<FormatUpdate> for Format {
    type Output = FormatUpdate;

    fn sub(self, other: FormatUpdate) -> Self::Output {
        self + (-other)
    }
}

impl std::ops::Sub<Format> for Formatting {
    type Output = FormatUpdate;

    fn sub(self, other: Format) -> Self::Output {
        FormatUpdate::new(other.into(), self)
    }
}

impl std::ops::Sub for Formatting {
    type Output = FormatUpdate;

    fn sub(self, other: Self) -> Self::Output {
        FormatUpdate::new(other, self)
    }
}

impl std::ops::Sub<FormatUpdate> for Formatting {
    type Output = FormatUpdate;

    fn sub(self, other: FormatUpdate) -> Self::Output {
        self + (-other)
    }
}

impl std::ops::Sub<Format> for FormatUpdate {
    type Output = FormatUpdate;

    fn sub(self, other: Format) -> Self::Output {
        self + (-other)
    }
}

impl std::ops::Sub<Formatting> for FormatUpdate {
    type Output = FormatUpdate;

    fn sub(self, other: Formatting) -> Self::Output {
        self + (-other)
    }
}

impl std::ops::Sub for FormatUpdate {
    type Output = FormatUpdate;

    fn sub(self, other: Self) -> Self::Output {
        self + (-other)
    }
}

// ----------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct Style {
    format: Option<Formatting>,
    foreground: Option<Colorant>,
    background: Option<Colorant>,
}

impl Style {
    pub fn with_format(&self, format: impl Into<Formatting>) -> Self {
        Self {
            format: Some(format.into()),
            foreground: self.foreground.clone(),
            background: self.background.clone(),
        }
    }

    pub fn with_foreground(&self, color: impl Into<Colorant>) -> Self {
        Self {
            format: self.format,
            foreground: Some(color.into()),
            background: self.background.clone(),
        }
    }

    pub fn with_background(&self, color: impl Into<Colorant>) -> Self {
        Self {
            format: self.format,
            foreground: self.foreground.clone(),
            background: Some(color.into()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.format.is_none() && self.foreground.is_none() && self.background.is_none()
    }

    pub fn format(&self) -> Option<Formatting> {
        self.format
    }

    pub fn foreground(&self) -> &Option<Colorant> {
        &self.foreground
    }

    pub fn background(&self) -> &Option<Colorant> {
        &self.background
    }
}

// ----------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use crate::style::EmbeddedRgb;

    use super::*;

    #[test]
    fn test_format_arithmetic() {
        use super::Format::*;

        let bold_underlined = Bold + Underlined;
        assert_eq!(bold_underlined.bits(), Bold.bits() | Underlined.bits());

        let thin_italic = Thin + Italic;
        assert_eq!(thin_italic, Italic + Thin);
        assert_eq!(thin_italic.bits(), Thin.bits() | Italic.bits());

        assert_eq!(bold_underlined + thin_italic, Underlined + Italic);
        assert_eq!(thin_italic + bold_underlined, Italic + Underlined);
        assert_eq!(Formatting::NONE, Formatting::default());
        assert_eq!(Bold + Thin, Formatting::default());
        assert_eq!(Thin + Bold, Formatting::default());
        assert_eq!(thin_italic + Bold, Formatting::from(Italic));
        assert_eq!(Bold + thin_italic, Formatting::from(Italic));
        assert_eq!((Bold + thin_italic).as_format(), Some(Italic));

        let update1 = -Thin;
        assert_eq!(update1.disable(), Formatting::from(Thin));
        assert_eq!(update1.enable(), Formatting::default());
        assert_eq!(update1.as_disable(), Some(Formatting::from(Thin)));

        let update2 = update1 + Bold;
        assert_eq!(update2.disable(), Formatting::default());
        assert_eq!(update2.enable(), Formatting::from(Bold));
        assert_eq!(update2.as_enable(), Some(Formatting::from(Bold)));

        let update3 = Thin + Italic - update2;
        assert_eq!(update3.disable(), Formatting::default());
        assert_eq!(update3.enable(), Thin + Italic);
        assert_eq!(update3.as_enable(), Some(Thin + Italic));

        assert_eq!(format!("{:?}", Bold + Underlined), "{Bold, Underlined}");
        assert_eq!(
            format!("{:?}", update3),
            "FormatUpdate { disable: {}, enable: {Thin, Italic} }"
        )
    }

    #[test]
    fn test_style() {
        use super::Format::*;

        let style = Style::default();
        assert_eq!(style.format(), None);
        assert_eq!(style.foreground(), &None);
        assert_eq!(style.background(), &None);

        let style = style.with_format(Bold + Underlined);
        assert_eq!(style.format(), Some(Bold + Underlined));
        assert_eq!(style.foreground(), &None);
        assert_eq!(style.background(), &None);

        let style = style.with_foreground(EmbeddedRgb::new(5, 3, 1).unwrap());
        assert_eq!(style.format(), Some(Bold + Underlined));
        assert_eq!(
            style.foreground(),
            &Some(Colorant::Embedded(EmbeddedRgb::new(5, 3, 1).unwrap()))
        );
        assert_eq!(style.background(), &None);
    }
}
