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
//! The [user guide](https://apparebit.github.io/prettypretty/) provides a
//! comprehensive overview and several deep dives covering both languages. There
//! also are the [type
//! stubs](https://github.com/apparebit/prettypretty/blob/main/prettypretty/color/__init__.pyi)
//! and [API documentation](https://apparebit.github.io/prettypretty/python/)
//! for Python as well as the shared [source
//! repository](https://github.com/apparebit/prettypretty), [Rust
//! crate](https://crates.io/crates/prettypretty), and [Python
//! package](https://pypi.org/project/prettypretty/).
//!
//!
//! ## 1. Overview
//!
//! Prettypretty's main abstractions are:
//!
//!   * [`Color`] implements **high-resolution colors** by combining a
//!     [`ColorSpace`] with three [`Float`] coordinates. Its methods expose much
//!     of prettypretty's functionality, including conversion between color
//!     spaces, interpolation between colors, calculation of perceptual
//!     contrast, as well as gamut testing, clipping, and mapping.
//!   * The [`termco`] module offers a choice of **terminal-specific color
//!     formats** [`AnsiColor`](termco::AnsiColor),
//!     [`EmbeddedRgb`](termco::EmbeddedRgb),
//!     [`GrayGradient`](termco::GrayGradient),
//!     [`EightBitColor`](termco::EightBitColor), [`Rgb`](termco::Rgb), as well
//!     as the wrapper [`Colorant`](termco::Colorant).
//!   * The [`style`] module defines **terminal [`Style`](style::Style)s** as a
//!     text [`FormatUpdate`](style::FormatUpdate) combined with foreground and
//!     background [`Colorant`](termco::Colorant)s. It also defines
//!     [`Layer`](style::Layer) to distinguish between the two colors and
//!     [`Fidelity`](style::Fidelity) to represent a terminal's styling
//!     capabilities.
//!   * [`Translator`] implements **translation between color formats**. To
//!     ensure high quality results, its preferred algorithms leverage the
//!     perceptually uniform Oklab/Oklch color space. For conversion to the 16
//!     ANSI colors, it also reqires the terminal's current color
//!     [`Theme`](theme::Theme).
#![cfg_attr(
    feature = "gamut",
    doc = "  * The optional [`gamut`] and [`spectrum`] submodules enable the traversal
    of **color space gamuts** and the **human visual gamut**, respectively."
)]
#![cfg_attr(
    not(feature = "gamut"),
    doc = "  * The optional `gamut` and `spectrum` submodules enable the traversal
    of **color space gamuts** and the **human visual gamut**, respectively."
)]
//!     The
//!     [plot.py](https://github.com/apparebit/prettypretty/blob/main/prettypretty/plot.py)
//!     and
//!     [viz3d.py](https://github.com/apparebit/prettypretty/blob/main/prettypretty/viz3d.py)
//!     scripts leverage the additional functionality for generating helpful
//!     color visualizations.
//!
//!
//! ## 2. One-Two-Three: Styles!
//!
//! Prettypretty's three-step workflow for awesome terminal styles works like
//! this.
//!
//! ### i. Assemble Your Styles
//!
//! First, assemble your application's styles by modifying the empty
//! [`Style::default()`](style::Style::default).
//!
//! ```
//! # use std::io::Result;
//! # use prettypretty::style::Style;
//! # use prettypretty::termco::Rgb;
//! # fn main() -> Result<()> {
//! // 1. Assemble application styles
//! let chic = Style::default()
//!     .bold()
//!     .underlined()
//!     .with_foreground(Rgb::new(215, 40, 39));
//! # Ok(())
//! # }
//! ```
//!
//! [`Style::with_foreground`](style::Style::with_foreground) and
//! [`Style::with_background`](style::Style::with_background) accept any of
//! prettypretty's color representations, i.e.,
//! [`AnsiColor`](termco::AnsiColor), [`EmbeddedRgb`](termco::EmbeddedRgb),
//! [`GrayGradient`](termco::GrayGradient),
//! [`EightBitColor`](termco::EightBitColor), [`Rgb`](termco::Rgb) or
//! high-resolution [`Color`].
//!
//!
//! ### ii. Adjust Your Styles
//!
//! Second, determine the terminal's current color theme with
//! [`Theme::query`](theme::Theme::query) and its color support with
//! [`Fidelity::from_environment`](style::Fidelity::from_environment).
//!
//! ```no_run
//! # use std::io::Result;
//! # use prettypretty::style::{Fidelity, Style};
//! # use prettypretty::termco::Rgb;
//! # use prettypretty::theme::{Theme, VGA_COLORS};
//! # use prettytty::Connection;
//! # use prettytty::opt::Options;
//! # fn main() -> Result<()> {
//! # #[cfg(not(target_family = "windows"))]
//! # {
//! # let chic = Style::default().bold().underlined().with_foreground(Rgb::new(215, 40, 39));
//! // 2a. Determine terminal's color support and theme
//! let options = Options::with_log();
//! let (has_tty, theme) = match Connection::with_options(options) {
//!     Ok(tty) => (true, Theme::query(&tty)?),
//!     Err(_) => (false, VGA_COLORS),
//! };
//! let fidelity = Fidelity::from_environment(has_tty);
//! # }
//! # Ok(())
//! # }
//! ```
//!
//! Use the `theme` to instantiate a [`Translator`](trans::Translator), which
//! specializes in complex color conversions and then adjust your application's
//! styles to the current color theme and fidelity.
//! [`Style::cap`](style::Style::cap) puts a cap on styles with the help of
//! [`Translator::cap`](Translator::cap), which takes care of colors.
//!
//! ```no_run
//! # use std::io::Result;
//! # use prettypretty::{OkVersion, Translator};
//! # use prettypretty::style::{Fidelity, Style};
//! # use prettypretty::termco::Rgb;
//! # use prettypretty::theme::{Theme, VGA_COLORS};
//! # use prettytty::Connection;
//! # use prettytty::opt::Options;
//! # fn main() -> Result<()> {
//! # #[cfg(not(target_family = "windows"))]
//! # {
//! # let chic = Style::default().bold().underlined().with_foreground(Rgb::new(215, 40, 39));
//! # let options = Options::with_log();
//! # let (has_tty, theme) = match Connection::with_options(options) {
//! #     Ok(tty) => (true, Theme::query(&tty)?),
//! #     Err(_) => (false, VGA_COLORS),
//! # };
//! # let fidelity = Fidelity::from_environment(has_tty);
//! // 2b. Actually adjust styles
//! let translator = Translator::new(OkVersion::Revised, theme);
//! let effective_chic = &chic.cap(fidelity, &translator);
//! # }
//! # Ok(())
//! # }
//! ```
//!
//! ### iii. Apply Your Styles
//!
//! Third, to apply a style, just write its display. To undo the style again,
//! just write the negation's display.
//!
//! ```no_run
//! # use std::io::Result;
//! # use prettypretty::{OkVersion, Translator};
//! # use prettypretty::style::{Fidelity, Style};
//! # use prettypretty::termco::Rgb;
//! # use prettypretty::theme::{Theme, VGA_COLORS};
//! # use prettytty::Connection;
//! # use prettytty::opt::Options;
//! # fn main() -> Result<()> {
//! # #[cfg(not(target_family = "windows"))]
//! # {
//! # let chic = Style::default().bold().underlined().with_foreground(Rgb::new(215, 40, 39));
//! # let options = Options::with_log();
//! # let (has_tty, theme) = match Connection::with_options(options) {
//! #     Ok(tty) => (true, Theme::query(&tty)?),
//! #     Err(_) => (false, VGA_COLORS),
//! # };
//! # let fidelity = Fidelity::from_environment(has_tty);
//! # let translator = Translator::new(OkVersion::Revised, theme);
//! # let effective_chic = &chic.cap(fidelity, &translator);
//! // 3. Apply and revert styles
//! println!("{}Wow!{}", effective_chic, -effective_chic);
//! # }
//! # Ok(())
//! # }
//! ```
//! And the terminal exclaims:
//! <img style="display: inline-block; vertical-align: top"
//! src="https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/wow.png"
//!      alt="wow!" width="44"> 🎉
//!
//!
//! ## 3. Optional Features
//!
//! Prettypretty supports four feature flags:
//!
//!   - **`f64`** selects the eponymous type as floating point type [`Float`]
//!     and `u64` as [`Bits`] instead of `f32` as [`Float`] and `u32` as
//!     [`Bits`]. This feature is enabled by default.
//!   - **`tty`** controls [`Theme::query`](theme::Theme::query) and its
//!     implementation with the [prettytty](https://crates.io/crates/prettytty)
//!     terminal crate. This feature is enabled by default.
//!   - **`gamut`** controls support for tracing the boundaries of color spaces
//!     (`mod gamut`, `ColorSpace::gamut`) and the human visual gamut (`mod
//!     spectrum`). This feature is disabled by default.
//!   - **`pyffi`** controls prettypretty's Python integration through
//!     [PyO3](https://pyo3.rs/). This feature is disabled by default.
//!
//! Prettypretty's Python extension module is built with
//! [Maturin](https://www.maturin.rs), PyO3's dedicated build tool. Since Python
//! packages typically come with "batteries included," the `gamut` feature is
//! also enabled when building the Python extension module. However, the `tty`
//! feature is disabled, and prettypretty's Python package includes its own
//! terminal abstraction.
//!
//! Throughout the API documentation, items that are only available in Rust are
//! decorated with <i class=rust-only>Rust only!</i>.
#![cfg_attr(
    feature = "pyffi",
    doc = "Items that are only available in Python are decorated with <i
    class=python-only>Python only!</i>."
)]
//! Similarly, items only available with the `tty` feature are decorated with <i
//! class=tty-only>TTY only!</i> and items only available with the `gamut`
//! feature are decorated with <i class=gamut-only>Gamut only!</i>.
//!
//!
//! ## 4. Acknowledgements
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
pub mod termco;
pub mod theme;
mod trans;
mod util;

