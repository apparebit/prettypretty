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
