//! # Oxidized colors for terminals
//!
//! This library supports terminal color formats through [`EightBitColor`] and
//! [`TrueColor`] and arbitrary but color-spaced colors through [`Color`]. The
//! 8-bit color format, in turn, comprises three more formats, [`AnsiColor`],
//! [`EmbeddedRgb`], and [`GrayGradient`], each of which has its own code range
//! amongst unsigned 8-bit numbers. All four can easily be converted from and to
//! `u8`.
//!
//!
//! # From Color Formats to Colors
//!
//! When converting color formats to color objects, this crate uses established
//! formulae for [`EmbeddedRgb`] and [`GrayGradient`]. However, unlike these
//! formats and color objects, ANSI colors have names but no intrinsic color
//! values. Consequently, to convert ANSI colors to instances of [`Color`], this
//! crate relies on a global theme providing the color values.
//!
//!
//! # From Colors to Color Formats
//!
//! Meanwhile, to perform high-quality conversions from arbitrary colors to the
//! 8-bit color format, this crate searches for the closest color amongst 8-bit
//! colors. Candidates are *all* colors from [`EmbeddedRgb`] and
//! [`GrayGradient`] when converting to 8-bit colors and *all* 16 extended
//! [`AnsiColor`] when converting to ANSI colors. The ANSI colors are *not*
//! considered when converting to 8-bit colors, even though they take up the
//! first 16 code points, because they tend to stick out and disrupt any
//! gradation.

use std::sync::{Mutex, MutexGuard};

mod color;
mod format;
mod util;

pub use color::Color;
pub use color::ColorSpace;
pub use color::Coordinate;

pub use format::AnsiColor;
pub use format::EightBitColor;
pub use format::EmbeddedRgb;
pub use format::GrayGradient;
pub use format::OutOfBoundsError;
pub use format::TrueColor;

/// A color theme.
///
/// ANSI colors do not have intrinsic color values, so we provide them through
/// the [`current_theme`]. In addition to the 16 extended ANSI colors, a theme
/// includes two more colors for the foreground and background defaults.
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
    pub fn foreground(&self) -> &Color {
        &self.foreground
    }

    /// Access the theme's background color.
    pub fn background(&self) -> &Color {
        &self.background
    }

    // Access the theme's ANSI colors.
    pub fn ansi(&self, value: AnsiColor) -> &Color {
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
        *theme.ansi(value)
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

// --------------------------------------------------------------------------------------------------------------------

// struct TerminalColorConverter {
//     candidates: [Color; 216 + 24]
// }

// impl TerminalColorConverter {
//     pub fn new() -> Self {
//         Self
//     }
// }