#[cfg(feature = "gamut")]
mod cie;
#[cfg(feature = "gamut")]
pub mod gamut {
    //! Optional module implementing the traversal of RGB gamut boundaries with
    //! [`ColorSpace::gamut`](crate::ColorSpace).
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
pub use trans::Translator;

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
    let modstyle_name = format!("{}.style", modcolor_name);
    let modtermco_name = format!("{}.termco", modcolor_name);
    let modtheme_name = format!("{}.theme", modcolor_name);

    // -------------------------------------------------------------------------- color
    m.add_function(wrap_pyfunction!(close_enough, m)?)?;

    m.add_class::<Color>()?;
    m.add_class::<ColorSpace>()?;
    m.add_class::<HueInterpolation>()?;
    m.add_class::<Interpolator>()?;
    m.add_class::<OkVersion>()?;
    m.add_class::<Translator>()?;

    // -------------------------------------------------------------------- color.style
    let modstyle = PyModule::new(m.py(), "style")?;
    modstyle.add("__package__", modcolor_name)?;
    modstyle.add_class::<style::Attribute>()?;
    modstyle.add_class::<style::AttributeIter>()?;
    modstyle.add_class::<style::Fidelity>()?;
    modstyle.add_class::<style::Format>()?;
    modstyle.add_class::<style::FormatUpdate>()?;
    modstyle.add_class::<style::Layer>()?;
    modstyle.add_class::<style::Style>()?;
    m.add_submodule(&modstyle)?;

