//! Optional module implementing the traversal of the human visual gamut.
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
        ignoring `#[cfg]` attributes. The impact is noticeable: Whereas Rust code
        might get by with four different implementations of [`SpectralDistribution`]'s
        methods, Python integration necessitates another three."
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
         SpectralDistribution<Value=Float> + Send + Sync>`."
)]
//!   * [`IlluminatedObserver`] is a table-driven implementation of
//!     `SpectralDistribution<Value=[Float;3]>`. Its data is the
//!     result of the per-wavelength multiplication of a
//!     `SpectralDistribution<Value=Float>` and a
//!     `SpectralDistribution<Value=[Float;3]>`, i.e., an illuminant
//!     and an observer.
//!
#![cfg_attr(
    feature = "pyffi",
    doc = "To play nice with PyO3 and Python, `Observer` and `IlluminatedObserver`
        reimplement all but one trait method in their `impl` blocks. `Illuminant`
        does the same, but is only defined if the `pyffi` feature is enabled. It
        makes two different trait implementations appear as the same type type in
        Python and allows instances to be passed back to Rust."
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
//! or the human visual gamut. It is instantiated with
//! [`IlluminatedObserver::visual_gamut`].
#![cfg_attr(
    feature = "pyffi",
    doc = "In Python, the constructor accepts `&Illuminant` and `&Observer`
        arguments only, which makes it monomorphic. While that is insufficient
        for generally overcoming PyO3's prohibition against polymorphic anything,
        in this particular case it suffices because spectrum traversal's
        constructors do not retain their inputs and instead keep the result of
        premultiplying the two spectral distributions. In other words, the rest
        of [`SpectrumTraversal`]'s implementation is strictly monomophic itself."
)]
//!

use std::sync::Arc;

#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

use crate::{
    core::{GamutTraversalStep, ThreeSum},
    Color, ColorSpace, Float,
};

/// A spectral distribution at nanometer resolution.
///
/// A concrete implementation of this trait must provide methods that return a
/// descriptive label, a start wavelength, a length, and the spectral
/// distribution's values.
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
/// In addition to implementing the [`SpectralDistribution`] trait, this struct
/// reimplements the trait's methods (other than `range`, which returns a
/// standard library type that is not supported by PyO3) as well as `__len__`,
/// `__getitem__`, and `__repr__` as regular methods for better Python
/// integration. It forwards all invocations to a `dyn
/// SpectralDistribution<Value=Float>`.
#[cfg(feature = "pyffi")]
#[pyclass(frozen, module = "prettypretty.color.trans")]
pub struct Illuminant {
    distribution: Box<dyn SpectralDistribution<Value = Float> + Send + Sync>,
}

#[cfg(feature = "pyffi")]
impl Illuminant {
    /// Create a new illuminant.
    pub fn new(distribution: Box<dyn SpectralDistribution<Value = Float> + Send + Sync>) -> Self {
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

/// A table-driven spectral distribution over floating point values.
#[derive(Clone, Debug)]
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

/// A spectral distribution with a fixed floating point value.
#[derive(Clone, Debug)]
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
///
/// The CIE's standard observers, or color matching functions, model human color
/// perception. Since humans are trichromatic, the per-wavelength values of
/// standard observers are triples of floating point numbers.
#[cfg_attr(
    feature = "pyffi",
    pyclass(frozen, module = "prettypretty.color.spectrum")
)]
#[derive(Clone, Debug)]
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
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl SpectralDistribution for Observer {
    type Value = [Float; 3];

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

/// An illuminated observer at nanometer resolution.
///
/// An illuminated observer is a spectral distribution representing a choice of
/// illuminant and observer when computing tristimulus values. Its
/// per-wavelength values are computed by premultiplying the illuminant's and
/// observer's per-wavelength values. As a result, an illuminated observer, just
/// like an observer, has three floating point numbers as values.
///
/// When compared to [`SpectralDistribution`] and [`Observer`], this spectral
/// distribution provides additional functionality through the
/// [`IlluminatedObserver::minimum`], [`IlluminatedObserver::maximum`],
/// [`IlluminatedObserver::luminosity`], [`IlluminatedObserver::white_point`],
/// and [`IlluminatedObserver::visual_gamut`] methods.
///
/// ASTM standard E308 refers to the premultiplied values as *weighting
/// factors*.
#[cfg_attr(
    feature = "pyffi",
    pyclass(frozen, module = "prettypretty.color.spectrum")
)]
#[derive(Debug, Default)]
pub struct IlluminatedObserver {
    label: String,
    start: usize,
    minimum: [Float; 3],
    maximum: [Float; 3],
    checksum: [Float; 3],
    data: Arc<Vec<[Float; 3]>>,
}

