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
pub(crate) use conversion::{convert, from_24bit_rgb, to_24bit_rgb};
pub use difference::HueInterpolation;
pub(crate) use difference::{
    delta_e_ok, find_closest, interpolate, normalize, prepare_to_interpolate, to_eq_bits,
};
pub(crate) use gamut::{clip, in_gamut, to_gamut};
pub use space::ColorSpace;
pub use string::ColorFormatError;
pub(crate) use string::{format, parse};
