mod d_series;
mod ten_deg;
mod two_deg;

pub use d_series::CIE_ILLUMINANT_D50;
pub use d_series::CIE_ILLUMINANT_D65;
pub use ten_deg::CIE_OBSERVER_10DEG_1964;
pub use two_deg::CIE_OBSERVER_2DEG_1931;

#[cfg(test)]
mod test {
    use super::{
        CIE_ILLUMINANT_D50, CIE_ILLUMINANT_D65, CIE_OBSERVER_10DEG_1964, CIE_OBSERVER_2DEG_1931,
    };
    use crate::core::{Sum, ThreeSum};
    use crate::spectrum::{IlluminatedObserver, SpectralDistribution, ONE_NANOMETER};
    use crate::Float;

    #[test]
    fn test_checksum() {
        for illuminant in [&CIE_ILLUMINANT_D50, &CIE_ILLUMINANT_D65] {
            let mut sum = Sum::new();
            for wavelength in illuminant.range() {
                sum += illuminant.at(wavelength).unwrap();
            }
            assert_eq!(sum.value(), illuminant.checksum());
        }

        for observer in [&CIE_OBSERVER_2DEG_1931, &CIE_OBSERVER_10DEG_1964] {
            let mut sum = ThreeSum::new();
            for wavelength in observer.range() {
                sum += observer.at(wavelength).unwrap();
            }
            assert_eq!(sum.value(), observer.checksum());
        }
    }

    #[test]
    fn test_tristimulus() {
        fn round5(c: Float) -> Float {
            (c * 100_000.0).round() / 100_000.0
        }

        // See https://en.wikipedia.org/wiki/Standard_illuminant#D65_values
        for (illuminant, observer, tristimulus, chromaticity) in [
            (
                &CIE_ILLUMINANT_D65,
                &CIE_OBSERVER_2DEG_1931,
                [0.95047, 1.0, 1.08883],
                (0.31273, 0.32902),
            ),
            (
                &CIE_ILLUMINANT_D65,
                &CIE_OBSERVER_10DEG_1964,
                [0.94811, 1.0, 1.07305],
                (0.31382, 0.33100),
            ),
        ] {
            let table = IlluminatedObserver::new(illuminant, observer);
            // Check results of range overlap computation
            assert_eq!(table.start(), 360);
            assert_eq!(table.end(), 831);

            // Compute sum for tristimulus
            let mut sum = ThreeSum::new();
            for index in table.start()..table.end() {
                sum += table.at(index).unwrap();
            }

            // Scale to tristimulus and compare to expected value
            let [x, y, z] = sum.value();
            let actual_tristimulus = [round5(x / y), 1.0, round5(z / y)];
            assert_eq!(actual_tristimulus, tristimulus);

            // Compute tristimulus again, this time with pulse_color()
            let color = table
                .visual_gamut(ONE_NANOMETER)
                .pulse_color(0, table.len());
            let [x, y, z] = *color.as_ref();
            let pulse_tristimulus = [round5(x), round5(y), round5(z)];
            assert_eq!(pulse_tristimulus, tristimulus);

            let (x, y) = color.xy_chromaticity();
            let actual_chromaticity = (round5(x), round5(y));
            assert_eq!(actual_chromaticity, chromaticity);
        }
    }
}
