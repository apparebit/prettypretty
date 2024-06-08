//! High-definition colors

mod core;

pub use self::core::ColorSpace;
use self::core::{
    clip, convert, delta_e_ok, in_gamut, map_to_gamut, normalize, parse, ParseColorError,
};
pub use super::util::Coordinate;

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
/// # Equality Testing and Hashing
///
/// The key requirement for equality testing and hashing is that colors that
/// compare [`Self::eq`] also have the same
/// [`Self::hash`](struct.Color.html#method.hash). To maintain this invariant,
/// the implementation of the two methods normalizes coordinates:
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
///     only after rounding, which may produce zeros);
///   * To convince Rust that coordinates are comparable, convert to bits.
///
/// While rounding isn't strictly necessary for correctness, it makes for a more
/// robust comparison without meaningfully reducing precision.
#[derive(Clone, Debug)]
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

    /// Instantiate a new Display P3 color with the given red, green, and blue
    /// coordinates.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let cyan = Color::p3(0.0, 1.0, 1.0);
    /// assert_eq!(cyan.coordinates(), &[0.0, 1.0, 1.0]);
    /// ```
    pub const fn p3(r: f64, g: f64, b: f64) -> Self {
        Color {
            space: ColorSpace::DisplayP3,
            coordinates: [r, g, b],
        }
    }

    /// Instantiate a new Oklab color with the given lightness, a, and b
    /// coordinates.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let blue_cyanish = Color::oklab(0.78, -0.1, -0.1);
    /// assert_eq!(blue_cyanish.space(), ColorSpace::Oklab);
    /// ```
    pub const fn oklab(l: f64, a: f64, b: f64) -> Self {
        Color {
            space: ColorSpace::Oklab,
            coordinates: [l, a, b],
        }
    }

    /// Instantiate a new Oklch color with the given lightness, chroma, and hue
    /// coordinates.
    ///
    /// ```
    /// # use prettypretty::{Color, ColorSpace};
    /// let deep_purple = Color::oklch(0.5, 0.25, 308.0);
    /// assert_eq!(deep_purple.space(), ColorSpace::Oklch);
    /// ```
    pub const fn oklch(l: f64, c: f64, h: f64) -> Self {
        Color {
            space: ColorSpace::Oklch,
            coordinates: [l, c, h],
        }
    }

    // ----------------------------------------------------------------------------------------------------------------

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

    // ----------------------------------------------------------------------------------------------------------------

    /// Convert this color to the target color space.
    ///
    ///
    /// # Challenge
    ///
    /// A color space is usually defined through a conversion from and to
    /// another color space. The color module includes handwritten functions
    /// that implement just those single-hop conversions. The basic challenge
    /// for arbitrary conversions, as implemented by this method, is to find a
    /// path through the graph of single-hop conversions. Dijkstra's algorithm
    /// would certainly work also is too general because the graph really is a
    /// tree rooted in XYZ D65 and edges only have unit weights. But even an
    /// optimized version would repeatedly find the same path through a rather
    /// small graph. It currently has seven nodes and is unlikely to grow beyond
    /// twice that size.
    ///
    ///
    /// # Algorithm
    ///
    /// Instead, this method implements the following algorithm, which requires
    /// a few more handwritten functions, but avoids most of the overhead of
    /// dynamic routing:
    ///
    ///  1. If the current color space *is* the target color space, simply
    ///     return the coordinates.
    ///  2. Handle all single-hop conversions that do not involve the root XYZ
    ///     D65. Since the gamma curve for sRGB and Display P3 is the same,
    ///     there really are only four conversions:
    ///
    ///      1. From sRGB to Linear sRGB, from Display P3 to Linear Display P3;
    ///      2. The inverse from linear to gamma-corrected coordinates;
    ///      3. From Oklab to Oklch;
    ///      4. From Oklch to Oklab.
    ///
    ///  3. With same hop and same branch conversions taken care of, we know
    ///     that the current and target color spaces are on separate branches,
    ///     with one of the two color spaces possibly XYZ itself. As a result,
    ///     all remaining conversions can be broken down into two simpler
    ///     conversions:
    ///
    ///      1. Along one branch from the current color space to the root XYZ;
    ///      2. Along another branch from the root XYZ to the target color space.
    ///
    /// By breaking conversions that go through XYZ into two steps, the
    /// conversion algorithm limits the number of hops that need to be supported
    /// by handwritten functions to two hops (currently). Furthermore,
    /// implementing these two-hop conversion functions by composing single-hop
    /// conversions is trivial. Altogether, the implementation relies on 10
    /// single-hop and 6 dual-hop conversions.
    ///
    ///
    /// # Trade-Offs
    ///
    /// The above algorithm represents a compromise between full specialization
    /// and entirely dynamic routing. Full specialization would require
    /// (7-1)(7-1) = 36 conversion functions, some of them spanning four hops.
    /// Its implementation would also require 7*7 = 49 matches on color space
    /// identifiers because it needs to look up the target color space *for
    /// each* source color space.
    ///
    /// In contrast, dynamic routing gets by with the 10 single-hop conversions
    /// but its implementation needs to recompute paths of up to 4 hops over and
    /// over again.
    ///
    /// Meanwhile, the above algorithm requires 6 additional dual-hop
    /// conversions. Its implementation comprises 6 matches on pairs, 7 matches
    /// on the source color space, and 7 matches on the target color space, *in
    /// series*, to a total of 20 matches. That's also the maximum number of
    /// matches it performs, which is 6 more than the fully specialized case. At
    /// the same time, its implementation requires much less machinery than the
    /// fully specialized one.
    ///
    /// Now, if branches were deeper, say, we also supported HSL, HSV, and HWB
    /// (with sRGB converting to HSL then HSV then HWB), the above algorithm
    /// would require three-, four-, and five-hop conversions as well, which
    /// would be cumbersome to implement. However, the general divide and
    /// conquer approach would apply to such long branches as well. For example,
    /// HSL could serve as midpoint. All other color spaces on the same branch
    /// are within two hops, with exception of XYZ, which requires three (i.e.,
    /// two three-hop conversion functions). In short, by performing *limited*
    /// dynamic look-ups, we can get most of the benefits of a fully specialized
    /// implementation.
    #[must_use = "method returns a new color and does not mutate original value"]
    pub fn to(&self, target: ColorSpace) -> Self {
        Self {
            space: target,
            coordinates: convert(self.space, target, &self.coordinates),
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
    #[must_use = "method returns a new color and does not mutate original value"]
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
    /// less chroma than the original (but the same lightness and hue), whose
    /// clipped version is within the *just noticeable difference* and in gamut
    /// for the current color space. That clipped color is the result.
    ///
    /// The algorithm nicely illustrates how different color spaces are best
    /// suited to different needs. First, it performs clipping and in-gamut
    /// testing in the current color space. After all, that's the color space
    /// the application requires the color to be in. Second, it performs color
    /// adjustments in Oklch. It is eminently suited to color manipulation
    /// because it is both perceptually uniform and has polar coordinates.
    /// Third, it measures distance in Oklab. Since the color space is
    /// perceptually uniform and has Cartesian coordinates, Euclidian distance
    /// is easy to compute and still accurate.
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
    #[must_use = "method returns a new color and does not mutate original value"]
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
    /// Create an instance of the default color. The chosen default for this
    /// crate is pitch black, i.e., the origin in XYZ.
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
    /// This method recognizes two hexadecimal notations for RGB colors, the
    /// hashed notation familiar from the web and an older notation used by X
    /// Windows. Even though the latter is intended to represent *device RGB*,
    /// this crate maps both to sRGB.
    ///
    /// The hashed notation has three or six hexadecimal digits, e.g., `#123` or
    /// #`cafe00`. Note that the three digit version is a short form of the six
    /// digit version with every digit repeated. In other words, the red
    /// coordinate in `#123` is not 0x1/0xf but 0x11/0xff.
    ///
    /// The X Windows notation has between one and four hexadecimal digits per
    /// coordinate, e.g., `rgb:1/00/cafe`. Here, every coordinate is scaled,
    /// i.e., the red coordinate in the example is 0x1/0xf.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse(s).map(|(space, coordinates)| Color { space, coordinates })
    }
}

// --------------------------------------------------------------------------------------------------------------------

impl std::hash::Hash for Color {
    /// Hash this color.
    ///
    /// See [`Color`] for an overview of equality testing and hashing.
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
    /// As discussed in the overview for [`Color`], [`Self::eq`] and
    /// [`Self::hash`](struct.Color.html#method.hash) normalize color
    /// coordinates before testing/hashing them. The following *equalities*
    /// illustrate how normalization handles not-a-numbers, very small numbers,
    /// and polar coordinates:
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