impl IlluminatedObserver {
    /// Create a new illuminated observer.
    pub fn new<Illuminant, Observer>(illuminant: &Illuminant, observer: &Observer) -> Self
    where
        Illuminant: SpectralDistribution<Value = Float>,
        Observer: SpectralDistribution<Value = [Float; 3]>,
    {
        let start = illuminant.start().max(observer.start());
        let end = illuminant.end().min(observer.end());

        let mut data: Vec<[Float; 3]> = Vec::with_capacity(end - start);
        let mut checksum = ThreeSum::new();
        let mut minimum = [Float::INFINITY, Float::INFINITY, Float::INFINITY];
        let mut maximum = [
            Float::NEG_INFINITY,
            Float::NEG_INFINITY,
            Float::NEG_INFINITY,
        ];

        for index in start..end {
            let [x, y, z] = observer.at(index).unwrap();
            let s = illuminant.at(index).unwrap() / 100.0;
            let value = [s * x, s * y, s * z];

            data.push(value);
            checksum += value;

            for c in 0..=2 {
                minimum[c] = minimum[c].min(value[c]);
                maximum[c] = maximum[c].max(value[c]);
            }
        }

        Self {
            label: format!("{} / {}", illuminant.label(), observer.label()),
            start,
            minimum,
            maximum,
            checksum: checksum.value(),
            data: Arc::new(data),
        }
    }
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl IlluminatedObserver {
    #[cfg(feature = "pyffi")]
    #[new]
    pub fn py_new(illuminant: &Illuminant, observer: &Observer) -> Self {
        Self::new(illuminant, observer)
    }

    /// Get a descriptive label for this illuminated observer.
    #[inline]
    pub fn label(&self) -> String {
        self.label.clone()
    }

    /// Get this illuminated observer's starting wavelength.
    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    /// Get this illuminated observer's ending wavelength.
    #[inline]
    pub fn end(&self) -> usize {
        self.start + self.data.len()
    }

    /// Determine whether this illuminated observer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.len() == 0
    }

    /// Determine the number of entries in this illuminated observer.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Get the premultiplied triple for the given wavelength.
    pub fn at(&self, wavelength: usize) -> Option<[Float; 3]> {
        if self.start <= wavelength && wavelength < self.start + self.data.len() {
            Some(self.data[wavelength - self.start])
        } else {
            None
        }
    }

    /// Get this illuminated observer's component-wise minimum.
    #[inline]
    pub fn minimum(&self) -> [Float; 3] {
        self.minimum
    }

    /// Get this illuminated observer's component-wise maximum.
    #[inline]
    pub fn maximum(&self) -> [Float; 3] {
        self.maximum
    }

    /// Get this illuminated observer's checksum.
    #[inline]
    pub fn checksum(&self) -> [Float; 3] {
        self.checksum
    }

    /// Get this illuminated observer's luminosity.
    ///
    /// The luminosity is the sum of all values for the second component.
    #[inline]
    pub fn luminosity(&self) -> Float {
        self.checksum[1]
    }

    /// Determine the white point for this illuminated observer.
    pub fn white_point(&self) -> Color {
        let [x, y, z] = self.checksum;
        // y/y to handle corner case of y being 0.0
        #[allow(clippy::eq_op)]
        Color::new(ColorSpace::Xyz, [x / y, y / y, z / y])
    }

    /// Create a new spectrum traversal with the given stride.
    ///
    /// This method returns an iterator tracing the limits of the human visual
    /// gamut in the CIE's XYZ color space, as parameterized by this illuminated
    /// observer's component distributions. The returned iterator implements the
    /// standard algorithm, shifting and rotating square pulses of unit height
    /// and increasing widths across this distribution's spectrum.
    pub fn visual_gamut(&self, stride: std::num::NonZeroUsize) -> SpectrumTraversal {
        SpectrumTraversal::new(stride, self.luminosity(), self.data.clone())
    }

