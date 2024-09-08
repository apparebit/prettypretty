//! Utility module with the spectral distributions for CIE standard observers
//! and illuminants. <i class=gamut-only>Gamut only!</i>
//!
#![cfg_attr(
    feature = "pyffi",
    doc = "This module reflects a somewhat unsatisfactory compromise between what's
        possible in Rust and what's necessary for PyO3. In particular, PyO3 requires
        monomorphic enums/structs with monomorphic method implementations and does not
        support generic anything. The PyO3 guide justifies this restriction with the
        fact that the Rust compiler produces monomorphic code only. While correct, PyO3
        complicates matters significantly by not supporting trait methods, its
        `#[pymethods]` macro disallowing item-level macros, and its error checking
        ignoring `#[cfg]` attributes. The impact is noticeable: Whereas the Rust-only
        version of this module contains three implementations of
        [`SpectralDistribution`]'s methods, enabling the `pyffi` feature adds three
        more implementations."
)]
//!
//! The [`SpectralDistribution`] trait defines an interface for mapping a fixed,
//! nanometer-aligned range of wavelengths at 1nm resolution to values. With
//! wavelengths restricted to integral nanometers, the interfaces uses `usize`
//! for their representation. Meanwhile, values are represented by an associated
//! type, which is `Float` for illuminants and `[Float; 3]` for observers.
//!
//! This module includes the following implementations of the trait:
//!
//!   * [`Observer`] is a table-driven implementation of
//!     `SpectralDistribution<Value=[Float;3]>`.
//!   * [`TabularDistribution`] is a table-driven implementation of
//!     `SpectralDistribution<Value=Float>`.
//!   * [`FixedDistribution`] is an implementation of
//!     `SpectralDistribution<Value=Float>` with a fixed value.
#![cfg_attr(
    feature = "pyffi",
    doc = "  * [`Illuminant`] is an implementation of
        `SpectralDistribution<Value=Float>` that wraps a `Box<dyn
         SpectralDistribution<Value=Float> + Send>`."
)]
//!
#![cfg_attr(
    feature = "pyffi",
    doc = "To play nice with PyO3 and Python, `Observer` reimplements all but one
        trait method in its `impl` block. `Illuminant` doesn't event exist unless
        the `pyffi` feature is enabled. But it still manages to adds two more
        implementations for most trait methods, one for the trait `impl` and one in
        its own `impl` block. While that does clutter the module sources somewhat,
        at least the methods' implementation was trivial. Each method simply forwards
        to another implementation of the same trait. Thereby, `Illuminant` makes
        two different trait implementations appear as the same type in Python and
        hence acceptable as argument for a spectrum traversal."
)]
//!
//! Using the above trait implementations, this module exports the following concrete
//! illuminants and observers, with data directly sourced from the CIE.
//!
//!   * [`CIE_ILLUMINANT_D50`] approximates daylight near sunrise and sunset.
//!   * [`CIE_ILLUMINANT_D65`] approximates daylight around noon.
//!   * [`CIE_ILLUMINANT_E`] has equal energy.
//!   * [`CIE_OBSERVER_2DEG_1931`] is the 1931 2º color matching function.
//!   * [`CIE_OBSERVER_10DEG_1964`] is the 1964 10º color matching function.
//!
//! A submodule provides [`std_observer::x()`], [`std_observer::y()`], and
//! [`std_observer::z()`] as analytical approximations for the 1931 2º observer.
//! As such, all three functions accept floating point arguments for
//! wavelengths.
//!
//! Finally, [`SpectrumTraversal`] is an iterator for tracing the spectral locus
//! or the human visual gamut. It requires an illuminant and observer as inputs.
//! In Rust, that can be any `SpectralDistribution<Value=Float>` and
//! `SpectralDistribution<Value=[Float;3]>`, respectively.
#![cfg_attr(
    feature = "pyffi",
    doc = "In Python, the constructor accepts `&Illuminant` and `&Observer`
        arguments only, which makes it monomorphic. While that is insufficient
        for generally overcoming the prohibition against polymorphic anything,
        in this particular case it suffices because spectrum traversal's
        constructors do not retain their input and instead keep the result of
        premultiplying the two spectral distributions. In other words, the rest
        of [`SpectrumTraversal`]'s implementation is strictly monomophic itself."
)]
//!

#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use crate::{
    core::{GamutTraversalStep, Sum},
    Color, ColorSpace, Float,
};

