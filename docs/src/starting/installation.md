# Installing Prettypretty

One of the challenges of bilingual software is the barrier to entry posed by
tooling. Projects such as prettypretty typically require working toolchains for
both programming languages and for bridging the foreign-function interface on
top.


## Ideally: Just Use Packages

Having said that, there is one convenient option that usually works for both
Rust and Python projects: Just install the prettypretty package.

You install the **Rust package** thusly:
```sh
$ cargo install prettypretty
```

If you are running Linux, macOS, or Windows, you install the **binary wheel for
Python** thusly:
```sh
$ pip install prettypretty
```

Either way, prettypretty leverages both programming languages to their fullest
and hence requires relatively recent versions:

  * According to [cargo-msrv](https://github.com/foresterre/cargo-msrv), **the
    minimum supported Rust version is 1.77.2**.
  * According to [vermin](https://github.com/netromdk/vermin), **the minimum
    supported Python version is 3.11.0.**


## Fallback: Compile the Extension Module

If installing a binary wheel for Python doesn't work for you, building the
extension module from source requires:

  * **Rust**: Use [rustup](https://rustup.rs). It is robust and makes staying
    up-to-date easy. By comparison, when I tried using APT on Linux, the most
    recent Rust version was 6 months behind the most recent release and couldn't
    compile prettypretty.
  * **Python**: Use [CPython](https://github.com/python/cpython).
    [Python.org](https://www.python.org/downloads/) offers binary installers for
    macOS and Windows. On Linux, beware of package manager shenanigans. For
    example, APT's `python3` package is missing Python's `venv` standard library
    package and you need to install `python3-venv`, too.

If no binary wheel is available and your system has both Rust and Python
installed, then pip should transparently fall back onto building from source.
In other words,
```sh
$ pip install prettypretty
```
should still work.

If it doesn't, building prettypretty's extension module requires a third tool:

  * **Build tool for extension module**: Use [Maturin](https://www.maturin.rs).
    Options for installing maturin include `pip`, `brew`, and `cargo`. The
    corresponding incantation starts with the tool name followed by `install`
    followed by `maturin`.

For example,
```sh
$ cargo install maturin
```
downloads the source code for maturin and builds the tool. If you still cannot
run maturin, check the `PATH` environment variable. `$HOME/.cargo/bin` must be
included.

Once maturin is installed, build the extension module thusly:
```sh
$ maturin dev --all-features
```


### Compile-Time Configuration

That last command enabled all of prettypretty's compile-time features. There are
two:

  * `pyffi` controls prettypretty's Python integration through
    [PyO3](https://pyo3.rs/), which mostly adds types and methods. It is
    disabled by default.
  * `f64` selects the eponymous type as floating point type [`Float`] and `u64`
    as [`Bits`] instead of `f32` as [`Float`] and `u32` as [`Bits`].  It is
    enabled by default.


## Stretch Goal: Install the Works

Whether you want to type-check the Python sources, build the documentation, run
the code blocks embedded in the user guide, or generate visualizations, you'll
need additional libraries and tools:

  * **Pyright** and **Node.js**: Prettypretty uses
    [Pyright](https://microsoft.github.io/pyright/#/) for type-checking Python
    code, largely because mypy is just too buggy. Alas, Pyright is written in
    JavaScript and requires [Node.js](https://nodejs.org/) to run.
  * **mdBook**: Building the user guide and running embedded code blocks
    requires [mdBook](https://github.com/rust-lang/mdBook). Your
    best best for installing it is `cargo install mdbook`.
  * **Sphinx**: Building the Python API documentation requires this Python tool
    and several extensions. Prettypretty's `pyproject.toml` lists all of them
    under the `project.optional-dependencies.dev` key.
  * **matplotlib** and **vedo**: Running
    [prettypretty.plot](https://github.com/apparebit/prettypretty/blob/main/prettypretty/plot.py)
    requires the matplotlib package and running
    [prettypretty.viz3d](https://github.com/apparebit/prettypretty/blob/main/prettypretty/viz3d.py)
    with the `--render` option requires the vedo package. Prettypretty's
    `pyproject.toml` lists them under the `project.optional-dependencies.viz`
    key.

Yikes! That's getting a bit much. Thankfully, there is another option.


## Salvation: Automating Everythin With r²

If you want the works, pulling everything together is a bit involved. That's why
I wrote [**r²**](https://github.com/apparebit/prettypretty/blob/main/rr.sh),
prettypretty's runner script. It installs all dependencies, builds the extension
module, performs extensive checks, and generates the documentation. To get
started, try:

```sh
$ ./rr.sh help
```


{{#include ../links.md}}
