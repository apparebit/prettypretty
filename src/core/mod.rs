#[cfg(test)]
mod test_util;

mod contrast;
mod conversion;
mod difference;
mod gamut;
mod space;
mod string;

pub(crate) use contrast::{
    scale_lightness, to_contrast, to_contrast_luminance_p3, to_contrast_luminance_srgb,
};
pub(crate) use conversion::{convert, from_24bit, to_24bit};
pub use difference::HueInterpolation;
pub(crate) use difference::{
    delta_e_ok, find_closest, interpolate, prepare_to_interpolate, to_eq_bits,
};
pub(crate) use gamut::{clip, in_gamut, to_gamut};
pub(crate) use space::normalize;
pub use space::ColorSpace;
pub use string::ColorFormatError;
pub(crate) use string::{format, parse};