/// A spectral distribution at nanometer resolution.
///
/// A concrete implementation must provide methods that return a descriptive
/// label, a start wavelength, a length, and the spectral distribution's values.
///
/// This trait requires implementation of `start()` and `len()` instead of just
/// `range()` because the former two methods allow for more performant default
/// implementations.
pub trait SpectralDistribution {
    /// The spectral distribution's value type.
    type Value;

    /// Get a descriptive label for this spectral distribution.
    fn label(&self) -> String;

    /// Get the starting wavelength for this spectral distribution.
    fn start(&self) -> usize;

    /// Get the ending wavelength for this spectral distribution.
    fn end(&self) -> usize {
        self.start() + self.len()
    }

    /// Get the range of this spectral distribution.
    fn range(&self) -> std::ops::Range<usize> {
        self.start()..self.end()
    }

    /// Determine whether this distribution is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the length of this spectral distribution.
    fn len(&self) -> usize;

    /// Get this spectral distribution's value for the given wavelength.
    ///
    /// If the wavelength is within this spectral distribution's range, this
    /// method returns some value. Otherwise, it returns none.
    fn at(&self, wavelength: usize) -> Option<Self::Value>;

    /// Get the checksum for this spectral distribution.
    ///
    /// The checksum is the componentwise sum of all values contained in the
    /// distribution. It must not be computed on the fly.
    fn checksum(&self) -> Self::Value;
}

// --------------------------------------------------------------------------------------------------------------------

/// An illuminant at nanometer resolution. <i class=python-only>Python only!</i>
///
/// This struct exposes all functionality as methods for Python
/// interoperability. But it doesn't actually implement the functionality of a
/// spectral distribution and instead forwards all method invocations to a `dyn
/// SpectralDistribution<Value=Float>`.
#[cfg(feature = "pyffi")]
#[pyclass(frozen, module = "prettypretty.color.trans")]
pub struct Illuminant {
    distribution: Box<dyn SpectralDistribution<Value = Float> + Send>,
}

#[cfg(feature = "pyffi")]
impl Illuminant {
    /// Create a new illuminant.
    pub fn new(distribution: Box<dyn SpectralDistribution<Value = Float> + Send>) -> Self {
        Self { distribution }
    }
}

#[cfg(feature = "pyffi")]
#[pymethods]
impl Illuminant {
    /// Get a descriptive label for this spectral distribution.
    pub fn label(&self) -> String {
        self.distribution.label()
    }

    /// Get this spectral distribution's starting wavelength.
    pub fn start(&self) -> usize {
        self.distribution.start()
    }

    /// Get this spectral distribution's ending wavelength.
    pub fn end(&self) -> usize {
        self.distribution.end()
    }

    /// Determine whether this distribution is empty.
    pub fn is_empty(&self) -> bool {
        self.distribution.is_empty()
    }

    /// Determine the number of entries in this distribution.
    pub fn len(&self) -> usize {
        self.distribution.len()
    }

    /// Get this spectral distribution's value for the given wavelength.
    pub fn at(&self, wavelength: usize) -> Option<Float> {
        self.distribution.at(wavelength)
    }

    /// Get this spectral distribution's checksum.
    pub fn checksum(&self) -> Float {
        self.distribution.checksum()
    }

    /// Get the number of entries.
    pub fn __len__(&self) -> usize {
        self.distribution.len()
    }

    /// Get the entry at the given index.
    pub fn __getitem__(&self, index: usize) -> PyResult<Float> {
        self.distribution
            .at(self.distribution.start() + index)
            .ok_or_else(|| {
                pyo3::exceptions::PyIndexError::new_err(format!(
                    "{} <= index for {}",
                    self.distribution.len(),
                    self.distribution.label()
                ))
            })
    }

    /// Get a debug representation.
    pub fn __repr__(&self) -> String {
        format!("Illuminant({})", self.label())
    }
}

#[cfg(feature = "pyffi")]
impl SpectralDistribution for Illuminant {
    type Value = Float;

    fn label(&self) -> String {
        self.distribution.label()
    }

    fn start(&self) -> usize {
        self.distribution.start()
    }

    fn end(&self) -> usize {
        self.distribution.end()
    }

    fn range(&self) -> std::ops::Range<usize> {
        self.distribution.range()
    }

    fn is_empty(&self) -> bool {
        self.distribution.is_empty()
    }

