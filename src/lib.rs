#![doc(
    html_logo_url = "https://repository-images.githubusercontent.com/796446264/7483a099-9280-489e-b1b0-119497d8c2da"
)]

//! # Pretty 🌸 Pretty
//!
//! Prettypretty brings 2020s color science to 1970 terminals.
#![cfg_attr(
    not(feature = "pyffi"),
    doc = " This version of the API documentation **covers native Rust interfaces
only**. You can find a version that also covers Python integration, with the `pyffi`
feature enabled,
[on GitHub](https://apparebit.github.io/prettypretty/prettypretty/). "
)]
#![cfg_attr(
    feature = "pyffi",
    doc = " This version of the API documentation **covers both Rust and Python
interfaces**. You can find a version without Python integration, with the `pyffi`
feature disabled, [on Docs.rs](https://docs.rs/prettypretty/latest/prettypretty/). "
)]
//! Separately, the [user guide](https://apparebit.github.io/prettypretty/)
//! provides a comprehensive overview and several deep dives covering both
//! languages. There also are the [type
//! stubs](https://github.com/apparebit/prettypretty/blob/main/prettypretty/color/__init__.pyi)
//! and [API documentation](https://apparebit.github.io/prettypretty/python/)
//! for Python as well as the shared [source
//! repository](https://github.com/apparebit/prettypretty), [Rust
//! crate](https://crates.io/crates/prettypretty), and [Python
//! package](https://pypi.org/project/prettypretty/).
//!
//!
//! ## Overview
//!
//!   * [`Color`] is prettypretty's abstraction for **high-resolution colors**.
//!     It combines a [`ColorSpace`] with three [`Float`] coordinates. Its
//!     methods expose much of prettypretty's functionality, including
//!     conversion between color spaces, interpolation between colors,
//!     calculation of perceptual contrast, as well as gamut testing, clipping,
//!     and mapping.
//!   * The [`style`] module's primary abstraction, [`Style`](style::Style),
//!     models **terminal appearance** as a text
//!     [`Format`](style::format::Format) combined with foreground and
//!     background [`Colorant`](style::Colorant)s. The latter maximizes
//!     expressivity by combining the terminal-specific color formats
//!     [`AnsiColor`](style::AnsiColor), [`EmbeddedRgb`](style::EmbeddedRgb),
//!     [`GrayGradient`](style::GrayGradient), and
//!     [`TrueColor`](style::TrueColor) with high-resolution [`Color`].
//!
//!     **Using styles** is straight-forward:
//!
//!       * Fluently assemble a style with
//!         [`Style::builder`](style::Style::builder) or
//!         [`stylist`](style::stylist()).
//!       * Adjust the style to terminal capabilities with
//!         [`Style::cap`](style::Style::cap).
//!       * Apply the style by displaying it, e.g., `print!("{}",style)`.
//!       * Undo the style by displaying its negation, e.g.,
//!         `print!("{}",!style)`.
//!
//!   * The [`trans`] module's [`Translator`](crate::trans::Translator)
//!     implements **stateful and lossy translation** between color formats,
//!     including high-quality conversions between ANSI/8-bit colors and
//!     high-resolution colors. Amongst these conversions, translation from
//!     high-resolution to ANSI colors is particularly challenging, as it
//!     requires mapping a practically infinite number of colors onto 16 colors,
//!     four of which are achromatic. `Translator` includes several algorithms
//!     for doing so.
//!   * The [`termio`] module's [`VtScanner`](crate::termio::VtScanner) tries to
//!     make **integration of terminal I/O** as simple as possible by
//!     encapsulating much of the machinery for parsing ANSI escape sequences.
//!     When combined with [`ThemeEntry`](crate::trans::ThemeEntry), the two
//!     types provide all the functionality for extracting the current color
//!     theme from the terminal and passing it to
//!     [`Translator`](crate::trans::Translator), with exception of actual I/O.
//!   * The optional [`gamut`] and [`spectrum`] submodules enable the traversal
//!     of **color space gamuts** and the **human visual gamut**, respectively.
//!     The
//!     [plot.py](https://github.com/apparebit/prettypretty/blob/main/prettypretty/plot.py)
//!     and
//!     [viz3d.py](https://github.com/apparebit/prettypretty/blob/main/prettypretty/viz3d.py)
//!     scripts leverage the additional functionality for generating helpful
//!     color visualizations.
//!
//!
//! ## Feature Flags
//!
//! Prettypretty supports three feature flags:
//!
//!   - `f64` selects the eponymous type as floating point type [`Float`] and
//!     `u64` as [`Bits`] instead of `f32` as [`Float`] and `u32` as [`Bits`].
//!     This feature is enabled by default.
//!   - `gamut` controls prettypretty's support for tracing the boundaries of
//!     color spaces (`mod gamut`, `ColorSpace::gamut`) and the human visual
//!     gamut (`mod spectrum`). This feature is disabled by default.
//!   - `pyffi` controls prettypretty's Python integration through
//!     [PyO3](https://pyo3.rs/). Enabling the feature activates handwritten
//!     code in prettypretty as well as new types and trait implementations
//!     generated by PyO3's macros. This feature is disabled by default.
//!
//! Prettypretty's Python extension module is best built with
//! [Maturin](https://www.maturin.rs), PyO3's dedicated build tool. It requires
//! the `pyffi` feature. Since Python packages typically come with "batteries
//! included," the `gamut` feature is highly recommended as well.
//!
#![cfg_attr(
    feature = "pyffi",
    doc = "Throughout the API documentation, items that are only available in
    one of the two languages are flagged as <i
    class=python-only>Python only!</i> or <i class=rust-only>Rust only!</i>.
    Items only available with the `gamut` feature are flagged as <i
    class=gamut-only>Gamut only!</i>."
)]
//!
//!
//! ## BYOIO: Bring Your Own (Terminal) I/O
//!
//! Unlike prettypretty's Python version, the Rust version has no facilities for
//! terminal I/O. In part, that reflects the limited facilities of Rust's
//! standard library. In part, that is a deliberate decision given the split
//! between sync and async Rust. At the same time, prettypretty's
//! [`Translator`](crate::trans::Translator) requires the terminal's current
//! color theme so that it can provide high-quality translation to ANSI colors.
//!
//! To simplify the integration effort, prettypretty includes
//! [`ThemeEntry`](crate::trans::ThemeEntry) for querying the terminal and
//! parsing the response as well as [`VtScanner`](crate::termio::VtScanner) for
//! reading just the bytes belonging to an ANSI escape sequence. The [`termio`]
//! module's documentation illustrates the use of the two types and also
//! discusses some of the finer points of error handling, including suggested
//! solution approaches.
//!
//!
//! ## Acknowledgements
//!
//! Implementing prettypretty's color support was a breeze. In part, that was
//! because I had been toying with different approaches to terminal styling for
//! a while and knew what I wanted to build. In part, that was because I
//! benefitted from [Lea Verou](http://lea.verou.me/)'s and [Chris
//! Lilley](https://svgees.us/)'s work on the [Color.js](https://colorjs.io)
//! library and [CSS Color 4](https://www.w3.org/TR/css-color-4/) specification.
//! Prettypretty directly reuses Color.js' formulae for conversion between color
//! spaces and implements several CSS Color 4 algorithms. Thank you! 🌸
//!
//!

