#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use crate::core::conversion::okxch_to_okxab;
use crate::core::{convert, delta_e_ok, normalize};
#[cfg(feature = "gamut")]
use crate::Color;
use crate::{ColorSpace, Float};

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
    if 1.0 <= l {
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

    while EPSILON < max - min {
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

// ====================================================================================================================

/// A step while traversing gamut boundaries.
///
/// Determining the gamut boundaries for an RGB color space is the same as
/// traversing the edges of the corresponding RGB cube. This enum defines the
/// corresponding operations. A traversal comprises several paths, each of which
/// starts with a `MoveTo` and ends with either a `LineTo` or `CloseWith`. The
/// latter repeats the color of the path's `MoveTo`.
#[cfg(feature = "gamut")]
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, frozen, hash, module = "prettypretty.color.gamut")
)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GamutTraversalStep {
    /// Move to the color's coordinates to start a new path.
    MoveTo(Color),
    /// Continue the current path with a line to the color's coordinates.
    LineTo(Color),
    /// Close the current path with a line to its starting color.
    CloseWith(Color),
}

#[cfg(feature = "gamut")]
#[cfg_attr(feature = "pyffi", pymethods)]
impl GamutTraversalStep {
    /// Get this step's color.
    pub fn color(&self) -> Color {
        match self {
            Self::MoveTo(color) => color.clone(),
            Self::LineTo(color) => color.clone(),
            Self::CloseWith(color) => color.clone(),
        }
    }

    /// Get a debug representation. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        match self {
            Self::MoveTo(ref color) => format!("GamutTraversalStep.MoveTo({:?}", color),
            Self::LineTo(ref color) => format!("GamutTraversalStep.LineTo({:?}", color),
            Self::CloseWith(ref color) => format!("GamutTraversalStep.CloseWith({:?}", color),
        }
    }
}

/// The gamut traversal's segment.
#[cfg(feature = "gamut")]
#[derive(Copy, Clone, Debug)]
enum GamutEdge {
    Start,
    Blue2Cyan,
    Cyan2Green,
    Green2Yellow,
    Yellow2Red,
    Red2Magenta,
    Magenta2Blue,
    Blue2Black,
    Cyan2White,
    Green2Black,
    Yellow2White,
    Red2Black,
    Magenta2White,
    Done,
}

/// An iterator for traversing RGB gamut boundaries.
///
/// Use [`ColorSpace::gamut`] to create new instances.
///
/// In the unit-normal representation used by prettypretty's [`Color`], any RGB
/// color space forms a cube with the following eight corners:
///
///   * the red, green, and blue primaries;
///   * the yellow, cyan, and magenta secondaries;
///   * black and white.
///
/// Hence, traversing the boundaries of its gamut is the same as traversing the
/// cube's twelve edges. This iterator yields [`GamutTraversalStep`] instances
/// for seven paths that cover each of the cube's twelve edges exactly once, in
/// the folling order:
///
///   * the closed path from blue to cyan to green to yellow to red to magenta
///     and blue again;
///   * the path from blue to black;
///   * the path from cyan to white;
///   * the path from green to black;
///   * the path from yellow to white;
///   * the path from red to black;
///   * the path from magenta to white.
///
/// Since the first path traverses six edges of the cube and the six remaining
/// paths traverse a single edge each, the seven paths together cover all twelve
/// edges of the cube.
///
/// Each path starts with a `MoveTo` step and ends with either `LineTo` if open
/// or `CloseWith` if closed. The step's color provides the coordinates for the
/// step. They always are for the color space whose boundaries are being traced
/// and in-gamut, if barely.
#[cfg(feature = "gamut")]
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.gamut"))]
#[derive(Debug)]
pub struct GamutTraversal {
    space: ColorSpace,
    max_component: usize,
    remaining: usize,
    edge: GamutEdge,
    r: usize,
    g: usize,
    b: usize,
}

#[cfg(feature = "gamut")]
impl GamutTraversal {
    pub(crate) fn new(space: ColorSpace, edge_length: usize) -> Option<Self> {
        if !space.is_rgb() || edge_length < 2 {
            None
        } else {
            Some(Self {
                space,
                max_component: edge_length - 1,
                remaining: 12 * edge_length - 5,
                edge: GamutEdge::Start,
                r: 0,
                g: 0,
                b: edge_length - 1,
            })
        }
    }
}

#[cfg(feature = "gamut")]
impl Iterator for GamutTraversal {
    type Item = GamutTraversalStep;