    fn len(&self) -> usize {
        self.distribution.len()
    }

    fn at(&self, wavelength: usize) -> Option<Self::Value> {
        self.distribution.at(wavelength)
    }

    fn checksum(&self) -> Self::Value {
        self.distribution.checksum()
    }
}

// --------------------------------------------------------------------------------------------------------------------

/// A table-driven spectral distribution.
#[derive(Debug)]
pub struct TabularDistribution {
    label: &'static str,
    start: usize,
    checksum: Float,
    data: &'static [Float],
}

impl TabularDistribution {
    /// Create a new tabular distribution.
    pub const fn new(
        label: &'static str,
        start: usize,
        checksum: Float,
        data: &'static [Float],
    ) -> Self {
        Self {
            label,
            checksum,
            start,
            data,
        }
    }
}

impl SpectralDistribution for TabularDistribution {
    type Value = Float;

    fn label(&self) -> String {
        self.label.to_string()
    }

    fn start(&self) -> usize {
        self.start
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn at(&self, wavelength: usize) -> Option<Self::Value> {
        if self.start <= wavelength && wavelength < self.start + self.data.len() {
            Some(self.data[wavelength - self.start])
        } else {
            None
        }
    }

    fn checksum(&self) -> Self::Value {
        self.checksum
    }
}

// --------------------------------------------------------------------------------------------------------------------

/// A spectral distribution with a fixed value.
pub struct FixedDistribution {
    label: &'static str,
    start: usize,
    len: usize,
    checksum: Float,
    value: Float,
}

impl FixedDistribution {
    /// Create a new spectral distribution with a fixed value.
    pub const fn new(
        label: &'static str,
        start: usize,
        len: usize,
        checksum: Float,
        value: Float,
    ) -> Self {
        Self {
            label,
            start,
            len,
            checksum,
            value,
        }
    }
}

impl SpectralDistribution for FixedDistribution {
    type Value = Float;

    fn label(&self) -> String {
        self.label.to_string()
    }

    fn start(&self) -> usize {
        self.start
    }

    fn len(&self) -> usize {
        self.len
    }

    fn at(&self, wavelength: usize) -> Option<Self::Value> {
        if self.start <= wavelength && wavelength < self.start + self.len {
            Some(self.value)
        } else {
            None
        }
    }

    fn checksum(&self) -> Self::Value {
        self.checksum
    }
}

// --------------------------------------------------------------------------------------------------------------------

/// A standard observer at nanometer resolution.
#[cfg_attr(
    feature = "pyffi",
    pyclass(frozen, module = "prettypretty.color.spectrum")
)]
#[derive(Debug)]
pub struct Observer {
    label: &'static str,
    start: usize,
    checksum: [Float; 3],
    data: &'static [[Float; 3]],
}

impl Observer {
    /// Create a new observer.
    pub const fn new(
        label: &'static str,
        start: usize,
        checksum: [Float; 3],
        data: &'static [[Float; 3]],
    ) -> Self {
        Self {
            label,
            start,
            checksum,
            data,
        }
    }
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl Observer {
    /// Get a descriptive label for this observer.
    #[inline]
    pub fn label(&self) -> String {
        self.label.to_string()
    }

    /// Get this observer's starting wavelength.
    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    /// Get this observer's ending wavelength.
    #[inline]
    pub fn end(&self) -> usize {
        self.start + self.data.len()
    }

    /// Determine whether this observer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.len() == 0
    }

    /// Determine the number of entries for this observer.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Get this observer's value for the given wavelength.
    #[inline]
    pub fn at(&self, wavelength: usize) -> Option<[Float; 3]> {
        if self.start <= wavelength && wavelength < self.start + self.data.len() {
            Some(self.data[wavelength - self.start])
        } else {
            None
        }
    }

    /// Get this observer's checksum.
    #[inline]
    pub fn checksum(&self) -> [Float; 3] {
        self.checksum
    }

    /// Get the number of entries. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    #[inline]
    pub fn __len__(&self) -> usize {
        self.data.len()
    }

