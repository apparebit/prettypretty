//! # The Core of Color Support
//!
//! This module implements key algorithms for high-resolution colors and color
//! spaces. It favors simplicity and uniformity, notably representing color
//! coordinates, no matter the color space, as three-element `f64` arrays and
//! color spaces as variant tags, *without* data. That simplifies, for example,
//! the compostion of conversions so that a color in every color space can be
//! easily converted to a color in every other color space. Alas, that also
//! allows for software bugs that Rust might otherwise catch. Hence, as the Rust
//! implementation matures, these decisions will likely be revisited.
//!
//! All conversions between color spaces preserve color values and do *not*
//! adjust the result to the gamut of the target color space, say, by clipping
//! or gamut mapping.
//!
//! Functions in this module order arguments for scalar values including color
//! space tags before coordinate arrays, which always come last to allow for
//! readable inline literals.

// ====================================================================================================================
// Color Space Tags
// ====================================================================================================================

/// All supported color spaces. This enumeration just collects the variant tags,
/// no more.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ColorSpace {
    Srgb,
    LinearSrgb,
    DisplayP3,
    LinearDisplayP3,
    Oklch,
    Oklab,
    Xyz,
}

impl ColorSpace {
    /// Determine whether this color space is polar. Out of the currently
    /// supported color spaces, only Oklch is polar.
    pub fn is_polar(&self) -> bool {
        if let Self::Oklch = *self {
            true
        } else {
            false
        }
    }
}

// ====================================================================================================================
// Conversions Between Color Spaces
// ====================================================================================================================

/// Multiply the 3 by 3 matrix and 3-element vector with each other.
#[inline]
fn multiply(matrix: &[[f64; 3]; 3], vector: &[f64; 3]) -> [f64; 3] {
    let [row1, row2, row3] = matrix;

    [
        row1[0].mul_add(vector[0], row1[1].mul_add(vector[1], row1[2] * vector[2])),
        row2[0].mul_add(vector[0], row2[1].mul_add(vector[1], row2[2] * vector[2])),
        row3[0].mul_add(vector[0], row3[1].mul_add(vector[1], row3[2] * vector[2])),
    ]
}

// --------------------------------------------------------------------------------------------------------------------

/// Convert coordinates from gamma-corrected RGB to linear RGB using sRGB's
/// gamma. Display P3 uses the very same gamma. This is a one-hop, direct
/// conversion.
pub fn rgb_to_linear_rgb(value: &[f64; 3]) -> [f64; 3] {
    fn convert(value: f64) -> f64 {
        let magnitude = value.abs();
        if magnitude <= 0.04045 {
            value / 12.92
        } else {
            ((magnitude + 0.055) / 1.055).powf(2.4).copysign(value)
        }
    }

    [convert(value[0]), convert(value[1]), convert(value[2])]
}

/// Convert coordinates from linear RGB to gamma-corrected RGB using sRGB's
/// gamma. Display P3 uses the very same gamma. This is a one-hop, direct
/// conversion.
pub fn linear_rgb_to_rgb(value: &[f64; 3]) -> [f64; 3] {
    fn convert(value: f64) -> f64 {
        let magnitude = value.abs();
        if magnitude <= 0.00313098 {
            value * 12.92
        } else {
            (magnitude.powf(1.0 / 2.4) * 1.055 - 0.055).copysign(value)
        }
    }

    [convert(value[0]), convert(value[1]), convert(value[2])]
}

// --------------------------------------------------------------------------------------------------------------------
// https://github.com/color-js/color.js/blob/a77e080a070039c534dda3965a769675aac5f75e/src/spaces/srgb-linear.js

#[rustfmt::skip]
const LINEAR_SRGB_TO_XYZ: [[f64; 3]; 3] = [
    [ 0.41239079926595934, 0.357584339383878,   0.1804807884018343  ],
    [ 0.21263900587151027, 0.715168678767756,   0.07219231536073371 ],
    [ 0.01933081871559182, 0.11919477979462598, 0.9505321522496607  ],
];

