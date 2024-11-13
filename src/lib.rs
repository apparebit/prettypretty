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
//!     [`GrayGradient`](style::GrayGradient),
//!     [`EightBitColor`](style::EightBitColor), and
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
//!   * The [`cmd`] module provides further control over terminals with a small
//!     library of **commands implemented as ANSI escape code**. They control
//!     not only window title, screen, and cursor position, but also group
//!     content for pasting and batch updates as well as hyperlinks.
//!   * The [`trans`] module's [`Translator`](crate::trans::Translator)
//!     implements **translation between color formats**. To ensure high quality
//!     results, its preferred algorithms leverage the perceptually uniform
//!     Oklab/Oklch color space. For conversion to the 16 ANSI colors, it also
//!     reqires the terminal's current color theme.
//!   * The [`theme`] module's
//!     [`Theme::query_terminal`](crate::theme::Theme::query_terminal) **queries
//!     the terminal for such a theme** and returns a
//!     [`Theme`](crate::theme::Theme) object with the default foreground and
//!     background as well as 16 ANSI colors.
//!   * The optional [`term`] module performs the necessary **terminal I/O**.
//!     Notably, [`Terminal`](crate::term::Terminal) configures the terminal to
//!     use raw mode and to time out reads. Meanwhile,
//!     [`VtScanner`](crate::term::VtScanner) implements the complete state
//!     machine for **parsing ANSI escape sequences**. Both Unix and Windows are
//!     supported, but Windows support is largely untested.
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
//! ## One-Two-Three: Styles!
//!
//! Prettypretty's three-step workflow for awesome terminal styles works like
//! this.
//!
//! ### 1. Declare Your Styles
//!
//! First, you declare your application's styles. If the
//! [`stylist`](style::stylist()) shantaying
//! [`Stylist::et_voila`](style::Stylist::et_voila) is too sassy for you,
//! [`Style::builder`](style::Style::builder) chanting
//! [`Stylist::build`](style::Stylist::build) works just as well.
//!
//! ```no_run
//! # use std::io::{stdout, ErrorKind, IsTerminal, Result};
//! # use prettypretty::{rgb, OkVersion, style::{Fidelity, stylist}};
//! # use prettypretty::theme::Theme;
//! # use prettypretty::trans::Translator;
//! # fn main() -> Result<()> {
//! // 1. Assemble application styles
//! let chic = stylist()
//!     .bold()
//!     .underlined()
//!     .rgb(215, 40, 39)
//!     .fg()
//!     .et_voila();
//! # Ok(())
//! # }
//! ```
//!
//! As illustrated above, you can use
//! [`Stylist::embedded_rgb`](style::Stylist::embedded_rgb),
//! [`Stylist::gray`](style::Stylist::gray), or
//! [`Stylist::rgb`](style::Stylist::rgb) followed by
//! [`Colorist::fg`](style::Colorist::fg),
//! [`Colorist::on`](style::Colorist::on), or
//! [`Colorist::bg`](style::Colorist::bg) to specify an 8-bit or 24-bit terminal
//! color. Alternatively, you can use
//! [`Stylist::foreground`](style::Stylist::foreground) or
//! [`Stylist::background`](style::Stylist::background), which accept any of
//! prettypretty's colors.
//!
//!
//! ### 2. Adjust Your Styles
//!
//! Second, determine the terminal's current color theme with
//! [`Theme::query_terminal`](theme::Theme::query_terminal) and `stdout`'s color
//! support with
//! [`Fidelity::from_environment`](style::Fidelity::from_environment).
//!
//! ```no_run
//! # use std::io::{stdout, ErrorKind, IsTerminal, Result};
//! # use prettypretty::{rgb, OkVersion, style::{Fidelity, stylist}};
//! # use prettypretty::theme::Theme;
//! # use prettypretty::trans::Translator;
//! # fn main() -> Result<()> {
//! # let chic = stylist().bold().underlined().rgb(215, 40, 39).fg().et_voila();
//! // 2a. Determine color theme, stdout's color support
//! let theme = Theme::query_terminal()?;
//! let fidelity = Fidelity::from_environment(stdout().is_terminal());
//! # Ok(())
//! # }
//! ```
//!
//! Use the `theme` to instantiate a [`Translator`](trans::Translator), which
//! specializes in complex color conversions and then adjust your application's
//! styles to the current color theme and fidelity.
//! [`Style::cap`](style::Style::cap) puts a cap on styles with the help of
//! [`Translator::cap`](trans::Translator::cap), which takes care of colors.
//!
//! ```no_run
//! # use std::io::{stdout, ErrorKind, IsTerminal, Result};
//! # use prettypretty::{rgb, OkVersion, style::{Fidelity, stylist}};
//! # use prettypretty::theme::Theme;
//! # use prettypretty::trans::Translator;
//! # fn main() -> Result<()> {
//! # let chic = stylist().bold().underlined().rgb(215, 40, 39).fg().et_voila();
//! # let theme = Theme::query_terminal()?;
//! # let fidelity = Fidelity::from_environment(stdout().is_terminal());
//! // 2b. Adjust fidelity of styles
//! let translator = Translator::new(OkVersion::Revised, theme);
//! let effective_chic = &chic.cap(fidelity, &translator);
//! # Ok(())
//! # }
//! ```
//!
//! ### 3. Apply Your Styles
//!
//! Third, to apply a style, just write its display. To undo the style again,
//! just write the negation's display.
//!
//! ```no_run
//! # use std::io::{stdout, ErrorKind, IsTerminal, Result};
//! # use prettypretty::{rgb, OkVersion, style::{Fidelity, stylist}};
//! # use prettypretty::theme::Theme;
//! # use prettypretty::trans::Translator;
//! # fn main() -> Result<()> {
//! # let chic = stylist().bold().underlined().rgb(215, 40, 39).fg().et_voila();
//! # let theme = Theme::query_terminal()?;
//! # let fidelity = Fidelity::from_environment(stdout().is_terminal());
//! # let translator = Translator::new(OkVersion::Revised, theme);
//! # let effective_chic = &chic.cap(fidelity, &translator);
//! // 3. Apply and revert styles
//! println!("{}Wow!{}", effective_chic, !effective_chic);
//! # Ok(())
//! # }
//! ```
//! And the terminal exclaims:<br>
//! <img style="margin-left: 2em;"
//! src="https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/wow.png"
//!      alt="wow!" width="77">
//!
//! Commands work the same way, too: Just write the display to the terminal, and
//! the terminal is updating. Well, at least in theory. In practice, support for
//! the different ANSI escape sequences varies widely by terminal.
//!
//! [Demicode](https://github.com/apparebit/demicode), my tool for exploring the
//! fixed-width rendering of Unicode, includes the
//! [orchastrate.py](https://github.com/apparebit/demicode/blob/boss/script/orchestrate.py)
//! script written in a combination of Python and AppleScript to automatically
//! collect screenshots for output printed to a dozen or so macOS terminals.
//! That just might be the starting point for a very useful testing tool.
//!
//!
//! ## Feature Flags
//!
//! Prettypretty supports four feature flags:
//!
//!   - `f64` selects the eponymous type as floating point type [`Float`] and
//!     `u64` as [`Bits`] instead of `f32` as [`Float`] and `u32` as [`Bits`].
//!     This feature is enabled by default.
//!   - `term` controls prettypretty's support for low-level protocol processing
//!     by configuring the terminal (Unix only, `mod term`) and parsing ANSI
//!     escape sequences (platform-independent, also `mod term`). This feature
//!     is enabled by default.
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
//! included," the `term` and `gamut` features are also enabled when building
//! the Python extension module.
//!
//! Throughout the API documentation, items that are only available in Rust are
//! decorated with <i class=rust-only>Rust only!</i>.
#![cfg_attr(
    feature = "pyffi",
    doc = "Items that are only available in Python are decorated with <i
    class=python-only>Python only!</i>."
)]
//! Similarly, items only available with the `term` feature are decorated with
//! <i class=term-only>Term only!</i> and the `gamut` feature are decorated
//! with <i class=gamut-only>Gamut only!</i>.
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

