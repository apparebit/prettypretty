#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use crate::core::{convert, ColorSpace};
use crate::{Bits, Float};

/// Normalize coordinates that are not-a-number.
#[inline]
fn normalize_nan_mut(space: ColorSpace, coordinates: &mut [Float; 3]) {
    let [c1, c2, c3] = coordinates;

    // Ensure all coordinates are numbers
    if c1.is_nan() {
        *c1 = 0.0;
    }

    if c2.is_nan() {
        *c2 = 0.0;
    }

    if c3.is_nan() {
        *c3 = 0.0;
        if space.is_polar() {
            *c2 = 0.0;
        }
    }
}

/// Normalize coordinates ranges.
///
/// For Ok*** coordinates, this function clamps (revised) lightness to `0..=1`
/// and chroma to `0..`. It also reduces hue to an equivalent angle `0..=360`.
fn normalize_range_mut(space: ColorSpace, coordinates: &mut [Float; 3]) {
    let [c1, c2, c3] = coordinates;

    // Clamp lightness and chroma in Oklab et al
    if space.is_ok() {
        *c1 = c1.clamp(0.0, 1.0);

        if space.is_polar() {
            *c2 = c2.max(0.0);
        }
    }

    // Normalize hue
    if space.is_polar() {
        *c3 = c3.rem_euclid(360.0)
    }
}

/// Normalize the color's coordinates by zeroing out not-a-numbers and enforcing
/// range limits.
pub(crate) fn normalize(space: ColorSpace, coordinates: &[Float; 3]) -> [Float; 3] {
    let mut coordinates = *coordinates;
    normalize_nan_mut(space, &mut coordinates);
    normalize_range_mut(space, &mut coordinates);
    coordinates
}

/// Normalize coordinates for testing equality and hashing.
fn normalize_eq_mut(space: ColorSpace, coordinates: &mut [Float; 3]) {
    let [c1, c2, c3] = coordinates;

    // Scale to unit range.
    if space.is_polar() {
        *c3 /= 360.0
    }

    // Drop one digit of precision.
    let factor = (10.0 as Float).powi((Float::DIGITS as i32) - 1);
    *c1 = (*c1 * factor).round();
    *c2 = (*c2 * factor).round();
    *c3 = (*c3 * factor).round();

    // Ensure canonical zero.
    if *c1 == -0.0 {
        *c1 = 0.0;
    }
    if *c2 == -0.0 {
        *c2 = 0.0
    }
    if *c3 == -0.0 {
        *c3 = 0.0
    }
}

/// Convert coordinates to equivalent bits suitable for testing equality and
/// hashing.
///
/// This function eliminates not-a-numbers, enforces correct ranges, scales
/// coordinates to have the same order of magnitude, and converts the result to
/// bits that are ready for equality comparison and/or hashing.
#[must_use = "function returns new color coordinates and does not mutate original value"]
pub(crate) fn to_eq_bits(space: ColorSpace, coordinates: &[Float; 3]) -> [Bits; 3] {
    let mut coordinates = *coordinates;
    normalize_nan_mut(space, &mut coordinates);
    normalize_range_mut(space, &mut coordinates);
    normalize_eq_mut(space, &mut coordinates);
    let [c1, c2, c3] = coordinates;
    [c1.to_bits(), c2.to_bits(), c3.to_bits()]
}

// --------------------------------------------------------------------------------------------------------------------

/// Compute Delta-E for Oklab or Oklrab.
#[inline]
#[allow(non_snake_case)]
pub(crate) fn delta_e_ok(coordinates1: &[Float; 3], coordinates2: &[Float; 3]) -> Float {
    let [L1, a1, b1] = coordinates1;
    let [L2, a2, b2] = coordinates2;

    let ΔL = L1 - L2;
    let Δa = a1 - a2;
    let Δb = b1 - b2;

    ΔL.mul_add(ΔL, Δa.mul_add(Δa, Δb * Δb)).sqrt()
}

