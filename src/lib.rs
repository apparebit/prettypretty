//! # Oxidized colors for terminals
//!
//! This library brings 2020s color science to 1970s terminals to build good
//! looking and adaptable terminal user interfaces. It supports high-resolution
//! colors, accurate conversion between color spaces, gamut testing and mapping,
//! finding the closest matching color, and computing text contrast against the
//! background.
//!
//! The 2020s are represented through two abstractions:
//!
//!   * [`ColorSpace`] enumerates supported color spaces
//!   * [`Color`] adds `f64` coordinates to precisely represent colors
//!
//! The 1970s (and subsequent decades) are represented with more limited
//! terminal color formats:
//!
//!   * [`EightBitColor`] combines [`AnsiColor`], [`EmbeddedRgb`], and
//!     [`GrayGradient`]
//!   * [`TrueColor`] represents 24-bit RGB colors; the historical term
//!     is a clear misnomer
//!
//! Two abstractions facilitate high-quality conversion between terminal color
//! formats and high-resolution colors:
//!
//!   * [`Theme`] provides high-resolution color values for the 16 extended ANSI
//!     colors and the terminal defaults
//!   * [`TerminalColorConverter`] keeps the state for converting high-resolution
//!     to terminal colors
//!
//!

use std::ops::RangeInclusive;
use std::sync::{Mutex, MutexGuard};

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
// Color Theme
// ====================================================================================================================

/// A color theme.
///
/// ANSI colors do not have intrinsic color values, so we provide them through
/// the [`current_theme`]. In addition to the 16 extended ANSI colors, a theme
/// includes two more colors for the foreground and background defaults.
#[derive(Clone, Debug)]
pub struct Theme {
    #[allow(dead_code)]
    foreground: Color,
    #[allow(dead_code)]
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
/// This theme exists to provide a well-defined initial value for the current
/// theme. It uses the colors of VGA text mode.
const DEFAULT_THEME: Theme = Theme {
    foreground: Color::srgb(0.0, 0.0, 0.0),
    background: Color::srgb(1.0, 1.0, 1.0),
    black: Color::srgb(0.0, 0.0, 0.0),
    red: Color::srgb(0.666666666666667, 0.0, 0.0),
    green: Color::srgb(0.0, 0.666666666666667, 0.0),
    yellow: Color::srgb(0.666666666666667, 0.333333333333333, 0.0),
    blue: Color::srgb(0.0, 0.0, 0.666666666666667),
    magenta: Color::srgb(0.666666666666667, 0.0, 0.666666666666667),
    cyan: Color::srgb(0.0, 0.666666666666667, 0.666666666666667),
    white: Color::srgb(0.666666666666667, 0.666666666666667, 0.666666666666667),
    bright_black: Color::srgb(0.333333333333333, 0.333333333333333, 0.333333333333333),
    bright_red: Color::srgb(1.0, 0.333333333333333, 0.333333333333333),
    bright_green: Color::srgb(0.333333333333333, 1.0, 0.333333333333333),
    bright_yellow: Color::srgb(1.0, 1.0, 0.333333333333333),
    bright_blue: Color::srgb(0.333333333333333, 0.333333333333333, 1.0),
    bright_magenta: Color::srgb(1.0, 0.333333333333333, 1.0),
    bright_cyan: Color::srgb(0.333333333333333, 1.0, 1.0),
    bright_white: Color::srgb(1.0, 1.0, 1.0),
};

// https://stackoverflow.com/questions/74085531/alternative-to-static-mut-and-unsafe-while-managing-global-application-state

static THEME: Mutex<Theme> = Mutex::new(DEFAULT_THEME);

/// Provide thread-safe access to the current theme, which is global state.
pub fn current_theme() -> MutexGuard<'static, Theme> {
    THEME.lock().unwrap()
}

// --------------------------------------------------------------------------------------------------------------------

impl From<TrueColor> for Color {
    /// Convert the "true" color object into a *true* color object... 🤪
    fn from(value: TrueColor) -> Color {
        let [r, g, b] = *value.coordinates();
        Color::srgb((r as f64) / 255.0, (g as f64) / 255.0, (b as f64) / 255.0)
    }
}

impl From<AnsiColor> for Color {
    /// Convert the ANSI color into a color object.
    ///
    /// Since ANSI colors do not have any standardized or intrinsic color
    /// values, this conversion uses the corresponding color from the current
    /// color theme.
    fn from(value: AnsiColor) -> Color {
        let theme = current_theme();
        // From<EmbeddedRgb> and From<GrayGradient> create a new color objects.
        // We do the same here, just with an explicit clone().
        theme.ansi(value).clone()
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

impl From<EightBitColor> for Color {
    /// Instantiate a new color from the 8-bit terminal color.
    fn from(value: EightBitColor) -> Color {
        match value {
            EightBitColor::Ansi(color) => Color::from(color),
            EightBitColor::Rgb(color) => Color::from(color),
            EightBitColor::Gray(color) => Color::from(color),
        }
    }
}

// ====================================================================================================================
// Terminal Color Converter
// ====================================================================================================================

/// A state container for converting colors
///
/// A terminal color converter owns the 256 color objects necessary for high
/// quality conversions from [`Color`] to terminals' 8-bit or ANSI colors. The
/// color theme current at creation time determines the color values for the
/// ANSI colors.
#[derive(Debug)]
pub struct TerminalColorConverter {
    ansi: Vec<Color>,
    eight_bit: Vec<Color>,
}

impl TerminalColorConverter {
    /// Create a new terminal color converter. This method initializes the
    /// internal state, which comprises 256 color objects, one each for every
    /// 8-bit color.
    pub fn new() -> Self {
        fn make_colors(range: RangeInclusive<u8>) -> Vec<Color> {
            range
                .into_iter()
                .map(|n| Color::from(EightBitColor::from(n)))
                .collect()
        }

        Self {
            ansi: make_colors(0..=15),
            eight_bit: make_colors(16..=255),
        }
    }

    /// Find the ANSI color that comes closest to the given color.
    pub fn to_ansi(&self, color: &Color) -> AnsiColor {
        // The first unwrap() is safe because there is at least one candidate.
        // The second unwrap() is safe because there are at most 16 candidates.
        AnsiColor::try_from(color.closest(&self.ansi).unwrap() as u8).unwrap()
    }

    /// Find the 8-bit color that comes closest to the given color.
    pub fn to_eight_bit(&self, color: &Color) -> EightBitColor {
        // The unwrap() is safe because there is at least one candidate.
        EightBitColor::new((color.closest(&self.eight_bit).unwrap() as u8) + 16)
    }
}

// ====================================================================================================================

#[cfg(test)]
mod test {
    use super::{AnsiColor, Color, TerminalColorConverter};

    #[test]
    fn test_converter() {
        let converter = TerminalColorConverter::new();
        let ansi = converter.to_ansi(&Color::srgb(1.0, 1.0, 0.0));
        assert_eq!(ansi, AnsiColor::BrightYellow);
    }
}
