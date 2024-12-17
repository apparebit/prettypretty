//! State and algorithms for the translation between high- and low-resolution
//! colors.

mod hue_lightness;
mod translator;

pub(crate) use hue_lightness::HueLightnessTable;
pub use translator::Translator;
