//! State and algorithms for the translation between high- and low-resolution
//! colors.

mod hue_lightness;
mod theme;
mod translator;

pub(crate) use hue_lightness::HueLightnessTable;
pub use theme::{Theme, ThemeEntry, ThemeEntryIterator, VGA_COLORS};
pub use translator::Translator;
