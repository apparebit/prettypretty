#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use crate::{
    AnsiColor, Color, ColorSpace, EmbeddedRgb, Fidelity, Float, GrayGradient, Layer, OkVersion,
    TerminalColor,
};

// ====================================================================================================================
// Color Themes
// ====================================================================================================================

/// An iterator over theme entries.
///
/// This iterator is returned by [`Theme::entries`] and is both fused and exact,
/// i.e., it will keep returning `None` after returning `None` once and its
/// `size_hint()` returns the exact number of remaining items.
#[doc = include_str!("style.html")]
#[cfg_attr(feature = "pyffi", pyclass)]
#[derive(Debug)]
pub struct ThemeEntryIterator {
    index: usize,
}

impl Iterator for ThemeEntryIterator {
    type Item = ThemeEntry;

    /// Access the next theme entry.
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= 18 {
            None
        } else {
            let item = ThemeEntry::try_from(self.index).unwrap();
            self.index += 1;
            Some(item)
        }
    }

    /// Get the number of remaining theme entry, color pairs.
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = 18 - self.index;
        (remaining, Some(remaining))
    }
}

impl std::iter::FusedIterator for ThemeEntryIterator {}
impl std::iter::ExactSizeIterator for ThemeEntryIterator {}

#[cfg(feature = "pyffi")]
impl ThemeEntryIterator {
    /// Access the next theme entry. <span class=python-only></span>
    pub fn __next__(&mut self) -> Option<ThemeEntry> {
        self.next()
    }

    /// Access this iterator. <span class=python-only></span>
    pub fn __iter__(&self) -> &Self {
        self
    }
}

// --------------------------------------------------------------------------------------------------------------------

macro_rules! count_ident {
    () => { 0 };
    ($odd:ident $($a:ident $b:ident)*) => { (count_ident!($($a)*) << 1) | 1 };
    ($($a:ident $even:ident)*) => { count_ident!($($a)*) << 1 };
}

macro_rules! enriched_theme_entry {
    (
        $( #[$meta:meta] )*
        enum $name:ident {
            $( $variant:ident ),*,
        }
    ) => {
        /// A theme entry.
        ///
        /// A color theme's entries effectively combine the variants of
        /// [`Layer`] and [`AnsiColor`] into a new enumeration. Since variants
        /// are directly listed, this enumeration is easier to use than the
        /// equivalent wrapped enumeration. [`From<Layer> as
        /// ThemeEntry`](enum.ThemeEntry.html#impl-From%3CLayer%3E-for-ThemeEntry)
        /// and [`From<AnsiColor> as
        /// ThemeEntry`](enum.ThemeEntry.html#impl-From%3CAnsiColor%3E-for-ThemeEntry)
        /// capture the semantic connection.
        ///
        /// Use [`Theme::entries`] to access an iterator over the theme entries.
        /// Use [`ThemeEntry::name`] to access a theme entry's name.
        #[doc = include_str!("style.html")]
        #[cfg_attr(feature = "pyffi", pyclass(eq, eq_int, frozen, hash))]
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
        pub enum $name {
            $($variant),*
        }

        #[cfg_attr(feature = "pyffi", pymethods)]
        impl $name {
            /// Convert an integer to a theme entry. <span
            /// class=python-only></span>
            #[cfg(feature = "pyffi")]
            #[staticmethod]
            pub fn from_index(index: usize) -> Result<Self, $crate::OutOfBoundsError> {
                $name::try_from(index)
            }

            /// Convert the ANSI color to a theme entry. <span
            /// class=python-only></span>
            #[cfg(feature = "pyffi")]
            #[staticmethod]
            pub fn from_ansi_color(color: AnsiColor) -> Self {
                    $name::from(color)
            }

            /// Get the variant name.
            pub fn name(&self) -> &'static str {
                match self {
                   $($name::$variant => stringify!($variant)),*
                }
            }
        }

        impl ::std::convert::TryFrom<usize> for $name {
            type Error = $crate::OutOfBoundsError;

            /// Convert an integer to a theme entry.
            fn try_from(value: usize) -> Result<Self, Self::Error> {
                match value {
                    $(x if x == $name::$variant as usize => Ok($name::$variant)),*,
                    _ => Err(Self::Error::new(
                        value,
                        0..=(count_ident!($($variant)*) - 1)
                    ))
                }
            }
        }
    }
}