pub mod cmd;
mod core;
pub mod error;
mod object;
pub mod style;
#[cfg(feature = "term")]
pub mod term;
pub mod theme;
pub mod trans;
mod util;

#[cfg(feature = "gamut")]
mod cie;
#[cfg(feature = "gamut")]
pub mod gamut {
    //! Optional utility module for traversing RGB gamut boundaries with
    //! [`ColorSpace::gamut`](crate::ColorSpace).  <i class=gamut-only>Gamut
    //! only!</i>
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
    let modterm_name = format!("{}.term", modcolor_name);
    let modtheme_name = format!("{}.theme", modcolor_name);
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
    modstyle.add_class::<style::AnsiColorIterator>()?;
    modstyle.add_class::<style::Colorant>()?;
    modstyle.add_class::<style::Colorist>()?;
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

    // --------------------------------------------------------------------- color.term
    let modterm = PyModule::new_bound(m.py(), "term")?;
    modterm.add("__package__", modcolor_name)?;
    modterm.add_class::<term::Action>()?;
    modterm.add_class::<term::Control>()?;
    modterm.add_class::<term::VtScanner>()?;
    m.add_submodule(&modterm)?;

    // Only change __name__ attribute after submodule has been added.
    modterm.setattr("__name__", &modterm_name)?;

    // -------------------------------------------------------------------- color.theme
    let modtheme = PyModule::new_bound(m.py(), "trans")?;
    modtheme.add("__package__", modcolor_name)?;
    modtheme.add_class::<theme::Theme>()?;
    modtheme.add_class::<theme::ThemeEntry>()?;
    modtheme.add_class::<theme::ThemeEntryIterator>()?;
    modtheme.add("VGA_COLORS", theme::VGA_COLORS)?;
    m.add_submodule(&modtheme)?;

    // Only change __name__ attribute after submodule has been added.
    modtheme.setattr("__name__", &modtheme_name)?;

    // -------------------------------------------------------------------- color.trans
    let modtrans = PyModule::new_bound(m.py(), "trans")?;
    modtrans.add("__package__", modcolor_name)?;
    modtrans.add_class::<trans::Translator>()?;
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
    py_modules.set_item(&modterm_name, modterm)?;
    py_modules.set_item(&modtheme_name, modtheme)?;
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
        spectrum::Illuminant::new(Box::new(spectrum::CIE_ILLUMINANT_D50.clone())
            as Box<dyn spectrum::SpectralDistribution<Value = Float> + Send>),
    )?;
    modspectrum.add(
        "CIE_ILLUMINANT_D65",
        spectrum::Illuminant::new(Box::new(spectrum::CIE_ILLUMINANT_D65.clone())
            as Box<dyn spectrum::SpectralDistribution<Value = Float> + Send>),
    )?;
    modspectrum.add(
        "CIE_ILLUMINANT_E",
        spectrum::Illuminant::new(Box::new(spectrum::CIE_ILLUMINANT_E.clone())
            as Box<dyn spectrum::SpectralDistribution<Value = Float> + Send>),
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
