#[cfg(feature = "pyffi")]
use pyo3::prelude::*;
#[cfg(feature = "pyffi")]
use pyo3::types::PyInt;

use crate::core::is_gray_chroma_hue;

use crate::{
    rgb, AnsiColor, Bits, Color, ColorSpace, DefaultColor, EmbeddedRgb, Fidelity, Float,
    GrayGradient, OkVersion, OutOfBoundsError, TerminalColor,
};

// ====================================================================================================================
// Color Themes
// ====================================================================================================================

/// An 18-entry slice with the color values for default and ANSI colors.
///
/// By now, a color theme is just an array with 18 colors. The implementation
/// started out as a more elaborate and encapsulated struct but ended up being
/// used just like a slice or vector. So, here we are.
pub type Theme = [Color; 18];

/// A helper for naming the slots of color themes.
///
/// This enumeration combines the default and ANSI colors to identify the 18
/// entries of a color theme in order.
#[cfg_attr(feature = "pyffi", pyclass(eq, frozen, hash, ord))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ThemeEntry {
    Default(DefaultColor),
    Ansi(AnsiColor),
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl ThemeEntry {
    /// Try getting the theme entry for the given index.
    #[cfg(feature = "pyffi")]
    #[staticmethod]
    pub fn try_from_index(value: usize) -> Result<ThemeEntry, OutOfBoundsError> {
        ThemeEntry::try_from(value)
    }

    /// Get this theme entry's human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Default(color) => color.name(),
            Self::Ansi(color) => color.name(),
        }
    }
}

impl From<DefaultColor> for ThemeEntry {
    fn from(value: DefaultColor) -> Self {
        ThemeEntry::Default(value)
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
            Ok(ThemeEntry::Default(DefaultColor::Foreground))
        } else if value == 1 {
            Ok(ThemeEntry::Default(DefaultColor::Background))
        } else if value <= 17 {
            Ok(ThemeEntry::Ansi(
                AnsiColor::try_from(value as u8 - 2).unwrap(),
            ))
        } else {
            Err(OutOfBoundsError::new(value, 0..=17))
        }
    }
}