/// Convert coordinates for linear sRGB to XYZ. This is a one-hop, direct conversion.
pub fn linear_srgb_to_xyz(value: &[f64; 3]) -> [f64; 3] {
    multiply(&LINEAR_SRGB_TO_XYZ, value)
}

// https://github.com/color-js/color.js/blob/a77e080a070039c534dda3965a769675aac5f75e/src/spaces/srgb-linear.js

#[rustfmt::skip]
const XYZ_TO_LINEAR_SRGB: [[f64; 3]; 3] = [
	[  3.2409699419045226,  -1.537383177570094,   -0.4986107602930034  ],
	[ -0.9692436362808796,   1.8759675015077202,   0.04155505740717559 ],
	[  0.05563007969699366, -0.20397695888897652,  1.0569715142428786  ],
];

/// Convert coordinates for XYZ to linear sRGB. THis is a one-hop, direct
/// conversion.
pub fn xyz_to_linear_srgb(value: &[f64; 3]) -> [f64; 3] {
    multiply(&XYZ_TO_LINEAR_SRGB, value)
}

// --------------------------------------------------------------------------------------------------------------------
// https://github.com/color-js/color.js/blob/a77e080a070039c534dda3965a769675aac5f75e/src/spaces/p3-linear.js

#[rustfmt::skip]
const LINEAR_DISPLAY_P3_TO_XYZ: [[f64; 3]; 3] = [
    [ 0.4865709486482162, 0.26566769316909306, 0.1982172852343625 ],
    [ 0.2289745640697488, 0.6917385218365064,  0.079286914093745  ],
    [ 0.0000000000000000, 0.04511338185890264, 1.043944368900976  ],
];

/// Convert coordinates for linear Display P3 to XYZ. This is a one-hop, direct
/// conversion.
pub fn linear_display_p3_to_xyz(value: &[f64; 3]) -> [f64; 3] {
    multiply(&LINEAR_DISPLAY_P3_TO_XYZ, value)
}

// https://github.com/color-js/color.js/blob/a77e080a070039c534dda3965a769675aac5f75e/src/spaces/p3-linear.js

#[rustfmt::skip]
const XYZ_TO_LINEAR_DISPLAY_P3: [[f64; 3]; 3] = [
    [  2.493496911941425,   -0.9313836179191239,  -0.40271078445071684  ],
    [ -0.8294889695615747,   1.7626640603183463,   0.023624685841943577 ],
    [  0.03584583024378447, -0.07617238926804182,  0.9568845240076872   ],
];

/// Convert coordinates for XYZ to linear Display P3. This is a one-hop, direct
/// conversion.
pub fn xyz_to_linear_display_p3(value: &[f64; 3]) -> [f64; 3] {
    multiply(&XYZ_TO_LINEAR_DISPLAY_P3, value)
}

// --------------------------------------------------------------------------------------------------------------------

/// Convert coordinates for Oklch to Oklab. This is a one-hop, direct
/// conversion.
pub fn oklch_to_oklab(value: &[f64; 3]) -> [f64; 3] {
    if value[2].is_nan() {
        [value[0], 0.0, 0.0]
    } else {
        let hue_radian = value[2] * std::f64::consts::PI / 180.0;
        [
            value[0],
            value[1] * hue_radian.cos(),
            value[1] * hue_radian.sin(),
        ]
    }
}

/// Convert coordinates for Oklab to Oklch. This is a one-hop, direct
/// conversion.
pub fn oklab_to_oklch(value: &[f64; 3]) -> [f64; 3] {
    let epsilon = 0.0002;

    let h = if value[1].abs() < epsilon && value[2].abs() < epsilon {
        f64::NAN
    } else {
        value[2].atan2(value[1]) * 180.0 / std::f64::consts::PI
    };

    [
        value[0],
        (value[1].powi(2) + value[2].powi(2)).sqrt(),
        h.rem_euclid(360.0),
    ]
}

