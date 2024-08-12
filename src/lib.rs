#![doc(
    html_logo_url = "https://repository-images.githubusercontent.com/796446264/7483a099-9280-489e-b1b0-119497d8c2da"
)]

//! # Pretty 🌸 Pretty
//!
//! This is the API documentation for prettypretty, which brings 2020s color
//! science to 1970s terminals. Please consult the [user
//! guide](https://apparebit.github.io/prettypretty/) for a structured overview
//! of this library. The [type
//! stub](https://github.com/apparebit/prettypretty/blob/main/prettypretty/color.pyi)
//! and [API documentation](https://apparebit.github.io/prettypretty/python/)
//! for Python are separate as well.
//!
#![cfg_attr(
    not(feature = "pyffi"),
    doc = " **This version of prettypretty's API documentation covers native Rust
interfaces only without Python integration, with the `pyffi` feature flag disabled**"
)]
#![cfg_attr(
    feature = "pyffi",
    doc = "**This version of prettypretty's API documentation covers Rust
interfaces as well as Python ingration, with the `pyffi` feature flag enabled.**"
)]
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
//!   * The [`style`] module models **terminal colors**. Notably, the
//!     [`TerminalColor`](crate::style::TerminalColor) enum combines, in order
//!     from lowest to highest resolution,
//!     [`DefaultColor`](crate::style::DefaultColor),
//!     [`AnsiColor`](crate::style::AnsiColor),
//!     [`EmbeddedRgb`](crate::style::EmbeddedRgb),
//!     [`GrayGradient`](crate::style::GrayGradient), and
//!     [`TrueColor`](crate::style::TrueColor). It also implements `From` and
//!     `FromTry` traits for converting between color representations. For
//!     embedded RGB colors, they include conversion from/to `u8` and wrapped
//!     [`TerminalColor`](crate::style::TerminalColor) instances as well as raw
//!     24-bit (`[u8; 3]`), 24-bit color objects
//!     ([`TrueColor`](crate::style::TrueColor)), and high-resolution colors
//!     ([`Color`]).
//!   * The [`trans`] module implements more **complicated color conversions**.
//!     Notably, [`Translation`](crate::trans::Translator) handles conversion
//!     between ANSI/8-bit colors and high-resolution colors. It includes
//!     several algorithms for translating from high-resolution to ANSI colors.
//!     Since that requires mapping practically infinite onto 16 colors, that
//!     translation is particularly challenging.
//!
//!
//! ## Feature Flags
//!
//! This crate has two feature flags:
//!
//!   - **`f64`**: This feature is enabled by default. When disabled, the crate
//!     uses `f32`. In either case, the currently active floating point type is
//!     [`Float`] and the same-sized unsigned integer bits are [`Bits`].
//!   - **`pyffi`**: This feature is disabled by default. When enabled, this
//!     crate uses [PyO3](https://pyo3.rs/) and
//!     [Maturin](https://www.maturin.rs) to export an extension module for
//!     Python that makes this crate's Rust-based colors available in Python.
//!
//! PyO3's deep integration between Rust and Python certainly is impressive. But
//! its macro-based implementation also is quite brittle. Seemingly, the only
//! practical work-around for many of the implementation's limitations is to
//! duplicate methods, one with `pyffi` disabled and one with `pyffi` enabled.
//! In some instances, the two versions must also be separated into distinct
//! `impl` blocks. Nonetheless, prettypretty's Rust and Python APIs are largely
//! equivalent.
#![cfg_attr(
    feature = "pyffi",
    doc = "For the few cases of a constant or function being available only in
    one of the two languages, the documentation clearly labels the items as
    <span class=python-only></span> or <span class=rust-only></span>."
)]
//!
//! Since many floating point operations require the `std` crate, a `no_std`
//! version of this crate is highly unlikely.
//!
//!
//! ## BYOIO: Bring Your Own (Terminal) I/O
//!
//! Currently, the one significant difference in features between the Python and
//! Rust versions is that the latter does not yet include its own facilities for
//! styled text and terminal I/O. To correctly use this crate, Rust code must
//! create its own instance of [`Translator`](crate::trans::Translator) with the
//! colors of the *current terminal theme*.
//!
//! An application should use the ANSI escape sequences
//! ```text
//! "{OSC}{10..=11};?{ST}"
//! ```
//! and
//! ```text
//! "{OSC}4;{0..=15};?{ST}"
//! ```
//! to query the terminal for the two default and 16 extended ANSI colors. The
//! responses are ANSI escape sequences with the exact same prefix as requests,
//! up to and excluding the question mark, followed by the color in X Windows
//! `rgb:` format, followed by ST. Once you stripped the prefix and suffix from
//! a response, you can use the `FromStr` trait to parse the X Windows color
//! format into a color object.
//!
//! As usual, OSC stands for the character sequence `\x1b]` (escape, closing
//! square bracket) and ST stands for the character sequence `\x1b\\` (escape,
//! backslash). Some terminals answer with `\x0b` (bell) instead of ST.
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
pub mod gamut {
    //! Machinery for traversing RGB gamut boundaries with
    //! [`ColorSpace::gamut`](crate::ColorSpace).
    pub use crate::core::{GamutTraversal, GamutTraversalStep};
}
mod object;
pub mod spectrum;
//#[doc(hidden)]
//pub mod style;
pub mod style;
pub mod trans;
mod util;

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
    let modgamut_name = format!("{}.gamut", modcolor_name);
    let modspectrum_name = format!("{}.spectrum", modcolor_name);
    let modstyle_name = format!("{}.style", modcolor_name);
    let modtrans_name = format!("{}.trans", modcolor_name);

    // --------------------------------------------------------------- color
    m.add_function(wrap_pyfunction!(close_enough, m)?)?;

    m.add_class::<Color>()?;
    m.add_class::<ColorSpace>()?;
    m.add_class::<HueInterpolation>()?;
    m.add_class::<Interpolator>()?;
    m.add_class::<OkVersion>()?;

    // --------------------------------------------------------------- color.gamut
    let modgamut = PyModule::new_bound(m.py(), "gamut")?;
    modgamut.add("__package__", &modcolor_name)?;
    modgamut.add_class::<crate::gamut::GamutTraversal>()?;
    modgamut.add_class::<crate::gamut::GamutTraversalStep>()?;
    m.add_submodule(&modgamut)?;

    // Only change __name__ attribute after submodule has been added.
    modgamut.setattr("__name__", &modgamut_name)?;

    // --------------------------------------------------------------- color.spectrum
    let modspectrum = PyModule::new_bound(m.py(), "spectrum")?;
    modspectrum.add("__package__", &modcolor_name)?;
    modspectrum.add_function(wrap_pyfunction!(crate::spectrum::sum_luminance, m)?)?;
    modspectrum.add("CIE_ILLUMINANT_D65", crate::spectrum::CIE_ILLUMINANT_D65)?;
    modspectrum.add(
        "CIE_OBSERVER_2DEG_1931",
        crate::spectrum::CIE_OBSERVER_2DEG_1931,
    )?;
    modspectrum.add(
        "CIE_OBSERVER_2DEG_2015",
        crate::spectrum::CIE_OBSERVER_2DEG_2015,
    )?;
    modspectrum.add_class::<crate::spectrum::Illuminant>()?;
    modspectrum.add_class::<crate::spectrum::IlluminantIter>()?;
    modspectrum.add_class::<crate::spectrum::Observer>()?;
    modspectrum.add_class::<crate::spectrum::ObserverIter>()?;
    m.add_submodule(&modspectrum)?;

    // Only change __name__ attribute after submodule has been added.
    modspectrum.setattr("__name__", &modspectrum_name)?;

    // --------------------------------------------------------------- color.style
    let modstyle = PyModule::new_bound(m.py(), "style")?;
    modstyle.add("__package__", &modcolor_name)?;
    modstyle.add_class::<crate::style::AnsiColor>()?;
    modstyle.add_class::<crate::style::DefaultColor>()?;
    modstyle.add_class::<crate::style::EmbeddedRgb>()?;
    modstyle.add_class::<crate::style::Fidelity>()?;
    modstyle.add_class::<crate::style::GrayGradient>()?;
    modstyle.add_class::<crate::style::Layer>()?;
    modstyle.add_class::<crate::style::TerminalColor>()?;
    modstyle.add_class::<crate::style::TrueColor>()?;
    m.add_submodule(&modstyle)?;

    // Only change __name__ attribute after submodule has been added.
    modstyle.setattr("__name__", &modstyle_name)?;

    // --------------------------------------------------------------- color.trans
    let modtrans = PyModule::new_bound(m.py(), "trans")?;
    modtrans.add("__package__", &modcolor_name)?;
    modtrans.add_class::<crate::trans::ThemeEntry>()?;
    modtrans.add_class::<crate::trans::ThemeEntryIterator>()?;
    modtrans.add_class::<crate::trans::Translator>()?;
    m.add_submodule(&modtrans)?;

    // Only change __name__ attribute after submodule has been added.
    modtrans.setattr("__name__", &modtrans_name)?;

    // --------------------------------------------------------------- sys.modules
    // Patch sys.modules
    let sys = PyModule::import_bound(m.py(), "sys")?;
    let py_modules: Bound<'_, PyDict> = sys.getattr("modules")?.downcast_into()?;
    py_modules.set_item(&modgamut_name, modgamut)?;
    py_modules.set_item(&modspectrum_name, modspectrum)?;
    py_modules.set_item(&modstyle_name, modstyle)?;
    py_modules.set_item(&modtrans_name, modtrans)?;

    Ok(())
}