/// The floating point type in use.
#[cfg(feature = "f64")]
pub type Float = f64;
/// The floating point type in use.
#[cfg(not(feature = "f64"))]
pub type Float = f32;

/// [`Float`]'s bits.
#[cfg(feature = "f64")]
pub type Bits = u64;
/// [`Float`]'s bits.
#[cfg(not(feature = "f64"))]
pub type Bits = u32;

mod core;
pub mod error;
mod object;
pub mod style;
pub mod termio;
pub mod trans;
mod util;

#[cfg(feature = "gamut")]
mod cie;
#[cfg(feature = "gamut")]
pub mod gamut {
    //! Utility module with the machinery for traversing RGB gamut boundaries
    //! with [`ColorSpace::gamut`](crate::ColorSpace).  <i
    //! class=gamut-only>Gamut only!</i>
    pub use crate::core::{GamutTraversal, GamutTraversalStep};
}
#[cfg(feature = "gamut")]
pub mod spectrum;

#[cfg(feature = "pyffi")]
pub use core::close_enough;

#[doc(hidden)]
pub use core::to_eq_bits;

pub use core::{ColorSpace, HueInterpolation};
pub use object::{Color, Interpolator, OkVersion};

#[cfg(feature = "pyffi")]
use pyo3::prelude::*;
#[cfg(feature = "pyffi")]
use pyo3::types::PyDict;