// --------------------------------------------------------------------------------------------------------------------
// https://github.com/color-js/color.js/blob/a77e080a070039c534dda3965a769675aac5f75e/src/spaces/oklab.js

#[rustfmt::skip]
const OKLAB_TO_OKLMS: [[f64; 3]; 3] = [
    [ 1.0000000000000000,  0.3963377773761749,  0.2158037573099136 ],
    [ 1.0000000000000000, -0.1055613458156586, -0.0638541728258133 ],
    [ 1.0000000000000000, -0.0894841775298119, -1.2914855480194092 ],
];

#[rustfmt::skip]
const OKLMS_TO_XYZ: [[f64; 3]; 3] = [
    [  1.2268798758459243, -0.5578149944602171,  0.2813910456659647 ],
    [ -0.0405757452148008,  1.1122868032803170, -0.0717110580655164 ],
    [ -0.0763729366746601, -0.4214933324022432,  1.5869240198367816 ],
];

/// Convert coordinates for Oklab to XYZ. This is a one-hop, direct conversion,
/// even though it requires two matrix multiplications and a coordinate-wise
/// exponential.
pub fn oklab_to_xyz(value: &[f64; 3]) -> [f64; 3] {
    let [l, m, s] = multiply(&OKLAB_TO_OKLMS, value);
    multiply(&OKLMS_TO_XYZ, &[l.powi(3), m.powi(3), s.powi(3)])
}

// https://github.com/color-js/color.js/blob/a77e080a070039c534dda3965a769675aac5f75e/src/spaces/oklab.js

#[rustfmt::skip]
const XYZ_TO_OKLMS: [[f64; 3]; 3] = [
    [ 0.8190224379967030, 0.3619062600528904, -0.1288737815209879 ],
    [ 0.0329836539323885, 0.9292868615863434,  0.0361446663506424 ],
    [ 0.0481771893596242, 0.2642395317527308,  0.6335478284694309 ],
];

#[rustfmt::skip]
const OKLMS_TO_OKLAB: [[f64; 3]; 3] = [
    [ 0.2104542683093140,  0.7936177747023054, -0.0040720430116193 ],
    [ 1.9779985324311684, -2.4285922420485799,  0.4505937096174110 ],
    [ 0.0259040424655478,  0.7827717124575296, -0.8086757549230774 ],
];

/// Convert coordinates for XYZ to Oklab. This is a one-hop, direct conversion,
/// even though it requires two matrix multiplications and a coordinate-wise
/// exponential.
pub fn xyz_to_oklab(value: &[f64; 3]) -> [f64; 3] {
    let [l, m, s] = multiply(&XYZ_TO_OKLMS, value);
    multiply(&OKLMS_TO_OKLAB, &[l.cbrt(), m.cbrt(), s.cbrt()])
}

// --------------------------------------------------------------------------------------------------------------------

/// Convert coordinates for sRGB to XYZ. This is a two-hop conversion.
pub fn srgb_to_xyz(value: &[f64; 3]) -> [f64; 3] {
    let linear_srgb = rgb_to_linear_rgb(value);
    linear_srgb_to_xyz(&linear_srgb)
}

/// Convert coordinates for XYZ to sRGB. This is a two-hop conversion.
pub fn xyz_to_srgb(value: &[f64; 3]) -> [f64; 3] {
    let linear_srgb = xyz_to_linear_srgb(value);
    linear_rgb_to_rgb(&linear_srgb)
}

/// Convert coordinates for Display P3 to XYZ. This is a two-hop conversion.
pub fn display_p3_to_xyz(value: &[f64; 3]) -> [f64; 3] {
    let linear_p3 = rgb_to_linear_rgb(value);
    linear_display_p3_to_xyz(&linear_p3)
}

/// Convert coordinates for XYZ to Display P3. This is a two-hop conversion.
pub fn xyz_to_display_p3(value: &[f64; 3]) -> [f64; 3] {
    let linear_p3 = xyz_to_linear_display_p3(value);
    linear_rgb_to_rgb(&linear_p3)
}

