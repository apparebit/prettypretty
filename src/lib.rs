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
//!   * [`Color`] is prettypretty's high-resolution color object. It combines a
//!     [`ColorSpace`] with three [`Float`] coordinates. Its methods expose much
//!     of prettypretty's functionality, including conversion between color
//!     spaces, interpolation between colors, calculation of perceptual
//!     contrast, as well as gamut testing, clipping, and mapping.
//!   * [`TerminalColor`] combines [`DefaultColor`], [`AnsiColor`],
//!     [`EmbeddedRgb`], [`GrayGradient`], and [`TrueColor`] to represent the
//!     different kinds of terminal colors.
//!   * A fair number of `From<T>` and `TryFrom<T>` implementations cover
//!     lossless and partial conversions between color representations
//!     including, for example, conversion from EmbeddedRgb to `u8` index values
//!     as well true, terminal, and high-resolution colors.
//!   * [`Sampler`] performs the more difficult translation from ANSI to
//!     high-resolution colors, from high-resolution to 8-bit or ANSI colors,
//!     and the downgrading of terminal colors based on terminal capabilities
//!     and user preferences.
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
//! PyO3's integration between Rust and Python goes well beyond what is offered
//! by other foreign function interfaces. However, the macro-based
//! implementation does seem rather brittle. In particular, `#[new]` and
//! `#[staticmethod]` don't respect `cfg_attr()` and thus require the
//! duplication of methods. `impl Into<T>` seem to further trip up PyO3,
//! necessitating full separation into distinct blocks, one for
//! `feature="pyffi"`, one for `not(feature="pyffi")`, and sometimes a third one
//! for shared helper methods.
#![cfg_attr(
    feature = "pyffi",
    doc = "The documentation tags Python-only methods as
<span class=python-only></span> and Rust-only methods as
<span class=rust-only></span>."
)]
//!
//! Despite these warts, the Python version offers the same functionality as the
//! Rust version. Since Python does not support traits such as `From` and
//! `TryFrom`, prettypretty includes additional methods that make the same
//! functionality available.
//!
//!
//! ## BYOIO: Bring Your Own (Terminal) I/O
//!
//! Unlike the Python version, the Rust version of prettypretty does not (yet?)
//! include its own facilities for styled text or terminal I/O. Instead, it is
//! designed to be a lightweight addition that focuses on color management only.
//! To use this crate, an application should create its own instance of
//! [`Sampler`] with the colors of the current terminal theme.
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
//! *before* the question mark, followed by the color in X Windows `rgb:`
//! format, followed by ST. Once you stripped the prefix and suffix from a
//! response, you can use the `FromStr` trait to parse the X Windows color
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
mod error;
mod object;
#[doc(hidden)]
pub mod style;
mod term_color;
mod translation;
mod util;

#[cfg(feature = "pyffi")]
pub use core::close_enough;

#[doc(hidden)]
pub use core::to_eq_bits;

pub use core::{ColorFormatError, ColorSpace, HueInterpolation};
pub use error::OutOfBoundsError;
pub use object::{Color, Interpolator, OkVersion};
pub use term_color::{
    AnsiColor, DefaultColor, EmbeddedRgb, Fidelity, GrayGradient, Layer, TerminalColor, TrueColor,
};
pub use translation::{Sampler, Theme, ThemeEntry, ThemeEntryIterator, VGA_COLORS};

#[cfg(feature = "pyffi")]
use pyo3::prelude::*;

/// Collect Rust functions and classes in a Python in the `color` extension
/// module. <span class=python-only></span>
#[cfg(feature = "pyffi")]
#[pymodule]
pub fn color(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(close_enough, m)?)?;
    m.add_class::<AnsiColor>()?;
    m.add_class::<Color>()?;
    m.add_class::<ColorSpace>()?;
    m.add_class::<DefaultColor>()?;
    m.add_class::<EmbeddedRgb>()?;
    m.add_class::<Fidelity>()?;
    m.add_class::<GrayGradient>()?;
    m.add_class::<HueInterpolation>()?;
    m.add_class::<Interpolator>()?;
    m.add_class::<Layer>()?;
    m.add_class::<OkVersion>()?;
    m.add_class::<Sampler>()?;
    m.add_class::<TerminalColor>()?;
    m.add_class::<ThemeEntry>()?;
    m.add_class::<ThemeEntryIterator>()?;
    m.add_class::<TrueColor>()?;
    Ok(())
}
