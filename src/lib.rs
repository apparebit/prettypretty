//! # Oxidized Colors for Terminals
//!
//! This library brings 2020s color science to 1970s terminals to build good
//! looking and adaptable terminal user interfaces. It supports high-resolution
//! colors, accurate conversion between color spaces, gamut testing and mapping,
//! finding the closest matching color, and computing text contrast against the
//! background.
//!
//!
//! ## High-Resolution Colors
//!
//! High-resolution colors from the 2020s have floating point coordinates and
//! explicit color spaces:
//!
//!   * [`ColorSpace`] enumerates supported color spaces.
//!   * [`Color`] adds `f64` coordinates to precisely represent colors.
//!
//! The example below instantiates a color in the polar Oklch color space. It
//! then converts the color to Display P3 and tests whether it is in gamut—it
//! is. Next, it converts the color sRGB and tests whether it is in gamut—it is
//! not. Finally, it maps the color into sRGB's gamut.
//!
//! ```
//! # use prettypretty::{Color, ColorSpace};
//! let oklch = Color::oklch(0.716, 0.349, 335);
//! let p3 = oklch.to(ColorSpace::DisplayP3);
//! assert!(p3.in_gamut());
//!
//! let srgb = oklch.to(ColorSpace::Srgb);
//! assert!(!srgb.in_gamut());
//!
//! let mapped = srgb.map_to_gamut();
//! assert_eq!(mapped, Color::srgb(1, 0.15942348587138203, 0.9222706101768445));
//! ```
//! <style>
//! .color-swatch {
//!     display: flex;
//! }
//! .color-swatch > div {
//!     height: 4em;
//!     width: 4em;
//!     border: black 0.5pt solid;
//!     display: flex;
//!     align-items: center;
//!     justify-content: center;
//! }
//! </style>
//! <div class=color-swatch>
//! <div style="background-color: oklch(0.716 0.349 335);"></div>
//! <div style="background-color: #fff;"></div>
//! </div>
//!
//!
//! ## Terminal Colors
//!
//! Terminal color formats from the 1970s and 1980s have integer coordinates at
//! best may are represented through the following abstractions:
//!
//!   * [`EightBitColor`] combines [`AnsiColor`], [`EmbeddedRgb`], and
//!     [`GrayGradient`].
//!   * [`TrueColor`] represents 24-bit RGB colors, presumably in the sRGB color
//!     space.
//!
//! Given contemporary wide-gamut, high-dynamic-range (HDR) displays,
//! [`TrueColor`] is anything but true. The term's use in this crate reflects
//! ironic detachment as much as nostalgia.
//!
//!
//! ## Conversion Between Colors and Color Formats
//!
//! Even if we limit ourselves to 8-bit color and (not really) true color,
//! conversion between the terminal color formats is rather tricky. First, ANSI
//! colors have only names, but no color values. Second, mapping 16 million
//! colors to 240 or 16 colors is inherently and very noticeably lossy.
//!
//! Still, this crate does significantly better than previous libraries by
//! taking themes into account and by searching for closest matching colors in
//! perceptually uniform color space:
//!
//!   * [`Theme`] provides high-resolution color values for the 16 extended ANSI
//!     colors and terminal defaults.
//!   * [`ThemeBuilder`] helps to incrementally initialize a theme.
//!   * [`ColorMatcher`] stores high-resolution color values for all
//!     8-bit terminal colors to find closest matching color.
//!
//!
//! # BYOIO: Bring Your Own (Terminal) I/O
//!
//! Unlike the Python version, the Rust version of prettypretty does not (yet?)
//! include its own facilities for styled text or terminal I/O. Instead, it is
//! designed to be a lightweight addition that focuses on color management only.
//! To use this crate, an application must create its own instances of [`Theme`]
//! and [`ColorMatcher`]. While this crate contains one default theme,
//! surprisingly called [`DEFAULT_THEME`], that theme is suitable for tests but
//! no more.
//!
//! To fill in an accurate terminal theme, the application should use the ANSI
//! escape sequences
//! ```text
//! "{OSC}{10..=11};?{ST}"
//! ```
//! and
//! ```text
//! "{OSC}4;{0..=15};?{ST}"
//! ```
//! to query the terminal for its two default and 16 extended ANSI colors. The
//! responses are ANSI escape sequences with the exact same prefix as requests,
//! *before* the question mark, followed by the color in X Windows `rgb:`
//! format, followed by ST. Once you stripped the prefix and suffix from a
//! response, you can use the `FromStr` trait to parse the X Windows color
//! format into a color object.
//!
//! As usual, OSC stands for the character sequence `\x1b]` (escape, closing
//! square bracket) and ST stands for the character sequence `\x1b\\` (escape,
//! backslash). Some terminals answer with `\x0b` (bell) instead of ST.
//!
//!
//! # Color Swatches
//!
//! Not surprisingly, doctests with example code for using [`Color`] require
//! their own color objects. To make the examples more approachable, each code
//! block is followed by a color swatch showing the example's colors. Swatch
//! colors use the same color space (sRGB, Display P3, Oklab, or Oklch) where
//! possible and an equivalent color in another space where necessary (Oklrab
//! and Oklrch).


