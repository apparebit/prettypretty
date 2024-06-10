//! # Core Color
//!
//! This module implements the high-resolution, high-quality algorithms enabling
//! the parent module's `Color` abstraction.
//!
//! Unlike that struct, this module eschews encapsulation in favor of simplicity
//! and uniformity. That has significant benefits in terms of reduced
//! complexity, notably for conversions, and hence seems acceptable. After all,
//! only [`ColorSpace`] is exposed outside this crate and this module largely is
//! an "implementation detail."
//!
//! In some more detail, this module represents color spaces as tag-like
//! variants *without* associated values and color coordinates as three-element
//! `f64` arrays. It does not (and cannot) enforce limits on bounded color
//! spaces' coordinates, preserving out of gamut colors instead. It does,
//! however, implement operations for in-gamut testing, clipping, and
//! gamut-mapping.
//!
//! All currently supported color spaces, including XYZ, use D65 as white point.
//! This module does not support chromatic adaptation.
//!
//! Not-a-number *is* a valid coordinate value for hue in Oklch. It necessarily
//! implies that the chroma is zero, i.e., the color is gray including black and
//! white. To correctly implement equality testing and hashing, this module
//! provides [`normalize`].
//!
//! Function arguments are ordered so that scalar arguments come before
//! coordinates, which may be inline array literals.

// ====================================================================================================================
// Color Space Tags
// ====================================================================================================================

/// The enumeration of supported color spaces
///
///
/// # The Oklab Variations
///
/// This crate supports the
/// [Oklab/Oklch](https://bottosson.github.io/posts/oklab/) and
/// [Oklrab/Oklrch](https://bottosson.github.io/posts/colorpicker/#intermission---a-new-lightness-estimate-for-oklab)
/// color spaces. All four really are variations of the same perceptually
/// uniform color space, which, like CIELAB, uses one coordinate for lightness
/// and two coordinates for "colorness."
///
/// Oklab and Oklch reflect the original design. They improve on CIELAB by using
/// the D65 standard illuminant (not the print-oriented D50), which is also used
/// by sRGB and Display P3. They further improve on CIELAB by avoiding visible
/// distortions around the blues. However, they also regress, since their
/// lightness is heavily biased towards dark tones. Oklrab and Oklrch result
/// from an update nine months later that changes lightness to closely resemble
/// CIELAB's far better, perceptually uniform lightness.
/// At the same time, Oklab's and Oklch's
///
/// Oklab/Oklrab use Cartesian coordinates a, b for colorness—with a varying
/// red/green and b varying blue/yellow. That makes both color spaces
/// well-suited to computing the relative distance of colors. In contrast,
/// Oklch/Oklrch use polar coordinates C, h—with C expressing chroma and h or hº
/// expressing hue. That makes both color spaces well-suited to modifying
/// colors.
///
/// Compared to the conversion between XYZ and Oklab, conversions between the
/// four variations are simpler mathematically and may not even change all
/// coordinates. After all, there are four three-dimensional color spaces but
/// only six distinct quantities:
///
/// | Color space | Lightness | Colorness 1 | Colorness 2 |
/// | :---------- | :-------: | :---------: | :---------: |
/// | Oklab       | L         | a           | b           |
/// | Oklch       | L         | C           | hº          |
/// | Oklrab      | Lr        | a           | b           |
/// | Oklrch      | Lr        | C           | hº          |
///
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    /// [sRGB](https://en.wikipedia.org/wiki/SRGB) has long served as the
    /// default color space for the web.
    Srgb,

    /// The linear version of sRGB.
    LinearSrgb,

    /// [Display P3](https://en.wikipedia.org/wiki/DCI-P3) has a wider gamut
    /// than sRGB, that is, it accommodates more colors. It seems
    /// well-positioned to become the next default color space.
    DisplayP3,

    /// The linear version of Display P3.
    LinearDisplayP3,

    /// [Oklab](https://bottosson.github.io/posts/oklab/) is a perceptually
    /// uniform color space that improves upon CIELAB.
    Oklab,

    /// Oklch is the polar version of Oklab.
    Oklch,

    /// Oklrab is Oklab but with an [improved lightness
    /// Lr](https://bottosson.github.io/posts/colorpicker/#intermission---a-new-lightness-estimate-for-oklab).
    Oklrab,

    /// Oklrch is Oklch, i.e., the polar version of Oklab, but with an [improved
    /// lightness
    /// Lr](https://bottosson.github.io/posts/colorpicker/#intermission---a-new-lightness-estimate-for-oklab).
    Oklrch,

    /// [XYZ](https://en.wikipedia.org/wiki/CIE_1931_color_space) is a
    /// foundational color space. This crate uses XYZ with a [D65 standard
    /// illuminant](https://en.wikipedia.org/wiki/Standard_illuminant) (or white
    /// point).
    Xyz,
}

