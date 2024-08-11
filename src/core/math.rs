use crate::Float;

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
    pub fn sum(&self) -> Float {
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
        value.sum + value.compensation
    }
}

impl std::convert::From<&Accumulator> for Float {
    fn from(value: &Accumulator) -> Self {
        value.sum + value.compensation
    }
}

/// Sum up the values yielded by the iterator.
pub(crate) fn sum(values: impl IntoIterator<Item = Float>) -> Float {
    let mut accum = Accumulator::default();
    for value in values {
        accum += value;
    }
    accum.sum()
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
        assert_eq!(accum.sum(), 2.0);
        assert_eq!(1.0 + 10e100 + 1.0 - 10e100, 0.0);
    }
}
