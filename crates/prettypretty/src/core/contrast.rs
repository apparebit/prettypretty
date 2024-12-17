use crate::core::{convert, ColorSpace};
use crate::Float;

/// Scale the lightness of the given color in Oklrch by the given factor.
#[inline]
pub(crate) fn scale_lightness(
    space: ColorSpace,
    coordinates: &[Float; 3],
    factor: Float,
) -> [Float; 3] {
    let [lr, c, h] = convert(space, ColorSpace::Oklrch, coordinates);
    [factor * lr, c, h]
}

/// The coefficients for computing the contrast luminance for sRGB
/// coordinates.
const SRGB_CONTRAST: &[Float; 3] = &[0.2126729, 0.7151522, 0.0721750];

/// The coefficients for computing the contrast luminance for Display P3
/// coordinates.
#[allow(clippy::excessive_precision)]
const P3_CONTRAST: &[Float; 3] = &[0.2289829594805780, 0.6917492625852380, 0.0792677779341829];

fn to_contrast_luminance(coefficients: &[Float; 3], coordinates: &[Float; 3]) -> Float {
    fn linearize(value: Float) -> Float {
        let magnitude = value.abs();
        magnitude.powf(2.4).copysign(value)
    }

    let [c1, c2, c3] = *coefficients;
    let [r, g, b] = *coordinates;

    linearize(r).mul_add(c1, linearize(g).mul_add(c2, linearize(b) * c3))
}

/// Compute the contrast luminance for the given sRGB coordinates.
pub(crate) fn to_contrast_luminance_srgb(coordinates: &[Float; 3]) -> Float {
    to_contrast_luminance(SRGB_CONTRAST, coordinates)
}

/// Compute the contrast luminance for the given Display P3 coordinates.
pub(crate) fn to_contrast_luminance_p3(coordinates: &[Float; 3]) -> Float {
    to_contrast_luminance(P3_CONTRAST, coordinates)
}

const BLACK_THRESHOLD: Float = 0.022;
const BLACK_EXPONENT: Float = 1.414;
const INPUT_CLAMP: Float = 0.0005;
const SCALE: Float = 1.14;
const OFFSET: Float = 0.027;
const OUTPUT_CLAMP: Float = 0.1;

/// Compute the perceptual contrast between text and background.
///
/// Using an algorithm that is surprisingly similar to the [Accessible
/// Perceptual Contrast Algorithm](https://github.com/Myndex/apca-w3),
/// version 0.0.98G-4g, this function computes the perceptual contrast
/// between the given contrast luminance for foreground and background.
///
/// The arguments to this function are *not* interchangeable. The first
/// argument must be the contrast luminance for the foreground, i.e., text,
/// and the second argument must be the contrast luminance for the
/// background.
///
/// Said contrast luminance is a non-standard quantity (i.e., *not* the Y in
/// XYZ). If both colors are in-gamut for sRGB, the contrast luminance
/// should be computed with [`to_contrast_luminance_srgb`], falling back on
/// [`to_contrast_luminance_p3`] otherwise.
pub(crate) fn to_contrast(text_luminance: Float, background_luminance: Float) -> Float {
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

        if -OUTPUT_CLAMP < contrast {
            0.0
        } else {
            contrast + OFFSET
        }
    }
}

#[cfg(test)]
mod test {
    use super::{to_contrast, to_contrast_luminance_srgb};
    use crate::assert_close_enough;

    #[test]
    fn test_contrast() {
        let blue = to_contrast_luminance_srgb(&[104.0 / 255.0, 114.0 / 255.0, 1.0]);

        // Compare contrast of black vs white against a medium blue tone:
        assert_close_enough!(to_contrast(0.0, blue), 0.38390416110716424);
        assert_close_enough!(to_contrast(1.0, blue), -0.7119199952225724);
    }
}