enriched_theme_entry! {
    enum ThemeEntry {
        Foreground,
        Background,
        Black,
        Red,
        Green,
        Yellow,
        Blue,
        Magenta,
        Cyan,
        White,
        BrightBlack,
        BrightRed,
        BrightGreen,
        BrightYellow,
        BrightBlue,
        BrightMagenta,
        BrightCyan,
        BrightWhite,
    }
}

impl From<Layer> for ThemeEntry {
    /// Convert the given layer to a theme entry.
    fn from(value: Layer) -> Self {
        match value {
            Layer::Foreground => ThemeEntry::Foreground,
            Layer::Background => ThemeEntry::Background,
        }
    }
}

impl From<AnsiColor> for ThemeEntry {
    /// Convert the given ANSI color to a theme entry.
    fn from(value: AnsiColor) -> Self {
        ThemeEntry::try_from(value as usize + 2).unwrap()
    }
}

// --------------------------------------------------------------------------------------------------------------------

/// A color theme with concrete color values.
///
/// A color theme provides concrete color values for the default foreground,
/// default background, and 16 extended ANSI colors. Once created, a theme is
/// immutable.
///
/// Rust code can use [`Layer`](crate::Layer) and [`AnsiColor`] values to
/// directly index a theme. Both Rust and Python can use integer indexes to look
/// up colors.
///
/// By itself, a theme enables the conversion of ANSI colors to high-resolution
/// colors. Through a [`Sampler`], a theme also enables the (lossy) conversion
/// of high-resolution colors to ANSI and 8-bit colors.
#[doc = include_str!("style.html")]
#[cfg_attr(feature = "pyffi", pyclass(eq, sequence))]
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    colors: [Color; 18],
}

#[cfg(feature = "pyffi")]
#[pymethods]
impl Theme {
    /// Instantiate a new theme.
    ///
    /// The 18 colors for the new theme are, in order, the default foreground,
    /// the default background, the ANSI colors black, red, green, yellow, blue,
    /// magenta, cyan, and white, as well as their bright versions in the same
    /// order.
    #[new]
    #[inline]
    pub const fn new(colors: [Color; 18]) -> Self {
        Theme { colors }
    }

    /// Determine the length of this theme, which is 18. <span
    /// class=python-only></span>
    ///
    /// This method is available in Python only.
    pub fn __len__(&self) -> usize {
        18
    }

    /// Get the color at the given index. <span class=python-only></span>
    ///
    /// This method clones the indexed color. It is available in Python only.
    pub fn __getitem__(&self, index: usize) -> PyResult<Color> {
        if (0..18).contains(&index) {
            Ok(self.colors[index].clone())
        } else {
            Err(pyo3::exceptions::PyIndexError::new_err(
                "index out of bounds",
            ))
        }
    }

    /// Convert this color theme to its debug representation. <span
    /// class=python-only></span>
    ///
    /// This method is available from Python only.
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    /// Create a new iterator over the theme entries.
    #[staticmethod]
    pub fn entries() -> ThemeEntryIterator {
        ThemeEntryIterator { index: 0 }
    }
}

#[cfg(not(feature = "pyffi"))]
impl Theme {
    /// Instantiate a new theme.
    ///
    /// The 18 colors for the new theme are, in order, the default foreground,
    /// the default background, the ANSI colors black, red, green, yellow, blue,
    /// magenta, cyan, and white, as well as their bright versions in the same
    /// order.
    #[inline]
    pub const fn new(colors: [Color; 18]) -> Self {
        Theme { colors }
    }

    /// Create a new iterator over the theme entries.
    pub fn entries() -> ThemeEntryIterator {
        ThemeEntryIterator { index: 0 }
    }
}

impl<T: Into<ThemeEntry>> std::ops::Index<T> for Theme {
    type Output = Color;

    /// Access the color value for the ANSI color.
    fn index(&self, index: T) -> &Self::Output {
        &self.colors[index.into() as usize]
    }
}

// --------------------------------------------------------------------------------------------------------------------