    /// Get the entry at the given index. <i class=python-only>Python
    /// only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __getitem__(&self, index: usize) -> PyResult<[Float; 3]> {
        self.at(self.start + index).ok_or_else(|| {
            pyo3::exceptions::PyIndexError::new_err(format!(
                "{} <= index for {}",
                self.data.len(),
                self.label
            ))
        })
    }

    /// Get a debug representation. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    #[inline]
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl SpectralDistribution for Observer {
    type Value = [Float; 3];

    #[inline]
    fn label(&self) -> String {
        self.label.to_string()
    }

    #[inline]
    fn start(&self) -> usize {
        self.start
    }

    #[inline]
    fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    fn at(&self, wavelength: usize) -> Option<Self::Value> {
        if self.start <= wavelength && wavelength < self.start + self.data.len() {
            Some(self.data[wavelength - self.start])
        } else {
            None
        }
    }

    #[inline]
    fn checksum(&self) -> Self::Value {
        self.checksum
    }
}

// --------------------------------------------------------------------------------------------------------------------

pub mod std_observer {
    //! Free-standing functions related to the CIE standard observer.

    #[cfg(feature = "pyffi")]
    use pyo3::prelude::pyfunction;

    use crate::Float;

    /// Compute an [analytical
    /// approximation](https://research.nvidia.com/publication/2013-07_simple-analytic-approximations-cie-xyz-color-matching-functions)
    /// for the 1931 2º standard observer's x.
    #[cfg_attr(feature = "pyffi", pyfunction)]
    pub fn x(wavelength: Float) -> Float {
        let p1 = (wavelength - 442.0) * (if wavelength < 442.0 { 0.0624 } else { 0.0374 });
        let p2 = (wavelength - 599.8) * (if wavelength < 599.8 { 0.0264 } else { 0.0323 });
        let p3 = (wavelength - 501.1) * (if wavelength < 501.1 { 0.0490 } else { 0.0382 });

        0.362 * (-0.5 * p1 * p1).exp() + 1.056 * (-0.5 * p2 * p2).exp()
            - 0.065 * (-0.5 * p3 * p3).exp()
    }

    /// Compute an [analytical
    /// approximation](https://research.nvidia.com/publication/2013-07_simple-analytic-approximations-cie-xyz-color-matching-functions)
    /// for the 1931 2º standard observer's y.
    #[cfg_attr(feature = "pyffi", pyfunction)]
    pub fn y(wavelength: Float) -> Float {
        let p1 = (wavelength - 568.8) * (if wavelength < 568.8 { 0.0213 } else { 0.0247 });
        let p2 = (wavelength - 530.9) * (if wavelength < 530.9 { 0.0613 } else { 0.0322 });
        0.821 * (-0.5 * p1 * p1).exp() + 0.286 * (-0.5 * p2 * p2).exp()
    }

    /// Compute an [analytical
    /// approximation](https://research.nvidia.com/publication/2013-07_simple-analytic-approximations-cie-xyz-color-matching-functions)
    /// for the 1931 2º standard observer's z.
    #[cfg_attr(feature = "pyffi", pyfunction)]
    pub fn z(wavelength: Float) -> Float {
        let p1 = (wavelength - 437.0) * (if wavelength < 437.0 { 0.0845 } else { 0.0278 });
        let p2 = (wavelength - 459.0) * (if wavelength < 459.0 { 0.0385 } else { 0.0725 });
        1.217 * (-0.5 * p1 * p1).exp() + 0.681 * (-0.5 * p2 * p2).exp()
    }
}

// --------------------------------------------------------------------------------------------------------------------

/// A convenient constant for 1nm.
pub const ONE_NANOMETER: std::num::NonZeroUsize =
    unsafe { std::num::NonZeroUsize::new_unchecked(1) };

/// The data for a spectrum traversal.
///
/// This struct stores the spectral distribution resulting from pre-multiplying
/// the illuminant and observer and then computing the sum totals. It exists to
/// ensures that the data is not mutated by accident. Like the rest of the
/// module, this struct assumes one-nanometer resolution.
#[derive(Debug, Default)]
struct SpectrumTraversalData {
    premultiplied: Vec<[Float; 3]>,
    total_xyz: [Float; 3],
}

impl SpectrumTraversalData {
    /// Create the spectrum traversal data for the observer and illuminant.
    fn new<I, O>(illuminant: &I, observer: &O) -> Self
    where
        I: SpectralDistribution<Value = Float>,
        O: SpectralDistribution<Value = [Float; 3]>,
    {
        let start = illuminant.start().max(observer.start());
        let end = illuminant.end().min(observer.end());
        let mut premultiplied: Vec<[Float; 3]> = Vec::with_capacity(end - start);

        for index in start..end {
            let [x, y, z] = observer.at(index).unwrap();
            let s = illuminant.at(index).unwrap() / 100.0;
            let ys = y * s;
            premultiplied.push([x * s, ys, z * s]);
        }

        let mut data = Self {
            premultiplied,
            total_xyz: [0.0, 0.0, 0.0],
        };
        data.total_xyz = data.pulse(0, end - start);

        data
    }

