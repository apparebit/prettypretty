mod contrast;
mod conversion;
mod difference;
mod equality;
mod gamut;
mod math;
mod space;
mod string;

// contrast
pub(crate) use contrast::{
    scale_lightness, to_contrast, to_contrast_luminance_p3, to_contrast_luminance_srgb,
};

// conversion
pub(crate) use conversion::{convert, from_24bit, to_24bit};

// difference
pub use difference::HueInterpolation;
pub(crate) use difference::{delta_e_ok, find_closest, interpolate, prepare_to_interpolate};

// equality
#[cfg(test)]
pub(crate) use equality::assert_same_coordinates;
#[cfg(feature = "pyffi")]
pub use equality::close_enough;
pub use equality::to_eq_bits;
pub(crate) use equality::{is_achromatic, is_achromatic_chroma_hue, normalize, to_eq_coordinates};

// gamut
pub(crate) use gamut::{clip, in_gamut, to_gamut};
#[cfg(feature = "gamut")]
pub use gamut::{GamutTraversal, GamutTraversalStep};

// math
#[cfg(all(feature = "gamut", test))]
pub(crate) use math::sum::Sum;
#[cfg(feature = "gamut")]
pub(crate) use math::sum::ThreeSum;
pub(crate) use math::FloatExt;

// space
pub use space::ColorSpace;

// string
pub(crate) use string::{format, parse};