#[doc(hidden)]
#[cfg(feature = "pyffi")]
#[pymodule]
pub fn color(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let modcolor_name = m.name()?;
    let modcolor_name = modcolor_name.to_str()?;
    let modformat_name = format!("{}.style.format", modcolor_name);
    let modstyle_name = format!("{}.style", modcolor_name);
    let modtermio_name = format!("{}.termio", modcolor_name);
    let modtrans_name = format!("{}.trans", modcolor_name);

    // -------------------------------------------------------------------------- color
    m.add_function(wrap_pyfunction!(close_enough, m)?)?;

    m.add_class::<Color>()?;
    m.add_class::<ColorSpace>()?;
    m.add_class::<HueInterpolation>()?;
    m.add_class::<Interpolator>()?;
    m.add_class::<OkVersion>()?;

    // -------------------------------------------------------------------- color.style
    let modstyle = PyModule::new_bound(m.py(), "style")?;
    modstyle.add("__package__", modcolor_name)?;
    modstyle.add_class::<style::AnsiColor>()?;
    modstyle.add_class::<style::Colorant>()?;
    modstyle.add_class::<style::EmbeddedRgb>()?;
    modstyle.add_class::<style::Fidelity>()?;
    modstyle.add_class::<style::GrayGradient>()?;
    modstyle.add_class::<style::Layer>()?;
    modstyle.add_function(wrap_pyfunction!(style::stylist, m)?)?;
    modstyle.add_class::<style::Style>()?;
    modstyle.add_class::<style::Stylist>()?;
    modstyle.add_class::<style::TrueColor>()?;
    m.add_submodule(&modstyle)?;

    // Only change __name__ attribute after submodule has been added.
    modstyle.setattr("__name__", &modstyle_name)?;

    // ------------------------------------------------------------- color.style.format
    let modformat = PyModule::new_bound(m.py(), "format")?;
    modformat.add("__package__", &modstyle_name)?;
    modformat.add_class::<style::format::AllAttributes>()?;
    modformat.add_class::<style::format::Attribute>()?;
    modformat.add_class::<style::format::AttributeIterator>()?;
    modformat.add_class::<style::format::Format>()?;
    modstyle.add_submodule(&modformat)?;

    modformat.setattr("__name__", &modformat_name)?;

    // ------------------------------------------------------------------- color.termio
    let modtermio = PyModule::new_bound(m.py(), "termio")?;
    modtermio.add("__package__", modcolor_name)?;
    modtermio.add_class::<termio::Action>()?;
    modtermio.add_class::<termio::Control>()?;
    modtermio.add_class::<termio::VtScanner>()?;
    m.add_submodule(&modtermio)?;

    // Only change __name__ attribute after submodule has been added.
    modtermio.setattr("__name__", &modtermio_name)?;

    // -------------------------------------------------------------------- color.trans
    let modtrans = PyModule::new_bound(m.py(), "trans")?;
    modtrans.add("__package__", modcolor_name)?;
    modtrans.add_class::<trans::ThemeEntry>()?;
    modtrans.add_class::<trans::ThemeEntryIterator>()?;
    modtrans.add_class::<trans::Translator>()?;
    modtrans.add("VGA_COLORS", trans::VGA_COLORS)?;
    m.add_submodule(&modtrans)?;

    // Only change __name__ attribute after submodule has been added.
    modtrans.setattr("__name__", &modtrans_name)?;

    // -------------------------------------------------------------------- sys.modules
    // Patch sys.modules
    //let sys = PyModule::import_bound(m.py(), "sys")?;
    let py_modules: Bound<'_, PyDict> = PyModule::import_bound(m.py(), "sys")?
        .getattr("modules")?
        .downcast_into()?;
    py_modules.set_item(&modstyle_name, modstyle)?;
    py_modules.set_item(&modformat_name, modformat)?;
    py_modules.set_item(&modtermio_name, modtermio)?;
    py_modules.set_item(&modtrans_name, modtrans)?;

    #[cfg(feature = "gamut")]
    register_modgamut(m)?;

    Ok(())
}