    #[inline]
    fn len(&self) -> usize {
        self.premultiplied.len()
    }

    #[inline]
    fn total(&self) -> [Float; 3] {
        self.total_xyz
    }

    fn pulse(&self, start: usize, width: usize) -> [Float; 3] {
        let mut xs = Sum::default();
        let mut ys = Sum::default();
        let mut zs = Sum::default();

        for index in start..start + width {
            let index = index % self.premultiplied.len();
            let [x, y, z] = self.premultiplied[index];
            xs += x;
            ys += y;
            zs += z;
        }

        [xs.value(), ys.value(), zs.value()]
    }

    fn pulse_color(&self, start: usize, width: usize) -> Color {
        let [x, y, z] = self.pulse(start, width);
        let luminosity = self.total_xyz[1];

        Color::new(
            ColorSpace::Xyz,
            [x / luminosity, y / luminosity, z / luminosity],
        )
    }
}

/// An iterator to trace the human visual gamut.
///
/// This iterator computes an observer's tristimulus values under a given
/// illuminant [for square wave
/// pulses](https://horizon-lab.org/colorvis/gamutvis.html) of [increasing
/// widths](https://www.russellcottrell.com/photo/visualGamut.htm) circling
/// through the spectrum shared between illuminant and observer. Moreover, it
/// consecutively yields the tristimulus values for each pulse as a line of
/// [`GamutTraversalStep`]s.
///
/// Pulse positions and widths grow with the same stride, which defaults to 5nm.
/// The first pulse position is 0, whereas the first pulse width is 1nm. As a
/// result, the first line yielded by this iterator is the spectral locus; it is
/// best rendered with a stride of 1nm.
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.spectrum"))]
#[derive(Debug)]
pub struct SpectrumTraversal {
    data: SpectrumTraversalData,
    stride: usize,
    position: usize,
    width: usize,
    remaining: usize,
    minimum: [Float; 3],
    maximum: [Float; 3],
}

impl SpectrumTraversal {
    /// Create a new spectrum traversal. <i class=rust-only>Rust only!</i>
    pub fn new<I, O>(illuminant: &I, observer: &O, stride: std::num::NonZeroUsize) -> Self
    where
        I: SpectralDistribution<Value = Float>,
        O: SpectralDistribution<Value = [Float; 3]>,
    {
        let data = SpectrumTraversalData::new(illuminant, observer);
        let stride = stride.get();
        let remaining = Self::total_steps(data.len(), stride);

        Self {
            data,
            stride,
            position: 0,
            width: 0,
            remaining,
            minimum: [Float::INFINITY; 3],
            maximum: [Float::NEG_INFINITY; 3],
        }
    }

