# Pretty 🌸 (Pre)tty

[![Run Tests, Build Wheels, & Publish to PyPI](https://github.com/apparebit/prettypretty/actions/workflows/ci.yml/badge.svg)](https://github.com/apparebit/prettypretty/actions/workflows/ci.yml)
[![Publish to GitHub Pages](https://github.com/apparebit/prettypretty/actions/workflows/gh-pages.yml/badge.svg)](https://github.com/apparebit/prettypretty/actions/workflows/gh-pages.yml)

\[  [**Documentation**](https://docs.rs/prettypretty/latest/prettypretty/)
| [**Rust Crate**](https://crates.io/crates/prettytty)
| [**Python Package**](https://pypi.org/project/prettypretty/)
| [**Repository**](https://github.com/apparebit/prettypretty)
\]

🎖️ As featured on [Real Python #211](https://realpython.com/podcasts/rpp/211/)

🎖️ Inspired [iTerm2's color preferences](https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/iterm2-color-preferences.jpg)

Prettypretty is a Rust library with optional Python integration that applies
**2020s color science to 1970s terminals** to facilitate scalable user
interfaces. However, instead of progressive enhancement, it primarily relies on
graceful degradation: (1) You declare high-resolution styles, (2) prettypretty
adjusts them to terminal capabilities and user preferences at program startup,
and (3) your app uses the resulting styles at will.

Of course, prettypretty integrates with prettytty, mostly to use
[`Theme::query`] for **querying the terminal for its current color theme**.
Said color theme then informs color conversions in step two above. But the
integration also is entirely optional, controlled by the `tty` feature, and
pretty small, comprising about 90 lines of code, 50 of which used for
reimplementing that query function two more times to help determine the most
performant approach.

As far as colors are concerned, prettypretty comes with all the expressivity and
convenience of **high-resolution, floating point colors and [color
spaces](https://lab.ardov.me/spaces-3d)**, including the perceptually uniform
[Oklab](https://bottosson.github.io/posts/oklab/), whether in Cartesian or polar
form, with original or [revised
lightness](https://bottosson.github.io/posts/colorpicker/#intermission---a-new-lightness-estimate-for-oklab).
It further implements state-of-the-art algorithms for
[gamut-mapping](https://www.w3.org/TR/css-color-4/#gamut-mapping), [color
interpolation](https://www.w3.org/TR/css-color-4/#interpolation), [perceptual
contrast](https://github.com/Myndex/apca-w3), as well as its own hue- and
lightness-based downsampling for optimal selection of ANSI colors.


## Python Integration

The optional Python integration is enabled with the `pyffi` feature flag and
relies on [PyO3](https://pyo3.rs/v0.22.0/) and [Maturin](https://www.maturin.rs)
for building an extension module with the same functionality. Only where the
Rust library uses trait implementations, the Python module [uses dedicated
methods](https://github.com/apparebit/prettypretty/blob/main/prettypretty/color/__init__.pyi).
While prettytty takes care of terminal access for Rust, the Python version of
prettypretty has its own terminal abstraction, with its own Pythonic interface.


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
