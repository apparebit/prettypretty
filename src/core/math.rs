use crate::Float;

/// An extension trait for floating point numbers.
///
/// For now, this trait exists solely to pre-compute the rounding factor for
/// equality comparisons, which depends on the floating point representation.
pub(crate) trait FloatExt {
    /// The factor determining rounding precision.
    ///
    /// When limiting a floating point number's precision, the number is
    /// multiplied by some factor, rounded, and divided by the same factor
    /// again. Typically, that factor is a power of ten, which directly
    /// translates into significant digits after the decimal.
    const ROUNDING_FACTOR: Self;
}

impl FloatExt for f64 {
    const ROUNDING_FACTOR: f64 = 1e12;
}

impl FloatExt for f32 {
    const ROUNDING_FACTOR: f32 = 1e4;
}

// ----------------------------------------------------------------------------------------------------------

/// A floating point sum.
///
/// Unlike plain summation with the `+` operator, this struct minimizes the
/// cumulative error by using [Kahan's algorithm with Neumaier's
/// improvements](https://en.wikipedia.org/wiki/Kahan_summation_algorithm).
#[derive(Debug, Default)]
pub(crate) struct Sum {
    sum: Float,
    compensation: Float,
}

impl Sum {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn value(&self) -> Float {
        self.sum + self.compensation
    }
}

impl std::ops::Add<Float> for Sum {
    type Output = Sum;

    /// Add a number to this sum.
    ///
    /// The sum effectively moves through this method. By contrast, the
    /// implementation of `AddAssign` uses a mutably borrowed reference.
    fn add(self, rhs: Float) -> Self::Output {
        let mut lhs = self;
        lhs += rhs;
        lhs
    }
}

impl std::ops::AddAssign<Float> for Sum {
    fn add_assign(&mut self, rhs: Float) {
        let t = self.sum + rhs;
        if rhs.abs() < self.sum.abs() {
            self.compensation += (self.sum - t) + rhs;
        } else {
            self.compensation += (rhs - t) + self.sum;
        }
        self.sum = t;
    }
}

impl std::iter::Sum<Float> for Sum {
    fn sum<I: Iterator<Item = Float>>(iter: I) -> Self {
        let mut sum = Sum::new();
        for num in iter {
            sum += num;
        }
        sum
    }
}

impl From<Sum> for Float {
    fn from(value: Sum) -> Self {
        value.value()
    }
}

#[cfg(test)]
mod test {
    use super::Sum;

    #[test]
    fn test_accumulator() {
        let mut accum = Sum::default();
        accum += 1.0;
        accum += 10e100;
        accum += 1.0;
        accum += -10e100;
        assert_eq!(accum.value(), 2.0);
        assert_eq!(1.0 + 10e100 + 1.0 - 10e100, 0.0);
    }
}
