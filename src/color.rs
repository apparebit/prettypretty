//! High-definition colors

mod core;

pub use self::core::ColorSpace;
use self::core::{
    clip, convert, delta_e_ok, in_gamut, map_to_gamut, normalize
};

/// A color object.
///
/// This struct does *not* enforce limits for in-gamut color coordinates. Use
/// [`Color::in_gamut`] to test for out-of-gamut colors. Use [`Color::clip`] or
/// [`Color::map_to_gamut`] to determine an in-gamut version of a color.
///
/// To correctly implement `Eq` and `Hash`, the corresponding methods normalize
/// color coordinates before comparing or hashing them. Normalization includes:
///
///   * Replacing not-a-numbers with positive zero;
///   * Converting hues to partial rotations and scaling them to unit range;
///   * Rounding away one significant digit;
///   * Replacing negative zero with positive zero;
///
///
#[derive(Copy, Clone, Debug)]
pub struct Color {
    space: ColorSpace,
    coordinates: [f64; 3],
}

impl Color {
    /// Instantiate a new color with the given color space and coordinates. The
    /// meaning of the coordinates and hence also the limits for in-gamut
    /// coordinates depends on the color space.
    pub fn new(space: ColorSpace, c1: f64, c2: f64, c3: f64) -> Self {
        Color { space, coordinates: [c1, c2, c3] }
    }

    /// Access the color space.
    pub fn space(&self) -> ColorSpace {
        self.space
    }

    /// Access the coordinates.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let green = Color::new(ColorSpace::DisplayP3, 0.0, 1.0, 0.0);
    /// assert_eq!(green.coordinates(), &[0.0, 1.0, 0.0]);
    /// ```
    pub fn coordinates(&self) -> &[f64; 3] {
        &self.coordinates
    }

    /// Convert this color to the given color space. This method does not
    /// enforce the target color space's gamut.
    pub fn to(&self, space: ColorSpace) -> Self {
        Self {
            space,
            coordinates: convert(self.space, space, &self.coordinates),
        }
    }

    /// Determine whether this color is in-gamut for its color space.
    pub fn in_gamut(&self) -> bool {
        in_gamut(self.space, &self.coordinates)
    }

    /// Clip this color to the gamut of its color space.
    pub fn clip(&self) -> Self {
        Self { space: self.space, coordinates: clip(self.space, &self.coordinates) }
    }

    /// Map this color into the gamut of its color space and return the result.
    /// This method uses the [CSS Color 4
    /// algorithm](https://drafts.csswg.org/css-color/#css-gamut-mapping) for
    /// gamut mapping. It performs a binary search in Oklch for a color with
    /// less chroma that is just barely out of gamut and measures color
    /// difference as the Euclidian distance in Oklab. The result has the same
    /// color space as this color.
    pub fn map_to_gamut(&self) -> Self {
        Self {
            space: self.space,
            coordinates: map_to_gamut(self.space, &self.coordinates),
        }
    }

    /// Determine the difference between the two colors. This method returns the
    /// delta E OK metric, which is the same as the Euclidian distance between
    /// the two colors in Oklab.
    pub fn difference(&self, other: Self) -> f64 {
        return delta_e_ok(&self.coordinates, &other.coordinates)
    }
}

// --------------------------------------------------------------------------------------------------------------------

impl std::hash::Hash for Color {
    /// Hash this color.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.space.hash(state);

        let n = normalize(self.space, &self.coordinates);
        n[0].hash(state);
        n[1].hash(state);
        n[2].hash(state);
    }
}

impl PartialEq for Color {
    /// Determine whether this color equals the other color.
    fn eq(&self, other: &Self) -> bool {
        if self.space != other.space {
            return false;
        }

        let n1 = normalize(self.space, &self.coordinates);
        let n2 = normalize(other.space, &other.coordinates);
        n1 == n2
    }
}

impl Eq for Color {}

// --------------------------------------------------------------------------------------------------------------------

/// The three color coordinates.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Coordinate {
    C1 = 0,
    C2 = 1,
    C3 = 2,
}

impl std::ops::Index<Coordinate> for Color {
    type Output = f64;

    /// Access the named coordinate.
    fn index(&self, index: Coordinate) -> &Self::Output {
        &self.coordinates[index as usize]
    }
}

impl std::ops::IndexMut<Coordinate> for Color {
    /// Mutably access the named coordinate.
    fn index_mut(&mut self, index: Coordinate) -> &mut Self::Output {
        &mut self.coordinates[index as usize]
    }
}
