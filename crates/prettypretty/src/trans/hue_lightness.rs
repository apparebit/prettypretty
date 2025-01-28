use crate::core::is_achromatic_chroma_hue;
use crate::termco::AnsiColor;
use crate::theme::Theme;
use crate::{Bits, Color, ColorSpace, Float};

/// A gray ANSI color and its concrete lightness value.
#[derive(Debug)]
struct GrayEntry {
    spec: AnsiColor,
    lr: Float,
}

impl GrayEntry {
    /// Create a new gray entry.
    ///
    /// This associated function returns `None` if the ANSI color or its color
    /// value is not gray.
    fn new(spec: AnsiColor, value: &Color) -> Option<GrayEntry> {
        let [lr, c, h] = *value.to(ColorSpace::Oklrch).as_ref();
        if !spec.is_achromatic()
            || !is_achromatic_chroma_hue(c, h, HueLightnessTable::ACHROMATIC_THRESHOLD)
        {
            return None;
        }

        Some(GrayEntry { spec, lr })
    }

    /// Get a key suitable for an ordering comparison.
    fn key(&self) -> Bits {
        // Conversion in new() normalizes lr to a number.
        self.lr.to_bits()
    }
}

/// A non-gray ANSI color and its concrete chroma and hue.
#[derive(Debug)]
struct ColorEntry {
    spec: AnsiColor,
    lr: Float,
    h: Float,
}

impl ColorEntry {
    /// Create a new color entry.
    ///
    /// This associated function returns `None` if the ANSI color or its
    /// concrete color value is gray.
    fn new(spec: AnsiColor, value: &Color) -> Option<Self> {
        let [lr, c, mut h] = *value.to(ColorSpace::Oklrch).as_ref();
        if spec.is_achromatic()
            || is_achromatic_chroma_hue(c, h, HueLightnessTable::ACHROMATIC_THRESHOLD)
        {
            return None;
        }
        h = h.rem_euclid(360.0); // Critical for correctness!

        Some(ColorEntry { spec, lr, h })
    }

    /// Get the 3-bit base color.
    fn base(&self) -> AnsiColor {
        self.spec.to_base()
    }
}

/// A table for matching by hue and lightness.
///
/// A hue and lightness table must observe the following invariants:
///
///   * The floating point fields of all entries are numbers.
///   * The gray entries represent grays. That applies to abstract ANSI colors
///     and concrete coordinates alike.
///   * The color entries represent colors, not grays. That applies to abstract
///     ANSI colors and concrete coordinates alike.
///   * The chroma of color entries must be non-zero. (Otherwise, they'd be
///     grays.)
///   * When traversing the hue circle counter-clockwise, the order of abstract
///     ANSI colors is red, yellow, green, cyan, blue, and magenta.
///
/// Note that the last invariant allows for hues to be rotated out of their
/// usual position and does not restrict the relative order between regular and
/// bright versions of the same abstract color.
///
/// Also note that the constructor returns `None` if the theme colors do not
/// observe the invariants, with exception of the first one on floating point
/// values, which is automatically observed.
#[derive(Debug)]
pub(crate) struct HueLightnessTable {
    grays: Vec<GrayEntry>,
    colors: Vec<ColorEntry>,
}

impl HueLightnessTable {
    const ACHROMATIC_THRESHOLD: Float = 0.05;

