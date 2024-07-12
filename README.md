# Pretty 🌸 Pretty

[![Run Tests](https://github.com/apparebit/prettypretty/actions/workflows/ci.yml/badge.svg)](https://github.com/apparebit/prettypretty/actions/workflows/ci.yml)
[![Publish to GitHub Pages](https://github.com/apparebit/prettypretty/actions/workflows/gh-pages.yml/badge.svg)](https://github.com/apparebit/prettypretty/actions/workflows/gh-pages.yml)
[![Build Wheels](https://github.com/apparebit/prettypretty/actions/workflows/dist.yml/badge.svg)](https://github.com/apparebit/prettypretty/actions/workflows/dist.yml)

\[ [**Documentation**](https://docs.rs/prettypretty/latest/prettypretty/)
 | [**Rust Crate**](https://crates.io/crates/prettypretty)
 | [**Python Package**](https://pypi.org/project/prettypretty/)
 | [**Repository**](https://github.com/apparebit/prettypretty)
\]

🎖️ As featured on [Real Python #211](https://realpython.com/podcasts/rpp/211/)

Prettypretty is a Rust library with optional Python integration that brings
2020s color science to 1970s terminals for building awesome terminal user
interfaces (TUIs). The intended benefits are twofold:

  * You get to design and build the TUI with all the expressivity and
    convenience of high-resolution color and [color
    spaces](https://lab.ardov.me/spaces-3d), including the perceptually uniform
    [Oklab](https://bottosson.github.io/posts/oklab/) whether in Cartesian or
    polar form, with original or [revised
    lightness](https://bottosson.github.io/posts/colorpicker/#intermission---a-new-lightness-estimate-for-oklab).
  * Prettypretty takes care of reconciling the intended appearance with the
    capabilities of the terminal, the current runtime context including light or
    dark mode, and the user's preferences, whether they lean
    [FORCE_COLOR](https://force-color.org) or [NO_COLOR](https://no-color.org).

To make that possible, prettypretty provides simple abstractions for terminal
and high-resolution colors alike, facilitates seamless conversion between them
and common color spaces, and implements state-of-the-art algorithms for
[gamut-mapping](https://www.w3.org/TR/css-color-4/#gamut-mapping), [color
interpolation](https://www.w3.org/TR/css-color-4/#interpolation), [perceptual
contrast](https://github.com/Myndex/apca-w3), as well as its own hue- and
lightness-based downsampling for optimal selection of ANSI colors.


## Python Integration

The optional Python integration is enabled with the `pyffi` feature flag and
relies on [PyO3](https://pyo3.rs/v0.22.0/) and [Maturin](https://www.maturin.rs)
for building an extension module with the same functionality. Only where the
Rust library uses trait implementations, the Python module [uses dedicated
methods](https://github.com/apparebit/prettypretty/blob/main/prettypretty/color.pyi).
Also, where the Rust library currently is BYO(T)IO, that is, bring your own
(terminal) I/O, the Python library comes with a powerful terminal abstraction
that makes, say, querying the terminal [for the current color
theme](https://github.com/apparebit/prettypretty/blob/61fb6d7c364c0d083e1073ead146834c1e0bc56d/prettypretty/terminal.py#L1039)
a breeze.

The [Python documentation](https://apparebit.github.io/prettypretty/python/)
covers the functionality that currently is Python-only. Over time, I expect to
port those features to Rust as well.


## Minimum Supported Rust and Python Versions

Prettypretty leverages both programming languages to their fullest and hence
requires relatively recent versions:

  * According to [cargo-msrv](https://github.com/foresterre/cargo-msrv), **the
    minimum supported Rust version is 1.77.2**.
  * According to [vermin](https://github.com/netromdk/vermin), **the minimum
    supported Python version is 3.11.0.**

I expect that, as the project matures, the version lag between minimum and
latest versions will grow, as it should.


## Scripts

Besides the [documentation](https://apparebit.github.io/prettypretty/), a good
starting point for familiarizing yourself with prettypretty are the scripts:

  * [prettypretty.progress](https://github.com/apparebit/prettypretty/blob/main/prettypretty/progress.py)
    illustrates the library's use on the example of a progress bar in less than
    100 lines of Python. The finished progress bar is shown below for both light
    and dark themes.

    <img src="https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/progress-bar-light.png"
         alt="a complete, green progress bar under light mode" width=293>
    <img src="https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/progress-bar-dark.png"
         alt="a complete, green progress bar under dark mode" width=298>

  * [prettypretty.plot](https://github.com/apparebit/prettypretty/blob/main/prettypretty/plot.py)
    charts colors on the chroma/hue plane of Oklab, if you don't feed it colors
    defaulting to your terminal's current color scheme. Here's the one for the
    basic theme in Apple's Terminal.app:

    <img src="https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/terminal.app-basic.svg"
         alt="colors from the basic theme for Apple's Terminal.app in Oklch" width=300px>

  * [prettypretty.grid](https://github.com/apparebit/prettypretty/blob/main/prettypretty/grid.py)
    visualizes perceptual contrast and color downsampling strategies,
    exhaustively for the 6x6x6 RGB cube embedded in 8-bit color and selectively
    for 32x32 slices through the much bigger 24-bit RGB cube.

    <img src="https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/rgb6-background.png"
         alt="a grid visualizing the 6x6x6 embedded RGB cube" width=300px>


## Acknowledgements

I wrote much of prettypretty over a two-month period in 2024. Twice. I first
implemented the core color routines in Python and then I did so again in Rust.
At this point, only the Rust version survives. But Python remains a tier-1
runtime target for prettypretty. Two things really helped with getting this
project started. First, I had been toying with different approaches to terminal
styles for a while and knew what I was looking for. Second, I benefitted
tremendously from [Lea Verou](http://lea.verou.me/)'s and [Chris
Lilley](https://svgees.us/)'s work on the [Color.js](https://colorjs.io) library
and [CSS Color 4](https://www.w3.org/TR/css-color-4/) specification.
Prettypretty directly reuses Color.js' formulae for conversion between color
spaces and implements several CSS Color 4 algorithms.

---

Copyright 2024 Robert Grimm. The code in this repository has been released as
open source under the [Apache
2.0](https://github.com/apparebit/prettypretty/blob/main/LICENSE) license.