/// Convert coordinates for Oklch to XYZ. This is a two-hop conversion.
pub fn oklch_to_xyz(value: &[f64; 3]) -> [f64; 3] {
    let oklab = oklch_to_oklab(value);
    oklab_to_xyz(&oklab)
}

/// Convert coordinates for XYZ to Oklch. This is a two-hop conversion.
pub fn xyz_to_oklch(value: &[f64; 3]) -> [f64; 3] {
    let oklab = xyz_to_oklab(value);
    oklab_to_oklch(&oklab)
}

// --------------------------------------------------------------------------------------------------------------------

/// Convert the coordinates from the `from_space` to the `to_space`. This
/// function leverages the fact that the one-hop conversions effectively form a
/// tree rooted at XYZ D65 and proceeds as following:
///
///  1. It first handles the trivial case, both color spaces being equal, and
///     if that's the case, simply returns the coordinates.
///  2. Then it handles all single-hop conversions that do not touch on the
///     effective root of the color space conversion tree, XYZ.
///  3. At this point, `from_space` and `to_space` must be on different branches
///     and this function handles all remaining conversions as the composition
///     of two conversions:
///
///      1. along one branch from `from_space` to the root XYZ;
///      2. along another branch from the root XYZ to `to_space`.
///
/// This requires matching six color space pairs in addition to exhaustively
/// matching on `from_space` and `to_space`, all of it *in sequence*. That is
/// critical because it ensures a simple, straight-forward implementation with
/// trivial control flow. By contrast, the naive approach to implementing this
/// function, which matches each `from_space` with every `to_space` to select
/// some conversion, executes fewer matches in the worst case (since it has no
/// steps 1 and 2) but its code complexity is O(n^2), whereas the code
/// complexity of this function is O(3n). Trees are nice that way!
///
/// If this function becomes a bottleneck, it can be replaced with a factory
/// function that amortizes the overhead of converter selection over more than
/// one conversion.
pub fn convert(from_space: ColorSpace, to_space: ColorSpace, coordinates: &[f64; 3]) -> [f64; 3] {
    use ColorSpace::*;

    // 1. Handle identities
    if from_space == to_space {
        return *coordinates;
    }

    // 2. Handle single-branch conversions, ignoring root
    match (from_space, to_space) {
        (Srgb, LinearSrgb) | (DisplayP3, LinearDisplayP3) => return rgb_to_linear_rgb(coordinates),
        (LinearSrgb, Srgb) | (LinearDisplayP3, DisplayP3) => return linear_rgb_to_rgb(coordinates),
        (Oklch, Oklab) => return oklch_to_oklab(coordinates),
        (Oklab, Oklch) => return oklab_to_oklch(coordinates),
        _ => (),
    };

    // 3a. Convert from source to XYZ
    let intermediate = match from_space {
        Srgb => srgb_to_xyz(coordinates),
        LinearSrgb => linear_srgb_to_xyz(coordinates),
        DisplayP3 => display_p3_to_xyz(coordinates),
        LinearDisplayP3 => linear_display_p3_to_xyz(coordinates),
        Oklch => oklch_to_xyz(coordinates),
        Oklab => oklab_to_xyz(coordinates),
        Xyz => *coordinates,
    };

    // 3b. Convert from XYZ to target on different branch
    match to_space {
        Srgb => xyz_to_srgb(&intermediate),
        LinearSrgb => xyz_to_linear_srgb(&intermediate),
        DisplayP3 => xyz_to_display_p3(&intermediate),
        LinearDisplayP3 => xyz_to_linear_display_p3(&intermediate),
        Oklch => xyz_to_oklch(&intermediate),
        Oklab => xyz_to_oklab(&intermediate),
        Xyz => intermediate,
    }
}

// ====================================================================================================================
// Gamut
// ====================================================================================================================

