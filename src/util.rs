/// A safe, symbolic index for the three color coordinates.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Coordinate {
    C1 = 0,
    C2 = 1,
    C3 = 2,
}

/// Determine whether the two floating point numbers are almost equal. This
/// function returns `true` if both arguments are not-a-number, have the same
/// value, or have the same value after rounding the 15th decimal. In other
/// words, this function treats not-a-number as comparable and tolerates a small
/// absolute error for numbers.
#[allow(dead_code)]
pub fn almost_eq(n1: f64, n2: f64) -> bool {
    if n1.is_nan() {
        return n2.is_nan();
    } else if n2.is_nan() {
        return false;
    } else if n1 == n2 {
        return true;
    }

    let factor = 10.0_f64.powi((f64::DIGITS as i32) - 1);
    (factor * n1).round() == (factor * n2).round()
}
