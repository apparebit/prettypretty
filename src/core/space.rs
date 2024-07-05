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
/// # Ok(l/lr)(ab/ch)
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
/// Oklab/Oklrab use Cartesian coordinates a, b for colorness—with a varying
/// red/green and b varying blue/yellow. That makes both color spaces
/// well-suited to computing the relative distance between colors. In contrast,
/// Oklch/Oklrch use polar coordinates C, h—with C expressing chroma and h or hº
/// expressing hue. That makes both color spaces well-suited to modifying
/// colors.
///
/// Compared to the conversion between XYZ and Oklab, conversions between the
/// four variations are mathematically simpler and may not even involve all
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
/// For all four color spaces, the (revised) lightness ranges `0..=1`. The a/b
/// coordinates are not restricted but pragmatically bounded `-0.4..=0.4`.
/// Chroma must be non-negative and is pragmatically bounded `0..=0.4`, which
/// suggests that the bounds for a/b are rather loose.
///
/// There may or may not be another, still outstanding issue with Oklrab, namely
/// that a and b need [to be scaled by a factor of around
/// 2.1](https://github.com/w3c/csswg-drafts/issues/6642#issuecomment-945714988).
///
/// # XYZ
///
/// [XYZ](https://en.wikipedia.org/wiki/CIE_1931_color_space) serves as
/// foundational color space. Notably, all conversions between unrelated color
/// spaces go through XYZ. This crate uses XYZ with the [D65 standard
/// illuminant](https://en.wikipedia.org/wiki/Standard_illuminant), *not* D50.
#[doc = include_str!("../style.html")]
#[cfg_attr(feature = "pyffi", pyclass(eq, eq_int, frozen, hash))]
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
}

#[cfg_attr(feature = "pyffi", pymethods)]
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

    /// Determine whether this color space is bounded. XYZ and the Oklab
    /// variations are *unbounded* and hence can model any color, whereas the
    /// RGB color spaces are *bounded* and hence colors may be in-gamut or
    /// out-of-gamut. Conveniently, the coordinates of in-gamut RGB colors range
    /// `0..=1`.
    pub const fn is_bounded(&self) -> bool {
        self.is_rgb()
    }

    /// Create a human-readable representation for this color space.
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
        };

        f.write_str(s)
    }
}