/// Determine whether the coordinates are in gamut for the color space.
pub fn in_gamut(space: ColorSpace, coordinates: &[f64; 3]) -> bool {
    use ColorSpace::*;

    match space {
        Srgb | LinearSrgb | DisplayP3 | LinearDisplayP3 => {
            coordinates.iter().all(|c| 0.0 <= *c && *c <= 1.0)
        }
        _ => true,
    }
}

/// Determine whether the coordinates are in gamut for the color space with some
/// tolerance.
pub fn approx_in_gamut(tolerance: f64, space: ColorSpace, coordinates: &[f64; 3]) -> bool {
    use ColorSpace::*;

    match space {
        Srgb | LinearSrgb | DisplayP3 | LinearDisplayP3 => {
            coordinates.iter().all(|c| 0.0 - tolerance <= *c && *c <= 1.0 + tolerance)
        }
        _ => true,
    }
}

/// Clip the coordinates to the gamut of the color space.
pub fn clip(space: ColorSpace, coordinates: &mut [f64; 3]) {
    use ColorSpace::*;

    match space {
        Srgb | LinearSrgb | DisplayP3 | LinearDisplayP3 => {
            coordinates.iter_mut().for_each(|c| (*c = c.clamp(0.0, 1.0)))
        }
        _ => (),
    }
}

const JND: f64 = 0.02;
const EPSILON: f64 = 0.0001;

/// Map the color into gamut by using the [CSS Color 4
/// algorithm](https://drafts.csswg.org/css-color/#css-gamut-mapping).
///
/// The algorithm performs an Oklch-based binary search across the chroma range
/// from 0 to the original level. It stops the search once the chroma-adjusted
/// color is within the just noticeable difference (JND) of its clipped version,
/// using that clipped version as result. JND is measured by deltaEOK, the Oklab
/// version of the deltaE distance/difference metrics for colors. In other words,
/// the algorithm relies on three distinct views of each color:
///
///  1. the target space view for gamut testing and clipping;
///  2. the Oklch-based view for producing candidate colors by changing the
///     chroma;
///  3. the Oklab-based view for measuring distance.
///
/// The simultaneous use of Oklab/Oklch nicely illustrates that both Cartesian
/// and polar coordinates are uniquely suitable for computing some color
/// properties but not nearly all.
pub fn into_gamut(target: ColorSpace, coordinates: &[f64; 3]) -> [f64; 3] {
    use ColorSpace::*;

    // If the color space is unbounded, there is nothing to map to
    match target {
        Xyz | Oklab | Oklch => return *coordinates,
        _ => ()
    }

    // Preliminary 1/2: Clamp Lightness
    let origin_as_oklch = convert(target, Oklch, coordinates);
    let l = origin_as_oklch[0];
    if l >= 1.0 {
        return convert(Oklch, target, &[1.0, 0.0, 0.0]);
    }
    if l <= 0.0 {
        return convert(Oklch, target, &[0.0, 0.0, 0.0]);
    }

    // Preliminary 2/2: Check gamut
    if in_gamut(target, coordinates) {
        return *coordinates;
    }

    // Goal: Minimize just noticeable difference between current and clipped
    // colors
    let mut current_as_oklch = origin_as_oklch;
    let mut clipped_as_target = convert(Oklch, target, &current_as_oklch);
    clip(target, &mut clipped_as_target);

    let difference = delta_e_ok(
        &convert(target, Oklab, &clipped_as_target),
        &oklch_to_oklab(&current_as_oklch),
    );

    if difference < JND {
        return clipped_as_target;
    }

    // Strategy: Binary search by adjusting chroma in Oklch
    let mut min = 0.0;
    let mut max = origin_as_oklch[1];
    let mut min_in_gamut = true;

    loop {
        println!("start loop body");
        if max - min <= EPSILON {
            break;
        }

        let chroma = (min + max) / 2.0;
        current_as_oklch = [current_as_oklch[0], chroma, current_as_oklch[2]];

        let current_as_target = convert(Oklch, target, &current_as_oklch);

        if min_in_gamut && in_gamut(target, &current_as_target) {
            min = chroma;
            continue;
        }

        clipped_as_target = current_as_target;
        clip(target, &mut clipped_as_target);

        let difference = delta_e_ok(
            &convert(target, Oklab, &clipped_as_target),
            &oklch_to_oklab(&current_as_oklch),
        );

        if difference < JND {
            if JND - difference < EPSILON {
                return clipped_as_target;
            }
            min_in_gamut = false;
            min = chroma;
        } else {
            max = chroma;
        }
    }

    clipped_as_target
}