mod color;
mod serde;
mod term_color;
mod util;

pub use color::Color;
pub use color::ColorSpace;
pub use util::Coordinate;

pub use term_color::AnsiColor;
pub use term_color::EightBitColor;
pub use term_color::EmbeddedRgb;
pub use term_color::GrayGradient;
pub use term_color::TrueColor;

pub use serde::ColorFormatError;
pub use term_color::OutOfBoundsError;

// ====================================================================================================================
// Color Themes
// ====================================================================================================================

/// A color theme.
///
/// ANSI colors do not have intrinsic color values. However, a color theme does
/// have color values for the 16 extended ANSI colors as well as the foreground
/// and background default colors. By itself, a theme enables the conversion of
/// ANSI colors to high-resolution colors. Through a [`ColorMatcher`], a
/// theme also enables conversion of high-resolution colors ANSI (and possibly
/// 8-bit) colors.
#[derive(Clone, Debug)]
pub struct Theme {
    foreground: Color,
    background: Color,
    black: Color,
    red: Color,
    green: Color,
    yellow: Color,
    blue: Color,
    magenta: Color,
    cyan: Color,
    white: Color,
    bright_black: Color,
    bright_red: Color,
    bright_green: Color,
    bright_yellow: Color,
    bright_blue: Color,
    bright_magenta: Color,
    bright_cyan: Color,
    bright_white: Color,
}

impl Theme {
    /// Access the theme's foreground color.
    pub const fn foreground(&self) -> &Color {
        &self.foreground
    }

    /// Access the theme's background color.
    pub const fn background(&self) -> &Color {
        &self.background
    }

    // Access the theme's ANSI colors.
    pub const fn ansi(&self, value: AnsiColor) -> &Color {
        use AnsiColor::*;

        match value {
            Black => &self.black,
            Red => &self.red,
            Green => &self.green,
            Yellow => &self.yellow,
            Blue => &self.blue,
            Magenta => &self.magenta,
            Cyan => &self.cyan,
            White => &self.white,
            BrightBlack => &self.bright_black,
            BrightRed => &self.bright_red,
            BrightGreen => &self.bright_green,
            BrightYellow => &self.bright_yellow,
            BrightBlue => &self.bright_blue,
            BrightMagenta => &self.bright_magenta,
            BrightCyan => &self.bright_cyan,
            BrightWhite => &self.bright_white,
        }
    }
}

