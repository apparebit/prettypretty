//! High-definition colors

mod core;

pub use self::core::ColorSpace;
use self::core::{
    clip, convert, delta_e_ok, in_gamut, map_to_gamut, normalize, parse, ParseColorError,
};

/// A color object.
///
/// # In-Gamut Colors
///
/// Color objects have a uniform representation for all supported color spaces,
/// combining a color space tag with a three-element array of coordinates. That
/// does get in the way of automatically enforcing typestate invariants for
/// color spaces, including the coordinate ranges of in-gamut colors. E.g., for
/// the RGB color spaces sRGB, linear sRGB, Display P3, and linear Display P3,
/// in-gamut red, green, and blue coordinates have unit range `0..=1`.
///
/// Instead of automatically limiting colors, color objects preserve computed
/// coordinate values even if they are out of gamut. That way, no information is
/// lost and colors in larger color spaces can easily be recovered by converting
/// them.
///
/// To explicitly manage gamut, use [`Color::in_gamut`] to test whether a color
/// is in gamut. Use [`Color::clip`] to quickly compute a low quality in-gamut
/// color. Use [`Color::map_to_gamut`] to more slowly search for a
/// higher-quality in-gamut color.
///
///
/// # Equality and Hashes
///
/// So that [`Self::hash`](struct.Color.html#method.hash) is the same for colors
/// that compare [`Self::eq`], the two methods use the same normalization
/// strategy:
///
///   * To make coordinates comparable, replace not-a-numbers with positive
///     zero;
///   * To preserve not-a-number semantics for hues, also zero out chroma for
///     not-a-number hues in Oklch;
///   * To preserve rotation semantics for hues, remove all full rotations;
///   * To prepare for rounding, scale down hues to unit range;
///   * To allow for floating point error, multiply by 1e14 and then round,
///     which drops the least significant digit;
///   * To make zeros comparable, replace negative zero with positive zero (but
///     only after rounding, which may produce zeros)
///
/// While rounding isn't strictly necessary for correctness, it makes for a more
/// robust comparison without giving up meaningful precision.
#[derive(Copy, Clone, Debug)]
pub struct Color {
    space: ColorSpace,
    coordinates: [f64; 3],
}

impl Color {
    /// Instantiate a new color with the given color space and coordinates.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let hot_pink = Color::new(ColorSpace::Oklch, 0.7, 0.22, 3.0);
    /// assert!(!hot_pink.to(ColorSpace::Srgb).in_gamut());
    /// ```
    pub const fn new(space: ColorSpace, c1: f64, c2: f64, c3: f64) -> Self {
        Color {
            space,
            coordinates: [c1, c2, c3],
        }
    }

    /// Instantiate a new sRGB color with the given red, green, and blue
    /// coordinates.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let fire_brick = Color::srgb(177.0/255.0, 31.0/255.0, 36.0/255.0);
    /// assert_eq!(fire_brick.space(), ColorSpace::Srgb);
    /// ```
    pub const fn srgb(r: f64, g: f64, b: f64) -> Self {
        Color {
            space: ColorSpace::Srgb,
            coordinates: [r, g, b],
        }
    }

    /// Access the color space.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let blue = Color::srgb(0.0, 0.0, 1.0);
    /// assert_eq!(blue.space(), ColorSpace::Srgb);
    /// ```
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

    /// Convert this color to the given color space.
    pub fn to(&self, space: ColorSpace) -> Self {
        Self {
            space,
            coordinates: convert(self.space, space, &self.coordinates),
        }
    }

    /// Determine whether this color is in-gamut for its color space.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let red = Color::srgb(1.0, 0.0, 0.0);
    /// assert!(red.in_gamut());
    ///
    /// let green = Color::new(ColorSpace::DisplayP3, 0.0, 1.0, 0.0);
    /// assert!(!green.to(ColorSpace::Srgb).in_gamut());
    /// ```
    pub fn in_gamut(&self) -> bool {
        in_gamut(self.space, &self.coordinates)
    }

