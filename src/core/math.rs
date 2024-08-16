use crate::Float;

pub(crate) trait FloatExt {
    const ROUNDING_FACTOR: Self;
}

impl FloatExt for f64 {
    const ROUNDING_FACTOR: f64 = 1e12;
}

impl FloatExt for f32 {
    const ROUNDING_FACTOR: f32 = 1e4;
}

/// A floating point accumulator.
///
/// Unlike plain summation with the `+` operator, this struct minimizes the
/// cumulative error by using [Kahan's algorithm with Neumaier's
/// improvements](https://en.wikipedia.org/wiki/Kahan_summation_algorithm).
#[derive(Debug, Default)]
pub(crate) struct Accumulator {
    sum: Float,
    compensation: Float,
}

impl Accumulator {
    pub fn total(&self) -> Float {
        self.sum + self.compensation
    }
}

impl std::ops::Add<Float> for Accumulator {
    type Output = Accumulator;

    fn add(self, rhs: Float) -> Self::Output {
        let mut lhs = self;
        lhs += rhs;
        lhs
    }
}

impl std::ops::AddAssign<Float> for Accumulator {
    fn add_assign(&mut self, rhs: Float) {
        let t = self.sum + rhs;
        if self.sum.abs() > rhs.abs() {
            self.compensation += (self.sum - t) + rhs;
        } else {
            self.compensation += (rhs - t) + self.sum;
        }
        self.sum = t;
    }
}

impl From<Accumulator> for Float {
    fn from(value: Accumulator) -> Self {
        value.total()
    }
}

impl From<&Accumulator> for Float {
    fn from(value: &Accumulator) -> Self {
        value.total()
    }
}

#[cfg(test)]
mod test {
    use super::Accumulator;

    #[test]
    fn test_accumulator() {
        let mut accum = Accumulator::default();
        accum += 1.0;
        accum += 10e100;
        accum += 1.0;
        accum += -10e100;
        assert_eq!(accum.total(), 2.0);
        assert_eq!(1.0 + 10e100 + 1.0 - 10e100, 0.0);
    }
}