    // Only change __name__ attribute after submodule has been added.
    modstyle.setattr("__name__", &modstyle_name)?;

    // ------------------------------------------------------------------- color.termco
    let modtermco = PyModule::new(m.py(), "termco")?;
    modtermco.add("__package__", modcolor_name)?;
    modtermco.add_class::<termco::AnsiColor>()?;
    modtermco.add_class::<termco::AnsiColorIterator>()?;
    modtermco.add_class::<termco::Colorant>()?;
    modtermco.add_class::<termco::EightBitColor>()?;
    modtermco.add_class::<termco::EmbeddedRgb>()?;
    modtermco.add_class::<termco::GrayGradient>()?;
    modtermco.add_class::<termco::Rgb>()?;
    m.add_submodule(&modtermco)?;

    // Only change __name__ attribute after submodule has been added.
    modtermco.setattr("__name__", &modtermco_name)?;

    // -------------------------------------------------------------------- color.theme
    let modtheme = PyModule::new(m.py(), "theme")?;
    modtheme.add("__package__", modcolor_name)?;
    modtheme.add_class::<theme::Theme>()?;
    modtheme.add_class::<theme::ThemeEntry>()?;
    modtheme.add_class::<theme::ThemeEntryIterator>()?;
    modtheme.add("VGA_COLORS", theme::VGA_COLORS)?;
    m.add_submodule(&modtheme)?;

