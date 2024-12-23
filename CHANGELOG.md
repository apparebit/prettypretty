# Changelog

## v0.11.0 (2024-11-03)

### A Seemingly Simpler, Yet More Powerful API

This version introduces significant new functionality while also reorganizing
the already existing functionality. To significantly reduce the cognitive
overhead of using prettypretty, the public API now is modularized. The primary
modules and their main types are:

  - `prettypretty` provides high-resolution colors through
    [`ColorSpace`](https://apparebit.github.io/prettypretty/prettypretty/enum.ColorSpace.html)
    and [`Color`](https://apparebit.github.io/prettypretty/prettypretty/struct.Color.html)
  - `prettypretty::style` abstracts over terminal styles with
    [`Style`](https://apparebit.github.io/prettypretty/prettypretty/style/struct.Style.html)
    and terminals' substantially different color representations with
    [`Colorant`](https://apparebit.github.io/prettypretty/prettypretty/style/enum.Colorant.html).
  - `prettypretty::trans` defines
    [`Translator`](https://apparebit.github.io/prettypretty/prettypretty/trans/struct.Translator.html)
    née `Sampler` for translating between the color representations.
  - `prettypretty::theme` uses
    [`Theme`](https://apparebit.github.io/prettypretty/prettypretty/theme/struct.Theme.html)
    to represent a terminal's current color theme.
  - `prettypretty::cmd` is a small library of further
    [`Command`](https://apparebit.github.io/prettypretty/prettypretty/cmd/trait.Command.html)s
    that let you control aspects other than style, including window title,
    terminal screen, and cursor position.
  - `prettypretty::term` provides direct access to the terminal device with
    [`Terminal`](https://apparebit.github.io/prettypretty/prettypretty/term/struct.Terminal.html)
    and parses UTF-8 and ANSI escape sequences with
    [`VtScanner`](https://apparebit.github.io/prettypretty/prettypretty/term/struct.VtScanner.html).
    `Terminal` is supported on Unix and Windows, though the latter version is
    largely untested.

Since `cmd` and `term` overlap with existing Rust terminal crates such as
[`crossterm`](https://github.com/crossterm-rs/crossterm), both modules are
optional. THe corresponding Cargo feature is `term`. It is enabled by default.

Prettypretty also has a utility module:

  - `prettypretty::error` is a utility module defining prettypretty's error
    types.

Finally, two more optional modules support color gamuts and the human visual
gamut. Both modules are disabled by default in Rust and enabled by default in
Python, with the latter adhering to that ecosystem's "batteries included"
approach.

  - `prettypretty::gamut` is a utility module defining an iterator for
    traversing color space gamuts. It is enabled with the `gamut` feature.
  - `prettypretty::spectrum` adds support for spectral distributions and their
    iterators, notably the CIE
    [D50](https://apparebit.github.io/prettypretty/prettypretty/spectrum/constant.CIE_ILLUMINANT_D50.html),
    [D65](https://apparebit.github.io/prettypretty/prettypretty/spectrum/constant.CIE_ILLUMINANT_D65.html),
    and
    [E](https://apparebit.github.io/prettypretty/prettypretty/spectrum/constant.CIE_ILLUMINANT_E.html)
    illuminants as well as the [2º
    (1931)](https://apparebit.github.io/prettypretty/prettypretty/spectrum/constant.CIE_OBSERVER_2DEG_1931.html)
    and [10º
    (1964)](https://apparebit.github.io/prettypretty/prettypretty/spectrum/constant.CIE_OBSERVER_10DEG_1964.html)
    observers. It too is enabled with the `gamut` feature.


### New Functionality

In addition to the entirely new `gamut`, `spectrum`, and `term` modules, new
features include:

  * [`Style`](https://apparebit.github.io/prettypretty/prettypretty/style/struct.Style.html)
    represents a terminal style and
    [`Colorant`](https://apparebit.github.io/prettypretty/prettypretty/style/enum.Colorant.html)
    represents a color, with the latter replacing `TerminalColor`.
  * [`ColorSpace::XyzD50`](https://apparebit.github.io/prettypretty/prettypretty/enum.ColorSpace.html#variant.XyzD50)
    adds support for the XYZ color space with a D50 illuminant. Chromatic
    adaptation uses the (linear) Bradford method.
  * [`Color::hue_chroma`](https://apparebit.github.io/prettypretty/prettypretty/struct.Color.html#method.hue_chroma)
    and
    [`Color::xy_chromaticity`](https://apparebit.github.io/prettypretty/prettypretty/struct.Color.html#method.xy_chromaticity)
    map a color's three dimensions down to two.
  * The output of the
    [`plot.py`](https://github.com/apparebit/prettypretty/blob/main/prettypretty/plot.py)
    script has been significantly improved; it now visualizes lightness as well,
    with regular and bright colors grouped together.
  * The new
    [`viz3d.py`](https://github.com/apparebit/prettypretty/blob/main/prettypretty/viz3d.py)
    script uses the `spectrum` module to generate a 3D mesh of the human visual
    gamut.

With these features, prettypretty is rapidly approaching feature-complete
status.


## v0.10.0 (2024-07-12)

### New Features

  * Like `Color::from_24bit`, the `rgb` macro creates sRGB colors from 24-bit
    integer coordinates. Unlike the method, the macro can appear in const
    expressions.


### Changes

  * Rename `Sampler::adjust` to `Sampler::cap`
  * Improve documentation with many small edits, new overview and summary,
    and disabling the `pyffi` feature flag on docs.rs


### Bug Fixes

  * `Fidelity::from_environment` now correctly classifies iTerm 3.x as having
    full fidelity.


## v0.9.0 (2024-07-09)

As it turns out, v0.9.0 is the new v0.1.0. At least, it feels that way writing
up the changes. Thankfully, minor version numbers are not limited to single
decimal digits…

### Unified Codebase

This release combines the Rust and Python versions of prettypretty into one
coherent codebase. The core color functionality is now implemented in Rust only
and, with the help of [PyO3](https://pyo3.rs/v0.22.0/) and
[Maturin](https://www.maturin.rs), exposed to Python as an extension module.
[Having published](https://dl.acm.org/doi/10.1145/1297027.1297030) several
[research papers](https://dl.acm.org/doi/10.1145/1640089.1640105) on [foreign
function interfaces](https://dl.acm.org/doi/10.1145/1806596.1806601), I can
state with some authority that PyO3's integration between Rust and Python is
nothing but impressive. But its macro-based implementation is brittle and often
gets in the way of building a pure Rust library and an extension module from the
same codebase. The only viable workarounds were to duplicate some methods and to
spread methods over up to three `impl` blocks. But hey, it works really well! 🎉

### Translating High-Resolution to ANSI Colors

`Sampler` is the new one-stop abstraction for translating colors, from terminal
to high-resolution, high-resolution to terminal, and terminal to terminal
colors. It makes translation between color representation more convenient and it
is more powerful.

Notably, `Sampler::to_ansi_hue_lightness` implements a new algorithm for
converting high-resolution colors to the 16 ANSI colors. For color themes that
roughly observe the semantics of ANSI colors (e.g., when sorted by hue, red and
bright red come before yellow and bright yellow, which in turn come before green
and bright green and so on), it produces more accurate results by first using
hue to select a pair of regular and bright ANSI colors and then lightness to
select the best match. For color themes that violate the new algorithm's
invariants, `Sampler::to_ansi` transparently and automatically falls back to
brute force search for the closest color.

### Other Major Improvements

Prettypretty now supports more color spaces:

  - Oklrab and Oklrch [revise Oklab's lightness
    estimate](https://bottosson.github.io/posts/colorpicker/#intermission---a-new-lightness-estimate-for-oklab)
    to be as uniform as CIELAB's equivalent and proven component.
  - [Rec. 2020](https://en.wikipedia.org/wiki/Rec._2020) is a popular but purely
    aspirational color space.

High-resolution colors are simpler to use and support more operations:

  - [`Color`](https://apparebit.github.io/prettypretty/prettypretty/struct.Color.html)
    now is *immutable*. While immutability may slightly increase prettypretty's
    memory overhead, a mutable color abstraction in Rust should probably use
    interior mutability and heap-allocated color coordinates. The potential for
    saving 16 or 24 bytes here or there does not seem to justify the attendant
    complexity. Immutability it is.
  - [`Color::interpolate`](https://apparebit.github.io/prettypretty/prettypretty/struct.Color.html#method.interpolate)
    implements [CSS Color 4](https://www.w3.org/TR/css-color-4/#interpolation)'s
    interpolation algorithm, including the specification's rather elaborate
    rules for carrying forward missing components and for selecting the hue
    interpolation strategy.
  - [`Color::lighten`](https://apparebit.github.io/prettypretty/prettypretty/struct.Color.html#method.lighten)
    and
    [`Color::darken`](https://apparebit.github.io/prettypretty/prettypretty/struct.Color.html#method.darken)
    keep colors in line with your moods.

Terminal colors have a more coherent model:

  - `TerminalColor` combines the different kinds of colors supported by
    terminals, i.e., the default foreground and background colors, ANSI colors,
    8-bit colors including the 6x6x6 embedded RGB cube and the 24-step gray
    gradient, as well as 24-bit "true" colors.
  - `DefaultColor` correctly models that there are two, albeit context-sensitive
    default colors, one for the foreground and one for the background.