    /// Clip this color to the gamut of its color space.
    ///
    ///
    /// # Example
    ///
    /// Display P3's green primary is out of gamut in sRGB. Clipping the
    /// converted color does bring it into gamut, though the result may be a
    /// rough match for the original color.
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let too_green = Color::new(ColorSpace::DisplayP3, 0.0, 1.0, 0.0)
    ///     .to(ColorSpace::Srgb);
    /// assert!(!too_green.in_gamut());
    ///
    /// let green = too_green.clip();
    /// assert!(green.in_gamut());
    /// ```
    pub fn clip(&self) -> Self {
        Self {
            space: self.space,
            coordinates: clip(self.space, &self.coordinates),
        }
    }

    /// Map this color into the gamut of its color space and return the result.
    ///
    ///
    /// # Algorithm
    ///
    /// This method uses the [CSS Color 4
    /// algorithm](https://drafts.csswg.org/css-color/#css-gamut-mapping) for
    /// gamut mapping. It performs a binary search in Oklch for a color with
    /// less chroma that is just barely within gamut. It measures color
    /// difference as the Euclidian distance in Oklab. The result has the same
    /// color space as this color.
    ///
    ///
    /// # Example
    ///
    /// Display P3's yellow secondary is out of gamut in sRGB. Gamut mapping the
    /// converted color does bring it into gamut while also perserving the hue
    /// and maximizing the chroma, all within sRGB's gamut.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let too_yellow = Color::new(ColorSpace::DisplayP3, 1.0, 1.0, 0.0)
    ///     .to(ColorSpace::Srgb);
    /// assert!(!too_yellow.in_gamut());
    ///
    /// let yellow = too_yellow.map_to_gamut();
    /// assert!(yellow.in_gamut());
    /// ```
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
        delta_e_ok(
            self.to(ColorSpace::Oklab).coordinates(),
            other.to(ColorSpace::Oklab).coordinates(),
        )
    }
}

// --------------------------------------------------------------------------------------------------------------------

impl Default for Color {
    /// Create an instance of the default color, which is black in XYZ.
    fn default() -> Self {
        Color {
            space: ColorSpace::Xyz,
            coordinates: [0.0, 0.0, 0.0],
        }
    }
}

impl std::str::FromStr for Color {
    type Err = ParseColorError;

    /// Instantiate a color from its string representation.
    ///
    /// This method recognizes the following color formats:
    ///
    ///   * The hashed hexadecimal format familiar from the web, e.g., `#0f0` or
    ///     `#00ff00`
    ///   * The X Windows hexadecimal format, e.g., `rgb:<hex>/<hex>/<hex>`.
    ///
    /// For the X Windows hexadecimal format, between 1 and 4 digits may be used
    /// per coordinate. Final values are appropriately scaled into the unit range.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse(s).map(|(space, coordinates)| Color { space, coordinates })
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
    ///
    /// The following equalities illustrate how normalization handles
    /// not-a-numbers, very small numbers, and polar coordinates:
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// assert_eq!(
    ///     Color::srgb(f64::NAN, 3e-15, 8e-15),
    ///     Color::srgb(0.0,      0.0,   1e-14)
    /// );
    ///
    /// assert_eq!(
    ///     Color::new(ColorSpace::Oklch, 0.5, 0.1, 665.0),
    ///     Color::new(ColorSpace::Oklch, 0.5, 0.1, 305.0)
    /// );
    /// ```
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

/// A safe, symbolic index for the three color coordinates.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Coordinate {
    C1 = 0,
    C2 = 1,
    C3 = 2,
}

impl std::ops::Index<Coordinate> for Color {
    type Output = f64;

    /// Access the named coordinate.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// use prettypretty::Coordinate::*;
    ///
    /// let blue = Color::srgb(0.0, 0.0, 1.0);
    /// assert_eq!(blue[C3], 1.0);
    /// ```
    fn index(&self, index: Coordinate) -> &Self::Output {
        &self.coordinates[index as usize]
    }
}

impl std::ops::IndexMut<Coordinate> for Color {
    /// Mutably access the named coordinate.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// use prettypretty::Coordinate::*;
    ///
    /// let mut magenta = Color::srgb(0.0, 0.0, 1.0);
    /// // Oops, we forgot to set the red coordinate. Let's fix that.
    /// magenta[C1] = 1.0;
    /// assert_eq!(magenta.coordinates(), &[1.0_f64, 0.0_f64, 1.0_f64]);
    /// ```
    fn index_mut(&mut self, index: Coordinate) -> &mut Self::Output {
        &mut self.coordinates[index as usize]
    }
}