/// A helper for iterating over the slot names of theme entries.
///
/// This iterator is fused, i.e., after returning `None` once, it will keep
/// returning `None`. This iterator also is exact, i.e., its `size_hint()`
/// returns the exact number of remaining items.
#[cfg_attr(feature = "pyffi", pyclass)]
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
        if self.index >= 18 {
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

impl std::iter::FusedIterator for ThemeEntryIterator {}
impl std::iter::ExactSizeIterator for ThemeEntryIterator {}

#[cfg(feature = "pyffi")]
#[pymethods]
impl ThemeEntryIterator {
    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<ThemeEntry> {
        slf.next()
    }
}

// --------------------------------------------------------------------------------------------------------------------

/// The color theme with the 2+16 colors of [VGA text
/// mode](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit).
pub const VGA_COLORS: Theme = [
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
];

// ====================================================================================================================
// Hue and Lightness Table
// ====================================================================================================================

/// A gray ANSI color and its concrete lightness value.
#[derive(Debug, Default)]
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
        if !spec.is_gray() || !is_gray_chroma_hue(c, h) {
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
#[derive(Debug, Default)]
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
        if is_gray_chroma_hue(c, h) {
            return None;
        }
        h = h.rem_euclid(360.0); // Critical for correctness!

        Some(ColorEntry { spec, lr, h })
    }

    /// Get the 3-bit base color.
    fn base(&self) -> AnsiColor {
        self.spec.to_3bit()
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
    /// Create a new hue lightness table.
    ///
    /// This associated function returns `None` if the theme colors violate any
    /// of the invariants.
    fn new(theme: &Theme) -> Option<HueLightnessTable> {
        // Prep the grays
        let mut grays = Vec::with_capacity(4);
        for index in [0_usize, 7, 8, 15] {
            grays.push(GrayEntry::new(
                AnsiColor::try_from(index as u8).unwrap(),
                &theme[index + 2],
            )?);
        }
        grays.sort_by_key(|entry| entry.key());

        // Prep the non-grays in hue order: red, yellow, green, cyan, blue, magenta.
        let mut colors = Vec::with_capacity(12);
        for index in [1_usize, 3, 2, 6, 4, 5] {
            let regular =
                ColorEntry::new(AnsiColor::try_from(index as u8).unwrap(), &theme[index + 2])?;
            let bright = ColorEntry::new(
                AnsiColor::try_from(index as u8 + 8).unwrap(),
                &theme[index + 8 + 2],
            )?;

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
        if min_index > 0 {
            colors.rotate_left(min_index);
        }

        // Now, if hues are some rotation of standard order, hues are sorted.
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
        if is_gray_chroma_hue(c, h) {
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
            if h > next_entry.h && (index != 0 || h < self.colors[length - 1].h) {
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
            if previous_hue > h {
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
// Sampler
// ====================================================================================================================

// Convert the constituent terminal colors to their wrapped versions.
#[cfg(feature = "pyffi")]
fn into_terminal_color(obj: &Bound<'_, PyAny>) -> PyResult<TerminalColor> {
    if obj.is_instance_of::<PyInt>() {
        return obj.extract::<u8>().map(|c| c.into());
    }

    obj.extract::<TerminalColor>()
        .or_else(|_| obj.extract::<AnsiColor>().map(|c| c.into()))
        .or_else(|_| obj.extract::<EmbeddedRgb>().map(|c| c.into()))
        .or_else(|_| obj.extract::<crate::TrueColor>().map(|c| c.into()))
        .or_else(|_| obj.extract::<GrayGradient>().map(|c| c.into()))
        .or_else(|_| obj.extract::<DefaultColor>().map(|c| c.into()))
}

/// A color sampler.
///
/// Instances of this struct translate between [`TerminalColor`] and [`Color`]
/// and maintain the state for doing so efficiently. The [user
/// guide](https://apparebit.github.io/prettypretty/overview/integration.html)
/// includes a detailed discussion of challenges posed by translation, solution
/// approaches, and sampler's interface.
///
/// Since a sampler incorporates theme colors, an application should regenerate
/// its sampler if the current theme changes.
#[cfg_attr(feature = "pyffi", pyclass)]
pub struct Sampler {
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
    for index in 0..=15 {
        coordinates[index] = *theme[index + 2].to(space).as_ref();
    }

    coordinates
}

/// Create the coordinates for the 8-bit colors in the given color space.
#[allow(clippy::needless_range_loop)]
fn eight_bit_coordinates(space: ColorSpace, theme: &Theme) -> [[Float; 3]; 256] {
    let mut coordinates: [[Float; 3]; 256] = [[0.0; 3]; 256];
    for index in 0..=15 {
        coordinates[index] = *theme[index].to(space).as_ref()
    }
    for index in 16..=231 {
        coordinates[index] = *Color::from(EmbeddedRgb::try_from(index as u8).unwrap())
            .to(space)
            .as_ref();
    }
    for index in 232..=255 {
        coordinates[index] = *Color::from(GrayGradient::try_from(index as u8).unwrap())
            .to(space)
            .as_ref();
    }

    coordinates
}

#[cfg(feature = "pyffi")]
#[pymethods]
impl Sampler {
    /// Create a new sampler for the given Oklab version and theme colors.
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

    /// Create a new iterator over theme entries.
    ///
    /// Prettypretty started out with a struct for color themes, but it was
    /// removed again when it effectively reimplemented the functionality of an
    /// 18-entry slice. So that's exactly what a color theme is
    #[staticmethod]
    pub fn theme_entries() -> ThemeEntryIterator {
        ThemeEntryIterator::new()
    }

    /// Determine whether this sampler's color theme is a dark theme.
    pub fn is_dark_theme(&self) -> bool {
        let yf = self.theme[0].to(ColorSpace::Xyz)[1];
        let yb = self.theme[1].to(ColorSpace::Xyz)[1];
        yf > yb
    }

    /// Resolve the terminal color to a high-resolution color.
    ///
    /// A custom type conversion function provides the same functionality as
    /// Rust's `impl Into<TerminalColor>`, with the result that this method can
    /// be invoked with wrapped terminal colors as much as with their
    /// constituent colors.
    ///
    /// # Examples
    ///
    /// ```
    /// # use prettypretty::{AnsiColor, Color, DefaultColor, OkVersion, Sampler, TrueColor};
    /// # use prettypretty::{VGA_COLORS};
    /// let black = sampler.resolve(DefaultColor::Foreground);
    /// assert_eq!(black, Color::srgb(0.0, 0.0, 0.0));
    ///
    /// let blue = sampler.resolve(AnsiColor::Blue);
    /// assert_eq!(blue, Color::srgb(0.0, 0.0, 0.666666666666667));
    ///
    /// let maroon = sampler.resolve(TrueColor::new(148, 23, 81));
    /// assert_eq!(maroon, Color::srgb(
    ///     0.5803921568627451, 0.09019607843137255, 0.3176470588235294
    /// ));
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #0000aa;"></div>
    /// <div style="background-color: #000000;"></div>
    /// <div style="background-color: #941751;"></div>
    /// </div>
    pub fn resolve(
        &self,
        #[pyo3(from_py_with = "into_terminal_color")] color: TerminalColor,
    ) -> Color {
        self.do_resolve(color)
    }

    /// Convert the high-resolution color into an ANSI color.
    ///
    /// If available, this method utilizes [`Sampler::to_ansi_hue_lightness`] to
    /// find a suitable ANSI color based on hue and lightness. If the current
    /// theme does not meet the requirements for that search, this method falls
    /// back onto [`Sampler::to_closest_ansi`], which searches for the closest
    /// matching ANSI color.
    pub fn to_ansi(&self, color: &Color) -> AnsiColor {
        self.to_ansi_hue_lightness(color)
            .unwrap_or_else(|| self.to_closest_ansi(color))
    }

    /// Determine whether this sampler instance supports color translation with
    /// hue/lightness algorithm.
    pub fn supports_hue_lightness(&self) -> bool {
        self.hue_lightness_table.is_some()
    }

    /// Convert the high-resolution color to ANSI based on hue and lightness.
    ///
    /// For grays, this method finds the ANSI gray with the closest lightness.
    /// For colors, this method first finds the pair of regular and bright ANSI
    /// colors with the closest hue and then selects the one with the closest
    /// lightness.
    ///
    /// This method requires that concrete theme colors and abstract ANSI colors
    /// are (loosely) aligned. Notably, the color values for pairs of regular
    /// and bright ANSI colors must be in order red, yellow, green, cyan, blue,
    /// and magenta when traversing hues counter-clockwise, i.e., with
    /// increasing hue magnitude. Note that this does allow hues to be
    /// arbitrarily shifted along the circle. Furthermore, it does not prescribe
    /// an order for regular and bright versions of the same abstract ANSI
    /// color. If the theme colors passed to this sampler's constructor did not
    /// meet this requirement, this method returns `None`.
    ///
    /// # Examples
    ///
    /// The documentation for [`Sampler::to_closest_ansi`] gives the example of
    /// two colors that yield subpar results with an exhaustive search for the
    /// closest color and then sketches an alternative approach that searches
    /// for the closest hue.
    ///
    /// The algorithm implemented by this method goes well beyond that sketch by
    /// not only leveraging color pragmatics (i.e., their coordinates) but also
    /// their semantics. Hence, it first searches for one out of six pairs of
    /// regular and bright ANSI colors with the closest hue and then picks the
    /// one out of two colors with the closest lightness.
    ///
    /// As this example illustrates, that strategy works well for the light
    /// orange colors from [`Sampler::to_closest_ansi`]. They both match the
    /// yellow pair by hue and then bright yellow by lightness. Alas, it is no
    /// panacea because color themes may not observe the necessary semantic
    /// constraints. This method detects such cases and returns `None`.
    /// [`Sampler::to_ansi`] instead automatically falls back onto searching for
    /// the closest color.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorFormatError, ColorSpace, Sampler};
    /// # use prettypretty::{VGA_COLORS, OkVersion};
    /// # use std::str::FromStr;
    /// let sampler = Sampler::new(
    ///     OkVersion::Revised, VGA_COLORS.clone());
    ///
    /// let orange1 = Color::from_str("#ffa563")?;
    /// let ansi = sampler.to_ansi_hue_lightness(&orange1);
    /// assert_eq!(ansi.unwrap(), AnsiColor::BrightYellow);
    ///
    /// let orange2 = Color::from_str("#ff9600")?;
    /// let ansi = sampler.to_ansi_hue_lightness(&orange2);
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
    /// # use prettypretty::{Color, ColorFormatError, ColorSpace, Sampler};
    /// # use prettypretty::{VGA_COLORS, OkVersion};
    /// # use std::str::FromStr;
    /// let original_sampler = Sampler::new(
    ///     OkVersion::Original, VGA_COLORS.clone());
    ///
    /// let orange1 = Color::from_str("#ffa563")?;
    /// let ansi = original_sampler.to_closest_ansi(&orange1);
    /// assert_eq!(ansi, AnsiColor::White);
    ///
    /// let orange2 = Color::from_str("#ff9600")?;
    /// let ansi = original_sampler.to_closest_ansi(&orange2);
    /// assert_eq!(ansi, AnsiColor::BrightRed);
    /// // ---------------------------------------------------------------------
    /// let revised_sampler = Sampler::new(
    ///     OkVersion::Revised, VGA_COLORS.clone());
    ///
    /// let ansi = revised_sampler.to_closest_ansi(&orange1);
    /// assert_eq!(ansi, AnsiColor::White);
    ///
    /// let ansi = revised_sampler.to_closest_ansi(&orange2);
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
    /// than a chromaless gray tone.
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
    /// [`Sampler::new`] does as well.
    /// ```
    /// # use prettypretty::{AnsiColor, Color, ColorFormatError, ColorSpace, VGA_COLORS};
    /// # use std::str::FromStr;
    /// let ansi_colors: Vec<Color> = (0..=15)
    ///     .map(|n| VGA_COLORS[n + 2].to(ColorSpace::Oklrch))
    ///     .collect();
    /// ```
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
    /// # use prettypretty::{AnsiColor, Color, ColorFormatError, ColorSpace, VGA_COLORS, Float};
    /// # use std::str::FromStr;
    /// # let ansi_colors: Vec<Color> = (0..=15)
    /// #     .map(|n| VGA_COLORS[n + 2].to(ColorSpace::Oklrch))
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
    /// too. The method to do so is [`Sampler::to_ansi_hue_lightness`].
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
        self.do_to_ansi_rgb(color)
    }

    /// Find the 8-bit color that comes closest to the given color.
    ///
    /// # Examples
    ///
    /// The example below converts every color of the RGB cube embedded in 8-bit
    /// colors to a high-resolution color in sRGB, which is validated by the
    /// first two assertions, and then uses a sampler to convert that color back
    /// to an embedded RGB color. The result is the original color, now wrapped
    /// as a terminal color, which is validated by the third assertion. The
    /// example demonstrates that the 216 colors in the embedded RGB cube still
    /// are closest to themselves after conversion to Oklrch.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace, VGA_COLORS, TerminalColor, Float};
    /// # use prettypretty::{EmbeddedRgb, OutOfBoundsError, Sampler, OkVersion};
    /// # use prettypretty::assert_close_enough;
    /// let sampler = Sampler::new(OkVersion::Revised, VGA_COLORS.clone());
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
    ///             let result = sampler.to_closest_8bit(&color);
    ///             assert_eq!(result, TerminalColor::Rgb6 { color: embedded });
    ///         }
    ///     }
    /// }
    /// # Ok::<(), OutOfBoundsError>(())
    /// ```
    pub fn to_closest_8bit(&self, color: &Color) -> TerminalColor {
        use crate::core::{delta_e_ok, find_closest};

        let color = color.to(self.space);
        let index = find_closest(
            color.as_ref(),
            self.eight_bit.last_chunk::<240>().unwrap(),
            delta_e_ok,
        )
        .map(|idx| idx as u8 + 16)
        .unwrap();

        TerminalColor::from(index)
    }

    /// Find the 8-bit color that comes closest to the given color.
    ///
    /// This method comparse *all* 8-bit colors including ANSI colors.
    pub fn to_closest_8bit_with_ansi(&self, color: &Color) -> TerminalColor {
        use crate::core::{delta_e_ok, find_closest};

        let color = color.to(self.space);
        let index = find_closest(color.as_ref(), &self.eight_bit, delta_e_ok)
            .map(|idx| idx as u8 + 16)
            .unwrap();

        TerminalColor::from(index)
    }

    /// Cap the terminal color by the fidelity.
    ///
    /// This method ensures that the terminal color can be rendered by a
    /// terminal with the given fidelity level. Depending on fidelity, it
    /// returns the following result:
    ///
    ///   * Plain-text or no-color
    ///         * `None`
    ///   * ANSI colors
    ///         * Downsampled 8-bit and 24-bit colors
    ///         * Unmodified ANSI colors
    ///   * 8-bit
    ///         * Downsampled 24-bit colors
    ///         * Unmodified ANSI and 8-bit colors
    ///   * Full
    ///         * Unmodified colors
    ///
    /// A custom type conversion function ensures that this method can be
    /// invoked on default, ANSI, 8-bit, 24-bit and terminal colors without
    /// prior conversion.
    pub fn cap(
        &self,
        #[pyo3(from_py_with = "into_terminal_color")] color: TerminalColor,
        fidelity: Fidelity,
    ) -> Option<TerminalColor> {
        self.do_cap(color, fidelity)
    }

    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[cfg(not(feature = "pyffi"))]
impl Sampler {
    /// Create a new sampler for the given Oklab version and theme colors.
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

    /// Create a new iterator over theme entries.
    pub fn theme_entries() -> ThemeEntryIterator {
        ThemeEntryIterator::new()
    }

    /// Determine whether this sampler's color theme is a dark theme.
    ///
    /// The Y component of a color in XYZ represents it luminance. This method
    /// exploits that property of XYZ and checks whether the default foreground
    /// has a larger luminance than the default background color.
    pub fn is_dark_theme(&self) -> bool {
        let yf = self.theme[0].to(ColorSpace::Xyz)[1];
        let yb = self.theme[1].to(ColorSpace::Xyz)[1];
        yf > yb
    }

    /// Resolve the terminal color to a high-resolution color.
    ///
    /// # Examples
    ///
    /// Thanks to Rust's `Into<TerminalColor>` trait, the terminal color wrapper
    /// type appears exactly zero times in the actual source code, just as it
    /// should be. The Python version uses a custom type conversion function to
    /// achieve the same effect.
    ///
    /// ```
    /// # use prettypretty::{AnsiColor, Color, DefaultColor, OkVersion, Sampler, TrueColor};
    /// # use prettypretty::{VGA_COLORS};
    /// let sampler = Sampler::new(OkVersion::Revised, VGA_COLORS.clone());
    /// let blue = sampler.resolve(AnsiColor::Blue);
    /// assert_eq!(blue, Color::srgb(0.0, 0.0, 0.666666666666667));
    ///
    /// let black = sampler.resolve(DefaultColor::Foreground);
    /// assert_eq!(black, Color::srgb(0.0, 0.0, 0.0));
    ///
    /// let maroon = sampler.resolve(TrueColor::new(148, 23, 81));
    /// assert_eq!(maroon, Color::srgb(
    ///     0.5803921568627451, 0.09019607843137255, 0.3176470588235294
    /// ));
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #0000aa;"></div>
    /// <div style="background-color: #000000;"></div>
    /// <div style="background-color: #941751;"></div>
    /// </div>
    pub fn resolve(&self, color: impl Into<TerminalColor>) -> Color {
        self.do_resolve(color)
    }

    /// Convert the high-resolution color into an ANSI color.
    ///
    /// If available, this method utilizes [`Sampler::to_ansi_hue_lightness`] to
    /// find a suitable ANSI color based on hue and lightness. If the current
    /// theme does not meet the requirements for that search, this method falls
    /// back onto [`Sampler::to_closest_ansi`], which searches for the closest
    /// matching ANSI color.
    pub fn to_ansi(&self, color: &Color) -> AnsiColor {
        self.to_ansi_hue_lightness(color)
            .unwrap_or_else(|| self.to_closest_ansi(color))
    }

    /// Determine whether this sampler instance supports color translation with
    /// hue/lightness algorithm.
    pub fn supports_hue_lightness(&self) -> bool {
        self.hue_lightness_table.is_some()
    }

    /// Convert the high-resolution color to ANSI based on hue (h) and lightness
    /// revised (lr).
    ///
    /// For grays, this method finds the ANSI gray with the closest lightness.
    /// For colors, this method first finds the pair of regular and bright ANSI
    /// colors with the closest hue and then selects the color with the closest
    /// lightness.
    ///
    /// This method requires that concrete theme colors and abstract ANSI colors
    /// are (loosely) aligned. Notably, the color values for pairs of regular
    /// and bright ANSI colors must be in order red, yellow, green, cyan, blue,
    /// and magenta when traversing hues counter-clockwise, i.e., with
    /// increasing hue magnitude. Note that this does allow hues to be
    /// arbitrarily shifted along the circle. Furthermore, it does not prescribe
    /// an order for regular and bright versions of the same abstract ANSI
    /// color. If the theme colors passed to this sampler's constructor did not
    /// meet this requirement, this method returns `None`.
    ///
    /// # Examples
    ///
    /// The documentation for [`Sampler::to_closest_ansi`] gives the example of
    /// two colors that yield subpar results with an exhaustive search for the
    /// closest color and then sketches an alternative approach that searches
    /// for the closest hue.
    ///
    /// The algorithm implemented by this method goes well beyond that sketch by
    /// not only leveraging color pragmatics (i.e., their coordinates) but also
    /// their semantics. Hence, it first searches for one out of six pairs of
    /// regular and bright ANSI colors with the closest hue and then picks the
    /// one out of two colors with the closest lightness.
    ///
    /// As this example illustrates, that strategy works well for the light
    /// orange colors from [`Sampler::to_closest_ansi`]. They both match the
    /// yellow pair by hue and then bright yellow by lightness. Alas, it is no
    /// panacea because color themes may not observe the necessary semantic
    /// constraints. This method detects such cases and returns `None`.
    /// [`Sampler::to_ansi`] instead automatically falls back onto searching for
    /// the closest color.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorFormatError, ColorSpace, Sampler};
    /// # use prettypretty::{VGA_COLORS, OkVersion, AnsiColor};
    /// # use std::str::FromStr;
    /// let sampler = Sampler::new(
    ///     OkVersion::Revised, VGA_COLORS.clone());
    ///
    /// let orange1 = Color::from_str("#ffa563")?;
    /// let ansi = sampler.to_ansi_hue_lightness(&orange1);
    /// assert_eq!(ansi.unwrap(), AnsiColor::BrightYellow);
    ///
    /// let orange2 = Color::from_str("#ff9600")?;
    /// let ansi = sampler.to_ansi_hue_lightness(&orange2);
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
    /// # use prettypretty::{Color, ColorFormatError, ColorSpace, Sampler};
    /// # use prettypretty::{VGA_COLORS, OkVersion, AnsiColor};
    /// # use std::str::FromStr;
    /// let original_sampler = Sampler::new(
    ///     OkVersion::Original, VGA_COLORS.clone());
    ///
    /// let orange1 = Color::from_str("#ffa563")?;
    /// let ansi = original_sampler.to_closest_ansi(&orange1);
    /// assert_eq!(ansi, AnsiColor::White);
    ///
    /// let orange2 = Color::from_str("#ff9600")?;
    /// let ansi = original_sampler.to_closest_ansi(&orange2);
    /// assert_eq!(ansi, AnsiColor::BrightRed);
    /// // ---------------------------------------------------------------------
    /// let revised_sampler = Sampler::new(
    ///     OkVersion::Revised, VGA_COLORS.clone());
    ///
    /// let ansi = revised_sampler.to_closest_ansi(&orange1);
    /// assert_eq!(ansi, AnsiColor::White);
    ///
    /// let ansi = revised_sampler.to_closest_ansi(&orange2);
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
    /// than a chromaless gray tone.
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
    /// [`Sampler::new`] does as well.
    /// ```
    /// # use prettypretty::{AnsiColor, Color, ColorFormatError, ColorSpace, VGA_COLORS};
    /// # use std::str::FromStr;
    /// let ansi_colors: Vec<Color> = (0..=15)
    ///     .map(|n| VGA_COLORS[n + 2].to(ColorSpace::Oklrch))
    ///     .collect();
    /// ```
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
    /// # use prettypretty::{AnsiColor, Color, ColorFormatError, ColorSpace, VGA_COLORS, Float};
    /// # use std::str::FromStr;
    /// # let ansi_colors: Vec<Color> = (0..=15)
    /// #     .map(|n| VGA_COLORS[n + 2].to(ColorSpace::Oklrch))
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
    /// too. The method to do so is [`Sampler::to_ansi_hue_lightness`].
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
        self.do_to_ansi_rgb(color)
    }

    /// Find the 8-bit color that comes closest to the given color.
    ///
    /// This method only compares to embedded RGB and gray gradient colors, not
    /// ANSI colors. It also is the recommended version because the ANSI colors
    /// can be visually disruptive amongst translated graduated peers.
    ///
    /// # Examples
    ///
    /// The example below converts every color of the RGB cube embedded in 8-bit
    /// colors to a high-resolution color in sRGB, which is validated by the
    /// first two assertions, and then uses a sampler to convert that color back
    /// to an embedded RGB color. The result is the original color, now wrapped
    /// as a terminal color, which is validated by the third assertion. The
    /// example demonstrates that the 216 colors in the embedded RGB cube still
    /// are closest to themselves after conversion to Oklrch.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace, VGA_COLORS, TerminalColor, Float};
    /// # use prettypretty::{EmbeddedRgb, OutOfBoundsError, Sampler, OkVersion};
    /// # use prettypretty::assert_close_enough;
    /// let sampler = Sampler::new(OkVersion::Revised, VGA_COLORS.clone());
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
    ///             let result = sampler.to_closest_8bit(&color);
    ///             assert_eq!(result, TerminalColor::Rgb6 { color: embedded });
    ///         }
    ///     }
    /// }
    /// # Ok::<(), OutOfBoundsError>(())
    /// ```
    pub fn to_closest_8bit(&self, color: &Color) -> TerminalColor {
        use crate::core::{delta_e_ok, find_closest};

        let color = color.to(self.space);
        let index = find_closest(
            color.as_ref(),
            self.eight_bit.last_chunk::<240>().unwrap(),
            delta_e_ok,
        )
        .map(|idx| idx as u8 + 16)
        .unwrap();

        TerminalColor::from(index)
    }

    /// Find the 8-bit color that comes closest to the given color.
    ///
    /// This method comparse *all* 8-bit colors including ANSI colors.
    pub fn to_closest_8bit_with_ansi(&self, color: &Color) -> TerminalColor {
        use crate::core::{delta_e_ok, find_closest};

        let color = color.to(self.space);
        let index = find_closest(color.as_ref(), &self.eight_bit, delta_e_ok)
            .map(|idx| idx as u8 + 16)
            .unwrap();

        TerminalColor::from(index)
    }

    /// Cap the terminal color by the fidelity.
    ///
    /// This method ensures that the terminal color can be rendered by a
    /// terminal with the given fidelity level. Depending on fidelity, it
    /// returns the following result:
    ///
    ///   * Plain-text or no-color
    ///         * `None`
    ///   * ANSI colors
    ///         * Downsampled 8-bit and 24-bit colors
    ///         * Unmodified ANSI colors
    ///   * 8-bit
    ///         * Downsampled 24-bit colors
    ///         * Unmodified ANSI and 8-bit colors
    ///   * Full
    ///         * Unmodified colors
    ///
    /// The Python version has a custom type conversion function and also
    /// accepts all kinds of terminal colors, whether wrapper or not.
    pub fn cap(
        &self,
        color: impl Into<TerminalColor>,
        fidelity: Fidelity,
    ) -> Option<TerminalColor> {
        self.do_cap(color, fidelity)
    }
}

impl Sampler {
    #[inline]
    fn do_resolve(&self, color: impl Into<TerminalColor>) -> Color {
        match color.into() {
            TerminalColor::Default { color: c } => self.theme[c as usize].clone(),
            TerminalColor::Ansi { color: c } => self.theme[c as usize + 2].clone(),
            TerminalColor::Rgb6 { color: c } => c.into(),
            TerminalColor::Gray { color: c } => c.into(),
            TerminalColor::Rgb256 { color: c } => c.into(),
        }
    }

    fn do_to_ansi_rgb(&self, color: &Color) -> AnsiColor {
        let color = color.to(ColorSpace::LinearSrgb).clip();
        let [r, g, b] = color.as_ref();
        let mut index = ((b.round() as u8) << 2) + ((g.round() as u8) << 1) + (r.round() as u8);
        // When we get to the threshold below, the color has already been
        // selected and can only be brightened. A threshold of 2 or 3 produces
        // the least bad results. In any case, prettypretty.grid's output shows
        // large striped rectangles, with 4x24 cells black/blue and 2x24 cells
        // green/cyan above 4x12 cells red/magenta and 2x12 cells yellow/white.
        if index >= 3 {
            index += 8;
        }

        AnsiColor::try_from(index).unwrap()
    }

    fn do_cap(&self, color: impl Into<TerminalColor>, fidelity: Fidelity) -> Option<TerminalColor> {
        let color = color.into();
        match fidelity {
            Fidelity::Plain | Fidelity::NoColor => None,
            Fidelity::Ansi => {
                let c = match color {
                    TerminalColor::Default { .. } | TerminalColor::Ansi { .. } => {
                        return Some(color);
                    }
                    TerminalColor::Rgb6 { color: c } => Color::from(c),
                    TerminalColor::Gray { color: c } => Color::from(c),
                    TerminalColor::Rgb256 { color: c } => Color::from(c),
                };

                Some(TerminalColor::Ansi {
                    color: self.to_ansi(&c),
                })
            }
            Fidelity::EightBit => {
                if let TerminalColor::Rgb256 { color: c } = color {
                    Some(self.to_closest_8bit(&Color::from(c)))
                } else {
                    Some(color)
                }
            }
            Fidelity::Full => Some(color),
        }
    }
}

impl std::fmt::Debug for Sampler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Sampler({}, [",
            if self.space == ColorSpace::Oklab {
                "OkVersion.Original"
            } else {
                "OkVersion.Revised"
            }
        )?;

        for color in self.theme.iter() {
            writeln!(f, "    {:?},", color)?;
        }

        write!(f, "])")
    }
}

// ====================================================================================================================

#[cfg(test)]
mod test {
    use super::{Sampler, VGA_COLORS};
    use crate::{AnsiColor, Color, OkVersion, OutOfBoundsError};

    #[test]
    fn test_sampler() -> Result<(), OutOfBoundsError> {
        let sampler = Sampler::new(OkVersion::Revised, VGA_COLORS.clone());

        let result = sampler.to_closest_ansi(&Color::srgb(1.0, 1.0, 0.0));
        assert_eq!(result, AnsiColor::BrightYellow);

        Ok(())
    }
}
