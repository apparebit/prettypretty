//! # Core Color
//!
//! This module implements the high-resolution, high-quality algorithms enabling
//! the parent modules `Color` abstraction.
//!
//! Unlike that struct, this module eschews encapsulation in favor of simplicity
//! and uniformity. Notably, it represents color spaces as tag-like variants
//! *without* associated values and color coordinates as three-element `f64`
//! arrays. Besides the overall benefit of reduced complexity, this particularly
//! aids conversion between color spaces. The `convert()` function performs 42
//! different conversions between color spaces using a repertoire of only 10
//! handwritten single-hop color space conversion functions and 6 two-hop color
//! space conversion functions.
//!
//! All currently supported color spaces, including XYZ, use D65 as white point.
//! The code neither performs nor supports chromatic adaptation.
//!
//! Conversion between color spaces preserves out-of-gamut values. It does *not*
//! clip coordinates. It does *not* gamut-map coordinates.
//!
//! Not-a-number *is* a valid coordinate value for hue in Oklch. It necessarily
//! implies that the chroma is zero, i.e., the color is gray (including black
//! and white).
//!
//! Function arguments are ordered so that scalar arguments come before
//! coordinates, which may be inline array literals.

// ====================================================================================================================
// Color Space Tags
// ====================================================================================================================

/// The enumeration of supported color spaces.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    /// [sRGB](https://en.wikipedia.org/wiki/SRGB) has long served as the
    /// default color space for the web.
    Srgb,

    /// The linear version of sRGB.
    LinearSrgb,

    /// [Display P3](https://en.wikipedia.org/wiki/DCI-P3) has a wider gamut,
    /// that is, accommodates more colors, than sRGB and is increasingly
    /// supported by new computer monitors.
    DisplayP3,

    /// The linear version of Display P3.
    LinearDisplayP3,

    /// [Oklab](https://bottosson.github.io/posts/oklab/) is a perceptually
    /// uniform color space that improves upon Lab.
    Oklab,

    /// Oklch is the polar version of Oklab.
    Oklch,

    /// [XYZ](https://en.wikipedia.org/wiki/CIE_1931_color_space) is a
    /// foundational color space. Here it assumes the [D65 standard
    /// illuminant](https://en.wikipedia.org/wiki/Standard_illuminant) (or white
    /// point).
    Xyz,
}

impl ColorSpace {
    /// Determine whether this color space is polar. Oklch is the only such
    /// color space.
    pub const fn is_polar(&self) -> bool {
        matches!(*self, Self::Oklch)
    }

    /// Determine whether this color space is RGB, that is, has red, green, and
    /// blue coordinates. In-gamut colors for RGB color spaces have coordinates
    /// in unit range `0..=1`.
    pub const fn is_rgb(&self) -> bool {
        use ColorSpace::*;
        matches!(*self, Srgb | LinearSrgb | DisplayP3 | LinearDisplayP3)
    }

