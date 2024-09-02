# Developing Prettypretty

Since prettypretty integrates Rust and Python, it requires tooling for both
programming languages as well as for integrating between the two. To keep
development tasks manageable, the runner or
[**r²**](https://github.com/apparebit/prettypretty/blob/main/rr.sh) script in
the repository root automates the most common ones. Its only argument is the
task to perform:

  * `install` updates or installs necessary command line tools, including the
    Rust compiler and Python runtime, using either the APT or Homebrew package
    manager.
  * `build` compiles the Python extension module as `prettypretty/color.pyd` (on
    Windows) or `prettypretty/color.abi3.so` (on Unix).
  * `check` runs linters, type checkers, and tests for both languages. Tests can
    be found at the end of Rust modules, embedded in the Rust API documentation,
    embedded in the user guide, and the `test` directory.
  * `doc` builds the guide as well as the API documentation for both languages
    combining all three in the `target/doc` directory.

**r²** only automates local tasks. Making a release requires manually tagging
the sources and cutting a release on GitHub. A [GitHub
action](https://github.com/apparebit/prettypretty/actions) then builds
prettypretty's extension modules for Linux, macOS, and Windows and uploads the
source distribution and platform binaries to the [Python package
index](https://pypi.org/project/prettypretty/). To validate that the
repository's main branch is, in fact, ready for release, that same action also
runs the linters, type checkers, and tests for both languages.

In other words, even though **r²** and the repository's GitHub actions have
entirely different specifications and runtime environments, they nonetheless
perform many of the same tasks. Hence, any substantial change to **r²** or
prettypretty's GitHub actions probably must be ported over as well.


# The Python Extension Module

Prettypretty's functionality is exposed to Python through a so-called extension
module, i.e., a native code library. Python's import machinery looks for
extension modules in the same directories as for regular packages. Once loaded,
Python's runtime interacts with the library through its C API. That includes
executing an initialization function to populate the module object with
bindings.

In case of PyO3, that initialization function is the `#[pymodule]` function,
which creates bindings for constants, `#[pyfunction]`s, `#[pyclass]`es, as well
as submodules. The latter are useful for structuring APIs that, like
prettypretty's, comprise more than a handful of abstractions. However, PyO3's
support for submodules is only rudimentary. Hence, prettypretty's initialization
function explicitly sets submodules' `__package__` and `__name__` attributes and
register them in `sys.modules`.

That last step has the welcome side-effect of making submodules loadable with
Python's import machinery without further customization. Let's say, Python is
executing a script with an import statement for `prettypretty.color.spectrum`.
As usual, Python's import machinery first imports `prettypretty` then
`prettypretty.color`. Since the latter is the extension module, Python loads the
native code library and executes its initialization function. That function, in
turn, adds all submodules to `sys.modules`. So, when Python's import machinery
finally gets to importing `prettypretty.color.spectrum` itself, it checks
`sys.modules` for an entry with that name, which was just added by the extension
module initialization function. Et voilà!

As suggested by [PEP 489](https://peps.python.org/pep-0489/), I also
experimented with symbolic links from the submodules to the actual native
library. But since all submodules implemented in Rust have `prettypretty.color`
as parent module, those symbolic links have no impact. Hence, I removed them
again.

Unfortunately, Pylance is confused about submodules of an extension module and
currently [generates a false
warning](https://github.com/microsoft/pylance-release/issues/6269). You'll find
comments that selectively disable this warning throughout prettypretty's Python
sources.
