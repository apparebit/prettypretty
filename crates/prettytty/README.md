# Pretty 🌸 Tty

[![Run Tests, Build Wheels, & Publish to PyPI](https://github.com/apparebit/prettypretty/actions/workflows/ci.yml/badge.svg)](https://github.com/apparebit/prettypretty/actions/workflows/ci.yml)
[![Publish to GitHub Pages](https://github.com/apparebit/prettypretty/actions/workflows/gh-pages.yml/badge.svg)](https://github.com/apparebit/prettypretty/actions/workflows/gh-pages.yml)

\[  [**Docs.rs**](https://docs.rs/prettytty/latest/prettytty/)
| [**GitHub Pages**](https://apparebit.github.io/prettypretty/prettytty/)
| [**Rust Crate**](https://crates.io/crates/prettytty)
| [**Repository**](https://github.com/apparebit/prettypretty)
\]

Prettytty is a **lightweight and flexible terminal library** for Rust that has
only one low-level dependency, i.e., [`libc`](https://crates.io/crates/libc) on
Unix and [`windows-sys`](https://crates.io/crates/windows-sys) on Windows. Its
API is clean and simple: Open a [`Connection`] to the terminal and share it
across threads as needed. Write [`Command`]s to [`Output`]. Read [`Query`]
responses from [`Input`]. [`Scan::read_token`] takes care of low-level UTF-8 and
ANSI escape sequence decoding and [`Query::parse`] turns token payloads into
objects. A [`cmd`] library with 80+ built-in commands covers basic needs and
then some. Commands generally are zero-sized. That is, unless they require
string arguments or are designed for dynamic state (and hence prefixed with
`Dyn` for *dynamic*).


## Example

Here's how the above mentioned abstractions are used in practice:

```rust
use prettytty::{Connection, Query, Scan};
use prettytty::cmd::{MoveToColumn, RequestCursorPosition};
use prettytty::opt::Options;

// Open a terminal connection with 1s timeout.
let tty = Connection::with_options(Options::with_log())?;

let pos = {
    let (mut input, mut output) = tty.io();

    // Move cursor, issue query for position.
    output.exec(MoveToColumn::<17>)?;
    output.exec(RequestCursorPosition)?;

    // Read and parse response.
    let response = input.read_sequence(
        RequestCursorPosition.control())?;
    RequestCursorPosition.parse(response)?
};

assert_eq!(pos.1, 17);
```

## Release History

### v0.3.0 (2025-02-xx)

  * The [`Command`] trait now has both `Debug` and `Display` as supertraits.
  * Commands synthesized with [`fuse!`] or [`fuse_sgr!`] display the macro name
    and arguments under the debug trait.
  * [`Output::exec_defer`] takes two commands as arguments. It immediately
    executes the first command but defers the second command until just before
    the [`Connection`] is closed.
  * The new [`Query::run`] method turns three-line boilerplate for querying the
    terminal into a two-argument method invocation.
  * The updated
    [progress.rs](https://github.com/apparebit/prettypretty/blob/main/crates/prettytty/examples/progress.rs)
    illustrates the use of `fuse!`, `Output::exec_defer`, and `Query::run`.


### v0.2.2 (2025-02-01)

Fix link to docs.rs.


### v0.2.1 (2025-02-01)

  * Fix the [`fuse!`] macro.
  * Update both [`fuse!`] and [`fuse_sgr!`] to generate commands that are
    consistent with all of prettytty's commands other than [`DynLink`] and
    [`DynSetWindowTitle`] and hence implement the `Copy`, `Clone`, `Debug`,
    `PartialEq`, and `Eq` traits.
  * Update [`SetForeground8`], [`SetBackground8`], and their `Dyn` versions to
    generate shorter ANSI escape sequences for the first 16 colors, which are
    the 8 ANSI colors and their bright variants.


### v0.2 (2025-01-31)

Add zero-sized generic versions for commands that set colors or move cursor.
Keep previous, argument-based versions with `Dyn` prefix. Rename other commands
with runtime arguments to also use `Dyn` prefix.

Rename `sgr!` macro for combining several [`Sgr`] commands into one command to
[`fuse_sgr!`] and introduce the more general [`fuse!`] macro for combining
arbitrary commands into one command.

Add [Rust
version](https://github.com/apparebit/prettypretty/blob/main/crates/prettytty/examples/progress.rs)
of progress bar to illustrate API differences from [Python
version](https://github.com/apparebit/prettypretty/blob/main/prettypretty/progress.py).


### v0.1 (2024-12-23)

Initial release.

---

Copyright 2024-2025 Robert Grimm. The code in this repository has been released
as open source under the [Apache
2.0](https://github.com/apparebit/prettypretty/blob/main/LICENSE) license.


[`cmd`]: https://apparebit.github.io/prettypretty/prettytty/cmd/index.html
[`Command`]: https://apparebit.github.io/prettypretty/prettytty/trait.Command.html
[`Connection`]: https://apparebit.github.io/prettypretty/prettytty/struct.Connection.html
[`DynLink`]: https://apparebit.github.io/prettypretty/prettytty/cmd/struct.DynLink.html
[`DynSetWindowTitle`]: https://apparebit.github.io/prettypretty/prettytty/cmd/struct.DynSetWindowTitle.html
[`fuse!`]: https://apparebit.github.io/prettypretty/prettytty/macro.fuse.html
[`fuse_sgr!`]: https://apparebit.github.io/prettypretty/prettytty/macro.fuse_sgr.html
[`Input`]: https://apparebit.github.io/prettypretty/prettytty/struct.Input.html
[`Output`]: https://apparebit.github.io/prettypretty/prettytty/struct.Output.html
[`Output::exec_defer`]: https://apparebit.github.io/prettypretty/prettytty/struct.Output.html#method.exec_defer
[`Query`]: https://apparebit.github.io/prettypretty/prettytty/trait.Query.html
[`Query::parse`]: https://apparebit.github.io/prettypretty/prettytty/trait.Query.html#method.parse
[`Query::run`]: https://apparebit.github.io/prettypretty/prettytty/trait.Query.html#method.run
[`Scan`]: https://apparebit.github.io/prettypretty/prettytty/trait.Scan.html
[`Scan::read_token`]: https://apparebit.github.io/prettypretty/prettytty/trait.Scan.html#method.read_token
[`SetBackground8`]: https://apparebit.github.io/prettypretty/prettytty/cmd/struct.SetBackground8.html
[`SetForeground8`]: https://apparebit.github.io/prettypretty/prettytty/cmd/struct.SetForeground8.html
[`Sgr`]: https://apparebit.github.io/prettypretty/prettytty/trait.Sgr.html
