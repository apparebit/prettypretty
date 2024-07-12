# Installation

Before covering the incantations necessary for actually installing prettypretty,
we do need to cover the requirements. That includes the necessary tools as well
as the minimum supported versions.


## Requirements

Independent of whether you want to use prettypretty from Rust or Python, you'll
need a **working Rust toolchain** to build the library or extension module from
source. The [rustup](https://rustup.rs) installer probably is your best option
for installing the necessary tools in the first place and thereafter for keeping
them up to date. When I started adding Rust support to prettypretty, I hadn't
touched Rust in a couple of years, simply ran `rustup update`, and moments later
had the latest version compiling my code.

Of course, to use the extension module from Python, you also need a **working
Python interpreter**. As part of continuous integration, prettypretty is tested
with CPython across Linux, macOS, and Windows.

Prettypretty leverages both programming languages to their fullest and hence
requires relatively recent versions:

  * According to [cargo-msrv](https://github.com/foresterre/cargo-msrv), **the
    minimum supported Rust version is 1.77.2**.
  * According to [vermin](https://github.com/netromdk/vermin), **the minimum
    supported Python version is 3.11.0.**

I expect that, as the project matures, the version lag between minimum and
latest versions will grow, as it should.


## Feature Flags

Prettypretty has two feature flags:

  * `f64` selects the eponymous type as floating point type [`Float`] and `u64`
    as [`Bits`] instead of `f32` as [`Float`] and `u32` as [`Bits`]. This
    feature flag is enabled by default.
  * `pyffi` enables Python integration through [PyO3](https://pyo3.rs/),
    changing some method signatures as well as adding several other methods. It
    is disabled by default.

The API documentation on
[docs.rs](https://docs.rs/prettypretty/latest/prettypretty/) is built without
`pyffi` and hence is Rust-only. The API documentation on
[GitHub](https://apparebit.github.io/prettypretty/prettypretty/) is built with
`pyffi` and hence covers Python integration as well. It tags Python-only methods
as <span class=python-only></span> and Rust-only methods as <span
class=rust-only></span>.


## Installation

With Rust installed or updated, you are ready to go and can use your favorite
package manager for installing prettypretty. For Rust, that is `cargo`:

```sh
$ cargo install prettypretty
```

For Python, `uv`, `pip`, or whatever else strikes your fancy are all good. To
namespace package management functionality, the incantation for `uv` even
mentions `pip`:

```sh
$ uv pip install prettypretty
```

The package's `pyproject.toml` configures the Python build to also enable the
`pyffi` feature flag.

When building the extension module from source, installation may take a moment.
It also involves one more tool, [`maturin`](https://github.com/PyO3/maturin). It
is responsible for creating the binary with the [PyO3](https://pyo3.rs/v0.22.1/)
integration layer. However, you shouldn't need to install `maturin` yourself.
Prettypretty's `pyproject.toml` declares `maturin` as its build backend and
hence `uv` or `pip` or whatever else strikes your fancy should automatically
install the `maturin` package and then run it. If your Python package manager
falls into the "whatever else strikes your fancy" category and it doesn't
install or run `maturin`, please fall back on `uv` or `pip`.


## prettypretty.plot

Prettypretty's plot script visualizes colors by plotting their chroma/hue on the
corresponding plane of the Oklrch color space (i.e., the [revised, cylindrical
version](https://bottosson.github.io/posts/colorpicker/#intermission---a-new-lightness-estimate-for-oklab)
of Oklab) and their lightness in a separate bar graph. It optionally includes
any or all boundaries of the sRGB, Display P3, and Rec. 2020 color spaces. For
this script to work, you also need to install matplotlib:

```sh
$ uv pip install matplotlib
```


## Documentation

As a bilingual library, prettypretty's documentation also needs to be bilingual
and hence requires additional tools to build. Notably, in addition to the
`rustdoc` API documentation generator for Rust, it also requires the `sphinx`
documentation generator for Python, and the
[mdBook](https://github.com/rust-lang/mdBook) documentation generator for the
user guide.

Probably the simplest option for installing `mdBook` is to build it from source:

```sh
$ cargo install mdbook
```

Installing the Python packages is a bit more tricky. At least if you are still
using `pip`. For reasons I don't comprehend, the project maintainers view the
option to read dependencies from `pyproject.toml` outside from very narrowly
prescribed circumstances as utterly unacceptable. With `uv`, you simply
incantate:

```sh
$ uv pip install --extra doc prettypretty
```


## Build Script

Outside of continuous integration, it's up to the developer to correctly invoke
the various tools with the appropriate options. In case of prettypretty, that's
quite the number of tools. Worse, since the Rust sources contain numerous
`#[cfg(...)]` and `#[cfg_attr(...)]` annotations to adjust for Rust or Python
usage, the `cargo check` and `cargo clippy` code quality checks need to be run
twice, once without the `pyffi` feature flag and once with it. However,
documentation is always generated with `pyffi` enabled. And the Rust tests are
always run with `pyffi` disabled.

If that sounds like too much too remember, I agree. Hence prettypretty's
repository includes `rr.sh` (for RunneR), a simple script to:

  * `check` the sources;
  * `build` the sources;
  * build the `docs`;
  * do `all` of the above.

The highlighted word also is the (only) argument to `rr.sh` for executing the
described job.


{{#include links.md}}
