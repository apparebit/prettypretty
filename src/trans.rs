//! State and algorithms for the translation between high- and low-resolution
//! colors.

#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use crate::core::{is_achromatic_chroma_hue, parse_x};

use crate::error::{ColorFormatError, OutOfBoundsError};
use crate::style::{
    AnsiColor, Colorant, EightBitColor, EmbeddedRgb, Fidelity, GrayGradient, Layer,
};
use crate::{rgb, Bits, Color, ColorSpace, Float, OkVersion};

// ====================================================================================================================
// Color Themes
// ====================================================================================================================

/// An 18-entry slice with the color values for default and ANSI colors. <i
/// class=rust-only>Rust only!</i>
///
/// By now, a color theme is just an array with 18 colors. The implementation
/// started out as a more elaborate and encapsulated struct but ended up being
/// used just like a slice or vector. So, here we are.
///
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.trans"))]
#[derive(Clone, Debug)]
pub struct Theme {
    inner: [Color; 18],
}

impl Theme {
    /// Create a new theme with the default color.
    pub fn new() -> Self {
        Self {
            inner: <[Color; 18]>::default(),
        }
    }

    /// Create a new theme with the given colors.
    #[cfg(not(feature = "pyffi"))]
    pub const fn with_colors(inner: [Color; 18]) -> Self {
        Self { inner }
    }
}

#[cfg(feature = "pyffi")]
#[pymethods]
impl Theme {
    /// Create a new color theme with the given colors.
    #[new]
    pub const fn with_colors(inner: [Color; 18]) -> Self {
        Self { inner }
    }

    /// Get the color for the given theme entry.
    pub fn __getitem__(&self, index: ThemeEntry) -> Color {
        self[index].clone()
    }