/// Find the candidate color closest to the origin.
///
/// This function compares the origin to every candidate color, computing the
/// distance metric with the given function, and returns the index of the
/// closest candidate color—or `None` if there are no candidates.
pub(crate) fn find_closest<'c, C, F>(
    origin: &[f64; 3],
    candidates: C,
    mut compute_distance: F,
) -> Option<usize>
where
    C: IntoIterator<Item = &'c [f64; 3]>,
    F: FnMut(&[f64; 3], &[f64; 3]) -> f64,
{
    let mut min_distance = f64::INFINITY;
    let mut min_index = None;

    for (index, candidate) in candidates.into_iter().enumerate() {
        let distance = compute_distance(origin, candidate);
        if distance < min_distance {
            min_distance = distance;
            min_index = Some(index);
        }
    }

    min_index
}

// --------------------------------------------------------------------------------------------------------------------

/// Determine how a coordinate carries forward.
///
/// This function determines how to [carry
/// forward](https://www.w3.org/TR/css-color-4/#interpolation-missing) a missing
/// coordinate, i.e., a coordinate that is not-a-number, from the source color
/// space to the interpolation color space. The caller specifies the coordinate
/// by its index (from 0 to 2) and, if the coordinate carries forward, the function
/// returns the index of the forwarded coordinate.
///
/// # Panics
///
/// This function panics if the index is out of bounds.
fn carry_forward(from_space: ColorSpace, to_space: ColorSpace, index: usize) -> Option<usize> {
    use ColorSpace::*;

    if !(0..=2).contains(&index) {
        panic!("0..=2.contains({}) does not hold!", index)
    }

    match (from_space, to_space, index) {
        // Analogous components are (r,x) -- (g,y) -- (b,z) -- (L) -- (Lr) -- (C) -- (h) -- (a) -- (b)
        (
            Srgb | LinearSrgb | DisplayP3 | LinearDisplayP3 | Rec2020 | LinearRec2020 | Xyz,
            Srgb | LinearSrgb | DisplayP3 | LinearDisplayP3 | Rec2020 | LinearRec2020 | Xyz,
            _,
        ) => Some(index),
        (Oklab | Oklch, Oklab | Oklch, 0) => Some(0),
        (Oklrab | Oklrch, Oklrab | Oklrch, 0) => Some(0),
        (Oklab | Oklrab, Oklab | Oklrab, 1 | 2) => Some(index),
        (Oklch | Oklrch, Oklch | Oklrch, 1 | 2) => Some(index),
        _ => None,
    }
}

/// Convert the coordinates while carrying forward missing values.
fn prepare_coordinate_interpolation(
    from_space: ColorSpace,
    to_space: ColorSpace,
    coordinates: &[Float; 3],
) -> [Float; 3] {
    // Normalize coordinates and convert to interpolation space
    let mut intermediate = *coordinates;
    normalize_nan_mut(from_space, &mut intermediate);
    normalize_range_mut(from_space, &mut intermediate);
    intermediate = convert(from_space, to_space, &intermediate);

    // Carry forward missing components
    for (index, coordinate) in coordinates.iter().enumerate() {
        if coordinate.is_nan() {
            if let Some(index) = carry_forward(from_space, to_space, index) {
                intermediate[index] = Float::NAN;
            }
        }
    }

    intermediate
}

/// A strategy for interpolating hues.
///
/// Since hues are expressed as angles, the same perceptual hue has an infinite
/// number of representations modulo 360. Furthermore, there are two ways of
/// interpolating between two hues, clockwise and counter-clockwise. Consistent
/// with [CSS Color 4](https://www.w3.org/TR/css-color-4/#hue-interpolation),
/// the interpolation strategy selects the way based either on the distance
/// between hues, [`HueInterpolation::Shorter`] and
/// [`HueInterpolation::Longer`], or on the direction,
/// [`HueInterpolation::Increasing`] and [`HueInterpolation::Decreasing`].
#[cfg_attr(feature = "pyffi", pyclass(eq, eq_int))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum HueInterpolation {
    /// Take the shorter arc between the two hue angles.
    Shorter,
    /// Take the longer arc between the two hue angles.
    Longer,
    /// Keep increasing hue angles.
    Increasing,
    /// Keep decreasing hue angles.
    Decreasing,
}