    fn next(&mut self) -> Option<Self::Item> {
        use GamutEdge::*;
        use GamutTraversalStep::*;

        if matches!(self.edge, Done) {
            return None;
        }

        self.remaining -= 1;
        let denominator = self.max_component as Float;
        let color = Color::new(
            self.space,
            [
                self.r as Float / denominator,
                self.g as Float / denominator,
                self.b as Float / denominator,
            ],
        );

        let result = match self.edge {
            Start => {
                self.g += 1;
                self.edge = Blue2Cyan;

                MoveTo(color)
            }
            Blue2Cyan => {
                if self.g < self.max_component {
                    self.g += 1;
                } else {
                    self.edge = Cyan2Green;
                    self.b -= 1;
                }

                LineTo(color)
            }
            Cyan2Green => {
                if 0 < self.b {
                    self.b -= 1;
                } else {
                    self.edge = Green2Yellow;
                    self.r += 1;
                }

                LineTo(color)
            }
            Green2Yellow => {
                if self.r < self.max_component {
                    self.r += 1;
                } else {
                    self.edge = Yellow2Red;
                    self.g -= 1;
                }

                LineTo(color)
            }
            Yellow2Red => {
                if 0 < self.g {
                    self.g -= 1;
                } else {
                    self.edge = Red2Magenta;
                    self.b += 1;
                }

                LineTo(color)
            }
            Red2Magenta => {
                if self.b < self.max_component {
                    self.b += 1;
                } else {
                    self.edge = Magenta2Blue;
                    self.r -= 1;
                }

                LineTo(color)
            }
            Magenta2Blue => {
                if 0 < self.r {
                    self.r -= 1;

                    LineTo(color)
                } else {
                    self.edge = Blue2Black;

                    CloseWith(color)
                }
            }
            Blue2Black => {
                if self.b == self.max_component {
                    self.b -= 1;

                    MoveTo(color)
                } else if 0 < self.b {
                    self.b -= 1;

                    LineTo(color)
                } else {
                    self.edge = Cyan2White;
                    self.g = self.max_component;
                    self.b = self.max_component;

                    LineTo(color)
                }
            }
            Cyan2White => {
                if self.r == 0 {
                    self.r += 1;

                    MoveTo(color)
                } else if self.r < self.max_component {
                    self.r += 1;

                    LineTo(color)
                } else {
                    self.edge = Green2Black;
                    self.r = 0;
                    self.b = 0;

                    LineTo(color)
                }
            }
            Green2Black => {
                if self.g == self.max_component {
                    self.g -= 1;

                    MoveTo(color)
                } else if 0 < self.g {
                    self.g -= 1;

                    LineTo(color)
                } else {
                    self.edge = Yellow2White;
                    self.r = self.max_component;
                    self.g = self.max_component;

                    LineTo(color)
                }
            }
            Yellow2White => {
                if self.b == 0 {
                    self.b += 1;

                    MoveTo(color)
                } else if self.b < self.max_component {
                    self.b += 1;

                    LineTo(color)
                } else {
                    self.edge = Red2Black;
                    self.g = 0;
                    self.b = 0;

                    LineTo(color)
                }
            }
            Red2Black => {
                if self.r == self.max_component {
                    self.r -= 1;

                    MoveTo(color)
                } else if 0 < self.r {
                    self.r -= 1;

                    LineTo(color)
                } else {
                    self.edge = Magenta2White;
                    self.r = self.max_component;
                    self.b = self.max_component;

                    LineTo(color)
                }
            }
            Magenta2White => {
                if self.g == 0 {
                    self.g += 1;

                    MoveTo(color)
                } else if self.g < self.max_component {
                    self.g += 1;

                    LineTo(color)
                } else {
                    self.edge = Done;

                    LineTo(color)
                }
            }
            Done => unreachable!(),
        };

        Some(result)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

#[cfg(feature = "gamut")]
impl std::iter::ExactSizeIterator for GamutTraversal {
    fn len(&self) -> usize {
        self.remaining
    }
}

#[cfg(feature = "gamut")]
impl std::iter::FusedIterator for GamutTraversal {}

#[cfg(all(feature = "gamut", feature = "pyffi"))]
#[pymethods]
impl GamutTraversal {
    /// Get the number of remaining steps.
    pub fn __len__(&self) -> usize {
        self.len()
    }

    /// Get this iterator. <i class=python-only>Python only!</i>
    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Get the next gamut traversal step. <i class=python-only>Python only!</i>
    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<GamutTraversalStep> {
        slf.next()
    }

    /// Get a debug representation. <i class=python-only>Python only!</i>
    pub fn __repr__(&self) -> String {
        format!(
            "GamutTraversal(space={:?}, len={}, edge={:?}, color=[{}, {}, {}])",
            self.space, self.max_component, self.edge, self.r, self.g, self.b
        )
    }
}

// ====================================================================================================================

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

    #[cfg(feature = "gamut")]
    #[test]
    fn test_gamut_iterator() {
        use super::{GamutTraversal, GamutTraversalStep, GamutTraversalStep::*};
        use crate::Color;

        let boundaries: Vec<GamutTraversalStep> =
            GamutTraversal::new(ColorSpace::Srgb, 2).unwrap().collect();
        assert_eq!(
            boundaries,
            vec![
                MoveTo(Color::srgb(0, 0, 1)),
                LineTo(Color::srgb(0, 1, 1)),
                LineTo(Color::srgb(0, 1, 0)),
                LineTo(Color::srgb(1, 1, 0)),
                LineTo(Color::srgb(1, 0, 0)),
                LineTo(Color::srgb(1, 0, 1)),
                CloseWith(Color::srgb(0, 0, 1)),
                MoveTo(Color::srgb(0, 0, 1)),
                LineTo(Color::srgb(0, 0, 0)),
                MoveTo(Color::srgb(0, 1, 1)),
                LineTo(Color::srgb(1, 1, 1)),
                MoveTo(Color::srgb(0, 1, 0)),
                LineTo(Color::srgb(0, 0, 0)),
                MoveTo(Color::srgb(1, 1, 0)),
                LineTo(Color::srgb(1, 1, 1)),
                MoveTo(Color::srgb(1, 0, 0)),
                LineTo(Color::srgb(0, 0, 0)),
                MoveTo(Color::srgb(1, 0, 1)),
                LineTo(Color::srgb(1, 1, 1)),
            ]
        );
    }
}