    fn total_steps(len: usize, stride: usize) -> usize {
        let mut lines = (len - 1) / stride;
        let points = lines + 1;
        let rem = (len - 1) % stride;
        if rem > 0 {
            lines += 1;
        }

        lines * points
    }
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl SpectrumTraversal {
    /// Create a new spectrum traversal. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    #[new]
    pub fn py_new(
        illuminant: &Illuminant,
        observer: &Observer,
        stride: std::num::NonZeroUsize,
    ) -> Self {
        Self::new(illuminant, observer, stride)
    }

    /// Get the traversal's stride.
    #[inline]
    pub fn stride(&self) -> usize {
        self.stride
    }

    /// Determine the white point for the spectrum traversal's illuminant and
    /// observer.
    pub fn white_point(&self) -> [Float; 3] {
        let [x, y, z] = self.data.total();
        if 0.0 < y {
            [x / y, 1.0, z / y]
        } else {
            [0.0, 0.0, 0.0]
        }
    }

    /// Get the smallest three tristimulus components encountered so far.
    pub fn minimum(&self) -> [Float; 3] {
        self.minimum
    }

    /// Get the largest three tristimulus components encountered so far.
    pub fn maximum(&self) -> [Float; 3] {
        self.maximum
    }

    /// Create a new spectrum traversal that reuses this instance's premultiplied
    /// illuminant and observer data.
    ///
    /// This method enables reuse of a spectrum traversal's internal state,
    /// which easily comprises 1,200 floating point values that took as many
    /// floating point multiplications to generate. It does so by creating a new
    /// iterator while also moving the premultiplied illuminant and observer
    /// data from this iterator to the new instance. To do so safely, it leaves
    /// behind an empty premultiplied data table without any entries. As a
    /// result, this method effectively terminates this iterator. However, the
    /// newly created iterator starts afresh.
    ///
    /// This method represents a practical compromise between common Rust
    /// practice (iterators are not reused), PyO3 disallowing methods that
    /// consume self, and the large size of premultiplied illuminant and
    /// observer data.
    pub fn restart(&mut self) -> SpectrumTraversal {
        // End this iterator.
        self.width = self.data.len();
        let remaining = Self::total_steps(self.data.len(), self.stride);

        Self {
            data: std::mem::take(&mut self.data),
            stride: self.stride,
            position: 0,
            width: 0,
            remaining,
            minimum: [Float::INFINITY; 3],
            maximum: [Float::NEG_INFINITY; 3],
        }
    }

    /// Get the number of remaining steps. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __len__(&self) -> usize {
        self.remaining
    }

    /// Get this iterator. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Get the next item. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<GamutTraversalStep> {
        slf.next()
    }

    /// Get a debug representation. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!(
            "SpectrumTraversal(stride={}, position={}, width={}, remaining={}, samples={})",
            self.stride,
            self.position,
            self.width,
            self.remaining,
            self.data.len(),
        )
    }
}

impl Iterator for SpectrumTraversal {
    type Item = GamutTraversalStep;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.len() <= self.width {
            return None;
        } else if self.width == 0 {
            self.width = 1;
        }

        self.remaining -= 1;
        let color = self.data.pulse_color(self.position, self.width);
        for index in 0..3 {
            self.minimum[index] = self.minimum[index].min(color[index]);
            self.maximum[index] = self.maximum[index].max(color[index]);
        }

        let result = if self.position == 0 {
            GamutTraversalStep::MoveTo(color)
        } else {
            GamutTraversalStep::LineTo(color)
        };

        self.position += self.stride;

        if self.data.len() <= self.position {
            self.width += self.stride;
            self.position = 0;
        }

        Some(result)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl std::iter::ExactSizeIterator for SpectrumTraversal {
    fn len(&self) -> usize {
        self.remaining
    }
}

impl std::iter::FusedIterator for SpectrumTraversal {}

// --------------------------------------------------------------------------------------------------------------------

pub use crate::cie::CIE_ILLUMINANT_D50;
pub use crate::cie::CIE_ILLUMINANT_D65;
pub use crate::cie::CIE_OBSERVER_10DEG_1964;
pub use crate::cie::CIE_OBSERVER_2DEG_1931;

/// The CIE E standard illuminant at one-nanometer resolution.
pub const CIE_ILLUMINANT_E: FixedDistribution =
    FixedDistribution::new("Illuminant E", 300, 530, 53_000.0, 100.0);

#[cfg(test)]
mod test {
    use super::{
        SpectralDistribution, CIE_ILLUMINANT_D50, CIE_ILLUMINANT_D65, CIE_OBSERVER_10DEG_1964,
        CIE_OBSERVER_2DEG_1931,
    };
    use crate::core::Sum;

    #[test]
    fn test_checksum() {
        for illuminant in [&CIE_ILLUMINANT_D50, &CIE_ILLUMINANT_D65] {
            let mut sum = Sum::new();

            for wavelength in illuminant.range() {
                sum += illuminant.at(wavelength).unwrap()
            }

            assert_eq!(sum.value(), illuminant.checksum());
        }

        for observer in [&CIE_OBSERVER_2DEG_1931, &CIE_OBSERVER_10DEG_1964] {
            let mut x_sum = Sum::new();
            let mut y_sum = Sum::new();
            let mut z_sum = Sum::new();

            for wavelength in observer.range() {
                let [x, y, z] = observer.at(wavelength).unwrap();
                x_sum += x;
                y_sum += y;
                z_sum += z;
            }

            assert_eq!(
                [x_sum.value(), y_sum.value(), z_sum.value()],
                observer.checksum()
            );
        }
    }
}