// ====================================================================================================================
// Equality and Difference
// ====================================================================================================================

/// Determine whether two coordinates are the same for all practical purposes.
/// This function correctly handles not-a-number, angles, floating point errors,
/// and floating point ranges. It is critical for testing this library.
pub fn same(c1: f64, c2: f64, is_angle: bool) -> bool {
    // Compare not-a-number for equality after all
    if c1.is_nan() {
        return c2.is_nan()
    } else if c2.is_nan() {
        return false
    }

    // Normalize angular coordinates
    let n1 = if is_angle { c1.rem_euclid(360.0) } else { c1 };
    let n2 = if is_angle { c2.rem_euclid(360.0) } else { c2 };

    // Round to some fixed number of decimals
    let decimals = if is_angle { 12 } else { 14 };
    let factor = 10.0_f64.powi(decimals);
    // Skip downscaling by factor since we don't reveal values.
    let n1 = (n1 * factor).round();
    let n2 = (n2 * factor).round();

    // Now, compare for equality
    n1 == n2
}

/// Determine whether the coordinates designate the same color in the color
/// space.
pub fn same_coordinates(
    space: ColorSpace,
    coordinates1: &[f64; 3],
    coordinates2: &[f64; 3],
) -> bool {
    let is_polar = space.is_polar();

    for index in 0..3 {
        let c1 = coordinates1[index];
        let c2 = coordinates2[index];
        if !same(c1, c2, is_polar && index == 2) {
            return false
        }
    };

    true
}

// --------------------------------------------------------------------------------------------------------------------

/// Compute the *Delta E* for the two coordinates in Oklab. Delta E is a generic
/// difference or distance metric for colors and multiple algorithms exist. THe
/// one for Oklab has the benefit of being fairly accurate and incredibly
/// simple, just the Euclidian distances between the two coordinates. However,
/// it appears that Ottosson [was a bit too
/// fast](https://github.com/w3c/csswg-drafts/issues/6642#issuecomment-945714988)
/// in defining Delta E that way...
#[allow(non_snake_case)]
pub fn delta_e_ok(coordinates1: &[f64; 3], coordinates2: &[f64; 3]) -> f64 {
    let [L1, a1, b1] = coordinates1;
    let [L2, a2, b2] = coordinates2;

    let ΔL = L1 - L2;
    let Δa = a1 - a2;
    let Δb = b1 - b2;

    ΔL.mul_add(ΔL, Δa.mul_add(Δa, Δb * Δb)).sqrt()
}

// ====================================================================================================================
// The Color Abstraction
// ====================================================================================================================

/// A color in one of the supported color spaces.
#[derive(Copy, Clone, Debug)]
pub struct Color {
    space: ColorSpace,
    coordinates: [f64; 3],
}

impl Color {
    /// Create a new color with the given color space and coordinates.
    pub fn new(space: ColorSpace, coordinates: [f64; 3]) -> Self {
        Color { space, coordinates }
    }

    /// The color's color space.
    pub fn space(&self) -> ColorSpace {
        self.space
    }

    /// The color's coordinates.
    pub fn coordinates(&self) -> &[f64; 3] {
        &self.coordinates
    }

    /// Convert this color to the given color space.
    pub fn to(&self, space: ColorSpace) -> Color {
        Self {
            space,
            coordinates: convert(self.space, space, &self.coordinates),
        }
    }

    /// Determine whether this and the other color represent the same color.
    pub fn same(&self, other: Self) -> bool {
        self.space == other.space && same_coordinates(
            self.space, &self.coordinates, &other.coordinates
        )
    }