    // Only change __name__ attribute after submodule has been added.
    modtheme.setattr("__name__", &modtheme_name)?;

    // -------------------------------------------------------------------- sys.modules
    // Patch sys.modules
    //let sys = PyModule::import_bound(m.py(), "sys")?;
    let py_modules: Bound<'_, PyDict> = PyModule::import(m.py(), "sys")?
        .getattr("modules")?
        .downcast_into()?;
    py_modules.set_item(&modstyle_name, modstyle)?;
    py_modules.set_item(&modtermco_name, modtermco)?;
    py_modules.set_item(&modtheme_name, modtheme)?;

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
    let modgamut = PyModule::new(m.py(), "gamut")?;
    modgamut.add("__package__", modcolor_name)?;
    modgamut.add_class::<gamut::GamutTraversal>()?;
    modgamut.add_class::<gamut::GamutTraversalStep>()?;
    m.add_submodule(&modgamut)?;

    // Only change __name__ attribute after submodule has been added.
    modgamut.setattr("__name__", &modgamut_name)?;

    // ----------------------------------------------------------------- color.spectrum
    let modspectrum = PyModule::new(m.py(), "spectrum")?;
    modspectrum.add("__package__", modcolor_name)?;
    modspectrum.add(
        "CIE_ILLUMINANT_D50",
        spectrum::Illuminant::new(Box::new(spectrum::CIE_ILLUMINANT_D50.clone())
            as Box<dyn spectrum::SpectralDistribution<Value = Float> + Send + Sync>),
    )?;
    modspectrum.add(
        "CIE_ILLUMINANT_D65",
        spectrum::Illuminant::new(Box::new(spectrum::CIE_ILLUMINANT_D65.clone())
            as Box<dyn spectrum::SpectralDistribution<Value = Float> + Send + Sync>),
    )?;
    modspectrum.add(
        "CIE_ILLUMINANT_E",
        spectrum::Illuminant::new(Box::new(spectrum::CIE_ILLUMINANT_E.clone())
            as Box<dyn spectrum::SpectralDistribution<Value = Float> + Send + Sync>),
    )?;
    modspectrum.add(
        "CIE_OBSERVER_2DEG_1931",
        spectrum::CIE_OBSERVER_2DEG_1931.clone(),
    )?;
    modspectrum.add(
        "CIE_OBSERVER_10DEG_1964",
        spectrum::CIE_OBSERVER_10DEG_1964.clone(),
    )?;
    modspectrum.add("ONE_NANOMETER", spectrum::ONE_NANOMETER)?;
    modspectrum.add_class::<spectrum::Illuminant>()?;
    modspectrum.add_class::<spectrum::IlluminatedObserver>()?;
    modspectrum.add_class::<spectrum::Observer>()?;
    modspectrum.add_class::<spectrum::SpectrumTraversal>()?;
    m.add_submodule(&modspectrum)?;

    // Only change __name__ attribute after submodule has been added.
    modspectrum.setattr("__name__", &modspectrum_name)?;

    // -------------------------------------------------------- color.spectrum.observer
    let modobserver = PyModule::new(m.py(), "std_observer")?;
    modobserver.add("__package__", modcolor_name)?;
    modobserver.add_function(wrap_pyfunction!(spectrum::std_observer::x, m)?)?;
    modobserver.add_function(wrap_pyfunction!(spectrum::std_observer::y, m)?)?;
    modobserver.add_function(wrap_pyfunction!(spectrum::std_observer::z, m)?)?;
    modspectrum.add_submodule(&modobserver)?;
    modobserver.setattr("__name__", &modobserver_name)?;

    // -------------------------------------------------------------------- sys.modules
    let py_modules: Bound<'_, PyDict> = PyModule::import(m.py(), "sys")?
        .getattr("modules")?
        .downcast_into()?;

    py_modules.set_item(&modgamut_name, modgamut)?;
    py_modules.set_item(&modspectrum_name, modspectrum)?;
    py_modules.set_item(&modobserver_name, modobserver)?;

    Ok(())
}
