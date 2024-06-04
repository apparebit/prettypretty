// ====================================================================================================================
// The Color Abstraction
// ====================================================================================================================

mod core;

pub use self::core::ColorSpace;
use self::core::{
    convert, in_gamut, into_gamut, clip, same_coordinates, delta_e_ok
};

/// A color. Every color has a color space and coordinates.
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