    /// Determine the difference or distance between the two colors. This method
    /// uses delta E Ok.
    pub fn distance(&self, other: Self) -> f64 {
        return delta_e_ok(&self.coordinates, &other.coordinates)
    }

    /// Determine whether this color is in-gamut for its color space.
    pub fn in_gamut(&self) -> bool {
        in_gamut(self.space, &self.coordinates)
    }

    /// Clip this color to the gamut of its color space.
    pub fn clip(&mut self) {
        clip(self.space, &mut self.coordinates)
    }

    /// Map this color into the gamut of its color space.
    pub fn into_gamut(&self) -> Self {
        Self {
            space: self.space,
            coordinates: into_gamut(self.space, &self.coordinates),
        }
    }
}

// ====================================================================================================================

#[cfg(test)]
mod test {
    use super::*;
    use super::ColorSpace::*;

    #[allow(dead_code)]
    struct Representations {
        spec: &'static str,
        srgb: [f64; 3],
        linear_srgb: [f64; 3],
        p3: [f64; 3],
        linear_p3: [f64; 3],
        oklch: [f64; 3],
        oklab: [f64; 3],
        xyz: [f64; 3],
    }

    const BLACK: Representations = Representations {
        spec: "#000000",
        srgb: [0.0, 0.0, 0.0],
        linear_srgb: [0.0, 0.0, 0.0],
        p3: [0.0, 0.0, 0.0],
        linear_p3: [0.0, 0.0, 0.0],
        oklch: [0.0, 0.0, f64::NAN],
        oklab: [0.0, 0.0, 0.0],
        xyz: [0.0, 0.0, 0.0],
    };

    const YELLOW: Representations = Representations {
        spec: "#ffca00",
        srgb: [1.0, 0.792156862745098, 0.0],
        linear_srgb: [1.0, 0.5906188409193369, 0.0],
        p3: [0.967346220711791, 0.8002244967941964, 0.27134084647161244],
        linear_p3: [0.9273192749713864, 0.6042079205196976, 0.059841923211596565],
        oklch: [0.8613332073307732, 0.1760097742886813, 89.440876452466],
        oklab: [
            0.8613332073307732,
            0.0017175723640959761,
            0.17600139371700052,
        ],
        xyz: [0.6235868473237722, 0.635031101987136, 0.08972950140152941],
    };

    const BLUE: Representations = Representations {
        spec: "#3178ea",
        srgb: [0.19215686274509805, 0.47058823529411764, 0.9176470588235294],
        linear_srgb: [
            0.030713443732993635,
            0.18782077230067787,
            0.8227857543962835,
        ],
        p3: [0.26851535563550943, 0.4644576150842869, 0.8876966971452301],
        linear_p3: [0.058605969547446124, 0.18260572039525869, 0.763285235993837],
        oklch: [0.5909012953108558, 0.18665606306724153, 259.66681920272595],
        oklab: [
            0.5909012953108558,
            -0.03348086515869664,
            -0.1836287492414715,
        ],
        xyz: [0.22832473003420622, 0.20025321836938534, 0.80506528557483],
    };

    const WHITE: Representations = Representations {
        spec: "#ffffff",
        srgb: [1.0, 1.0, 1.0],
        linear_srgb: [1.0, 1.0, 1.0],
        p3: [0.9999999999999999, 0.9999999999999997, 0.9999999999999999],
        linear_p3: [1.0, 0.9999999999999998, 1.0],
        oklch: [1.0000000000000002, 0.0, f64::NAN],
        oklab: [1.0000000000000002, -4.996003610813204e-16, 0.0],
        xyz: [0.9504559270516717, 1.0, 1.0890577507598784],
    };

