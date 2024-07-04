use crate::Float;

fn close_enough_numbers(value1: Float, value2: Float, is_hue: bool) -> bool {
    if value1.is_nan() {
        if is_hue {
            return value2.is_nan();
        }
    }

    let decimals = if is_hue {
        Float::DIGITS - 3
    } else {
        Float::DIGITS - 1
    } as i32;
    let factor = (10.0 as Float).powi(decimals);

    (value1 * factor).round() == (value2 * factor).round()
}

pub(crate) fn close_enough_internal(
    coordinates1: &[Float; 3],
    coordinates2: &[Float; 3],
    is_polar: bool,
) -> bool {
    for index in 0..=2 {
        let c1 = coordinates1[index];
        let c2 = coordinates2[index];

        if !close_enough_numbers(c1, c2, is_polar && index == 2) {
            return false;
        }
    }

    return true;
}