/// The default theme.
///
/// This theme exists to demonstrate the functionality enabled by themes as well
/// as for testing. It uses the colors of [VGA text
/// mode](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit).
pub const DEFAULT_THEME: Theme = Theme {
    foreground: Color::new(ColorSpace::Srgb, 0.0, 0.0, 0.0),
    background: Color::new(ColorSpace::Srgb, 1.0, 1.0, 1.0),
    black: Color::new(ColorSpace::Srgb, 0.0, 0.0, 0.0),
    red: Color::new(ColorSpace::Srgb, 0.666666666666667, 0.0, 0.0),
    green: Color::new(ColorSpace::Srgb, 0.0, 0.666666666666667, 0.0),
    yellow: Color::new(ColorSpace::Srgb, 0.666666666666667, 0.333333333333333, 0.0),
    blue: Color::new(ColorSpace::Srgb, 0.0, 0.0, 0.666666666666667),
    magenta: Color::new(ColorSpace::Srgb, 0.666666666666667, 0.0, 0.666666666666667),
    cyan: Color::new(ColorSpace::Srgb, 0.0, 0.666666666666667, 0.666666666666667),
    white: Color::new(ColorSpace::Srgb, 0.666666666666667, 0.666666666666667, 0.666666666666667),
    bright_black: Color::new(ColorSpace::Srgb, 0.333333333333333, 0.333333333333333, 0.333333333333333),
    bright_red: Color::new(ColorSpace::Srgb, 1.0, 0.333333333333333, 0.333333333333333),
    bright_green: Color::new(ColorSpace::Srgb, 0.333333333333333, 1.0, 0.333333333333333),
    bright_yellow: Color::new(ColorSpace::Srgb, 1.0, 1.0, 0.333333333333333),
    bright_blue: Color::new(ColorSpace::Srgb, 0.333333333333333, 0.333333333333333, 1.0),
    bright_magenta: Color::new(ColorSpace::Srgb, 1.0, 0.333333333333333, 1.0),
    bright_cyan: Color::new(ColorSpace::Srgb, 0.333333333333333, 1.0, 1.0),
    bright_white: Color::new(ColorSpace::Srgb, 1.0, 1.0, 1.0),
};

/// An incremental theme builder.
#[derive(Clone, Debug)]
pub struct ThemeBuilder {
    foreground: Option<Color>,
    background: Option<Color>,
    black: Option<Color>,
    red: Option<Color>,
    green: Option<Color>,
    yellow: Option<Color>,
    blue: Option<Color>,
    magenta: Option<Color>,
    cyan: Option<Color>,
    white: Option<Color>,
    bright_black: Option<Color>,
    bright_red: Option<Color>,
    bright_green: Option<Color>,
    bright_yellow: Option<Color>,
    bright_blue: Option<Color>,
    bright_magenta: Option<Color>,
    bright_cyan: Option<Color>,
    bright_white: Option<Color>,
}

impl ThemeBuilder {
    /// Create a new theme builder.
    pub fn new() -> Self {
        Self {
            foreground: None,
            background: None,
            black: None,
            red: None,
            green: None,
            yellow: None,
            blue: None,
            magenta: None,
            cyan: None,
            white: None,
            bright_black: None,
            bright_red: None,
            bright_green: None,
            bright_yellow: None,
            bright_blue: None,
            bright_magenta: None,
            bright_cyan: None,
            bright_white: None,
        }
    }

    /// Update the default foreground color.
    pub fn foreground(self, value: Color) -> Self {
        Self {
            foreground: Some(value),
            ..self
        }
    }

    /// Update the default background color.
    pub fn background(self, value: Color) -> Self {
        Self {
            background: Some(value),
            ..self
        }
    }