#[doc(hidden)]
#[cfg(all(feature = "pyffi", feature = "gamut"))]
fn register_modgamut(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let modcolor_name = m.name()?;
    let modcolor_name = modcolor_name.to_str()?;
    let modgamut_name = format!("{}.gamut", modcolor_name);
    let modspectrum_name = format!("{}.spectrum", modcolor_name);
    let modobserver_name = format!("{}.std_observer", modspectrum_name);

    // -------------------------------------------------------------------- color.gamut
    let modgamut = PyModule::new_bound(m.py(), "gamut")?;
    modgamut.add("__package__", modcolor_name)?;
    modgamut.add_class::<gamut::GamutTraversal>()?;
    modgamut.add_class::<gamut::GamutTraversalStep>()?;
    m.add_submodule(&modgamut)?;

    // Only change __name__ attribute after submodule has been added.
    modgamut.setattr("__name__", &modgamut_name)?;

    // ----------------------------------------------------------------- color.spectrum
    let modspectrum = PyModule::new_bound(m.py(), "spectrum")?;
    modspectrum.add("__package__", modcolor_name)?;
    modspectrum.add(
        "CIE_ILLUMINANT_D50",
        spectrum::Illuminant::new(Box::new(spectrum::CIE_ILLUMINANT_D50)
            as Box<dyn spectrum::SpectralDistribution<Value = Float> + Send>),
    )?;
    modspectrum.add(
        "CIE_ILLUMINANT_D65",
        spectrum::Illuminant::new(Box::new(spectrum::CIE_ILLUMINANT_D65)
            as Box<dyn spectrum::SpectralDistribution<Value = Float> + Send>),
    )?;
    modspectrum.add(
        "CIE_ILLUMINANT_E",
        spectrum::Illuminant::new(Box::new(spectrum::CIE_ILLUMINANT_E)
            as Box<dyn spectrum::SpectralDistribution<Value = Float> + Send>),
    )?;
    modspectrum.add("CIE_OBSERVER_2DEG_1931", spectrum::CIE_OBSERVER_2DEG_1931)?;
    modspectrum.add("CIE_OBSERVER_10DEG_1964", spectrum::CIE_OBSERVER_10DEG_1964)?;
    modspectrum.add("ONE_NANOMETER", spectrum::ONE_NANOMETER)?;
    modspectrum.add_class::<spectrum::Illuminant>()?;
    modspectrum.add_class::<spectrum::IlluminatedObserver>()?;
    modspectrum.add_class::<spectrum::Observer>()?;
    modspectrum.add_class::<spectrum::SpectrumTraversal>()?;
    m.add_submodule(&modspectrum)?;

    // Only change __name__ attribute after submodule has been added.
    modspectrum.setattr("__name__", &modspectrum_name)?;

    // -------------------------------------------------------- color.spectrum.observer
    let modobserver = PyModule::new_bound(m.py(), "std_observer")?;
    modobserver.add("__package__", modcolor_name)?;
    modobserver.add_function(wrap_pyfunction!(spectrum::std_observer::x, m)?)?;
    modobserver.add_function(wrap_pyfunction!(spectrum::std_observer::y, m)?)?;
    modobserver.add_function(wrap_pyfunction!(spectrum::std_observer::z, m)?)?;
    modspectrum.add_submodule(&modobserver)?;
    modobserver.setattr("__name__", &modobserver_name)?;

    // -------------------------------------------------------------------- sys.modules
    let py_modules: Bound<'_, PyDict> = PyModule::import_bound(m.py(), "sys")?
        .getattr("modules")?
        .downcast_into()?;

    py_modules.set_item(&modgamut_name, modgamut)?;
    py_modules.set_item(&modspectrum_name, modspectrum)?;
    py_modules.set_item(&modobserver_name, modobserver)?;

    Ok(())
}