impl ColorSpace {
    /// Determine whether this color space is polar. Oklch and Oklrch currently
    /// are the only polar color spaces.
    pub const fn is_polar(&self) -> bool {
        matches!(*self, Self::Oklch | Self::Oklrch)
    }

    /// Determine whether this color space is RGB, that is, has red, green, and
    /// blue coordinates. In-gamut colors for RGB color spaces have coordinates
    /// in unit range `0..=1`.
    pub const fn is_rgb(&self) -> bool {
        use ColorSpace::*;
        matches!(*self, Srgb | LinearSrgb | DisplayP3 | LinearDisplayP3)
    }

    /// Determine whether this color space is bounded. XYZ and the Okl** color
    /// spaces are *unbounded*, whereas the RGB color spaces are *bounded*.
    pub const fn is_bounded(&self) -> bool {
        use ColorSpace::*;
        matches!(*self, Srgb | LinearSrgb | DisplayP3 | LinearDisplayP3)
    }
}

// ====================================================================================================================
// Equality and Difference
// ====================================================================================================================

/// Normalize the coordinates in preparation of hashing and equality testing.
///
/// Note: In-gamut RGB coordinates and the lightness for Oklab and Oklch have
/// unit range already. No coordinate-specific normalization is required.
///
/// The implementation normalizes hues by computing the remainder of division by
/// 360, which removes full rotations, and then dividing by 360 to scale down to
/// unit range.
///
/// The implementation currently does not normalize chroma for Oklch as well as
/// a and b for Oklab because all three have bounds similar to the unit range.
/// Notably, chroma ranges from 0 to 0.5, and a/b range from -0.5 to 0.5. These
/// bounds are generous; the current CSS 4 Color draft uses 0.4 as the magnitude
/// for all non-zero bounds.
pub(crate) fn normalize(space: ColorSpace, coordinates: &[f64; 3]) -> [u64; 3] {
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
pub(crate) fn delta_e_ok(coordinates1: &[f64; 3], coordinates2: &[f64; 3]) -> f64 {
    let [L1, a1, b1] = coordinates1;
    let [L2, a2, b2] = coordinates2;

    let ΔL = L1 - L2;
    let Δa = a1 - a2;
    let Δb = b1 - b2;

    ΔL.mul_add(ΔL, Δa.mul_add(Δa, Δb * Δb)).sqrt()
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
fn rgb_to_linear_rgb(value: &[f64; 3]) -> [f64; 3] {
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
fn linear_rgb_to_rgb(value: &[f64; 3]) -> [f64; 3] {
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
fn linear_srgb_to_xyz(value: &[f64; 3]) -> [f64; 3] {
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
fn xyz_to_linear_srgb(value: &[f64; 3]) -> [f64; 3] {
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
fn linear_display_p3_to_xyz(value: &[f64; 3]) -> [f64; 3] {
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
fn xyz_to_linear_display_p3(value: &[f64; 3]) -> [f64; 3] {
    multiply(&XYZ_TO_LINEAR_DISPLAY_P3, value)
}

// --------------------------------------------------------------------------------------------------------------------

/// Convert coordinates for Oklch to Oklab or for Oklrch to Oklrab. This is a
/// one-hop, direct conversion.
#[allow(non_snake_case)]
fn oklch_to_oklab(value: &[f64; 3]) -> [f64; 3] {
    let [L, C, h] = *value;

    if h.is_nan() {
        [L, 0.0, 0.0]
    } else {
        let hue_radian = h.to_radians();
        [L, C * hue_radian.cos(), C * hue_radian.sin()]
    }
}

/// Convert coordinates for Oklab to Oklch or for Oklrab to Oklrch. This is a
/// one-hop, direct conversion.
#[allow(non_snake_case)]
fn oklab_to_oklch(value: &[f64; 3]) -> [f64; 3] {
    const EPSILON: f64 = 0.0002;

    let [L, a, b] = *value;
    let h = if a.abs() < EPSILON && b.abs() < EPSILON {
        f64::NAN
    } else {
        b.atan2(a).to_degrees()
    };

    [L, (a.powi(2) + b.powi(2)).sqrt(), h.rem_euclid(360.0)]
}

mod oklr {
    const K1: f64 = 0.206;
    const K2: f64 = 0.03;

    /// Convert coordinates for Oklab to Oklrab or for Oklch to Oklrch. This
    /// function replaces the lightness L with the [improved lightness
    /// Lr](https://bottosson.github.io/posts/colorpicker/#intermission---a-new-lightness-estimate-for-oklab).
    /// This is a one-hop, direct conversion.
    #[allow(non_snake_case)]
    pub(crate) fn oklab_to_oklrab(value: &[f64; 3]) -> [f64; 3] {
        let k3 = (1.0 + K1) / (1.0 + K2);
        let [L, a, b] = *value;
        let k3L = k3 * L;
        [
            0.5 * (k3L - K1 + ((k3L - K1) * (k3L - K1) + 4.0 * K2 * k3L).sqrt()),
            a,
            b,
        ]
    }

    /// Convert coordinates for Oklrab to Oklab or for Oklrch to Oklch. This
    /// function replaces the [improved lightness
    /// Lr](https://bottosson.github.io/posts/colorpicker/#intermission---a-new-lightness-estimate-for-oklab)
    /// with the original lightness L. This is a one-hop, direct conversion.
    #[allow(non_snake_case)]
    pub(crate) fn oklrab_to_oklab(value: &[f64; 3]) -> [f64; 3] {
        let k3 = (1.0 + K1) / (1.0 + K2);
        let [Lr, a, b] = *value;
        [(Lr * (Lr + K1)) / (k3 * (Lr + K2)), a, b]
    }
}

use oklr::{oklab_to_oklrab, oklrab_to_oklab};

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
fn oklab_to_xyz(value: &[f64; 3]) -> [f64; 3] {
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
fn xyz_to_oklab(value: &[f64; 3]) -> [f64; 3] {
    let [l, m, s] = multiply(&XYZ_TO_OKLMS, value);
    multiply(&OKLMS_TO_OKLAB, &[l.cbrt(), m.cbrt(), s.cbrt()])
}

// --------------------------------------------------------------------------------------------------------------------

/// Convert coordinates for sRGB to XYZ. This is a two-hop conversion.
fn srgb_to_xyz(value: &[f64; 3]) -> [f64; 3] {
    let linear_srgb = rgb_to_linear_rgb(value);
    linear_srgb_to_xyz(&linear_srgb)
}

/// Convert coordinates for XYZ to sRGB. This is a two-hop conversion.
fn xyz_to_srgb(value: &[f64; 3]) -> [f64; 3] {
    let linear_srgb = xyz_to_linear_srgb(value);
    linear_rgb_to_rgb(&linear_srgb)
}

/// Convert coordinates for Display P3 to XYZ. This is a two-hop conversion.
fn display_p3_to_xyz(value: &[f64; 3]) -> [f64; 3] {
    let linear_p3 = rgb_to_linear_rgb(value);
    linear_display_p3_to_xyz(&linear_p3)
}

/// Convert coordinates for XYZ to Display P3. This is a two-hop conversion.
fn xyz_to_display_p3(value: &[f64; 3]) -> [f64; 3] {
    let linear_p3 = xyz_to_linear_display_p3(value);
    linear_rgb_to_rgb(&linear_p3)
}

/// Convert coordinates for Oklch to XYZ. This is a two-hop conversion.
fn oklch_to_xyz(value: &[f64; 3]) -> [f64; 3] {
    let oklab = oklch_to_oklab(value);
    oklab_to_xyz(&oklab)
}

/// Convert coordinates for XYZ to Oklch. This is a two-hop conversion.
fn xyz_to_oklch(value: &[f64; 3]) -> [f64; 3] {
    let oklab = xyz_to_oklab(value);
    oklab_to_oklch(&oklab)
}

/// Convert coordinates for Oklrab to XYZ. This is a two-hop conversion.
fn oklrab_to_xyz(value: &[f64; 3]) -> [f64; 3] {
    let oklab = oklrab_to_oklab(value);
    oklab_to_xyz(&oklab)
}
/// Convert coordinates for XYZ to Oklrab. This is a two-hop conversion.
fn xyz_to_oklrab(value: &[f64; 3]) -> [f64; 3] {
    let oklab = xyz_to_oklab(value);
    oklab_to_oklrab(&oklab)
}

/// Convert coordinates for Oklab to Oklrch. This is a two-hop conversion.
fn oklab_to_oklrch(value: &[f64; 3]) -> [f64; 3] {
    let oklch = oklab_to_oklch(value);
    oklab_to_oklrab(&oklch)
}

/// Convert coordinates for Oklrch to Oklab. This is a two-hop conversion.
fn oklrch_to_oklab(value: &[f64; 3]) -> [f64; 3] {
    let oklch = oklrab_to_oklab(value);
    oklch_to_oklab(&oklch)
}

/// Convert coordinates for Oklrab to Oklch. This is a two-hop conversion.
fn oklrab_to_oklch(value: &[f64; 3]) -> [f64; 3] {
    let oklab = oklrab_to_oklab(value);
    oklab_to_oklch(&oklab)
}

/// Convert coordinates for Oklch to Oklrab. This is a two-hop conversion.
fn oklch_to_oklrab(value: &[f64; 3]) -> [f64; 3] {
    let oklab = oklch_to_oklab(value);
    oklab_to_oklrab(&oklab)
}

// --------------------------------------------------------------------------------------------------------------------

/// Convert coordinates for Oklrch to XYZ. This is a three-hop conversion.
fn oklrch_to_xyz(value: &[f64; 3]) -> [f64; 3] {
    let oklch = oklrab_to_oklab(value);
    oklch_to_xyz(&oklch)
}
/// Convert coordinates for XYZ to Oklrab. This is a three-hop conversion.
fn xyz_to_oklrch(value: &[f64; 3]) -> [f64; 3] {
    let oklch = xyz_to_oklch(value);
    oklab_to_oklrab(&oklch)
}

// --------------------------------------------------------------------------------------------------------------------

/// Convert the coordinates from the `from_space` to the `to_space`.
pub(crate) fn convert(
    from_space: ColorSpace,
    to_space: ColorSpace,
    coordinates: &[f64; 3],
) -> [f64; 3] {
    use ColorSpace::*;

    // 1. Handle identities
    if from_space == to_space {
        return *coordinates;
    }

    // 2. Handle single-branch conversions, ignoring root
    match (from_space, to_space) {
        // Single-hop RGB conversions
        (Srgb, LinearSrgb) | (DisplayP3, LinearDisplayP3) => return rgb_to_linear_rgb(coordinates),
        (LinearSrgb, Srgb) | (LinearDisplayP3, DisplayP3) => return linear_rgb_to_rgb(coordinates),

        // Single-hop Ok* conversions
        (Oklch, Oklab) | (Oklrch, Oklrab) => return oklch_to_oklab(coordinates),
        (Oklab, Oklch) | (Oklrab, Oklrch) => return oklab_to_oklch(coordinates),
        (Oklab, Oklrab) | (Oklch, Oklrch) => return oklab_to_oklrab(coordinates),
        (Oklrab, Oklab) | (Oklrch, Oklch) => return oklrab_to_oklab(coordinates),

        // Two-hop Ok* conversions
        (Oklrch, Oklab) => return oklrch_to_oklab(coordinates),
        (Oklch, Oklrab) => return oklch_to_oklrab(coordinates),
        (Oklab, Oklrch) => return oklab_to_oklrch(coordinates),
        (Oklrab, Oklch) => return oklrab_to_oklch(coordinates),
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
        Oklrch => oklrch_to_xyz(coordinates),
        Oklrab => oklrab_to_xyz(coordinates),
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
        Oklrch => xyz_to_oklrch(&intermediate),
        Oklrab => xyz_to_oklrab(&intermediate),
        Xyz => intermediate,
    }
}

// ====================================================================================================================
// Gamut
// ====================================================================================================================

/// Determine whether the coordinates are in gamut for the color space.
pub(crate) fn in_gamut(space: ColorSpace, coordinates: &[f64; 3]) -> bool {
    use ColorSpace::*;

    match space {
        Srgb | LinearSrgb | DisplayP3 | LinearDisplayP3 => {
            coordinates.iter().all(|c| 0.0 <= *c && *c <= 1.0)
        }
        _ => true,
    }
}

/// Clip the coordinates to the gamut of the color space.
pub(crate) fn clip(space: ColorSpace, coordinates: &[f64; 3]) -> [f64; 3] {
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
pub(crate) fn map_to_gamut(target: ColorSpace, coordinates: &[f64; 3]) -> [f64; 3] {
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
// Color Contrast
// ====================================================================================================================

// Limit visibility of many contrast-specific constants
mod contrast {
    pub(crate) const SRGB_CONTRAST: [f64; 3] = [0.2126729, 0.7151522, 0.0721750];
    #[allow(clippy::excessive_precision)]
    pub(crate) const P3_CONTRAST: [f64; 3] =
        [0.2289829594805780, 0.6917492625852380, 0.0792677779341829];

    /// Convert the given color coordinates to perceptual contrast luminance.
    /// The coefficients are [`SRGB_CONTRAST`] for sRGB coordinates and
    /// [`P3_CONTRAST`] for Display P3 coordinates. Though Display P3 should
    /// only be used for colors that are out of gamut for sRGB.
    pub(crate) fn to_contrast_luminance(coefficients: &[f64; 3], coordinates: &[f64; 3]) -> f64 {
        fn linearize(value: f64) -> f64 {
            let magnitude = value.abs();
            magnitude.powf(2.4).copysign(value)
        }

        let [c1, c2, c3] = *coefficients;
        let [r, g, b] = *coordinates;

        c1 * linearize(r) + c2 * linearize(g) + c3 * linearize(b)
    }

    const BLACK_THRESHOLD: f64 = 0.022;
    const BLACK_EXPONENT: f64 = 1.414;
    const INPUT_CLAMP: f64 = 0.0005;
    const SCALE: f64 = 1.14;
    const OFFSET: f64 = 0.027;
    const OUTPUT_CLAMP: f64 = 0.1;

    /// Compute the perceptual contrast for the text and background luminance
    /// values. This function uses an algorithm that is surprisingly similar to
    /// the [Accessible Perceptual Contrast
    /// Algorithm](https://github.com/Myndex/apca-w3), version 0.0.98G-4g.
    pub(crate) fn to_contrast(text_luminance: f64, background_luminance: f64) -> f64 {
        // Also see https://github.com/w3c/silver/issues/645

        // Make sure the luminance values are legit
        if text_luminance.is_nan()
            || !(0.0..=1.1).contains(&text_luminance)
            || background_luminance.is_nan()
            || !(0.0..=1.1).contains(&background_luminance)
        {
            return 0.0;
        }

        // Soft clip black
        let text_luminance = if text_luminance < BLACK_THRESHOLD {
            text_luminance + (BLACK_THRESHOLD - text_luminance).powf(BLACK_EXPONENT)
        } else {
            text_luminance
        };

        let background_luminance = if background_luminance < BLACK_THRESHOLD {
            background_luminance + (BLACK_THRESHOLD - background_luminance).powf(BLACK_EXPONENT)
        } else {
            background_luminance
        };

        // Clamp small ΔY  to zero
        if (text_luminance - background_luminance).abs() < INPUT_CLAMP {
            return 0.0;
        };

        // Compute Lc (lightness contrast)
        if text_luminance < background_luminance {
            // Black on white
            let contrast = SCALE * (background_luminance.powf(0.56) - text_luminance.powf(0.57));

            if contrast < OUTPUT_CLAMP {
                0.0
            } else {
                contrast - OFFSET
            }
        } else {
            // White on black
            let contrast = SCALE * (background_luminance.powf(0.65) - text_luminance.powf(0.62));

            if contrast > -OUTPUT_CLAMP {
                0.0
            } else {
                contrast + OFFSET
            }
        }
    }
}

pub(crate) use contrast::{to_contrast, to_contrast_luminance, P3_CONTRAST, SRGB_CONTRAST};

// ====================================================================================================================

#[cfg(test)]
mod test {
    use super::ColorSpace::*;
    use super::*;
    use crate::util::almost_eq;

    #[allow(dead_code)]
    struct Representations {
        spec: &'static str,
        srgb: [f64; 3],
        linear_srgb: [f64; 3],
        p3: [f64; 3],
        linear_p3: [f64; 3],
        oklch: [f64; 3],
        oklab: [f64; 3],
        oklrch: [f64; 3],
        oklrab: [f64; 3],
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
        oklrch: [0.0, 0.0, f64::NAN],
        oklrab: [0.0, 0.0, 0.0],
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
        oklrch: [0.8385912822460642, 0.1760097742886813, 89.440876452466],
        oklrab: [
            0.8385912822460642,
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
        oklrch: [0.5253778775789848, 0.18665606306724153, 259.66681920272595],
        oklrab: [
            0.5253778775789848,
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
        oklrch: [1.0000000000000002, 0.0, f64::NAN],
        oklrab: [1.0000000000000002, 0.0, 0.0],
    };

    pub fn same_coordinates(
        space: ColorSpace,
        coordinates1: &[f64; 3],
        coordinates2: &[f64; 3],
    ) -> bool {
        let n1 = dbg!(normalize(space, coordinates1));
        let n2 = dbg!(normalize(space, coordinates2));

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

            let oklrab = oklab_to_oklrab(&oklab);

            assert!(same_coordinates(Oklrab, &oklrab, &color.oklrab));

            let oklab_too = oklrab_to_oklab(&oklrab);
            assert!(same_coordinates(Oklab, &oklab_too, &color.oklab));

            let oklrch = oklab_to_oklrab(&oklch);
            assert!(same_coordinates(Oklrch, &oklrch, &color.oklrch));

            let oklch_too = oklrab_to_oklab(&oklrch);
            assert!(same_coordinates(Oklch, &oklch_too, &color.oklch));
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

    #[test]
    fn test_contrast() {
        let blue = to_contrast_luminance(&SRGB_CONTRAST, &[104.0 / 255.0, 114.0 / 255.0, 1.0]);

        // Compare contrast of black vs white against a medium blue tone:
        assert!(almost_eq(dbg!(to_contrast(0.0, blue)), 0.38390416110716424));
        assert!(almost_eq(dbg!(to_contrast(1.0, blue)), -0.7119199952225724));
    }
}