    pub fn with_ansi_color(self, term: AnsiColor, value: Color) -> Self {
        use AnsiColor::*;

        match term {
            Black => Self { black: Some(value), .. self },
            Red => Self { red: Some(value), .. self },
            Green => Self { green: Some(value), .. self },
            Yellow => Self { yellow: Some(value), .. self },
            Blue => Self { blue: Some(value), .. self },
            Magenta => Self { magenta: Some(value), .. self },
            Cyan => Self { cyan: Some(value), .. self },
            White => Self { white: Some(value), .. self },
            BrightBlack => Self { bright_black: Some(value), .. self },
            BrightRed => Self { bright_red: Some(value), .. self },
            BrightGreen => Self { bright_green: Some(value), .. self },
            BrightYellow => Self { bright_yellow: Some(value), .. self },
            BrightBlue => Self { bright_blue: Some(value), .. self },
            BrightMagenta => Self { bright_magenta: Some(value), .. self },
            BrightCyan => Self { bright_cyan: Some(value), .. self },
            BrightWhite => Self { bright_white: Some(value), .. self },
        }
    }



    /// Update the color value for black.
    pub fn black(self, value: Color) -> Self {
        Self {
            black: Some(value),
            ..self
        }
    }

    /// Update the color value for red.
    pub fn red(self, value: Color) -> Self {
        Self {
            red: Some(value),
            ..self
        }
    }

    /// Update the color value for green.
    pub fn green(self, value: Color) -> Self {
        Self {
            green: Some(value),
            ..self
        }
    }

    /// Update the color value for yellow.
    pub fn yellow(self, value: Color) -> Self {
        Self {
            yellow: Some(value),
            ..self
        }
    }

    /// Update the color value for blue.
    pub fn blue(self, value: Color) -> Self {
        Self {
            blue: Some(value),
            ..self
        }
    }

    /// Update the color value for magenta.
    pub fn magenta(self, value: Color) -> Self {
        Self {
            magenta: Some(value),
            ..self
        }
    }

    /// Update the color value for cyan.
    pub fn cyan(self, value: Color) -> Self {
        Self {
            cyan: Some(value),
            ..self
        }
    }

    /// Update the color value for white.
    pub fn white(self, value: Color) -> Self {
        Self {
            white: Some(value),
            ..self
        }
    }

    /// Update the color value for bright black.
    pub fn bright_black(self, value: Color) -> Self {
        Self {
            bright_black: Some(value),
            ..self
        }
    }

    /// Update the color value for bright red.
    pub fn bright_red(self, value: Color) -> Self {
        Self {
            bright_red: Some(value),
            ..self
        }
    }

    /// Update the color value for bright green.
    pub fn bright_green(self, value: Color) -> Self {
        Self {
            bright_green: Some(value),
            ..self
        }
    }

    /// Update the color value for bright yellow.
    pub fn bright_yellow(self, value: Color) -> Self {
        Self {
            bright_yellow: Some(value),
            ..self
        }
    }

    /// Update the color value for bright blue.
    pub fn bright_blue(self, value: Color) -> Self {
        Self {
            bright_blue: Some(value),
            ..self
        }
    }

    /// Update the color value for bright magenta.
    pub fn bright_magenta(self, value: Color) -> Self {
        Self {
            bright_magenta: Some(value),
            ..self
        }
    }

    /// Update the color value for bright cyan.
    pub fn bright_cyan(self, value: Color) -> Self {
        Self {
            bright_cyan: Some(value),
            ..self
        }
    }

    /// Update the color value for bright white.
    pub fn bright_white(self, value: Color) -> Self {
        Self {
            bright_white: Some(value),
            ..self
        }
    }

    /// Determine whether this theme builder is ready, i.e., all fields have
    /// some color value. Admittedly, this method is next to useless without
    /// some interface for finding out what colors are still missing. That's why
    /// it is not public for now.
    fn ready(&self) -> bool {
        self.black.is_some()
            && self.red.is_some()
            && self.green.is_some()
            && self.yellow.is_some()
            && self.blue.is_some()
            && self.magenta.is_some()
            && self.cyan.is_some()
            && self.white.is_some()
            && self.bright_black.is_some()
            && self.bright_red.is_some()
            && self.bright_green.is_some()
            && self.bright_yellow.is_some()
            && self.bright_blue.is_some()
            && self.bright_magenta.is_some()
            && self.bright_cyan.is_some()
            && self.bright_white.is_some()
    }