/// The default theme.
///
/// This theme exists to demonstrate the functionality enabled by themes as well
/// as for testing. It uses the colors of [VGA text
/// mode](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit).
pub const DEFAULT_THEME: Theme = Theme::new([
    Color::new(ColorSpace::Srgb, [0.0, 0.0, 0.0]),
    Color::new(ColorSpace::Srgb, [1.0, 1.0, 1.0]),
    Color::new(ColorSpace::Srgb, [0.0, 0.0, 0.0]),
    Color::new(ColorSpace::Srgb, [0.666666666666667, 0.0, 0.0]),
    Color::new(ColorSpace::Srgb, [0.0, 0.666666666666667, 0.0]),
    Color::new(
        ColorSpace::Srgb,
        [0.666666666666667, 0.333333333333333, 0.0],
    ),
    Color::new(ColorSpace::Srgb, [0.0, 0.0, 0.666666666666667]),
    Color::new(
        ColorSpace::Srgb,
        [0.666666666666667, 0.0, 0.666666666666667],
    ),
    Color::new(
        ColorSpace::Srgb,
        [0.0, 0.666666666666667, 0.666666666666667],
    ),
    Color::new(
        ColorSpace::Srgb,
        [0.666666666666667, 0.666666666666667, 0.666666666666667],
    ),
    Color::new(
        ColorSpace::Srgb,
        [0.333333333333333, 0.333333333333333, 0.333333333333333],
    ),
    Color::new(
        ColorSpace::Srgb,
        [1.0, 0.333333333333333, 0.333333333333333],
    ),
    Color::new(
        ColorSpace::Srgb,
        [0.333333333333333, 1.0, 0.333333333333333],
    ),
    Color::new(ColorSpace::Srgb, [1.0, 1.0, 0.333333333333333]),
    Color::new(
        ColorSpace::Srgb,
        [0.333333333333333, 0.333333333333333, 1.0],
    ),
    Color::new(ColorSpace::Srgb, [1.0, 0.333333333333333, 1.0]),
    Color::new(ColorSpace::Srgb, [0.333333333333333, 1.0, 1.0]),
    Color::new(ColorSpace::Srgb, [1.0, 1.0, 1.0]),
]);

// ====================================================================================================================
// Sampler
// ====================================================================================================================

/// A color sampler.
///
/// Instances of this struct translate between [`TerminalColor`] and [`Color`]
/// and maintain the state for doing so efficiently. Compared to conversion of
/// high-resolution colors between color spaces, translation between terminal
/// and high-resolution colors is more complicated:
///
///   * ANSI colors are abstract colors, i.e., have no intrinsic color values,
///     and hence require a color theme for translation from and to
///     high-resolution colors.
///   * The default color is not only abstract but also context-sensitive, i.e.,
///     its value depends on whether it is being used as foreground or
///     background color. Hence translation to high-resolution colors requires
///     that context and translation to terminal colors best avoids the default
///     colors. Despite these major limitations, setting the default color is
///     the universal solution to undoing a terminal color change.
///   * Because there are so few of them, translation to ANSI or 8-bit colors
///     entails a significant loss of resolution, with attendant changes in hue,
///     lightness, and chroma. At the same time, because there are so few target
///     colors, exhaustive search for the best match becomes eminently feasible.
///   * 8-bit colors comprise ANSI, embedded RGB, and gray gradient colors. But
///     when translating high-resolution to 8-bit colors, the ANSI colors are
///     best avoided, especially when translating more than one color. If the
///     current theme assigns color values also contained in the 6x6x6 RGB cube
///     or 24-step gray gradient, then the ANSI colors do not add anything.
///     However, if the theme assigns different color values, then those values
///     will stick out amongst other translated colors and be visually
///     disruptive.
///
/// Since a sampler incorporates the color values from a [`Theme`], an
/// application should regenerate its sampler if the current theme changes.
#[doc = include_str!("style.html")]
#[cfg_attr(feature = "pyffi", pyclass)]
#[derive(Debug)]
pub struct Sampler {
    theme: Theme,
    space: ColorSpace,
    ansi: Vec<[Float; 3]>,
    eight_bit: Vec<[Float; 3]>,
}

/// Create the coordinates for the ANSI colors in the given color space.
fn ansi_coordinates(space: ColorSpace, theme: &Theme) -> Vec<[Float; 3]> {
    (0..=15)
        .map(|n| *theme[AnsiColor::try_from(n).unwrap()].to(space).as_ref())
        .collect()
}

