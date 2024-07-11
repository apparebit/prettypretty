# Changelog

## --

### New Features

  * Like [`Color::from_24bit`], the `rgb` macro creates sRGB colors from 24-bit
    integer coordinates. Unlike the method, the macro can appear in const
    expressions.


### Changes

  * Rename `Sampler::adjust` to [`Sampler::cap`]


### Bug Fixes

  * [`Fidelity::from_environment`] now correctly classifies iTerm 3.x as having
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

[`Sampler`](https://apparebit.github.io/prettypretty/prettypretty/struct.Sampler.html)
is the new one-stop abstraction for translating colors, from terminal to
high-resolution, high-resolution to terminal, and terminal to terminal colors.
It makes translation between color representation more convenient and it is more
powerful.

Notably,
[`Sampler::to_ansi_hue_lightness`](https://apparebit.github.io/prettypretty/prettypretty/struct.Sampler.html#method.to_ansi_hue_lightness)
implements a new algorithm for converting high-resolution colors to the 16 ANSI
colors. For color themes that roughly observe the semantics of ANSI colors
(e.g., when sorted by hue, red and bright red come before yellow and bright
yellow, which in turn come before green and bright green and so on), it produces
more accurate results by first using hue to select a pair of regular and bright
ANSI colors and then lightness to select the best match. For color themes that
violate the new algorithm's invariants,
[`Sampler::to_ansi`](https://apparebit.github.io/prettypretty/prettypretty/struct.Sampler.html#method.to_ansi)
transparently and automatically falls back to brute force search for the closest
color.

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

  - [`TerminalColor`](https://apparebit.github.io/prettypretty/prettypretty/enum.TerminalColor.html)
    combines the different kinds of colors supported by terminals, i.e., the
    default foreground and background colors, ANSI colors, 8-bit colors
    including the 6x6x6 embedded RGB cube and the 24-step gray gradient, as well
    as 24-bit "true" colors.
  - [`DefaultColor`](https://apparebit.github.io/prettypretty/prettypretty/enum.DefaultColor.html)
    correctly models that there are two, albeit context-sensitive default
    colors, one for the foreground and one for the background.


