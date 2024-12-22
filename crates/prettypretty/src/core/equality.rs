#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use super::ColorSpace;
use crate::core::{conversion::okxab_to_okxch, convert, FloatExt};
use crate::{Bits, Float};

/// Test macro for asserting the equality of floating point numbers.
///
/// This macro relies on [`to_eq_bits`] to normalize the two floating point
/// numbers by zeroing out not-a-numbers, reducing resolution, and dropping the
/// sign of negative zeros and then compares the resulting bit strings.
///
/// # Panics
///
/// This macro panics if the normalized bit strings are not identical. Its
/// message places the numbers below each other at the beginning of subsequent
/// lines for easy comparability.
#[macro_export]
macro_rules! assert_close_enough {
    ($f1:expr, $f2:expr $(,)?) => {
        let (f1, f2) = ($f1, $f2);
        let bits1 = $crate::to_eq_bits(f1);
        let bits2 = $crate::to_eq_bits(f2);
        assert_eq!(bits1, bits2, "quantities differ:\n{:?}\n{:?}", f1, f2);
    };
}

/// Test macro for asserting that two color coordinate slices describe the same
/// color.
///
/// Given a color space and two coordinate arrays, this macro normalizes the
/// coordinates by zeroing out not-a-numbers, clamping the a/b/c components of
/// Oklab colors, scaling the hue of Oklch/Oklrch, reducing resolution, and
/// dropping the sign of negative zeros before comparing the resulting bit
/// strings.
///
/// # Panics
///
/// This macro panics if the normalized bit strings are not identical. Its
/// message places the coordinates below each other at the beginning of
/// subsequent lines for easy comparability.
#[cfg(test)]
macro_rules! assert_same_coordinates {
    ($space:expr , $cs1:expr , $cs2:expr $(,)?) => {
        let (space, cs1, cs2) = ($space, $cs1, $cs2);
        let bits1 = $crate::core::to_eq_coordinates(space, cs1);
        let bits2 = $crate::core::to_eq_coordinates(space, cs2);
        assert_eq!(
            bits1, bits2,
            "color coordinates differ:\n{:?}\n{:?}",
            cs1, cs2
        );
    };
}

#[cfg(test)]
pub(crate) use assert_same_coordinates;

/// Test macro for asserting the equality of colors.
///
/// This macro tests the color objects for equality using the `Eq` trait. The
/// implementation, in turn, normalizes the coordinates of colors with the same
/// color space by zeroing out not-a-numbers, clamping the a/b/c components of
/// Oklab colors, scaling the hue of Oklch/Oklrch, reducing resolution, and
/// dropping the sign of negative zeros before comparing the resulting bit
/// strings.
///
/// # Panics
///
/// This macro panics if the normalized bit strings are not identical. Its
/// message places either color spaces or the coordinates below each other at
/// the beginning of subsequent lines for easy comparability.
#[macro_export]
macro_rules! assert_same_color {
    ($c1:expr, $c2:expr $(,)?) => {
        let (c1, c2) = ($c1, $c2);
        if c1.space() != c2.space() {
            assert_eq!(
                c1,
                c2,
                "color spaces differ:\n{:?}\n{:?}",
                c1.space(),
                c2.space()
            );
        }

        assert_eq!(
            c1,
            c2,
            "color coordinates differ:\n{:?}\n{:?}",
            c1.as_ref(),
            c2.as_ref()
        );
    };
}

// --------------------------------------------------------------------------------------------------------------------

/// Normalize the color coordinates.
///
/// This function ensures that coordinates are well-formed. In particular, it
/// replaces not-a-number coordinates with zero. For the Oklab variations, it
/// also ensures that (revised) lightness is in `0..=1` and chroma is in `0..`.
/// For semantic consistency, if the hue in Oklch/Oklrch is not-a-number, it
/// also replaces chroma with zero.
#[inline]
pub(crate) fn normalize(space: ColorSpace, coordinates: &[Float; 3]) -> [Float; 3] {
    let [mut c1, mut c2, mut c3] = *coordinates;

    if c1.is_nan() {
        c1 = 0.0;
    }
    if c2.is_nan() {
        c2 = 0.0;
    }
    if c3.is_nan() {
        c3 = 0.0;
        if space.is_polar() {
            c2 = 0.0;
        }
    }

    if space.is_ok() {
        c1 = c1.clamp(0.0, 1.0);
        if space.is_polar() {
            c2 = c2.max(0.0);
        }
    }

    [c1, c2, c3]
}

