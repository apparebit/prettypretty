# Pretty 🌸 Tty

[![Run Tests, Build Wheels, & Publish to PyPI](https://github.com/apparebit/prettypretty/actions/workflows/ci.yml/badge.svg)](https://github.com/apparebit/prettypretty/actions/workflows/ci.yml)
[![Publish to GitHub Pages](https://github.com/apparebit/prettypretty/actions/workflows/gh-pages.yml/badge.svg)](https://github.com/apparebit/prettypretty/actions/workflows/gh-pages.yml)

\[  [**Docs.rs**](https://docs.rs/prettypretty/latest/prettytty/)
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
objects. A [`cmd`] library with 70+ built-in commands covers basic needs and
then some.


## Example

Here's how the above mentioned abstractions are used in practice:

```rust
use prettytty::{Connection, Query, Scan};
use prettytty::cmd::{MoveToColumn, RequestCursorPosition};
use prettytty::opt::Options;

// Open a terminal connection with 1s timeout.
let tty = Connection::with_options(
    Options::builder().timeout(10).build())?;

let pos = {
    let (mut input, mut output) = tty.io();

    // Move cursor, issue query for position.
    output.exec(MoveToColumn(17))?;
    output.exec(RequestCursorPosition)?;

    // Read and parse response.
    let response = input.read_sequence(
        RequestCursorPosition.control())?;
    RequestCursorPosition.parse(response)?
};

assert_eq!(pos.1, 17);
```

[`cmd`]: https://apparebit.github.io/prettypretty/prettytty/cmd/index.html
[`Command`]: https://apparebit.github.io/prettypretty/prettytty/trait.Command.html
[`Connection`]: https://apparebit.github.io/prettypretty/prettytty/struct.Connection.html
[`Input`]: https://apparebit.github.io/prettypretty/prettytty/struct.Input.html
[`Output`]: https://apparebit.github.io/prettypretty/prettytty/struct.Output.html
[`Query`]: https://apparebit.github.io/prettypretty/prettytty/trait.Query.html
[`Query::parse`]: https://apparebit.github.io/prettypretty/prettytty/trait.Query.html#method.parse
[`Scan`]: https://apparebit.github.io/prettypretty/prettytty/trait.Scan.html
[`Scan::read_token`]: https://apparebit.github.io/prettypretty/prettytty/trait.Scan.html#method.read_token


---

Copyright 2024 Robert Grimm. The code in this repository has been released as
open source under the [Apache
2.0](https://github.com/apparebit/prettypretty/blob/main/LICENSE) license.