    #[test]
    fn test_equivalence() {
        // Good grief: In Pyhon 0.5 rounds down. In Rust, it rounds up.
        let f00 = 0.0;
        let f01 = 1e-15_f64;
        let f02 = 2e-15_f64;
        let f03 = 3e-15_f64;
        let f05 = 5e-15_f64;
        let f07 = 7e-15_f64;
        let f09 = 9e-15_f64;
        let f10 = 1e-14_f64;
        let f20 = 2e-14_f64;

        assert!(same_coordinates(
            Srgb,
            &[f01, f02, f03],
            &[f00, f00, f00],
        ));

        assert!(same_coordinates(
            Srgb,
            &[f05, f07, f09],
            &[f10, f10, f10],
        ));

        assert!(!same_coordinates(
            Srgb,
            &[f10, f10, f10],
            &[f20, f20, f20],
        ));
    }

    #[test]
    fn test_colors() {
        for &color in [&BLACK, &YELLOW, &BLUE, &WHITE].iter() {
            // Test all one-hop conversions
            let linear_srgb = rgb_to_linear_rgb(&color.srgb);
            assert!(same_coordinates(LinearSrgb, &linear_srgb, &color.linear_srgb));

            let srgb = linear_rgb_to_rgb(&linear_srgb);
            assert!(same_coordinates(Srgb, &srgb, &color.srgb));

            let xyz = linear_srgb_to_xyz(&linear_srgb);
            assert!(same_coordinates(Xyz, &xyz, &color.xyz));

            let also_linear_srgb = xyz_to_linear_srgb(&xyz);
            assert!(same_coordinates(LinearSrgb, &also_linear_srgb, &linear_srgb));

            let linear_p3 = xyz_to_linear_display_p3(&xyz);
            assert!(same_coordinates(LinearDisplayP3, &linear_p3, &color.linear_p3));

            let also_xyz = linear_display_p3_to_xyz(&linear_p3);
            assert!(same_coordinates(Xyz, &also_xyz, &xyz));

            let p3 = linear_rgb_to_rgb(&linear_p3);
            assert!(same_coordinates(DisplayP3, &p3, &color.p3));

            let also_linear_p3 = rgb_to_linear_rgb(&p3);
            assert!(same_coordinates(LinearDisplayP3, &also_linear_p3, &linear_p3));

            let oklab = xyz_to_oklab(&xyz);
            assert!(same_coordinates(Oklab, &oklab, &color.oklab));

            let and_again_xyz = oklab_to_xyz(&oklab);
            assert!(same_coordinates(Xyz, &and_again_xyz, &xyz));

            let oklch = oklab_to_oklch(&oklab);
            assert!(same_coordinates(Oklch, &oklch, &color.oklch));

            let also_oklab = oklch_to_oklab(&oklch);
            assert!(same_coordinates(Oklab, &also_oklab, &oklab));
        }
    }

    #[test]
    fn test_gamut_mapping() {
        // A very green green.
        let p3 = [0.0, 1.0, 0.0];
        let srgb = convert(DisplayP3, Srgb, &p3);
        assert!(same_coordinates(Srgb, &srgb, &[
            -0.5116049825853448, 1.0182656579378029, -0.3106746212905826
        ]));

        let srgb_mapped = into_gamut(Srgb, &srgb);
        assert!(same_coordinates(Srgb, &srgb_mapped, &[
            0.0, 0.9857637107710327, 0.15974244397343723
        ]));

        // A very yellow yellow.
        let p3 = [1.0, 1.0, 0.0];
        let srgb = convert(DisplayP3, Srgb, &p3);
        assert!(same_coordinates(Srgb, &srgb, &[
            0.9999999999999999, 0.9999999999999999, -0.3462679629331063
        ]));

        let linear_srgb = convert(DisplayP3, LinearSrgb, &p3);
        assert!(same_coordinates(LinearSrgb, &linear_srgb, &[
            1.0, 1.0000000000000002, -0.09827360014096621
        ]));

        let linear_srgb_mapped = into_gamut(LinearSrgb, &linear_srgb);
        assert!(same_coordinates(LinearSrgb, &linear_srgb_mapped, &[
            0.9914525477996114, 0.9977581974546286, 0.0
        ]));
    }
}