/// Normalize coordinates for equality testing and hashing.
#[must_use = "function returns new color coordinates and does not mutate original value"]
pub(crate) fn to_eq_coordinates(space: ColorSpace, coordinates: &[Float; 3]) -> [Bits; 3] {
    // Zero out not-a-numbers and clamp Oklab's a/b/c.
    let [mut c1, mut c2, mut c3] = normalize(space, coordinates);

    // Normalize rotation and scale to unit range.
    if space.is_polar() {
        c3 = c3.rem_euclid(360.0) / 360.0
    }

    // Reduce precision.
    let factor = <Float as FloatExt>::ROUNDING_FACTOR;
    c1 = (c1 * factor).round();
    c2 = (c2 * factor).round();
    c3 = (c3 * factor).round();

    // Prevent too much negativity.
    if c1 == -0.0 {
        c1 = 0.0;
    }
    if c2 == -0.0 {
        c2 = 0.0
    }
    if c3 == -0.0 {
        c3 = 0.0
    }

    [c1.to_bits(), c2.to_bits(), c3.to_bits()]
}

// --------------------------------------------------------------------------------------------------------------------

/// Determine whether the two floats are close enough to be considered equal.
/// <i class=python-only>Python only!</i>
///
/// This function relies on [`to_eq_bits`] to normalize the two floating point
/// numbers by zeroing out not-a-numbers, reducing resolution, and dropping the
/// sign of negative zeros and then compares the resulting bit strings. It
/// **must not** be used for testing color coordinates; they require additional
/// normalization steps, as implemented by [`Color::eq`](crate::Color::eq).
#[cfg(feature = "pyffi")]
#[pyfunction]
pub fn close_enough(f1: Float, f2: Float) -> bool {
    to_eq_bits(f1) == to_eq_bits(f2)
}

/// Helper function to normalize a floating point number before hashing or
/// equality testing.
///
/// This function zeros out not-a-number, reduces significant digits after the
/// decimal, and drops the sign of negative zero and returns the result as a bit
/// string. It is only public because the [`assert_close_enough`] test macro
/// uses it.
#[doc(hidden)]
#[inline]
pub fn to_eq_bits(f: Float) -> Bits {
    // Eliminate not-a-number.
    let mut f = if f.is_nan() { 0.0 } else { f };

    // Reduce precision.
    f = (<Float as FloatExt>::ROUNDING_FACTOR * f).round();

    // Too much negativity!
    if f == -0.0 {
        f = 0.0
    }

    f.to_bits()
}

// --------------------------------------------------------------------------------------------------------------------

/// Determine whether the color is achromatic or gray-ish.
///
/// This function determines whether hue is not-a-number or chroma is smaller
/// than or equal to the given threshold in Oklch/Oklrch, converting the
/// coordinates if necessary.
pub(crate) fn is_achromatic(space: ColorSpace, coordinates: &[Float; 3], threshold: Float) -> bool {
    let coordinates = match space {
        ColorSpace::Oklch | ColorSpace::Oklrch => *coordinates,
        ColorSpace::Oklrab => okxab_to_okxch(coordinates),
        _ => convert(space, ColorSpace::Oklch, coordinates),
    };

    is_achromatic_chroma_hue(coordinates[1], coordinates[2], threshold)
}

/// Determine whether the chroma and hue are gray-ish.
///
/// This function treats the chroma and hue as gray-ish if either the hue is
/// not-a-number or the chroma is smaller than or equal to the given threshold.
#[inline]
pub(crate) fn is_achromatic_chroma_hue(chroma: Float, hue: Float, threshold: Float) -> bool {
    hue.is_nan() || chroma <= threshold
}
