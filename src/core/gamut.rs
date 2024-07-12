use crate::core::conversion::okxch_to_okxab;
use crate::core::{convert, delta_e_ok, normalize};
use crate::{ColorSpace, Float};

/// Determine whether the color is a gray. For maximally consistent results,
/// this functions tests chroma and hue in Oklch/Oklrch. If the color is in
/// neither color space, this function first converts the coordinates.
pub(crate) fn is_gray(space: ColorSpace, coordinates: &[Float; 3]) -> bool {
    let coordinates = match space {
        ColorSpace::Oklch | ColorSpace::Oklrch => *coordinates,
        _ => convert(space, ColorSpace::Oklch, coordinates),
    };

    is_gray_chroma_hue(coordinates[1], coordinates[2])
}

const MAX_GRAY_CHROMA: Float = 0.01;

/// Determine whether the chroma and hue are gray.
#[inline]
pub(crate) fn is_gray_chroma_hue(chroma: Float, hue: Float) -> bool {
    hue.is_nan() || chroma < MAX_GRAY_CHROMA
}

/// Determine whether the coordinates are in gamut for their color space.
pub(crate) fn in_gamut(space: ColorSpace, coordinates: &[Float; 3]) -> bool {
    if space.is_rgb() {
        coordinates.iter().all(|c| 0.0 <= *c && *c <= 1.0)
    } else {
        true
    }
}

/// Clip the coordinates to the gamut of their color space.
pub(crate) fn clip(space: ColorSpace, coordinates: &[Float; 3]) -> [Float; 3] {
    if space.is_rgb() {
        let [r, g, b] = coordinates;
        [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)]
    } else {
        *coordinates
    }
}

const JND: Float = 0.02;
const EPSILON: Float = 0.0001;

/// Map the given color coordinates into the gamut of their color space.
///
/// This function implements the CSS Color 4 [gamut mapping
/// algorithm](https://drafts.csswg.org/css-color/#css-gamut-mapping). It
/// basically performs a binary search in Oklch for a color with less chroma
/// than the original, whose clipped version is within the *just noticeable
/// difference*. Since, by definition, the clipped version also is in gamut, it
/// becomes the result of the search.
pub(crate) fn to_gamut(space: ColorSpace, coordinates: &[Float; 3]) -> [Float; 3] {
    use ColorSpace::*;

    let coordinates = normalize(space, coordinates);

    // If the color space is unbounded, there is nothing to map to
    if !space.is_bounded() {
        return coordinates;
    }

    // Preliminary 1/2: Clamp Lightness
    let origin_as_oklch = convert(space, Oklch, &coordinates);
    let l = origin_as_oklch[0];
    if l >= 1.0 {
        return convert(Oklch, space, &[1.0, 0.0, 0.0]);
    }
    if l <= 0.0 {
        return convert(Oklch, space, &[0.0, 0.0, 0.0]);
    }

    // Preliminary 2/2: Check gamut
    if in_gamut(space, &coordinates) {
        return coordinates;
    }

    // Goal: Minimize just noticeable difference between current and clipped
    // colors
    let mut current_as_oklch = origin_as_oklch;
    let mut clipped_as_target = clip(space, &convert(Oklch, space, &current_as_oklch));

    let difference = delta_e_ok(
        &convert(space, Oklab, &clipped_as_target),
        &okxch_to_okxab(&current_as_oklch),
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

        let current_as_target = convert(Oklch, space, &current_as_oklch);

        if min_in_gamut && in_gamut(space, &current_as_target) {
            min = chroma;
            continue;
        }

        clipped_as_target = clip(space, &current_as_target);

        let difference = delta_e_ok(
            &convert(space, Oklab, &clipped_as_target),
            &okxch_to_okxab(&current_as_oklch),
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

#[cfg(test)]
mod test {
    use super::to_gamut;
    use crate::core::{assert_same_coordinates, convert, ColorSpace};

    #[test]
    fn test_gamut() {
        // A very green green.
        let p3 = [0.0, 1.0, 0.0];
        let srgb = convert(ColorSpace::DisplayP3, ColorSpace::Srgb, &p3);
        assert_same_coordinates!(
            ColorSpace::Srgb,
            &srgb,
            &[-0.5116049825853448, 1.0182656579378029, -0.3106746212905826],
        );

        let srgb_mapped = to_gamut(ColorSpace::Srgb, &srgb);
        assert_same_coordinates!(
            ColorSpace::Srgb,
            &srgb_mapped,
            &[0.0, 0.9857637107710327, 0.15974244397343723],
        );

        // A very yellow yellow.
        let p3 = [1.0, 1.0, 0.0];
        let srgb = convert(ColorSpace::DisplayP3, ColorSpace::Srgb, &p3);
        assert_same_coordinates!(
            ColorSpace::Srgb,
            &srgb,
            &[0.9999999999999999, 0.9999999999999999, -0.3462679629331063],
        );

        let linear_srgb = convert(ColorSpace::DisplayP3, ColorSpace::LinearSrgb, &p3);
        assert_same_coordinates!(
            ColorSpace::LinearSrgb,
            &linear_srgb,
            &[1.0, 1.0000000000000002, -0.09827360014096621],
        );

        let linear_srgb_mapped = to_gamut(ColorSpace::LinearSrgb, &linear_srgb);
        assert_same_coordinates!(
            ColorSpace::LinearSrgb,
            &linear_srgb_mapped,
            &[0.9914525477996114, 0.9977581974546286, 0.0],
        );
    }
}