    /// Set the color for the given theme entry.
    pub fn __setitem__(&mut self, index: ThemeEntry, value: Color) {
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
pub const VGA_COLORS: Theme = Theme::with_colors([
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

// ====================================================================================================================
// Hue and Lightness Table
// ====================================================================================================================

/// A gray ANSI color and its concrete lightness value.
#[derive(Debug)]
struct GrayEntry {
    spec: AnsiColor,
    lr: Float,
}

impl GrayEntry {
    /// Create a new gray entry.
    ///
    /// This associated function returns `None` if the ANSI color or its color
    /// value is not gray.
    fn new(spec: AnsiColor, value: &Color) -> Option<GrayEntry> {
        let [lr, c, h] = *value.to(ColorSpace::Oklrch).as_ref();
        if !spec.is_achromatic()
            || !is_achromatic_chroma_hue(c, h, HueLightnessTable::ACHROMATIC_THRESHOLD)
        {
            return None;
        }

        Some(GrayEntry { spec, lr })
    }

    /// Get a key suitable for an ordering comparison.
    fn key(&self) -> Bits {
        // Conversion in new() normalizes lr to a number.
        self.lr.to_bits()
    }
}

/// A non-gray ANSI color and its concrete chroma and hue.
#[derive(Debug)]
struct ColorEntry {
    spec: AnsiColor,
    lr: Float,
    h: Float,
}

impl ColorEntry {
    /// Create a new color entry.
    ///
    /// This associated function returns `None` if the ANSI color or its
    /// concrete color value is gray.
    fn new(spec: AnsiColor, value: &Color) -> Option<Self> {
        let [lr, c, mut h] = *value.to(ColorSpace::Oklrch).as_ref();
        if spec.is_achromatic()
            || is_achromatic_chroma_hue(c, h, HueLightnessTable::ACHROMATIC_THRESHOLD)
        {
            return None;
        }
        h = h.rem_euclid(360.0); // Critical for correctness!

        Some(ColorEntry { spec, lr, h })
    }

    /// Get the 3-bit base color.
    fn base(&self) -> AnsiColor {
        self.spec.to_base()
    }
}

/// A table for matching by hue and lightness.
///
/// A hue and lightness table must observe the following invariants:
///
///   * The floating point fields of all entries are numbers.
///   * The gray entries represent grays. That applies to abstract ANSI colors
///     and concrete coordinates alike.
///   * The color entries represent colors, not grays. That applies to abstract
///     ANSI colors and concrete coordinates alike.
///   * The chroma of color entries must be non-zero. (Otherwise, they'd be
///     grays.)
///   * When traversing the hue circle counter-clockwise, the order of abstract
///     ANSI colors is red, yellow, green, cyan, blue, and magenta.
///
/// Note that the last invariant allows for hues to be rotated out of their
/// usual position and does not restrict the relative order between regular and
/// bright versions of the same abstract color.
///
/// Also note that the constructor returns `None` if the theme colors do not
/// observe the invariants, with exception of the first one on floating point
/// values, which is automatically observed.
#[derive(Debug)]
struct HueLightnessTable {
    grays: Vec<GrayEntry>,
    colors: Vec<ColorEntry>,
}

impl HueLightnessTable {
    const ACHROMATIC_THRESHOLD: Float = 0.05;

    /// Create a new hue lightness table.
    ///
    /// This associated function returns `None` if the theme colors violate any
    /// of the invariants.
    fn new(theme: &Theme) -> Option<HueLightnessTable> {
        // Prep the grays
        let mut grays = Vec::with_capacity(4);
        for index in [0_usize, 7, 8, 15] {
            let index_color = AnsiColor::try_from(index as u8).unwrap();
            grays.push(GrayEntry::new(index_color, &theme[index_color])?);
        }
        grays.sort_by_key(|entry| entry.key());

        // Prep the non-grays in hue order: red, yellow, green, cyan, blue, magenta.
        let mut colors = Vec::with_capacity(12);
        for index in [1_usize, 3, 2, 6, 4, 5] {
            let index_color = AnsiColor::try_from(index as u8).unwrap();
            let regular = ColorEntry::new(index_color, &theme[index_color])?;
            let index_color = AnsiColor::try_from(index as u8 + 8).unwrap();
            let bright = ColorEntry::new(index_color, &theme[index_color])?;

            // Order each color pair by hue
            if regular.h <= bright.h {
                colors.push(regular);
                colors.push(bright);
            } else {
                colors.push(bright);
                colors.push(regular);
            }
        }

        // Find entry with smallest hue
        let mut min_hue = Float::MAX;
        let mut min_index = usize::MAX;
        for (index, entry) in colors.iter().enumerate() {
            if entry.h < min_hue {
                min_hue = entry.h;
                min_index = index;
            }
        }

        // Rotate entry with smallest hue into first position.
        if 0 < min_index {
            colors.rotate_left(min_index);
        }

        // We added the each regular/bright pair by smaller hue first. So if
        // pairs are in standard order, all hues are sorted as well.
        min_hue = -1.0;
        for entry in colors.iter() {
            if entry.h < min_hue {
                return None;
            }
            min_hue = entry.h;
        }

        Some(HueLightnessTable { grays, colors })
    }

    /// Find matching color.
    ///
    /// For grays, this method finds the ANSI gray with the closest lightness.
    /// For colors, this method first finds the pair of regular and bright
    /// abstract ANSI colors with the closest hue and then selects the one with
    /// the closest lightness.
    fn find_match(&self, color: &Color) -> AnsiColor {
        let [lr, c, h] = *color.to(ColorSpace::Oklrch).as_ref();

        // Select gray index by lr only. Not that there is anything else to go by...
        if is_achromatic_chroma_hue(c, h, Self::ACHROMATIC_THRESHOLD) {
            for index in 0..(self.grays.len() - 1) {
                let entry1 = &self.grays[index];
                let entry2 = &self.grays[index + 1];

                // The midpoint between grays serves as boundary.
                if lr < entry1.lr + (entry2.lr - entry1.lr) / 2.0 {
                    return entry1.spec;
                }
            }
            return self.grays[self.grays.len() - 1].spec;
        }

        // Select pair of color versions by hue and then pick one by lightness.
        // Humans are less sensitive to chroma, so ignoring it seems reasonable.
        let length = self.colors.len();
        for index in 0..length {
            // We are looking for the first entry with a larger hue.
            let next_entry = &self.colors[index];
            if next_entry.h < h && (index != 0 || h < self.colors[length - 1].h) {
                // The first interval starts with the last color.
                continue;
            }

            // (index - 1) is unsafe, but (index + length - 1) isn't. Go rem, go!
            let previous_entry = &self.colors[(index + length - 1).rem_euclid(length)];
            if previous_entry.base() == next_entry.base() {
                // Hue is bracketed by versions of same color.
                let result = self.pick_lightness(lr, previous_entry, next_entry);
                return result;
            }

            // We need previous_hue < h <= next_hue to determine closer one.
            let mut previous_hue = previous_entry.h;
            let next_hue = next_entry.h;
            if h < previous_hue {
                assert!(index == 0);
                previous_hue -= 360.0
            }

            // Pick closer color pair.
            if h - previous_hue <= next_hue - h {
                // Hue is closer to previous color
                let twice_previous_entry = &self.colors[(index + length - 2).rem_euclid(length)];
                return self.pick_lightness(lr, twice_previous_entry, previous_entry);
            } else {
                // Hue is closer to next color
                let twice_next_entry = &self.colors[(index + 1).rem_euclid(length)];
                return self.pick_lightness(lr, next_entry, twice_next_entry);
            }
        }

        unreachable!();
    }

    /// Use lightness to pick an entry's ANSI color.
    fn pick_lightness(&self, lr: Float, entry1: &ColorEntry, entry2: &ColorEntry) -> AnsiColor {
        if (entry1.lr - lr).abs() <= (entry2.lr - lr).abs() {
            entry1.spec
        } else {
            entry2.spec
        }
    }
}

// ====================================================================================================================
// Translator
// ====================================================================================================================

/// A color translator.
///
/// Instances of this struct translate between [`Color`] and other color
/// representations. They also maintain the state for doing so efficiently. The
/// [user
/// guide](https://apparebit.github.io/prettypretty/overview/integration.html)
/// includes a detailed discussion of challenges posed by translation, solution
/// approaches, and this struct's interface.
///
/// Since a translator incorporates theme colors, an application should
/// regenerate its translator if the current theme changes.
///
/// [`Style`](crate::style::Style) uses a translator instance to cap styles.
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.trans"))]
pub struct Translator {
    /// The theme colors. For converting *to* high-resolution colors.
    theme: Theme,
    /// The table for matching by hue and lightness.
    hue_lightness_table: Option<HueLightnessTable>,
    /// The color space for the ANSI and 8-bit color coordinates.
    space: ColorSpace,
    /// The ANSI color coordinates for matching to closest color.
    ansi: [[Float; 3]; 16],
    /// The 8-bit color coordinates for matching to closest color.
    eight_bit: [[Float; 3]; 256],
}

/// Create the coordinates for the 16 extended ANSI colors in the given color
/// space.
fn ansi_coordinates(space: ColorSpace, theme: &Theme) -> [[Float; 3]; 16] {
    let mut coordinates: [[Float; 3]; 16] = [[0.0; 3]; 16];
    for index in AnsiColor::all() {
        coordinates[index as usize] = *theme[index].to(space).as_ref();
    }

    coordinates
}

/// Create the coordinates for the 8-bit colors in the given color space.
#[allow(clippy::needless_range_loop)]
fn eight_bit_coordinates(space: ColorSpace, theme: &Theme) -> [[Float; 3]; 256] {
    let mut coordinates: [[Float; 3]; 256] = [[0.0; 3]; 256];
    for index in AnsiColor::all() {
        coordinates[index as usize] = *theme[index].to(space).as_ref();
    }
    for index in 16..=231 {
        // Unwrap is safe b/c we are iterating over EmbeddedRgb's index range.
        coordinates[index] = *Color::from(EmbeddedRgb::try_from(index as u8).unwrap())
            .to(space)
            .as_ref();
    }
    for index in 232..=255 {
        // Unwrap is safe b/c we are iterating over GrayGradient's index range.
        coordinates[index] = *Color::from(GrayGradient::try_from(index as u8).unwrap())
            .to(space)
            .as_ref();
    }

    coordinates
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Translator {
    /// Create a new translator for the given Oklab version and theme colors.
    #[cfg(feature = "pyffi")]
    #[new]
    pub fn new(version: OkVersion, theme: Theme) -> Self {
        let hue_lightness_table = HueLightnessTable::new(&theme);
        let space = version.cartesian_space();
        let ansi = ansi_coordinates(space, &theme);
        let eight_bit = eight_bit_coordinates(space, &theme);

        Self {
            theme,
            hue_lightness_table,
            space,
            ansi,
            eight_bit,
        }
    }

    /// Determine whether this translator's color theme is a dark theme.
    ///
    /// The Y component of a color in XYZ represents it luminance. This method
    /// exploits that property of XYZ and checks whether the default foreground
    /// color has a larger luminance than the default background color.
    pub fn is_dark_theme(&self) -> bool {
        let yf = self.theme[Layer::Foreground].to(ColorSpace::Xyz)[1];
        let yb = self.theme[Layer::Background].to(ColorSpace::Xyz)[1];
        yb < yf
    }

    /// Resolve a colorant other than the default to a high-resolution color. <i
    /// class=python-only>Python only!</i>
    ///
    /// This method is exposed as `resolve` in Python. It uses a custom
    /// conversion function for [`Colorant`]s and hence accepts any color as is.
    /// The one exception is the default color, see below.
    ///
    ///
    /// # Panics
    ///
    /// If the colorant is [`Colorant::Default`]. Use
    /// [`Translator::py_resolve_all`] if the default colorant needs to be
    /// resolved, too.
    #[cfg(feature = "pyffi")]
    #[pyo3(name = "resolve")]
    pub fn py_resolve(
        &self,
        #[pyo3(from_py_with = "crate::style::into_colorant")] colorant: Colorant,
    ) -> Color {
        self.resolve(colorant)
    }

    /// Resolve any colorant to a high-resolution color. <i
    /// class=python-only>Python only!</i>
    ///
    /// This method is exposed as `resolve_all` in Python. It uses a custom
    /// conversion function for [`Colorant`]s and hence accepts any color as is.
    /// If the colorant is guaranteed not to be [`Colorant::Default`],
    /// [`Translator::resolve`] does not require a layer argument.
    #[cfg(feature = "pyffi")]
    #[pyo3(name = "resolve_all")]
    pub fn py_resolve_all(
        &self,
        #[pyo3(from_py_with = "crate::style::into_colorant")] colorant: Colorant,
        layer: Layer,
    ) -> Color {
        self.resolve_all(colorant, layer)
    }

    /// Convert the high-resolution color into an ANSI color.
    ///
    /// If the current theme meets the requirements for hue/lightness search,
    /// this method forwards to [`Translator::to_ansi_hue_lightness`].
    /// Otherwise, it falls back on [`Translator::to_closest_ansi`]. Use
    /// [`Translator::supports_hue_lightness`] to test whether the current theme
    /// supports hue-lightness search.
    pub fn to_ansi(&self, color: &Color) -> AnsiColor {
        self.to_ansi_hue_lightness(color)
            .unwrap_or_else(|| self.to_closest_ansi(color))
    }

    /// Determine whether this translator instance supports color translation
    /// with the hue/lightness search algorithm.
    pub fn supports_hue_lightness(&self) -> bool {
        self.hue_lightness_table.is_some()
    }

    /// Convert the high-resolution color to ANSI based on Oklab's hue (h) and
    /// revised lightness (Lr).
    ///
    /// This method performs all color comparisons in the cylindrical version of
    /// the revised Oklab color space. For grays, it finds the ANSI gray with
    /// the closest revised lightness. For colors, this method first finds the
    /// pair of regular and bright ANSI colors with the closest hue and then
    /// selects the color with the closest lightness.
    ///
    /// This method requires that concrete theme colors and abstract ANSI colors
    /// are (loosely) aligned. Notably, the color values for pairs of regular
    /// and bright ANSI colors must be in order red, yellow, green, cyan, blue,
    /// and magenta when traversing hues counter-clockwise, i.e., with
    /// increasing hue magnitude. Note that this does allow hues to be
    /// arbitrarily shifted along the circle. Furthermore, it does not prescribe
    /// an order for regular and bright versions of the same abstract ANSI
    /// color. If the theme colors passed to this translator's constructor did
    /// not meet this requirement, this method returns `None`.
    ///
    /// # Examples
    ///
    /// The documentation for [`Translator::to_closest_ansi`] gives the example
    /// of two colors that yield subpar results with an exhaustive search for
    /// the closest color and then sketches an alternative approach that
    /// searches for the closest hue.
    ///
    /// The algorithm implemented by this method goes well beyond that sketch by
    /// not only leveraging color pragmatics (i.e., their coordinates) but also
    /// their semantics. Hence, it first searches for one out of six pairs of
    /// regular and bright ANSI colors with the closest hue and then picks the
    /// one out of two colors with the closest lightness.
    ///
    /// As this example illustrates, that strategy works well for the light
    /// orange colors from [`Translator::to_closest_ansi`]. They both match the
    /// yellow pair by hue and then bright yellow by lightness. Alas, it is no
    /// panacea because color themes may not observe the necessary semantic
    /// constraints. This method detects such cases and returns `None`.
    /// [`Translator::to_ansi`] instead automatically falls back onto searching
    /// for the closest color.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace, OkVersion};
    /// # use prettypretty::style::AnsiColor;
    /// # use prettypretty::trans::{Translator, VGA_COLORS};
    /// # use prettypretty::error::ColorFormatError;
    /// # use std::str::FromStr;
    /// let translator = Translator::new(
    ///     OkVersion::Revised, VGA_COLORS.clone());
    ///
    /// let orange1 = Color::from_str("#ffa563")?;
    /// let ansi = translator.to_ansi_hue_lightness(&orange1);
    /// assert_eq!(ansi.unwrap(), AnsiColor::BrightYellow);
    ///
    /// let orange2 = Color::from_str("#ff9600")?;
    /// let ansi = translator.to_ansi_hue_lightness(&orange2);
    /// assert_eq!(ansi.unwrap(), AnsiColor::BrightYellow);
    /// # Ok::<(), ColorFormatError>(())
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #ffa563;"></div>
    /// <div style="background-color: #ffff55;"></div>
    /// <div style="background-color: #ff9600;"></div>
    /// <div style="background-color: #ffff55;"></div>
    /// </div>
    pub fn to_ansi_hue_lightness(&self, color: &Color) -> Option<AnsiColor> {
        self.hue_lightness_table
            .as_ref()
            .map(|t| t.find_match(color))
    }

    /// Find the ANSI color that comes closest to the given color.
    ///
    /// # Examples
    ///
    /// The example code below matches the shades of orange `#ffa563` and
    /// `#ff9600` to ANSI colors under the default VGA theme in both Oklab and
    /// Oklrab. In both versions of the color space, the first orange
    /// consistently matches ANSI white and the second orange consistently
    /// matches bright red. Visually, the second match seems reasonable given
    /// that there are at most 12 colors and 4 grays to pick from. But the first
    /// match seems off. Gray simply isn't a satisfactory replacement for a
    /// (more or less) saturated color.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace, OkVersion};
    /// # use prettypretty::style::AnsiColor;
    /// # use prettypretty::trans::{Translator, VGA_COLORS};
    /// # use prettypretty::error::ColorFormatError;
    /// # use std::str::FromStr;
    /// let original_translator = Translator::new(
    ///     OkVersion::Original, VGA_COLORS.clone());
    ///
    /// let orange1 = Color::from_str("#ffa563")?;
    /// let ansi = original_translator.to_closest_ansi(&orange1);
    /// assert_eq!(ansi, AnsiColor::White);
    ///
    /// let orange2 = Color::from_str("#ff9600")?;
    /// let ansi = original_translator.to_closest_ansi(&orange2);
    /// assert_eq!(ansi, AnsiColor::BrightRed);
    /// // ---------------------------------------------------------------------
    /// let revised_translator = Translator::new(
    ///     OkVersion::Revised, VGA_COLORS.clone());
    ///
    /// let ansi = revised_translator.to_closest_ansi(&orange1);
    /// assert_eq!(ansi, AnsiColor::White);
    ///
    /// let ansi = revised_translator.to_closest_ansi(&orange2);
    /// assert_eq!(ansi, AnsiColor::BrightRed);
    /// # Ok::<(), ColorFormatError>(())
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #ffa563;"></div>
    /// <div style="background-color: #aaaaaa;"></div>
    /// <div style="background-color: #ff9600;"></div>
    /// <div style="background-color: #ff5555;"></div>
    /// <div style="background-color: #ffa563;"></div>
    /// <div style="background-color: #aaaaaa;"></div>
    /// <div style="background-color: #ff9600;"></div>
    /// <div style="background-color: #ff5555;"></div>
    /// </div>
    /// <br>
    ///
    /// That isn't just my subjective judgement, but human color perception is
    /// more sensitive to changes in hue than chroma or lightness. By that
    /// standard, the match actually is pretty poor. To see that, consider the
    /// figure below showing the chroma/hue plane. It plots the 12 ANSI colors
    /// (as circles), the 4 ANSI grays (as one circle with averaged lightness),
    /// and the 2 orange tones (as narrow diamonds) on that plane (hence the
    /// 12+4+2 in the title). As it turns out, `#ffa563` is located right next
    /// to the default theme's ANSI yellow, which really is a dark orange or
    /// brown. The primary difference between the two colors are neither chroma
    /// (0.13452 vs 0.1359) nor hue (55.6 vs 54.1) but lightness only (0.79885
    /// vs 0.54211). Depending on the use case, the theme's yellow may be an
    /// acceptable match. Otherwise the bright red probably is a better match
    /// than the achromatic gray.
    ///
    /// ![The colors plotted on Oklab's chroma and hue plane](https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/vga-colors.svg)
    ///
    /// Reflecting that same observation about color perception, the [CSS Color
    /// 4](https://www.w3.org/TR/css-color-4/#gamut-mapping) gamut-mapping
    /// algorithm improves on MINDE algorithms (Minimum Delta-E) such as this
    /// method's closest match in Oklab by systematically reducing chroma and
    /// tolerating small lightness and hue variations (caused by clipping).
    /// Given the extremely limited color repertoire, we can't use a similar,
    /// directed search. But we should do better than brute-force search.
    ///
    /// Let's explore that idea a little further. Since the revised lightness is
    /// more accurate, we'll be comparing colors in Oklrch. We start by
    /// preparing a list with the color values for the 16 extended ANSI colors
    /// in that color space. That, by the way, is pretty much what
    /// [`Translator::new`] does as well.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// # use prettypretty::style::AnsiColor;
    /// # use prettypretty::trans::VGA_COLORS;
    /// # use prettypretty::error::ColorFormatError;
    /// # use std::str::FromStr;
    /// let ansi_colors: Vec<Color> = AnsiColor::all()
    ///     .map(|c| VGA_COLORS[c].to(ColorSpace::Oklrch))
    ///     .collect();
    /// ```
    ///
    /// [`VGA_COLORS`] is a builtin color [`Theme`] that maps the default
    /// foreground, default background, and ANSI colors to high-resolution
    /// colors. It conveniently can be indexed by ANSI colors.
    ///
    /// Next, we need a function that calculates the distance between the
    /// coordinates of two colors in Oklrch. Since we are exploring non-MINDE
    /// approaches, we focus on hue alone and use the minimum degree of
    /// separation as a metric. Degrees being circular, computing the remainder
    /// of the difference is not enough. We need to consider both differences.
    ///
    /// The function uses prettypretty's [`Float`], which serves as alias to
    /// either `f64` (the default) or `f32` (when the `f32` feature is enabled).
    ///
    /// ```
    /// use prettypretty::Float;
    /// fn minimum_degrees_of_separation(c1: &[Float; 3], c2: &[Float; 3]) -> Float {
    ///     (c1[2] - c2[2]).rem_euclid(360.0)
    ///         .min((c2[2] - c1[2]).rem_euclid(360.0))
    /// }
    /// ```
    ///
    /// That's it. We have everything we need. All that's left to do is to
    /// instantiate the same orange again and find the closest matching color on
    /// our list with the new distance metric.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace, Float};
    /// # use prettypretty::style::AnsiColor;
    /// # use prettypretty::trans::VGA_COLORS;
    /// # use prettypretty::error::ColorFormatError;
    /// # use std::str::FromStr;
    /// # let ansi_colors: Vec<Color> = AnsiColor::all()
    /// #     .map(|c| VGA_COLORS[c].to(ColorSpace::Oklrch))
    /// #     .collect();
    /// # fn minimum_degrees_of_separation(c1: &[Float; 3], c2: &[Float; 3]) -> Float {
    /// #     (c1[2] - c2[2]).rem_euclid(360.0)
    /// #         .min((c2[2] - c1[2]).rem_euclid(360.0))
    /// # }
    /// let orange = Color::from_str("#ffa563")?;
    /// let closest = orange.find_closest(
    ///     &ansi_colors,
    ///     ColorSpace::Oklrch,
    ///     minimum_degrees_of_separation,
    /// ).unwrap();
    /// assert_eq!(closest, 3);
    /// # Ok::<(), ColorFormatError>(())
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #ffa563;"></div>
    /// <div style="background-color: #a50;"></div>
    /// </div>
    /// <br>
    ///
    /// The hue-based comparison picks ANSI color 3, VGA's orange yellow, just
    /// as expected. It appears that our hue-based proof-of-concept works.
    /// However, a production-ready version does need to account for lightness,
    /// too. The method to do so is [`Translator::to_ansi_hue_lightness`].
    pub fn to_closest_ansi(&self, color: &Color) -> AnsiColor {
        use crate::core::{delta_e_ok, find_closest};

        let color = color.to(self.space);
        find_closest(color.as_ref(), &self.ansi, delta_e_ok)
            .map(|idx| AnsiColor::try_from(idx as u8).unwrap())
            .unwrap()
    }

    /// Convert the high-resolution color to an ANSI color in RGB.
    ///
    /// This method performs a conversion from high-resolution color to ANSI
    /// color solely based on linear RGB coordinates. Since the ANSI colors
    /// essentially are 3-bit RGB colors with an additional bit for brightness,
    /// it converts the given color to linear sRGB, clipping out of gamut
    /// coordinates, and then rounds each coordinate to 0 or 1. It determines
    /// whether to set the brightness bit based on a heuristically weighted sum
    /// of the individual coordinates.
    ///
    /// The above algorithm uses *linear* sRGB because gamma-corrected sRGB, by
    /// definition, skews the coordinate space and hence is ill-suited to
    /// manipulation based on component magnitude. Alas, that is a common
    /// mistake.
    ///
    /// While the algorithm does seem a bit odd, it is an improved version of
    /// the approach implemented by
    /// [Chalk](https://github.com/chalk/chalk/blob/main/source/vendor/ansi-styles/index.js),
    /// only one of the most popular terminal color libraries for JavaScript.
    pub fn to_ansi_rgb(&self, color: &Color) -> AnsiColor {
        let color = color.to(ColorSpace::LinearSrgb).clip();
        let [r, g, b] = color.as_ref();
        let mut index = ((b.round() as u8) << 2) + ((g.round() as u8) << 1) + (r.round() as u8);
        // When we get to the threshold below, the color has already been
        // selected and can only be brightened. A threshold of 2 or 3 produces
        // the least bad results. In any case, prettypretty.grid's output shows
        // large striped rectangles, with 4x24 cells black/blue and 2x24 cells
        // green/cyan above 4x12 cells red/magenta and 2x12 cells yellow/white.
        if 3 <= index {
            index += 8;
        }

        AnsiColor::try_from(index).unwrap()
    }

    /// Find the 8-bit color that comes closest to the given color.
    ///
    /// This method only compares to embedded RGB and gray gradient colors, not
    /// ANSI colors. That is because the ANSI colors can be visually disruptive
    /// when using several, graduated colors. For that reason, prefer this
    /// method over [`Translator::to_closest_8bit_with_ansi`].
    ///
    ///
    /// # Examples
    ///
    /// The example below converts every color of the RGB cube embedded in 8-bit
    /// colors to a high-resolution color in sRGB, which is validated by the
    /// first two assertions, and then uses a translator to convert that color
    /// back to an embedded RGB color. The result is the original color, now
    /// wrapped as a colorant, which is validated by the third assertion. The
    /// example demonstrates that the 216 colors in the embedded RGB cube still
    /// are closest to themselves after conversion to Oklrch.
    ///
    /// ```
    /// # use prettypretty::{assert_close_enough, Color, ColorSpace, Float, OkVersion};
    /// # use prettypretty::error::OutOfBoundsError;
    /// # use prettypretty::style::{EightBitColor, EmbeddedRgb};
    /// # use prettypretty::trans::{Translator, VGA_COLORS};
    /// let translator = Translator::new(OkVersion::Revised, VGA_COLORS.clone());
    ///
    /// for r in 0..5 {
    ///     for g in 0..5 {
    ///         for b in 0..5 {
    ///             let embedded = EmbeddedRgb::new(r, g, b)?;
    ///             let color = Color::from(embedded);
    ///             assert_eq!(color.space(), ColorSpace::Srgb);
    ///
    ///             let c1 = if r == 0 {
    ///                 0.0
    ///             } else {
    ///                 (55.0 + 40.0 * (r as Float)) / 255.0
    ///             };
    ///             assert_close_enough!(color[0], c1);
    ///
    ///             let result = translator.to_closest_8bit(&color);
    ///             assert_eq!(
    ///                 result,
    ///                 EightBitColor::Embedded(embedded)
    ///             );
    ///         }
    ///     }
    /// }
    /// # Ok::<(), OutOfBoundsError>(())
    /// ```
    pub fn to_closest_8bit(&self, color: &Color) -> EightBitColor {
        use crate::core::{delta_e_ok, find_closest};

        let color = color.to(self.space);
        let index = find_closest(
            color.as_ref(),
            self.eight_bit.last_chunk::<240>().unwrap(),
            delta_e_ok,
        )
        .map(|idx| idx as u8 + 16)
        .unwrap();

        EightBitColor::from(index)
    }

    /// Find the 8-bit color that comes closest to the given color.
    ///
    /// This method comparse *all* 8-bit colors including ANSI colors. Prefer to
    /// use [`Translator::to_closest_8bit`] instead, which produces better
    /// results when converting several graduated colors.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace, Float, OkVersion};
    /// # use prettypretty::style::{AnsiColor, EightBitColor, EmbeddedRgb};
    /// # use prettypretty::trans::{Translator, VGA_COLORS};
    /// let bright_magenta = Color::from_24bit(255, 85, 255);
    /// let translator = Translator::new(OkVersion::Revised, VGA_COLORS.clone());
    /// let result = translator.to_closest_8bit_with_ansi(&bright_magenta);
    /// assert_eq!(result, EightBitColor::Ansi(AnsiColor::BrightMagenta));
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: rgb(255, 85, 255);"></div>
    /// </div>
    pub fn to_closest_8bit_with_ansi(&self, color: &Color) -> EightBitColor {
        use crate::core::{delta_e_ok, find_closest};

        let color = color.to(self.space);
        let index = find_closest(color.as_ref(), &self.eight_bit, delta_e_ok).unwrap() as u8;

        EightBitColor::from(index)
    }

    /// Cap the high-resolution color by the given fidelity.
    ///
    /// This method borrows the high-resolution color and clones the color only
    /// in the uncommon case that the fidelity is high-resolution. For that
    /// reason, prefer this method over [`Translator::cap`] when capping known
    /// high-resolution colors.
    pub fn cap_hires(&self, color: &Color, fidelity: Fidelity) -> Option<Colorant> {
        match fidelity {
            Fidelity::Plain | Fidelity::NoColor => None,
            Fidelity::Ansi => Some(Colorant::Ansi(self.to_ansi(color))),
            Fidelity::EightBit => Some(self.to_closest_8bit(color).into()),
            Fidelity::TwentyFourBit => Some(Colorant::Rgb(color.into())),
            Fidelity::HiRes => Some(Colorant::HiRes(color.clone())),
        }
    }

    /// Cap the colorant by the given fidelity.
    ///
    /// This method borrows the colorant. It only clones colorants when no
    /// conversion is necessary and it needs to return the colorant wrapped as
    /// an option. As a result, it only clones a high-resolution color in the
    /// uncommon case that the fidelity level also is high-resolution. For that
    /// reason, prefer this method over [`Translator::cap`] when capping known
    /// colorants.
    pub fn cap_colorant(&self, colorant: &Colorant, fidelity: Fidelity) -> Option<Colorant> {
        match fidelity {
            Fidelity::Plain | Fidelity::NoColor => None,
            Fidelity::Ansi => {
                let hires_color = match colorant {
                    Colorant::Default() | Colorant::Ansi(_) => return Some(colorant.clone()),
                    Colorant::Embedded(embedded_rgb) => &Color::from(embedded_rgb),
                    Colorant::Gray(gray_gradient) => &Color::from(gray_gradient),
                    Colorant::Rgb(true_color) => &Color::from(true_color),
                    Colorant::HiRes(hires_color) => hires_color,
                };

                Some(Colorant::Ansi(self.to_ansi(hires_color)))
            }
            Fidelity::EightBit => {
                let hires_color = match colorant {
                    Colorant::Rgb(true_color) => &Color::from(true_color),
                    Colorant::HiRes(ref hires_color) => hires_color,
                    _ => return Some(colorant.clone()),
                };

                Some(self.to_closest_8bit(hires_color).into())
            }
            Fidelity::TwentyFourBit => {
                if let Colorant::HiRes(ref hires_color) = colorant {
                    Some(Colorant::Rgb(hires_color.into()))
                } else {
                    Some(colorant.clone())
                }
            }
            Fidelity::HiRes => Some(colorant.clone()),
        }
    }

    /// Cap the colorant by the given fidelity. <i class=python-only>Python
    /// only!</i>
    ///
    /// This method is exposed as `cap` in Python. It ensures that that a
    /// terminal with the fidelity level can render the resulting color as
    /// follows:
    ///
    ///   * `Plain`, `NoColor` (fidelity)
    ///       * `None` (result)
    ///   * `Ansi`
    ///       * Unmodified ANSI colors
    ///       * Downsampled 8-bit, 24-bit, and high-resolution colors
    ///   * `EightBit`
    ///       * Unmodified ANSI and 8-bit colors
    ///       * Downsampled 24-bit and high-resolution colors
    ///   * `TwentyFourBit`
    ///       * Unmodified ANSI, 8-bit, and 24-bit colors
    ///       * Downsampled high-resolution colors
    ///   * `HiRes`
    ///       * Unmodified colors
    ///
    /// To achieve parity with [`Translator::cap`], this method uses a custom
    /// type conversion.
    #[cfg(feature = "pyffi")]
    #[pyo3(name = "cap")]
    pub fn py_cap(
        &self,
        #[pyo3(from_py_with = "crate::style::into_colorant")] colorant: Colorant,
        fidelity: Fidelity,
    ) -> Option<Colorant> {
        self.cap(colorant, fidelity)
    }

    /// Return a debug representation. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[cfg(not(feature = "pyffi"))]
impl Translator {
    /// Create a new translator for the given Oklab version and theme colors.
    pub fn new(version: OkVersion, theme: Theme) -> Self {
        let hue_lightness_table = HueLightnessTable::new(&theme);
        let space = version.cartesian_space();
        let ansi = ansi_coordinates(space, &theme);
        let eight_bit = eight_bit_coordinates(space, &theme);

        Self {
            theme,
            hue_lightness_table,
            space,
            ansi,
            eight_bit,
        }
    }
}

impl Translator {
    /// Resolve a colorant other than the default to a high-resolution color.
    ///
    ///
    /// # Panics
    ///
    /// If the colorant is [`Colorant::Default`]. If the colorant may include
    /// the default colorant, use [`Translator::resolve_all`] instead.
    pub fn resolve(&self, color: impl Into<Colorant>) -> Color {
        let color = color.into();
        if matches!(color, Colorant::Default()) {
            panic!("Translator::resolve() cannot process the default colorant; use Translator::resolve_all() instead.")
        }
        self.resolve_all(color, Layer::Foreground)
    }

    /// Resolve any colorant to a high-resolution color.
    ///
    ///
    /// # Examples
    ///
    /// Thanks to Rust's `Into<Colorant>` trait, callers need not wrap their
    /// ANSI, embedded RGB, gray gradient, and true colors before calling this
    /// method. The Python version uses a custom type conversion function to
    /// achieve the same effect.
    ///
    /// ```
    /// # use prettypretty::{Color, OkVersion};
    /// # use prettypretty::style::{AnsiColor, Colorant, Layer, TrueColor};
    /// # use prettypretty::trans::{Translator, VGA_COLORS};
    /// let translator = Translator::new(OkVersion::Revised, VGA_COLORS.clone());
    /// let blue = translator.resolve_all(
    ///     AnsiColor::Blue, Layer::Foreground);
    /// assert_eq!(blue, Color::srgb(0.0, 0.0, 0.666666666666667));
    ///
    /// let black = translator.resolve_all(
    ///     Colorant::Default(), Layer::Foreground);
    /// assert_eq!(black, Color::srgb(0.0, 0.0, 0.0));
    ///
    /// let maroon = translator.resolve_all(
    ///     TrueColor::new(148, 23, 81), Layer::Background);
    /// assert_eq!(maroon, Color::srgb(
    ///     0.5803921568627451, 0.09019607843137255, 0.3176470588235294
    /// ));
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #0000aa;"></div>
    /// <div style="background-color: #000000;"></div>
    /// <div style="background-color: #941751;"></div>
    /// </div>
    pub fn resolve_all(&self, color: impl Into<Colorant>, layer: Layer) -> Color {
        match color.into() {
            Colorant::Default() => self.theme[layer].clone(),
            Colorant::Ansi(c) => self.theme[c].clone(),
            Colorant::Embedded(c) => c.into(),
            Colorant::Gray(c) => c.into(),
            Colorant::Rgb(c) => c.into(),
            Colorant::HiRes(c) => c,
        }
    }

    /// Cap the colorant by the given fidelity.
    ///
    /// This method ensures that that a terminal with the fidelity level
    /// can render the resulting color as follows:
    ///
    ///   * `Plain`, `NoColor` (fidelity)
    ///       * `None` (result)
    ///   * `Ansi`
    ///       * Unmodified ANSI colors
    ///       * Downsampled 8-bit, 24-bit, and high-resolution colors
    ///   * `EightBit`
    ///       * Unmodified ANSI and 8-bit colors
    ///       * Downsampled 24-bit and high-resolution colors
    ///   * `TwentyFourBit`
    ///       * Unmodified ANSI, 8-bit, and 24-bit colors
    ///       * Downsampled high-resolution colors
    ///   * `HiRes`
    ///       * Unmodified colors
    ///
    /// The Rust-only implementation uses an `impl` `Into` trait as color
    /// argument so that it can be invoked with ANSI, embedded RGB, gray
    /// gradient, and true colors without prior conversion. The version exposed
    /// to Python uses a custom type conversion function to provide the exact
    /// same capability.
    ///
    /// Instead of calling this method, whenever possible, Rust code should use
    /// [`Translator::cap_hires`] or [`Translator::cap_colorant`].
    pub fn cap(&self, colorant: impl Into<Colorant>, fidelity: Fidelity) -> Option<Colorant> {
        self.cap_colorant(&colorant.into(), fidelity)
    }
}

impl std::fmt::Debug for Translator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let version = if self.space == ColorSpace::Oklab {
            "OkVersion.Original"
        } else {
            "OkVersion.Revised"
        };

        f.write_fmt(format_args!("Translator({}, [\n", version))?;
        for color in self.theme.as_ref().iter() {
            f.write_fmt(format_args!("    {:?},\n", color))?;
        }

        f.write_str("])")
    }
}

// ====================================================================================================================

#[cfg(test)]
mod test {
    use super::{ThemeEntry, Translator, VGA_COLORS};
    use crate::error::OutOfBoundsError;
    use crate::style::AnsiColor;
    use crate::{Color, OkVersion};

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

    #[test]
    fn test_translator() -> Result<(), OutOfBoundsError> {
        let translator = Translator::new(OkVersion::Revised, VGA_COLORS.clone());

        let result = translator.to_closest_ansi(&Color::srgb(1.0, 1.0, 0.0));
        assert_eq!(result, AnsiColor::BrightYellow);

        Ok(())
    }
}