    /// Get this illuminated observer's number of values. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __len__(&self) -> usize {
        self.data.len()
    }

    /// Get the entry at the given zero-based index, not wavelength. <i
    /// class=python-only>Python only!</i>
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

    /// Get a debug representation for this illuminated observer. <i
    /// class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl SpectralDistribution for IlluminatedObserver {
    type Value = [Float; 3];

    fn label(&self) -> String {
        self.label.clone()
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

/// An iterator tracing the visual gamut.
///
/// This iterator traces the boundaries of the human visual gamut assuming a
/// specific illuminant and observer by determining the colors resulting from
/// [square wave pulses](https://horizon-lab.org/colorvis/gamutvis.html) of
/// [increasing widths](https://www.russellcottrell.com/photo/visualGamut.htm)
/// shifted and rotated across the illuminant's and observer's shared spectrum.
/// Colors resulting from a pulse with the same width form a line.
///
/// Pulse positions and widths grow with the same stride. The first pulse
/// position is 0, whereas the first pulse width is 1nm. As a result, the first
/// line yielded by this iterator is the spectral locus; it is best rendered
/// with a stride of 1nm.
#[cfg_attr(feature = "pyffi", pyclass(module = "prettypretty.color.spectrum"))]
#[derive(Debug)]
pub struct SpectrumTraversal {
    data: Arc<Vec<[Float; 3]>>,
    luminosity: Float,
    stride: usize,
    position: usize,
    width: usize,
    remaining: usize,
}

impl SpectrumTraversal {
    /// Create a new spectral traversal.
    ///
    /// The given data must be the result of premultiplying an illuminant with
    /// an observer, e.g., an illuminated observer.
    fn new(stride: std::num::NonZeroUsize, luminosity: Float, data: Arc<Vec<[Float; 3]>>) -> Self {
        let stride = stride.get();
        let remaining = Self::derive_total_count(data.len(), stride);

        Self {
            data,
            luminosity,
            stride,
            position: 0,
            width: 0,
            remaining,
        }
    }

    fn derive_total_count(len: usize, stride: usize) -> usize {
        Self::derive_line_count(len, stride) * Self::derive_line_length(len, stride)
    }

    #[inline]
    fn derive_line_count(len: usize, stride: usize) -> usize {
        let mut count = (len - 1) / stride;
        if (len - 1) % stride > 0 {
            count += 1;
        }

        count
    }

    #[inline]
    fn derive_line_length(len: usize, stride: usize) -> usize {
        1 + (len - 1) / stride
    }
}

#[cfg_attr(feature = "pyffi", pymethods)]
impl SpectrumTraversal {
    /// Get the traversal's stride.
    #[inline]
    pub fn stride(&self) -> usize {
        self.stride
    }

    /// Get the total number of lines yielded by this spectrum traversal.
    #[inline]
    pub fn line_count(&self) -> usize {
        Self::derive_line_count(self.data.len(), self.stride)
    }

    /// Get the total number of colors per line yielded by this spectrum traversal.
    #[inline]
    pub fn line_length(&self) -> usize {
        Self::derive_line_length(self.data.len(), self.stride)
    }

    /// Compute the triple for a square, unit-height pulse.
    ///
    /// This method computes the sum of the underlying illuminated observer's
    /// values from the given start zero-based index (not wavelength) and width.
    /// Effective index values greater or equal to the underling illuminated
    /// observer's data table transparently wrap to the beginning of the table.
    pub fn pulse(&self, start: usize, width: usize) -> [Float; 3] {
        let mut sum = ThreeSum::new();
        for index in start..start + width {
            let index = index % self.data.len();
            sum += self.data[index];
        }
        sum.value()
    }

    /// Compute the XYZ color for a square, unit-height pulse.
    ///
    /// This method normalizes the result of [`SpectrumTraversal::pulse`] by
    /// dividing it with the illuminated observer's luminosity before wrapping
    /// it as a high-resolution color in the XYZ color space.
    pub fn pulse_color(&self, start: usize, width: usize) -> Color {
        let [x, y, z] = self.pulse(start, width);

        Color::new(
            ColorSpace::Xyz,
            [
                x / self.luminosity,
                y / self.luminosity,
                z / self.luminosity,
            ],
        )
    }

    /// Get the number of remaining steps. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __len__(&self) -> usize {
        self.len()
    }

    /// Get this iterator. <i class=python-only>Python only!</i>
    #[cfg(feature = "pyffi")]
    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Get the next step. <i class=python-only>Python only!</i>
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
        let color = self.pulse_color(self.position, self.width);

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

/// The CIE standard illuminant E at 1nm resolution.
pub static CIE_ILLUMINANT_E: FixedDistribution =
    FixedDistribution::new("Illuminant E", 300, 531, 53_100.0, 100.0);

// --------------------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::{
        GamutTraversalStep, IlluminatedObserver, SpectralDistribution, CIE_ILLUMINANT_D50,
        CIE_ILLUMINANT_D65, CIE_OBSERVER_10DEG_1964, CIE_OBSERVER_2DEG_1931,
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

    #[test]
    fn test_spectrum_traversal() {
        for (stride, line_count, line_length) in [(9, 53, 53), (10, 47, 48)] {
            let total = line_count * line_length;

            let mut traversal =
                IlluminatedObserver::new(&CIE_ILLUMINANT_D65, &CIE_OBSERVER_2DEG_1931)
                    .visual_gamut(std::num::NonZeroUsize::new(stride).unwrap());

            assert_eq!(traversal.line_count(), line_count);
            assert_eq!(traversal.line_length(), line_length);
            assert_eq!(traversal.len(), total);

            for index in 0..(line_count * line_length) {
                let step = traversal.next();

                assert_eq!(traversal.len(), total - index - 1);
                if index % line_length == 0 {
                    assert!(matches!(step, Some(GamutTraversalStep::MoveTo(_))));
                } else {
                    assert!(matches!(step, Some(GamutTraversalStep::LineTo(_))));
                }
            }

            assert!(matches!(traversal.next(), None));
        }
    }
}