    /// Create a new hue lightness table.
    ///
    /// This associated function returns `None` if the theme colors violate any
    /// of the invariants.
    pub fn new(theme: &Theme) -> Option<HueLightnessTable> {
        use AnsiColor::*;

        // Prep the grays
        let mut grays = Vec::with_capacity(4);
        for index in [Black, White, BrightBlack, BrightWhite] {
            grays.push(GrayEntry::new(index, &theme[index])?);
        }
        grays.sort_by_key(|entry| entry.key());

        // Prep the non-grays in hue order: red, yellow, green, cyan, blue, magenta.
        let mut colors = Vec::with_capacity(12);
        for index in [Red, Yellow, Green, Cyan, Blue, Magenta] {
            let regular = ColorEntry::new(index, &theme[index])?;
            let index = index.to_bright();
            let bright = ColorEntry::new(index, &theme[index])?;

            // Order each color pair by hue
            if regular.h <= bright.h {
                colors.push(regular);
                colors.push(bright);
            } else {
                colors.push(bright);
                colors.push(regular);
            }
        }

        // Find entry with smallest hue
        let mut min_hue = Float::MAX;
        let mut min_index = usize::MAX;
        for (index, entry) in colors.iter().enumerate() {
            if entry.h < min_hue {
                min_hue = entry.h;
                min_index = index;
            }
        }

        // Rotate entry with smallest hue into first position.
        if 0 < min_index {
            colors.rotate_left(min_index);
        }

        // We added each regular/bright pair by smaller hue first. So if
        // pairs are in standard order, all hues are sorted as well.
        min_hue = -1.0;
        for entry in colors.iter() {
            if entry.h < min_hue {
                return None;
            }
            min_hue = entry.h;
        }

        Some(HueLightnessTable { grays, colors })
    }

    /// Find matching color.
    ///
    /// For grays, this method finds the ANSI gray with the closest lightness.
    /// For colors, this method first finds the pair of regular and bright
    /// abstract ANSI colors with the closest hue and then selects the one with
    /// the closest lightness.
    pub fn find_match(&self, color: &Color) -> AnsiColor {
        let [lr, c, h] = *color.to(ColorSpace::Oklrch).as_ref();

        // Select gray index by lr only. Not that there is anything else to go by...
        if is_achromatic_chroma_hue(c, h, Self::ACHROMATIC_THRESHOLD) {
            for index in 0..(self.grays.len() - 1) {
                let entry1 = &self.grays[index];
                let entry2 = &self.grays[index + 1];

                // The midpoint between grays serves as boundary.
                if lr < entry1.lr + (entry2.lr - entry1.lr) / 2.0 {
                    return entry1.spec;
                }
            }
            return self.grays[self.grays.len() - 1].spec;
        }

        // Select pair of color versions by hue and then pick one by lightness.
        // Humans are less sensitive to chroma, so ignoring it seems reasonable.
        let length = self.colors.len();
        for index in 0..length {
            // We are looking for the first entry with a larger hue.
            let next_entry = &self.colors[index];
            if next_entry.h < h && (index != 0 || h < self.colors[length - 1].h) {
                // The first interval starts with the last color.
                continue;
            }

            // index has type usize. Hence (index - 1) is unsafe,
            // but (index - 1 + length) isn't. Go rem, go!
            let previous_entry = &self.colors[(index - 1 + length).rem_euclid(length)];
            if previous_entry.base() == next_entry.base() {
                // Hue is bracketed by versions of same color.
                let result = self.pick_lightness(lr, previous_entry, next_entry);
                return result;
            }

            // We need previous_hue < h <= next_hue to determine closer one.
            let mut previous_hue = previous_entry.h;
            let next_hue = next_entry.h;
            if h < previous_hue {
                assert!(index == 0);
                previous_hue -= 360.0
            }

            // Pick closer color pair.
            if h - previous_hue <= next_hue - h {
                // Hue is closer to previous color
                let twice_previous_entry = &self.colors[(index + length - 2).rem_euclid(length)];
                return self.pick_lightness(lr, twice_previous_entry, previous_entry);
            } else {
                // Hue is closer to next color
                let twice_next_entry = &self.colors[(index + 1).rem_euclid(length)];
                return self.pick_lightness(lr, next_entry, twice_next_entry);
            }
        }

        unreachable!();
    }

    /// Use lightness to pick an entry's ANSI color.
    fn pick_lightness(&self, lr: Float, entry1: &ColorEntry, entry2: &ColorEntry) -> AnsiColor {
        if (entry1.lr - lr).abs() <= (entry2.lr - lr).abs() {
            entry1.spec
        } else {
            entry2.spec
        }
    }
}
