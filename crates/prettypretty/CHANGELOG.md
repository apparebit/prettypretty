# Changelog


## Prettypretty v0.11.0 (2025-01-31)

### A Simpler, More Powerful API

To reduce the cognitive overhead of using prettypretty, its functionality has
been reorganized into modules. The primary modules and abstractions now are:

  * `prettypretty` provides high-resolution colors through
    [`ColorSpace`](https://apparebit.github.io/prettypretty/prettypretty/enum.ColorSpace.html)
    and
    [`Color`](https://apparebit.github.io/prettypretty/prettypretty/struct.Color.html).
    It also defines
    [`Translator`](https://apparebit.github.io/prettypretty/prettypretty/trans/struct.Translator.html)
    for translating between color representations.
  * `prettypretty::termco` offers a choice of terminal color representations as
    well as
    [`Colorant`](https://apparebit.github.io/prettypretty/prettypretty/termco/enum.Colorant.html)
    as wrapper that accommodates all of them including high-resolution colors.
  * `prettypretty::style` implements terminal
    [`Style`](https://apparebit.github.io/prettypretty/prettypretty/style/struct.Style.html)s
    as a
    [`FormatUpdate`](https://apparebit.github.io/prettypretty/prettypretty/style/struct.FormatUpdate.html),
    a foreground
    [`Colorant`](https://apparebit.github.io/prettypretty/prettypretty/termco/enum.Colorant.html),
    and a background
    [`Colorant`](https://apparebit.github.io/prettypretty/prettypretty/termco/enum.Colorant.html).
  * `prettypretty::theme` represents a terminal's color
    [`Theme`](https://apparebit.github.io/prettypretty/prettypretty/theme/struct.Theme.html).
    The `tty` feature enables prettypretty's integration with the
    [prettytty](https://crates.io/crates/prettytty) crate, which adds the
    ability to query the terminal for its theme.
  * `prettypretty::error` defines the crate's error types.

Prettypretty always includes the above modules. The `gamut` feature enables two
more:

  - `prettypretty::gamut` is a utility module defining an iterator for
    traversing color space gamuts.
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
    observers.

Since Python packages usually ship with "batteries included," prettypretty's
Python package always includes these two optional modules.


### New Functionality

Notable new features, beyond the already mentioned
[`Colorant`](https://apparebit.github.io/prettypretty/prettypretty/termco/enum.Colorant.html),
[`Style`](https://apparebit.github.io/prettypretty/prettypretty/style/struct.Style.html),
and integration with [prettytty](https://crates.io/crates/prettytty), include:

  * [`ColorSpace::XyzD50`](https://apparebit.github.io/prettypretty/prettypretty/enum.ColorSpace.html#variant.XyzD50)
    adds support for the XYZ color space with a D50 illuminant. Chromatic
    adaptation uses the (linear) Bradford method.
  * [`Color::hue_chroma`](https://apparebit.github.io/prettypretty/prettypretty/struct.Color.html#method.hue_chroma)
    and
    [`Color::xy_chromaticity`](https://apparebit.github.io/prettypretty/prettypretty/struct.Color.html#method.xy_chromaticity)
    map a color's three dimensions down to two.
  * The significantly improved
    [`plot.py`](https://github.com/apparebit/prettypretty/blob/main/prettypretty/plot.py)
    script adds support for visualizing arbitrary CSS colors read from a file,
    Oklch's lightness values (with regular and bright theme colors grouped
    together), as well as chromaticity diagrams.
  * The new
    [`viz3d.py`](https://github.com/apparebit/prettypretty/blob/main/prettypretty/viz3d.py)
    script uses the `spectrum` module to generate a 3D mesh of the human visual
    gamut. The mesh forms a closed volume using only triangles, though most of
    them are generated in pairs representing quadrilaterals. A stride of 1
    results in a smooth mesh, but also generates too many vertexes at the
    extrema. Hence post-processing of the mesh is recommended.


## Prettypretty v0.10.0 (2024-07-12)

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


## Prettypretty v0.9.0 (2024-07-09)

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


