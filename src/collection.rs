#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use crate::{
    AnsiColor, Color, ColorSpace, EightBitColor, EmbeddedRgb, Float, GrayGradient, OkVersion,
};

// ====================================================================================================================
// Color Themes
// ====================================================================================================================

/// The layer for rendering to the terminal.
#[cfg_attr(feature = "pyffi", pyclass(eq, eq_int, frozen, hash))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Layer {
    /// The foreground or text layer.
    Foreground = 0,
    /// The background layer.
    Background = 10,
}

/// A color theme with concrete color values.
///
/// A color theme provides concrete color values for the foreground and
/// background defaults as well as for the 16 extended ANSI colors. They are
/// accessed (and also updated) through index expressions using [`Layer`] and
/// [`AnsiColor`].
///
/// By itself, a theme enables the conversion of ANSI colors to high-resolution
/// colors. Through a [`ColorMatcher`], a theme also enables the (lossy)
/// conversion of high-resolution colors to ANSI and 8-bit colors.
///
/// <style>
/// .python-only::before, .rust-only::before {
///     font-size: 0.8em;
///     display: inline-block;
///     border-radius: 0.5em;
///     padding: 0 0.6em;
///     font-family: -apple-system, BlinkMacSystemFont, avenir next, avenir, segoe ui,
///         helvetica neue, helvetica, Cantarell, Ubuntu, roboto, noto, arial, sans-serif;
///     font-weight: 600;
/// }
/// .python-only::before {
///     content: "Python only!";
///     background: #84c5fb;
/// }
/// .rust-only::before {
///     content: "Rust only!";
///     background: #f0ac84;
/// }
/// </style>
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
    /// The 18 colors for the new theme are, in order, the foreground default,
    /// the background default, and the 16 extended ANSI colors in standard
    /// order.
    #[new]
    #[inline]
    pub const fn new(colors: [Color; 18]) -> Self {
        Theme { colors }
    }

    /// Instantiate a new default theme. <span class=python-only></span>
    ///
    /// This method implements the same functionality as [`Theme as
    /// Default`](struct.Theme.html#impl-Default-for-Theme), is available in
    /// Python only, and returns a clone of the default theme with VGA text
    /// colors.
    #[staticmethod]
    pub fn new_default() -> Self {
        Self::default()
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

    /// Set the color at the given index. <span class=python-only></span>
    ///
    /// This method clones the given color. It is available in Python only.
    pub fn __setitem__(&mut self, index: usize, color: &Color) -> PyResult<()> {
        if (0..18).contains(&index) {
            self.colors[index] = color.clone();
            Ok(())
        } else {
            Err(pyo3::exceptions::PyIndexError::new_err(
                "theme index out of bounds 0..18",
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
}

#[cfg(not(feature = "pyffi"))]
impl Theme {
    /// Instantiate a new theme.
    ///
    /// The 18 colors for the new theme are, in order, the foreground default,
    /// the background default, and the 16 extended ANSI colors in standard
    /// order. Rust code accesses these 18 colors by indexing with the variants
    /// of the [`Layer`] and [`AnsiColor`] enumerations. Python code instead
    /// indexes with integers in range `0..=17`.
    #[inline]
    pub const fn new(colors: [Color; 18]) -> Self {
        Theme { colors }
    }
}

impl Default for Theme {
    /// Create a new default theme instance.
    ///
    /// This method returns a clone of the default theme, which comprises the
    /// VGA text colors.
    fn default() -> Self {
        DEFAULT_THEME.clone()
    }
}

impl std::ops::Index<Layer> for Theme {
    type Output = Color;

    /// Access the color value for the layer's default color.
    fn index(&self, index: Layer) -> &Self::Output {
        match index {
            Layer::Foreground => &self.colors[0],
            Layer::Background => &self.colors[1],
        }
    }
}

impl std::ops::IndexMut<Layer> for Theme {
    /// Mutably access the color value for the layer's default color.
    fn index_mut(&mut self, index: Layer) -> &mut Self::Output {
        match index {
            Layer::Foreground => &mut self.colors[0],
            Layer::Background => &mut self.colors[1],
        }
    }
}

impl std::ops::Index<AnsiColor> for Theme {
    type Output = Color;

    /// Access the color value for the ANSI color.
    fn index(&self, index: AnsiColor) -> &Self::Output {
        &self.colors[2 + index as usize]
    }
}

impl std::ops::IndexMut<AnsiColor> for Theme {
    /// Mutably access the color value for the ANSI color.
    fn index_mut(&mut self, index: AnsiColor) -> &mut Self::Output {
        &mut self.colors[2 + index as usize]
    }
}

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
// Color Matcher
// ====================================================================================================================

/// Conversion from high-resolution to terminal colors by exhaustive search
///
/// A color matcher converts [`Color`] objects to [`EightBitColor`] or
/// [`AnsiColor`] by comparing the original color to all 8-bit colors but the
/// ANSI colors or all ANSI colors, respectively, and returning the closest
/// matching color. Conversion to 8-bit colors does not consider the ANSI colors
/// because they become highly visible outliers when converting several
/// graduated colors.
///
/// To be meaningful, the search for the closest color is performed in a
/// perceptually uniform color space and uses high-resolution colors that are
/// equivalent to the 8-bit terminal colors, including the ANSI color values
/// provided by a [`Theme`]. This struct computes all necessary color
/// coordinates upon creation and caches them for its lifetime, which maximizes
/// the performance of conversion.
///
/// Since a color matcher incorporates the color values from a [`Theme`], an
/// application should regenerate its color matcher if the current theme
/// changes.
///
/// <style>
/// .color-swatch {
///     display: flex;
/// }
/// .color-swatch > div {
///     height: 4em;
///     width: 4em;
///     border: black 0.5pt solid;
///     display: flex;
///     align-items: center;
///     justify-content: center;
/// }
/// .python-only::before, .rust-only::before {
///     font-size: 0.8em;
///     display: inline-block;
///     border-radius: 0.5em;
///     padding: 0 0.6em;
///     font-family: -apple-system, BlinkMacSystemFont, avenir next, avenir, segoe ui,
///         helvetica neue, helvetica, Cantarell, Ubuntu, roboto, noto, arial, sans-serif;
///     font-weight: 600;
/// }
/// .python-only::before {
///     content: "Python only!";
///     background: #84c5fb;
/// }
/// .rust-only::before {
///     content: "Rust only!";
///     background: #f0ac84;
/// }
/// </style>
#[cfg_attr(feature = "pyffi", pyclass)]
#[derive(Debug)]
pub struct ColorMatcher {
    space: ColorSpace,
    ansi: Vec<[Float; 3]>,
    eight_bit: Vec<[Float; 3]>,
}

/// Create the coordinates for the 8-bit colors in the given color space.
#[inline]
fn eight_bit_coordinates(theme: &Theme, space: ColorSpace) -> (Vec<[Float; 3]>, Vec<[Float; 3]>) {
    let ansi = (0..=15)
        .map(|n| *theme[AnsiColor::try_from(n).unwrap()].to(space).as_ref())
        .collect();
    let eight_bit: Vec<[Float; 3]> = (16..=231)
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
        .collect();

    (ansi, eight_bit)
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl ColorMatcher {
    /// Create a new terminal color matcher. This method initializes the
    /// matcher's internal state, which comprises the coordinates for the 256
    /// 8-bit colors, 16 for the ANSI colors based on the provided theme, 216
    /// for the embedded RGB colors, and 24 for the gray gradient, all in the
    /// requested color space.
    #[cfg(feature = "pyffi")]
    #[new]
    pub fn new(theme: &Theme, ok_version: OkVersion) -> Self {
        let space = ok_version.cartesian_space();
        let (ansi, eight_bit) = eight_bit_coordinates(theme, space);

        Self {
            space,
            ansi,
            eight_bit,
        }
    }

    /// Create a new terminal color matcher. This method initializes the
    /// matcher's internal state, which comprises the coordinates for the 256
    /// 8-bit colors, 16 for the ANSI colors based on the provided theme, 216
    /// for the embedded RGB colors, and 24 for the gray gradient, all in the
    /// requested color space.
    #[cfg(not(feature = "pyffi"))]
    pub fn new(theme: &Theme, ok_version: OkVersion) -> Self {
        let space = ok_version.cartesian_space();
        let (ansi, eight_bit) = eight_bit_coordinates(theme, space);

        Self {
            space,
            ansi,
            eight_bit,
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
    /// # use prettypretty::{Color, ColorFormatError, ColorMatcher, ColorSpace};
    /// # use prettypretty::{DEFAULT_THEME, OkVersion};
    /// # use std::str::FromStr;
    /// let original_matcher = ColorMatcher::new(&DEFAULT_THEME, OkVersion::Original);
    ///
    /// let orange1 = Color::from_str("#ffa563")?;
    /// let ansi = original_matcher.to_ansi(&orange1);
    /// assert_eq!(u8::from(ansi), 7);
    ///
    /// let orange2 = Color::from_str("#ff9600")?;
    /// let ansi = original_matcher.to_ansi(&orange2);
    /// assert_eq!(u8::from(ansi), 9);
    /// // ---------------------------------------------------------------------
    /// let revised_matcher = ColorMatcher::new(&DEFAULT_THEME, OkVersion::Revised);
    ///
    /// let ansi = revised_matcher.to_ansi(&orange1);
    /// assert_eq!(u8::from(ansi), 7);
    ///
    /// let ansi = revised_matcher.to_ansi(&orange2);
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
    /// [`ColorMatcher::new`] does as well.
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
    pub fn to_ansi(&self, color: &Color) -> AnsiColor {
        use crate::core::{delta_e_ok, find_closest};

        let color = color.to(self.space);
        find_closest(color.as_ref(), &self.ansi, delta_e_ok)
            .map(|idx| AnsiColor::try_from(idx as u8).unwrap())
            .unwrap()
    }

    /// Find the 8-bit color that comes closest to the given color.
    ///
    /// # Examples
    ///
    /// The example below converts every color of the RGB cube embedded in 8-bit
    /// colors to a high-resolution color in sRGB, which is validated by the
    /// first two assertions, and then uses a color matcher to convert that
    /// color back to an embedded RGB color. The result is the original color,
    /// now wrapped as an 8-bit color, which is validated by the third
    /// assertion. The example demonstrates that the 216 colors in the embedded
    /// RGB cube still are closest to themselves after conversion to Oklrch.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace, DEFAULT_THEME, EightBitColor, Float};
    /// # use prettypretty::{EmbeddedRgb, OutOfBoundsError, ColorMatcher, OkVersion};
    /// let matcher = ColorMatcher::new(&DEFAULT_THEME, OkVersion::Revised);
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
    ///             let result = matcher.to_eight_bit(&color);
    ///             assert_eq!(result, EightBitColor::Rgb(embedded));
    ///         }
    ///     }
    /// }
    /// # Ok::<(), OutOfBoundsError>(())
    /// ```
    pub fn to_eight_bit(&self, color: &Color) -> EightBitColor {
        use crate::core::{delta_e_ok, find_closest};

        let color = color.to(self.space);
        find_closest(color.as_ref(), &self.eight_bit, delta_e_ok)
            .map(|idx| EightBitColor::from(idx as u8 + 16))
            .unwrap()
    }
}

// ====================================================================================================================

#[cfg(test)]
mod test {
    use super::{ColorMatcher, DEFAULT_THEME};
    use crate::{AnsiColor, Color, OkVersion, OutOfBoundsError};

    #[test]
    fn test_matcher() -> Result<(), OutOfBoundsError> {
        let matcher = ColorMatcher::new(&DEFAULT_THEME, OkVersion::Revised);

        let result = matcher.to_ansi(&Color::srgb(1.0, 1.0, 0.0));
        assert_eq!(result, AnsiColor::BrightYellow);

        Ok(())
    }
}
