# Developing Prettypretty

Since prettypretty integrates Rust and Python, it not only requires tooling for
both programming languages but also technology for integrating the two
ecosystems. To keep development tasks nonetheless manageable, the runner or
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
    embedded in the project guide, and the `test` directory.
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

Prettypretty's functionality is exposed to Python through an extension module.
That is a native library that is dynamically loaded by the Python runtime and
uses Python's C API to integrate seamlessly with the interpreter. As a result,
prettypretty does *not* require glue code written in Python, but Python
interfaces directly with prettypretty's Rust code. The Python package does
contain some additional modules but they are slowly being replaced by equivalent
native code.

To make the Rust and Python API's more accessible to developers, I modularized
the public API for prettypretty, including the API exposed by the extension
module, with version 0.11.0. As a result, prettypretty's functionality is
exposed through `prettypretty.color` as well as a few submodules including
`prettypretty.color.gamut` and `prettypretty.color.style`. Despite the presence
of submodules, all native code still is contained by a single native library.

PyO3's support for submodules is rudimentary and does not follow a number of
Python conventions. However, prettypretty's module initialization function tries
to patch the most glaring oversights. In particular, it ensures that every
module has the correct `__name__` and `__package__` attributes and that
submodules are installed in Python's module registry `sys.modules`.

If your installation creates symbolic links to the dynamically linked library
for each submodule, then Python's import functionality can load any of the
modules first. If you don't have those symbolic links, then `prettypretty.color`
must be imported first. At least, [PEP 489](https://peps.python.org/pep-0489/)
says so and it worked in my testing.