    /// Build the theme. If any color is missing, this builder vanishes into
    /// `None` and the application needs to start from scratch, with a new
    /// builder again. As an escape hatch during development, theme builder
    /// *does* support `clone()`.
    pub fn build(self) -> Option<Theme> {
        if !self.ready() {
            None
        } else {
            Some(Theme {
                foreground: self.foreground.unwrap(),
                background: self.background.unwrap(),
                black: self.black.unwrap(),
                red: self.red.unwrap(),
                green: self.green.unwrap(),
                yellow: self.yellow.unwrap(),
                blue: self.blue.unwrap(),
                magenta: self.magenta.unwrap(),
                cyan: self.cyan.unwrap(),
                white: self.white.unwrap(),
                bright_black: self.bright_black.unwrap(),
                bright_red: self.bright_red.unwrap(),
                bright_green: self.bright_green.unwrap(),
                bright_yellow: self.bright_yellow.unwrap(),
                bright_blue: self.bright_blue.unwrap(),
                bright_magenta: self.bright_magenta.unwrap(),
                bright_cyan: self.bright_cyan.unwrap(),
                bright_white: self.bright_white.unwrap(),
            })
        }
    }
}

// https://stackoverflow.com/questions/74085531/alternative-to-static-mut-and-unsafe-while-managing-global-application-state

// --------------------------------------------------------------------------------------------------------------------

impl From<TrueColor> for Color {
    /// Convert the "true" color object into a *true* color object... 🤪
    fn from(value: TrueColor) -> Color {
        let [r, g, b] = *value.coordinates();
        Color::srgb(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0)
    }
}

impl From<EmbeddedRgb> for Color {
    /// Instantiate a new color from the embedded RGB value.
    fn from(value: EmbeddedRgb) -> Color {
        TrueColor::from(value).into()
    }
}

impl From<GrayGradient> for Color {
    /// Instantiate a new color from the embedded RGB value.
    fn from(value: GrayGradient) -> Color {
        TrueColor::from(value).into()
    }
}

// ====================================================================================================================
// Color Matcher
// ====================================================================================================================

/// A state container for matching terminal colors.
///
/// A color matcher owns the 256 color objects necessary for high-quality
/// conversion from arbitrary instances of [`Color`] to 8-bit or ANSI colors.
/// Conversion to 8-bit colors does *not* consider the 16 extended ANSI colors
/// as candidates because they become highly visible outliers when matching
/// several graduated colors.
///
/// Every color matcher instance incorporates the colors from the theme passed
/// to its constructor. Hence, if the theme changes, so should the color
/// matcher.
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
/// </style>
#[derive(Debug)]
pub struct ColorMatcher {
    ansi: Vec<Color>,
    eight_bit: Vec<Color>,
}

impl ColorMatcher {
    /// Create a new terminal color matcher. This method initializes the
    /// internal state, which comprises 256 color objects, 16 for the ANSI
    /// colors (based on the theme), 216 for the embedded RGB colors, and 24 for
    /// the gray gradient colors.
    pub fn new(theme: &Theme) -> Self {
        let ansi = (0..=15)
            .into_iter()
            .map(|n| {
                theme
                    .ansi(AnsiColor::try_from(n).unwrap())
                    .to(ColorSpace::Oklrab)
            })
            .collect();

        let eight_bit: Vec<Color> = (16..=231)
            .into_iter()
            .map(|n| Color::from(EmbeddedRgb::try_from(n).unwrap()).to(ColorSpace::Oklrab))
            .chain(
                (232..=255)
                    .into_iter()
                    .map(|n| Color::from(GrayGradient::try_from(n).unwrap()).to(ColorSpace::Oklrab)),
            )
            .collect();

        Self { ansi, eight_bit }
    }