/// Create the coordinates for the embedded RGB and gray gradient colors in the
/// given color space.
fn eight_bit_coordinates(space: ColorSpace) -> Vec<[Float; 3]> {
    (16..=231)
        .map(|n| {
            *Color::from(EmbeddedRgb::try_from(n).unwrap())
                .to(space)
                .as_ref()
        })
        .chain((232..=255).map(|n| {
            *Color::from(GrayGradient::try_from(n).unwrap())
                .to(space)
                .as_ref()
        }))
        .collect()
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Sampler {
    /// Create a new sampler for the given theme and Oklab version.
    #[cfg(feature = "pyffi")]
    #[new]
    pub fn new(theme: &Theme, ok_version: OkVersion) -> Self {
        let space = ok_version.cartesian_space();
        let ansi = ansi_coordinates(space, theme);
        let eight_bit = eight_bit_coordinates(space);

        Self {
            theme: theme.clone(),
            space,
            ansi,
            eight_bit,
        }
    }

    /// Create a new sampler for the given theme and Oklab version.
    #[cfg(not(feature = "pyffi"))]
    pub fn new(theme: &Theme, ok_version: OkVersion) -> Self {
        let space = ok_version.cartesian_space();
        let ansi = ansi_coordinates(space, theme);
        let eight_bit = eight_bit_coordinates(space);

        Self {
            theme: theme.clone(),
            space,
            ansi,
            eight_bit,
        }
    }

    /// Convert the 8-bit index color to a high-resolution color.
    pub fn to_high_res_8bit(&self, index: u8) -> Color {
        self.try_high_res(&TerminalColor::from(index)).unwrap()
    }

    /// Try to convert the terminal color to a high-resolution color.
    ///
    /// This method directly translates embedded RGB, gray gradient, and true
    /// colors to the corresponding sRGB colors. It uses the current theme for
    /// ANSI colors. However, since it lacks the necessary context, it cannot
    /// translate the default color and returns `None`. To also translate
    /// the default color, use [`Sampler::to_high_res`].
    pub fn try_high_res(&self, color: &TerminalColor) -> Option<Color> {
        match *color {
            TerminalColor::Default() => None,
            TerminalColor::Ansi(c) => Some(self.theme[ThemeEntry::from(c)].clone()),
            TerminalColor::Rgb6(c) => Some(Color::from(c)),
            TerminalColor::Gray(c) => Some(Color::from(c)),
            TerminalColor::Rgb256(c) => Some(Color::from(c)),
        }
    }

    /// Convert the terminal color to a high-resolution color.
    ///
    /// The layer argument is necessary for translating default colors.
    pub fn to_high_res(&self, color: &TerminalColor, layer: Layer) -> Color {
        match color {
            TerminalColor::Default() => self.theme[ThemeEntry::from(layer)].clone(),
            _ => self.try_high_res(color).unwrap(),
        }
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
    /// # use prettypretty::{DEFAULT_THEME, OkVersion};
    /// # use std::str::FromStr;
    /// let original_sampler = Sampler::new(&DEFAULT_THEME, OkVersion::Original);
    ///
    /// let orange1 = Color::from_str("#ffa563")?;
    /// let ansi = original_sampler.to_closest_ansi(&orange1);
    /// assert_eq!(u8::from(ansi), 7);
    ///
    /// let orange2 = Color::from_str("#ff9600")?;
    /// let ansi = original_sampler.to_closest_ansi(&orange2);
    /// assert_eq!(u8::from(ansi), 9);
    /// // ---------------------------------------------------------------------
    /// let revised_sampler = Sampler::new(&DEFAULT_THEME, OkVersion::Revised);
    ///
    /// let ansi = revised_sampler.to_closest_ansi(&orange1);
    /// assert_eq!(u8::from(ansi), 7);
    ///
    /// let ansi = revised_sampler.to_closest_ansi(&orange2);
    /// assert_eq!(u8::from(ansi), 9);
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
    /// # use prettypretty::{AnsiColor, Color, ColorFormatError, ColorSpace, DEFAULT_THEME};
    /// # use std::str::FromStr;
    /// let ansi_colors: Vec<Color> = (0..=15)
    ///     .map(|n| {
    ///         DEFAULT_THEME[AnsiColor::try_from(n).unwrap()]
    ///             .to(ColorSpace::Oklrch)
    ///     })
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
    /// # use prettypretty::{AnsiColor, Color, ColorFormatError, ColorSpace, DEFAULT_THEME, Float};
    /// # use std::str::FromStr;
    /// # let ansi_colors: Vec<Color> = (0..=15)
    /// #     .map(|n| {
    /// #         DEFAULT_THEME[AnsiColor::try_from(n).unwrap()]
    /// #             .to(ColorSpace::Oklrch)
    /// #     })
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
    /// too.
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
    /// color solely based on RGB coordinates. Since the ANSI colors essentially
    /// are 3-bit RGB colors with an additional bit for brightness, it converts
    /// the given color to linear sRGB, clipping out of gamut coordinates, and
    /// then rounds each coordinate to 0 or 1. It determines whether to set the
    /// brightness bit based on a heuristically weighted sum of the individual
    /// coordinates.
    ///
    /// While that may seem an unusual choice for conversion algorithm, it is a
    /// significant improvement over the algorithm used by
    /// [Chalk](https://github.com/chalk/chalk/blob/main/source/vendor/ansi-styles/index.js),
    /// which is one of the most popular terminal color libraries for JavaScript.
    pub fn to_ansi_in_rgb(&self, color: &Color) -> AnsiColor {
        let color = color.to(ColorSpace::LinearSrgb).clip();
        let [r, g, b] = color.as_ref();
        let mut index = ((b.round() as u8) << 2) + ((g.round() as u8) << 1) + (r.round() as u8);
        if index >= 2 {
            index += 8;
        }

        AnsiColor::try_from(index).unwrap()
    }

    /// Find the 8-bit color that comes closest to the given color.
    ///
    /// This method does most of the heavy lifting for
    /// [`Sampler::to_closest_8bit`] but does not wrap the 8-bit index.
    pub fn to_closest_8bit_raw(&self, color: &Color) -> u8 {
        use crate::core::{delta_e_ok, find_closest};

        let color = color.to(self.space);
        find_closest(color.as_ref(), &self.eight_bit, delta_e_ok)
            .map(|idx| idx as u8 + 16)
            .unwrap()
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
    /// # use prettypretty::{Color, ColorSpace, DEFAULT_THEME, TerminalColor, Float};
    /// # use prettypretty::{EmbeddedRgb, OutOfBoundsError, Sampler, OkVersion};
    /// let sampler = Sampler::new(&DEFAULT_THEME, OkVersion::Revised);
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
    ///             assert!((color[0] - c1).abs() < Float::EPSILON);
    ///
    ///             let result = sampler.to_closest_8bit(&color);
    ///             assert_eq!(result, TerminalColor::Rgb6(embedded));
    ///         }
    ///     }
    /// }
    /// # Ok::<(), OutOfBoundsError>(())
    /// ```
    pub fn to_closest_8bit(&self, color: &Color) -> TerminalColor {
        TerminalColor::from(self.to_closest_8bit_raw(color))
    }

    /// Adjust the terminal color to the fidelity.
    ///
    /// This method ensures that the given color can be displayed with the given
    /// fidelity, downsampling it if necessary. In particular:
    ///
    ///   * Return `None` if the fidelity is plain-text or no-color ;
    ///   * Downsample 24-bit and 8-bit colors if the fidelity is ANSI-colors;
    ///   * Downsample 24-bit colors if the fidelity is 8-bit-colors;
    ///   * Pass through color if the fidelity is 24-bit-colors.
    ///
    pub fn adjust(&self, color: TerminalColor, fidelity: Fidelity) -> Option<TerminalColor> {
        match fidelity {
            Fidelity::Plain | Fidelity::NoColor => None,
            Fidelity::Ansi => {
                if matches!(color, TerminalColor::Default() | TerminalColor::Ansi(_)) {
                    Some(color)
                } else {
                    let c = match color {
                        TerminalColor::Rgb6(c) => Color::from(c),
                        TerminalColor::Gray(c) => Color::from(c),
                        TerminalColor::Rgb256(c) => Color::from(c),
                        _ => unreachable!(),
                    };
                    Some(TerminalColor::Ansi(self.to_closest_ansi(&c)))
                }
            }
            Fidelity::EightBit => {
                if let TerminalColor::Rgb256(c) = color {
                    Some(self.to_closest_8bit(&Color::from(c)))
                } else {
                    Some(color)
                }
            }
            Fidelity::Full => Some(color),
        }
    }
}

// ====================================================================================================================

#[cfg(test)]
mod test {
    use super::{Sampler, DEFAULT_THEME};
    use crate::{AnsiColor, Color, OkVersion, OutOfBoundsError};

    #[test]
    fn test_sampler() -> Result<(), OutOfBoundsError> {
        let sampler = Sampler::new(&DEFAULT_THEME, OkVersion::Revised);

        let result = sampler.to_closest_ansi(&Color::srgb(1.0, 1.0, 0.0));
        assert_eq!(result, AnsiColor::BrightYellow);

        Ok(())
    }
}