    /// Determine whether this color space is bounded. XYZ is the only such
    /// color space, but for now Oklab and Oklch are also treated as unbounded.
    pub const fn is_bounded(&self) -> bool {
        !matches!(*self, Self::Xyz | Self::Oklab | Self::Oklch)
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
#[allow(non_snake_case)]
pub fn oklch_to_oklab(value: &[f64; 3]) -> [f64; 3] {
    let [L, C, h] = *value;

    if h.is_nan() {
        [L, 0.0, 0.0]
    } else {
        let hue_radian = h.to_radians();
        [L, C * hue_radian.cos(), C * hue_radian.sin()]
    }
}

/// Convert coordinates for Oklab to Oklch. This is a one-hop, direct
/// conversion.
#[allow(non_snake_case)]
pub fn oklab_to_oklch(value: &[f64; 3]) -> [f64; 3] {
    const EPSILON: f64 = 0.0002;

    let [L, a, b] = *value;
    let h = if a.abs() < EPSILON && b.abs() < EPSILON {
        f64::NAN
    } else {
        b.atan2(a).to_degrees()
    };

    [L, (a.powi(2) + b.powi(2)).sqrt(), h.rem_euclid(360.0)]
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
#[allow(clippy::excessive_precision)]
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
#[allow(clippy::excessive_precision)]
const XYZ_TO_OKLMS: [[f64; 3]; 3] = [
    [ 0.8190224379967030, 0.3619062600528904, -0.1288737815209879 ],
    [ 0.0329836539323885, 0.9292868615863434,  0.0361446663506424 ],
    [ 0.0481771893596242, 0.2642395317527308,  0.6335478284694309 ],
];

#[rustfmt::skip]
#[allow(clippy::excessive_precision)]
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

/// Clip the coordinates to the gamut of the color space.
pub fn clip(space: ColorSpace, coordinates: &[f64; 3]) -> [f64; 3] {
    use ColorSpace::*;

    match space {
        Srgb | LinearSrgb | DisplayP3 | LinearDisplayP3 => {
            let [r, g, b] = *coordinates;
            [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)]
        }
        _ => *coordinates,
    }
}

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
pub fn map_to_gamut(target: ColorSpace, coordinates: &[f64; 3]) -> [f64; 3] {
    use ColorSpace::*;

    const JND: f64 = 0.02;
    const EPSILON: f64 = 0.0001;

    // If the color space is unbounded, there is nothing to map to
    if !target.is_bounded() {
        return *coordinates;
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
    let mut clipped_as_target = clip(target, &convert(Oklch, target, &current_as_oklch));

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

    while max - min > EPSILON {
        let chroma = (min + max) / 2.0;
        current_as_oklch = [current_as_oklch[0], chroma, current_as_oklch[2]];

        let current_as_target = convert(Oklch, target, &current_as_oklch);

        if min_in_gamut && in_gamut(target, &current_as_target) {
            min = chroma;
            continue;
        }

        clipped_as_target = clip(target, &current_as_target);

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

/// Normalize the coordinates in preparation of hashing and equality testing.
/// This function performs the following transformations:
///
///   * To ensure coordinates are numbers, zero out not-a-numbers. If the hue in
///     Oklch is not-a-number, also zero out the chroma.
///   * To ensure hues are comparable with each other, remove full rotations.
///   * To ensure hues can be rounded like other coordinates, divide by 360.
///   * To allow for small floating point errors, drop one significant digit.
///     Thanks to the previous step, all coordinates have (roughly) unit range.
///     Hence, multiply by suitable power of 10 and round the result.
///   * To ensure equal numbers are equal, normalize negative zeroes to their
///     positive equivalent.
///   * To produce a result suitable to hashing and equality testing in Rust,
///     convert to bits.
///
/// The above steps ensure that all coordinates have a range close to the unit
/// range before the scaling and rounding step, which suffices for correctness.
/// Full normalization would need to account for a and b in Oklab ranging from
/// -0.5 to 0.5 and chroma in Oklch from 0 to 0.5.
pub fn normalize(space: ColorSpace, coordinates: &[f64; 3]) -> [u64; 3] {
    let [mut c1, mut c2, mut c3] = *coordinates;

    // Ensure all coordinates are numbers
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

    // Ensure only partial rotations and unit range
    if space.is_polar() {
        c3 = c3.rem_euclid(360.0) / 360.0
    }

    // Ensure one less significant digit
    let factor = 10.0_f64.powi((f64::DIGITS as i32) - 1);
    c1 = (factor * c1).round();
    c2 = (factor * c2).round();
    c3 = (factor * c3).round();

    // Ensure canonical zero
    if c1 == -c1 {
        c1 = 0.0;
    }
    if c2 == -c2 {
        c2 = 0.0
    }
    if c3 == -c3 {
        c3 = 0.0
    }

    // Et voilà!
    [c1.to_bits(), c2.to_bits(), c3.to_bits()]
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
// Parse String
// ====================================================================================================================

use std::error::Error;

/// A parse error for colors.
#[derive(Debug)]
pub struct ParseColorError {
    /// The offending color string.
    pub text: String,
    /// Optionally, an underlying error.
    pub source: Option<Box<dyn Error>>,
}

impl ParseColorError {
    /// Create a new parse color error with just the offending text.
    pub fn with_text(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            source: None,
        }
    }

    /// Create a new parse color error with offending text and source error.
    pub fn new<E: Error + 'static>(text: &str, source: E) -> Self {
        Self {
            text: text.to_owned(),
            source: Some(Box::new(source)),
        }
    }
}

impl std::fmt::Display for ParseColorError {
    /// Format this parse color error.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\"{}\" is not a valid color", self)
    }
}

impl Error for ParseColorError {
    /// Access the source for this parse color error.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref()
    }
}

/// Parse the textual representation of a color. This function recognizes
/// the following formats:
///
///   * the hashed hexadecimal format familiar from the web, e.g., `#efbbfc`;
///   * the X Windows color format, e.g., `rgb:<hex>/<hex>/<hex>`;
///
/// The hashed format allows for 1 digit per coordinate and 2 digits per
/// coordinate, consistently for all coordinates. The X Windows format allows
/// between 1 and 4 coordinates, seemingly independently for each
/// coordinate.
pub fn parse(s: &str) -> Result<(ColorSpace, [f64; 3]), ParseColorError> {
    // Both

    let mut t1 = None;
    let mut t2 = None;
    let mut t3 = None;
    let mut scale = true;

    if s.starts_with("rgb:") {
        let mut coor_iter = s.get(4..).unwrap().split('/');
        t1 = coor_iter.next();
        t2 = coor_iter.next();
        t3 = coor_iter.next();
    } else if s.starts_with('#') {
        scale = false;

        if s.len() == 4 {
            t1 = s.get(1..2);
            t2 = s.get(2..3);
            t3 = s.get(3..4);
        } else if s.len() == 7 {
            t1 = s.get(1..3);
            t2 = s.get(3..5);
            t3 = s.get(5..7);
        }
    }

    if t1.is_none() || t2.is_none() || t3.is_none() {
        return Err(ParseColorError::with_text(s));
    }

    let t1 = t1.unwrap();
    let t2 = t2.unwrap();
    let t3 = t3.unwrap();
    if t1.len() > 4 || t2.len() > 4 || t3.len() > 4 {
        return Err(ParseColorError::with_text(s));
    }

    let make_error = |e| ParseColorError::new(s, e);
    let mut n1 = u8::from_str_radix(t1, 16).map_err(make_error)?;
    let mut n2 = u8::from_str_radix(t2, 16).map_err(make_error)?;
    let mut n3 = u8::from_str_radix(t3, 16).map_err(make_error)?;

    if scale {
        return Ok((
            ColorSpace::Srgb,
            [
                (n1 as f64) / (16.0_f64.powi(t1.len() as i32) - 1.0),
                (n2 as f64) / (16.0_f64.powi(t2.len() as i32) - 1.0),
                (n3 as f64) / (16.0_f64.powi(t3.len() as i32) - 1.0),
            ],
        ));
    }

    if t1.len() == 1 {
        n1 = 16 * n1 + n1;
        n2 = 16 * n2 + n2;
        n3 = 16 * n3 + n3;
    }

    Ok((
        ColorSpace::Srgb,
        [
            (n1 as f64) / 255.0,
            (n2 as f64) / 255.0,
            (n3 as f64) / 255.0,
        ],
    ))
}

// ====================================================================================================================

#[cfg(test)]
mod test {
    use super::ColorSpace::*;
    use super::*;

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

    pub fn same_coordinates(
        space: ColorSpace,
        coordinates1: &[f64; 3],
        coordinates2: &[f64; 3],
    ) -> bool {
        let n1 = normalize(space, coordinates1);
        let n2 = normalize(space, coordinates2);

        n1 == n2
    }

    #[test]
    fn test_equivalence() {
        // Good grief: In Python 0.5 rounds down. In Rust, it rounds up.
        let base = 10.0_f64.powi(-(f64::DIGITS as i32));
        let f00 = 0.0;
        let f01 = base;
        let f02 = 2.0 * base;
        let f03 = 3.0 * base;
        let f05 = 5.0 * base;
        let f07 = 7.0 * base;
        let f09 = 9.0 * base;
        let f10 = 10.0 * base;
        let f20 = 20.0 * base;

        assert!(same_coordinates(Srgb, &[f01, f02, f03], &[f00, f00, f00],));

        assert!(same_coordinates(Srgb, &[f05, f07, f09], &[f10, f10, f10],));

        assert!(!same_coordinates(Srgb, &[f10, f10, f10], &[f20, f20, f20],));
    }

    #[test]
    fn test_colors() {
        for &color in [&BLACK, &YELLOW, &BLUE, &WHITE].iter() {
            // Test all one-hop conversions
            let linear_srgb = rgb_to_linear_rgb(&color.srgb);
            assert!(same_coordinates(
                LinearSrgb,
                &linear_srgb,
                &color.linear_srgb
            ));

            let srgb = linear_rgb_to_rgb(&linear_srgb);
            assert!(same_coordinates(Srgb, &srgb, &color.srgb));

            let xyz = linear_srgb_to_xyz(&linear_srgb);
            assert!(same_coordinates(Xyz, &xyz, &color.xyz));

            let also_linear_srgb = xyz_to_linear_srgb(&xyz);
            assert!(same_coordinates(
                LinearSrgb,
                &also_linear_srgb,
                &linear_srgb
            ));

            let linear_p3 = xyz_to_linear_display_p3(&xyz);
            assert!(same_coordinates(
                LinearDisplayP3,
                &linear_p3,
                &color.linear_p3
            ));

            let also_xyz = linear_display_p3_to_xyz(&linear_p3);
            assert!(same_coordinates(Xyz, &also_xyz, &xyz));

            let p3 = linear_rgb_to_rgb(&linear_p3);
            assert!(same_coordinates(DisplayP3, &p3, &color.p3));

            let also_linear_p3 = rgb_to_linear_rgb(&p3);
            assert!(same_coordinates(
                LinearDisplayP3,
                &also_linear_p3,
                &linear_p3
            ));

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
        assert!(same_coordinates(
            Srgb,
            &srgb,
            &[-0.5116049825853448, 1.0182656579378029, -0.3106746212905826]
        ));

        let srgb_mapped = map_to_gamut(Srgb, &srgb);
        assert!(same_coordinates(
            Srgb,
            &srgb_mapped,
            &[0.0, 0.9857637107710327, 0.15974244397343723]
        ));

        // A very yellow yellow.
        let p3 = [1.0, 1.0, 0.0];
        let srgb = convert(DisplayP3, Srgb, &p3);
        assert!(same_coordinates(
            Srgb,
            &srgb,
            &[0.9999999999999999, 0.9999999999999999, -0.3462679629331063]
        ));

        let linear_srgb = convert(DisplayP3, LinearSrgb, &p3);
        assert!(same_coordinates(
            LinearSrgb,
            &linear_srgb,
            &[1.0, 1.0000000000000002, -0.09827360014096621]
        ));

        let linear_srgb_mapped = map_to_gamut(LinearSrgb, &linear_srgb);
        assert!(same_coordinates(
            LinearSrgb,
            &linear_srgb_mapped,
            &[0.9914525477996114, 0.9977581974546286, 0.0]
        ));
    }
}
