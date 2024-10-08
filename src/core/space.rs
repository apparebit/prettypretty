#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

/// The enumeration of supported color spaces.
///
/// # RGB
///
/// This crate supports several RGB color spaces, each in its gamma-corrected
/// and its linear form. From smallest to largest gamut, they are:
///
///   * [sRGB](https://en.wikipedia.org/wiki/SRGB), which has long served as the
///     default color space for the web.
///   * [Display P3](https://en.wikipedia.org/wiki/DCI-P3), which is
///     well-positioned to become sRGB's successor.
///   * [Rec. 2020](https://en.wikipedia.org/wiki/Rec._2020), which is the
///     standard color space for ultra-high-definition (UDH) video and, when it
///     comes to display hardware, currently aspirational.
///
/// For all three color spaces as well as all three linear versions, in-gamut
/// coordinates range from 0 to 1, inclusive.
///
/// # The Oklab Variations
///
/// This crate supports the
/// [Oklab/Oklch](https://bottosson.github.io/posts/oklab/) and
/// [Oklrab/Oklrch](https://bottosson.github.io/posts/colorpicker/#intermission---a-new-lightness-estimate-for-oklab)
/// color spaces. All four are variations of the same perceptually uniform color
/// space, which, like CIELAB, uses one coordinate for lightness and two
/// coordinates for "colorness."
///
/// Oklab and Oklch reflect the original design. They improve on CIELAB by using
/// the D65 standard illuminant (not the print-oriented D50), which is also used
/// by sRGB and Display P3. They further improve on CIELAB by avoiding visible
/// distortions around the blues. However, they also regress, as their lightness
/// L is visibly biased towards dark tones. Oklrab and Oklrch, which were
/// introduced nine months after Oklab/Oklch, feature a revised lightness Lr
/// that closely resembles CIELAB's uniform lightness.
///
/// Oklab/Oklrab use Cartesian coordinates a, b for colorness—with the a axis
/// varying red/green and the b axis varying blue/yellow. Because they use
/// Cartesian coordinates, computing color difference in Oklab/Oklrab is
/// straight-forward: It simply is the Euclidian distance. In contrast,
/// Oklch/Oklrch use polar coordinates C/h—with C expressing chroma and h or
/// also hº expressing hue. That makes both color spaces well-suited to
/// synthesizing and modifying colors.
///
/// Compared to the most other conversions between color spaces, conversions
/// between the four Oklab variations are mathematically simpler and may not
/// involve all coordinates. After all, there are four three-dimensional color
/// spaces but only six distinct quantities:
///
/// | Color space | Lightness | Colorness 1 | Colorness 2 |
/// | ----------- | :-------: | :---------: | :---------: |
/// | Oklab       | L         | a           | b           |
/// | Oklch       | L         | C           | hº          |
/// | Oklrab      | Lr        | a           | b           |
/// | Oklrch      | Lr        | C           | hº          |
///
/// Valid coordinates observe the following invariants:
///
///   * The (revised) lightness for all four color spaces is limited to `0..=1`.
///   * The a/b coordinates for Oklab/Oklrab have no set limits, but in practice
///     can be bounded `-0.4..=0.4`.
///   * The chroma for Oklch/Oklrch must be non-negative and in practice can be
///     bounded `0..=0.4`.
///   * The hue for Oklch/Oklrch may be not-a-number, which indicates a
///     powerless component, i.e., gray tone. In that case, the chroma must
///     necessarily be zero.
///
/// Fundamentally, Oklab and Oklch are the *same* color space, only using
/// different coordinate systems. Of course, that also is the case for Oklrab
/// and Oklrch. The chroma bond corresponds to a circle with radius 0.4 that is
/// centered at the origin. The a/b bounds correspond to a square with sides 0.8
/// that is also centered at the origin. The circle just fits into the square
/// and covers an area of π×0.4². Meanwhile, the square covers an area of
/// (2×0.4)², i.e., it is 4/π or 1.273 times larger. In other words, the a/b
/// bounds are somewhat looser than the chroma bound.
///
/// There may or may not be another, still outstanding issue with Oklrab, namely
/// that a and b need [to be scaled by a factor of around
/// 2.1](https://github.com/w3c/csswg-drafts/issues/6642#issuecomment-945714988).
///
/// There also is an extended Oklab, which behaves better for [imaginary
/// colors](https://github.com/w3c/csswg-drafts/issues/9449). As shown in the
/// [corresponding
/// notebook](https://colab.research.google.com/drive/1_uoLM95LJKTiI7MECG_PjBrd32v-3W3o),
/// the implementation compresses and shifts the LMS coordinates during
/// conversion.
///
/// # XYZ
///
/// [XYZ](https://en.wikipedia.org/wiki/CIE_1931_color_space) serves as
/// foundational color space. Notably, all conversions between unrelated color
/// spaces go through XYZ. Since sRGB, Display P3, and Oklab use the [D65
/// standard illuminant](https://en.wikipedia.org/wiki/Standard_illuminant),
/// this crate uses XYZ with D65 as its reference color space. But XYZ with the
/// D50 standard illuminant is available, too. Chromatic adaptation between the
/// two versions of XYZ uses the (linear) Bradford method.
#[cfg_attr(
    feature = "pyffi",
    pyclass(eq, eq_int, frozen, hash, module = "prettypretty.color")
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    Srgb,
    LinearSrgb,
    DisplayP3,
    LinearDisplayP3,
    Rec2020,
    LinearRec2020,
    Oklab,
    Oklch,
    Oklrab,
    Oklrch,
    Xyz,
    XyzD50,
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl ColorSpace {
    /// Determine whether this color space is polar.
    ///
    /// Oklch and Oklrch currently are the only polar color spaces.
    pub const fn is_polar(&self) -> bool {
        matches!(*self, Self::Oklch | Self::Oklrch)
    }

    /// Determine whether this color space is XYZ.
    pub const fn is_xyz(&self) -> bool {
        matches!(self, Self::Xyz)
    }

    /// Determine whether this color space is RGB.
    ///
    /// RGB color spaces are additive and have red, green, and blue coordinates.
    /// In-gamut colors have coordinates in unit range `0..=1`.
    pub const fn is_rgb(&self) -> bool {
        use ColorSpace::*;
        matches!(
            *self,
            Srgb | LinearSrgb | DisplayP3 | LinearDisplayP3 | Rec2020 | LinearRec2020
        )
    }

    /// Determine whether this color space is one of the Oklab variations.
    pub const fn is_ok(&self) -> bool {
        use ColorSpace::*;
        matches!(*self, Oklab | Oklch | Oklrab | Oklrch)
    }

    /// Determine whether this color space is bounded.
    ///
    /// XYZ and the Oklab variations are *unbounded* and hence can model any
    /// color. By contrast, RGB color spaces are *bounded*, with coordinates
    /// of in-gamut colors ranging `0..=1`.
    pub const fn is_bounded(&self) -> bool {
        self.is_rgb()
    }

    /// Create an iterator over this color space's gamut boundaries. <i
    /// class=gamut-only>Gamut only</i>
    ///
    /// For bounded or RGB color spaces, this method returns an iterator that
    /// traces the boundaries of the color space's gamut. As described in detail
    /// for [`GamutTraversal`](crate::gamut::GamutTraversal), the iterator does
    /// so by yielding [`GamutTraversalStep`](crate::gamut::GamutTraversalStep)s
    /// that trace paths along the edges of this color space's RGB cube.
    ///
    /// Altogether, the iterator yields steps for a closed path covering six
    /// edges followed by another six paths each covering one edge. Each step
    /// includes exactly one in-gamut color that also is in this color space.
    /// There are `edge_length` steps per edge, though the first path yields
    /// corners other than the blue primary only once.
    ///
    /// If this color space is not bounded or the segment size is 0 or 1, this
    /// method returns `None`.
    #[cfg(feature = "gamut")]
    pub fn gamut(&self, edge_length: usize) -> Option<crate::gamut::GamutTraversal> {
        crate::gamut::GamutTraversal::new(*self, edge_length)
    }

    /// Create a human-readable representation for this color space. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __str__(&self) -> String {
        format!("{}", self)
    }
}

impl std::fmt::Display for ColorSpace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ColorSpace::*;

        let s = match self {
            Srgb => "sRGB",
            LinearSrgb => "linear sRGB",
            DisplayP3 => "Display P3",
            LinearDisplayP3 => "linear Display P3",
            Rec2020 => "Rec. 2020",
            LinearRec2020 => "linear Rec. 2020",
            Oklab => "Oklab",
            Oklrab => "Oklrab",
            Oklch => "Oklch",
            Oklrch => "Oklrch",
            Xyz => "XYZ D65",
            XyzD50 => "XYZ D50",
        };

        f.write_str(s)
    }
}