/// Adjust the pair of hues based on interpolation strategy.
fn prepare_hue_interpolation(strategy: HueInterpolation, h1: Float, h2: Float) -> [Float; 2] {
    match strategy {
        HueInterpolation::Shorter => {
            if h2 - h1 > 180.0 {
                return [h1 + 360.0, h2];
            } else if h2 - h1 < -180.0 {
                return [h1, h2 + 360.0];
            }
        }
        HueInterpolation::Longer => {
            if (0.0..=180.0).contains(&(h2 - h1)) {
                return [h1 + 360.0, h2];
            } else if (-180.0..=0.0).contains(&(h2 - h1)) {
                return [h1, h2 + 360.0];
            }
        }
        HueInterpolation::Increasing => {
            if h2 < h1 {
                return [h1, h2 + 360.0];
            }
        }
        HueInterpolation::Decreasing => {
            if h1 < h2 {
                return [h1 + 360.0, h2];
            }
        }
    }

    [h1, h2]
}

/// Prepare coordinates for interpolation.
///
/// This function prepares a pair of coordinates for interpolation with
/// [`interpolate`] accorrding to the rules of [CSS Color
/// 4](https://www.w3.org/TR/css-color-4/#interpolation). Those rules are
/// surprisingly complex thanks to the specification's support for missing
/// components and hue interpolation strategies.
///
/// As required by the specification, this function carries missing components
/// forward when converting to the interpolation color space and then tries to
/// fill them with the other color's component. It also implements all four
/// interpolation strategies for hues, which select one of the two available
/// arcs between the two colors.
///
/// By separating preparation from actual interpolation, it becomes possible to
/// amortize the overhead of the former when generating several interpolated
/// colors, e.g., when computing a gradient.
#[must_use = "function returns new color coordinates and does not mutate original values"]
pub(crate) fn prepare_to_interpolate(
    space1: ColorSpace,
    coordinates1: &[Float; 3],
    space2: ColorSpace,
    coordinates2: &[Float; 3],
    interpolation_space: ColorSpace,
    strategy: HueInterpolation,
) -> ([Float; 3], [Float; 3]) {
    let mut coordinates1 =
        prepare_coordinate_interpolation(space1, interpolation_space, coordinates1);
    let mut coordinates2 =
        prepare_coordinate_interpolation(space2, interpolation_space, coordinates2);

    // Fill in missing components
    for index in 0..=2 {
        if coordinates1[index].is_nan() {
            // Technically, only do this if coordinates2[index] is a number.
            coordinates1[index] = coordinates2[index];
        } else if coordinates2[index].is_nan() {
            coordinates2[index] = coordinates1[index];
        }
    }

    // Adjust hue based on interpolation strategy
    if interpolation_space.is_polar() {
        [coordinates1[2], coordinates2[2]] =
            prepare_hue_interpolation(strategy, coordinates1[2], coordinates2[2])
    }

    (coordinates1, coordinates2)
}

/// Interpolate between the prepared coordinates.
///
/// This function calculates the linear interpolation for the given factor
/// between equivalent coordinates of the two colors. For the result to be
/// meaningful, the coordinates must be prepared with
/// [`prepare_to_interpolate`].
#[must_use = "function returns new color coordinates and does not mutate original values"]
pub(crate) fn interpolate(
    fraction: Float,
    coordinates1: &[Float; 3],
    coordinates2: &[Float; 3],
) -> [Float; 3] {
    [
        coordinates1[0] + fraction * (coordinates2[0] - coordinates1[0]),
        coordinates1[1] + fraction * (coordinates2[1] - coordinates1[1]),
        coordinates1[2] + fraction * (coordinates2[2] - coordinates1[2]),
    ]
}