    /// Find the ANSI color that comes closest to the given color.
    ///
    ///
    /// # Example
    ///
    /// The example code below matches `#ffa563` and `#ff9600` to ANSI colors
    /// under the default theme. The first color matches ANSI cyan, which is a
    /// very poor fit and demonstrates that even high-resolution, perceptually
    /// uniform colors cannot make up for the extremely limited choices. It also
    /// suggests that, maybe, finding matches in polar coordinates may be
    /// preferable for ANSI colors, since it can prioritize hues over chroma.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorFormatError, ColorMatcher, ColorSpace};
    /// # use prettypretty::{DEFAULT_THEME};
    /// # use std::str::FromStr;
    /// let matcher = ColorMatcher::new(&DEFAULT_THEME);
    ///
    /// let color = Color::from_str("#ffa563")?;
    /// let ansi = matcher.to_ansi(&color);
    /// assert_eq!(u8::from(ansi), 7);
    ///
    /// let color = Color::from_str("#ff9600")?;
    /// let ansi = matcher.to_ansi(&color);
    /// assert_eq!(u8::from(ansi), 9);
    /// # Ok::<(), ColorFormatError>(())
    /// ```
    /// <div class=color-swatch>
    /// <div style="background-color: #ffa563;"></div>
    /// <div style="background-color: #00aaaa;"></div>
    /// <div style="background-color: #ff9600;"></div>
    /// <div style="background-color: #ff5555;"></div>
    /// </div>
    pub fn to_ansi(&self, color: &Color) -> AnsiColor {
        // SAFETY: self.ansi holds 16 elements, hence closest() returns index 0..=15.
        color
            .closest(&self.ansi)
            .map(|idx| AnsiColor::try_from(idx as u8))
            .unwrap()
            .unwrap()
    }

    /// Find the 8-bit color that comes closest to the given color.
    ///
    ///
    /// # Example
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
    /// # use prettypretty::{Color, ColorSpace, DEFAULT_THEME, EightBitColor};
    /// # use prettypretty::{EmbeddedRgb, OutOfBoundsError, ColorMatcher};
    /// # use prettypretty::Coordinate::C1;
    /// let matcher = ColorMatcher::new(&DEFAULT_THEME);
    ///
    /// for r in 0..5 {
    ///     for g in 0..5 {
    ///         for b in 0..5 {
    ///             let embedded = EmbeddedRgb::new(r, g, b)?;
    ///             let color = Color::from(embedded);
    ///             assert_eq!(color.space(), ColorSpace::Srgb);
    ///
    ///             let c1 = (55.0 + 40.0 * (r as f64)) / 255.0;
    ///             assert!((color[C1] - c1).abs() < f64::EPSILON);
    ///
    ///             let result = matcher.to_eight_bit(&color);
    ///             assert_eq!(result, EightBitColor::Rgb(embedded));
    ///         }
    ///     }
    /// }
    /// # Ok::<(), OutOfBoundsError>(())
    /// ```
    pub fn to_eight_bit(&self, color: &Color) -> EightBitColor {
        // SAFETY: self.eight_bit holds 240 elements, hence closest() returns
        // index 0..=239, which becomes 16..=255 after addition.
        color
            .closest(&self.eight_bit)
            .map(|idx| EightBitColor::from(idx as u8 + 16))
            .unwrap()
    }
}

// ====================================================================================================================

#[cfg(test)]
mod test {
    use super::{AnsiColor, Color, DEFAULT_THEME, OutOfBoundsError, ColorMatcher};

    #[test]
    fn test_matcher() -> Result<(), OutOfBoundsError> {
        let matcher = ColorMatcher::new(&DEFAULT_THEME);

        let result = matcher.to_ansi(&Color::srgb(1.0, 1.0, 0.0));
        assert_eq!(result, AnsiColor::BrightYellow);

        Ok(())
    }
}
