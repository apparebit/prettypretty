# Rust's Underappreciated Superpower

Prettypretty brings 2020s color science to 1970s terminals, enabling command
line tools to gracefully adapt visual styles to terminal capabilities and user
preferences. Most of prettypretty's code—according to GitHub, currently 67%—is
written in Rust. But thanks to [PyO3](https://pyo3.rs/), prettypretty's
functionality is available in Python, too. This being my first time using PyO3,
I am nothing short of impressed.


## PyO3: The Good

The remarkable parts are the seamlessness and extent of PyO3's integration
between the two programming languages as well as the resulting developer
experience. For starters, PyO3 automatically converts common types, including
collections, from both ecosystems. More interestingly, annotating a data
structure with `#[pyclass]` exposes it to Python as a class. Adding
`#[pymethods]` to an `impl` block exposes the contained methods to Python as
well. Adding further methods that adhere to Python's convention of
doubly-underscored or *dunder* names activates the same customizations for
converting to strings, comparing for (in)equality, performing arithmetic
operations, yielding a container's elements, and so on, all implemented *in pure
Rust*. To top it off, PyO3's [maturin](https://www.maturin.rs) command line tool
builds and links extension modules on Linux, macOS, and Windows, reliably, with
a minimum of fuss and configuration. Maturin also helps write GitHub actions for
continuous integration (CI) and uploads binary packages to PyPI, directly from
CI.

The overall result is something highly desirable and yet rare in software: PyO3
almost vanquishes the underlying technology, the foreign function interface
(FFI), which invariably features byzantine, low-level calling conventions and
what appear to be arbitrary restrictions on data layout. Instead, at its best,
PyO3 requires nothing more than a light dusting of `#[pyclass]` and
`#[pymethods]` attributes to make Rust crates readily scriptable with Python,
too.


## Vanquishing the Foreign Function Interface

Now, I know a little about vanquishing the FFI because that's just what [Martin
Hirzel](http://hirzels.com/martin/) and I were going for in 2007 with
[Jeannie](https://dl.acm.org/doi/10.1145/1297027.1297030), a programming
language that combines *all* of Java with *all* of C and relies on a then novel
backtick operator for seamlessly switching between language contexts at
expression granularity. The example below provides a flavor of what Jeannie's
backtick operator can do. Starting with a static Java method, it switches (1) to
C for the method implementation, (2) back to Java to start a computation, (3) to
C again to update a variable, and (4) to Java to update the argument, four
levels deep, at expression granularity.

<figure>
<img src=macropower/jeannie.svg
    alt="Java and C code nested within each other"
    width=300px>
</figure>

The minimal surface syntax of Jeannie, the mighty `` ` ``, stands in stark
contrast to programming with the [Java Native
Interface](https://en.wikipedia.org/wiki/Java_Native_Interface). Said foreign
function interface for the Java virtual machine may be well-encapsulated, but
its reflection-based interface also is verbose and surprisingly tricky to use
correctly.

Alas, there was one pesky detail: Demonstrating the viability of our design
required implementing a compiler for Jeannie, a language that incorporates all
of Java, all of C, and then some. The primary reason that Martin and I could
pull that off, in roughly six months nonetheless, was that we didn't build a
compiler from scratch. Instead, we composed, as much as possible, existing
monolingual code and tools, including `javac` and `gcc` for compiler backends.
It also helped that I had been building a toolkit for front-end extensibility,
called xtc for extensible compiler. Critically, it included a parser generator
that leveraged parsing expression grammars for [fully modular
syntax](https://dl.acm.org/doi/10.1145/1133981.1133987). Together with
colleagues, we subsequently applied the same compositional approach to
[multilingual debugging](https://dl.acm.org/doi/10.1145/1640089.1640105) (Java
and C), [multilingual bug
checking](https://dl.acm.org/doi/10.1145/1806596.1806601) (JNI), and
[multilingual
macros](https://link.springer.com/chapter/10.1007/978-3-642-31057-7_26) (C++ and
SQL).


## Rust's Underappreciated Superpower

While PyO3 and Jeannie share the goal of vanquishing the foreign-function
interface, they also differ significantly, in targeted programming languages,
remaining surface syntax, compositional granularity, pragmatic approach, and
implementation complexity. Without trying to quantify the latter, the key
trade-off is between compositional granularity and implementation complexity,
with Jeannie's finer granularity entailing higher complexity, and the existence
of language-supported extension mechanisms for Rust and Python, i.e., macros for
Rust and dunder methods for Python. By comparison, Java's attributes are just
that, inert annotations that require external tools for processing, and the C
preprocessor amounts to a somewhat confounding template engine only.

<img src=macropower/most-downloaded.png
     alt="screenshot of crates.io's most downloaded list"
     width=200px
     style="float:right;">

The thing is, PyO3 is not the only Rust-based library and tool to make heavy use
of macros. At the time of this writing, `bitflags` is the fourth most downloaded
crate on crates.io. Its claim to fame is extending Rust with support for bit
flags, which are ubiquitous in C APIs and network protocols. Astonishingly, the
top three most downloaded crates, `syn`, `proc-macro2`, and `quote`, are
designed to aid the implementation of procedural macros. Out of the remaining
six in the top ten most downloaded crates, one more crate, `cfg-if`, exists to
wrangle syntax, two more crates, `rand_core` and `rand`, make liberal use of
macros for their implementation, two more crates, `libc` and `hashbrown`, make
use of a few macros, notably their own vendored versions of `cfg-if`, and only
one crate, `base64` appears to neither contain nor use any macros.

The fact that macro-related crates are the most popular crates period points
towards a truth about Rust that doesn't get nearly enough attention: The
language's underappreciated superpower aren't affine types and the borrow
checker. They *are* the language's marquee features and hence the primary
reasons developers try out Rust. But they probably aren't enough to make them
stay. In fact, why would anyone stick with Rust when the developer experience
invariably entails needing something that isn't available in stable Rust but has
a GitHub issue indicating that the feature is fully implemented but stuck
somewhere just before or in stabilization. It happens less frequently in 2024
than in 2019, but it still happens with reliability.

The other feature frequently mentioned in the context of Rust is async. But the
state of async in Rust is such that async is closer to being Rust's kryptonite
than a superpower. I am aware of the project goals for [the second half of
2024](https://github.com/rust-lang/rfcs/blob/master/text/3672-Project-Goals-2024h2.md)
and they sound like steps in the right direction. But as a Rust user, I remain
sore that generators, no matter their color, continue to be one feature
seemingly stuck forever in unstable land, in 2024 as much as in 2019. (The fact
that Rust 2024 will reserve the `gen` keyword for just that use case is welcome
news nonetheless.)

The most downloaded list further suggests that async might not be that important
to Rust developers. The most downloaded crates clearly devoted to async
programming are `mio` and `tokyo` at positions 57 and 59. However, a closer look
at the two features and the most popular crates supporting them is instructive,
as it clearly shows the limits of that particular popularity contest:

|Feature     |Crate|First Commit|Stable Rust          |Download Count|
|:-----------|:----|:-----------|:--------------------|-------------:|
|Proc. Macros|`syn`|[2016-09-03]|1.30.0 ([2018-10-25])|  549,096,465 |
|Async       |`mio`|[2014-08-20]|1.39.0 ([2019-11-08])|  210,860,066 |

[2019-11-08]: https://blog.rust-lang.org/2019/11/07/Rust-1.39.0.html
[2018-10-25]: https://blog.rust-lang.org/2018/10/25/Rust-1.30.0.html
[2016-09-03]: https://github.com/dtolnay/syn/commit/35161fff39430bd1d41bf92f28cce10d0cfb5c0e
[2014-08-20]: https://github.com/tokio-rs/mio/commit/0e711c4cc5e61c55223e33fe78fad0e9f1b372e5

The table identifies the feature, the most popular crate supporting that
feature, the crate's first commit, when the feature first became stable, and the
total number of downloads from crates.io as of early morning, 20 August 2024.
Oh, by the way:

🍾 *Happy 10th birthday, `mio`!* 🎉

The table's correlation between language feature and popular crate is justified
by the fact that fully leveraging either feature pretty much requires ecosystem
support. With `syn` having 2.6× the downloads of `mio`, the *Download Count*
does indeed suggest that procedural macros are more frequently used than async.
But upon reflection, that seems hardly surprising. Macros are generally useful
whereas async has pretty much only one use case, albeit a critically important
one. The *First Commit* and *Stable Rust* columns make clear that both language
features had been in development for a long time before initial stabilization.
They also confirm that ecosystem demand and uptake for both features started
well before stabilization. So just maybe, integration of async into Rust is more
complicated than that of procedural macros.

*Duh!*


## Rust's Many Macros

Still, it is safe to conclude that Rust macros are very very popular. And that
*is* surprising in the context of systemsy programming languages. Historically,
the brittleness of the C preprocessor caused many developers to swear off
syntactic extensibility. How else to explain the terrible reputation of operator
overloading in C++, even though C++ developers use overloaded operators for I/O
all the time? Rust's peers, i.e., languages that emerged roughly ten years ago,
don't seem particularly well-disposed towards macros and the like either. Zig's
home page loudly proclaims that it is "A Simple Language" and the third and
final bullet belonging to that claim states "No preprocessor, no macros." By
contrast, Swift does support macros. But it only gained the feature a year ago,
[with version
5.9](https://github.com/swiftlang/swift/blob/main/CHANGELOG.md#swift-59) and
after [long
preparation](https://mjtsai.com/blog/2022/10/17/a-possible-vision-for-macros-in-swift/).
Despite all that preparation, the rollout was still marred by [slow compilation
times](https://mjtsai.com/blog/2024/02/27/slow-swift-macro-compilation/),
causing yet more developers to swear off syntactic extensibility.

So why is Rust different? The snarky answer is that macros help deal with the
language's short-comings as well as unstable features. As we'll see in the next
section, there is some truth to that. A (hopefully) more insightful answer is
that Rust doesn't have one macro facility but, depending on how you count, two
or three.

There are two ways of implementing macros in Rust. First, declarative macros are
far simpler but also less expressive. Second, procedural macros are less
approachable but also far more powerful. The fact that Rust offers options for
the trade-off between complexity and expressivity is fantastic. It enables
experienced developers to make an informed choice depending on use case. And it
lets inexperienced developers get their hands dirty without having to deal with
the complexity of procedural macros.

There are three ways of applying macros in Rust. Function-like macros. Derive
macros. And attribute macros. Out of the three, function-like macros are the
only ones that can introduce new surface syntax. Derive and attribute macros are
limited to attribute annotations. In fact, at first glance, derive and attribute
macros don't seem all that different. After all, they can both annotate
`struct`, `enum`, and `union` definitions. But at second glance, the differences
become more pronounced. Derive macros can only appear in `#[derive(..)]`
annotations on data type definitions, whereas attribute macros can appear on all
Rust items including, for example, module, trait, and method definitions.
Meanwhile, derive macros are the only macros that can define *helper
attributes*, i.e., attributes valid for the scope of the data definition.

If you are not familiar with Rust macros and this seems all a bit *too much
information*, then that's ok. Just stick to declarative macros for now. They are
far less complicated.


## A Rust Macro to Save Rust From Itself

<div class=color-swatch style="float: right;"><div style="background: rgb(254 156 201);"></div></div>

Here's an example from prettypretty. The code needs to define the occasional
color constant. To facilitate straight-forward conversion between color spaces,
the crate's representation for high-resolution colors, `Color`, uses normalized
floating point coordinates. While that is the more appropriate representation
for processing colors, integer coordinates are more readable and familiar to us
humans. Hence they seem more appropriate for specifying colors, at least in the
common case where colors are in the sRGB color space. For example,
```rust,ignore
Color::srgb(0.996078431372549, 0.611764705882353, 0.788235294117647)
```
creates the nice light pink shown floating on the right side. So does
```rust,ignore
Color::from_24bit(0xfe, 0x9c, 0xc9)
```
The latter is much closer to the `#fe9cc9` notation familiar from the web and
hence seems preferable for specifying color constants. But current Rust (1.80.1)
does not allow floating point arithmetic in `const` functions, that is,
functions that can be executed at compile time. Worse, it does not allow `const`
traits at all. However, it does allow floating point arithmetic in `const`
expressions. The restriction on floating point arithmetic in `const` functions
will probably be lifted pretty soon. Stabilization [started a few weeks
ago](https://github.com/rust-lang/rust/pull/128596). And `const` traits are [on
the list of project
goals](https://github.com/rust-lang/rust-project-goals/issues/106) for the
second half of 2014. That doesn't mean they will be supported within that time
frame. But it does mean they are on the Rust team's radar screen.

The fact that stable Rust supports floating point arithmetic in expressions
isn't just a painful reminder that the other restrictions are somewhat
arbitrary. It also is key to solving the problem *today*, in stable Rust
nonetheless. Consider the following declarative macro, straight from
prettypretty's sources:

```rust
#[macro_export]
macro_rules! rgb {
    ($r:expr, $g:expr, $b:expr) => {
        $crate::Color::new(
            $crate::ColorSpace::Srgb,
            [
                ($r) as $crate::Float / 255.0,
                ($g) as $crate::Float / 255.0,
                ($b) as $crate::Float / 255.0,
            ],
        )
    };
}
```

<div class=color-swatch style="float: right;"><div style="background: rgb(254 156 201);"></div></div>

The `rgb` macro takes three arguments `$r`, `$g`, and `$b`, all of which must be
expressions, converts them to floating point numbers, and normalizes them by
dividing by 255.0 before invoking the `const` constructor function `Color::new`
with just the right arguments, i.e., a color space and three floating point
coordinates between 0.0 and 1.0 for in-gamut colors. Using the macro, I can
write
```rust,ignore
rgb!(254, 156, 201)
```
or also
```rust,ignore
rgb!(0xfe, 0x9c, 0xc9)
```
to create a color constant for that oh so pretty pink. I like the color so much,
I've added a swatch to this paragraph as well. 🌸

That's one real-world example for Rust macros saving the language from itself.
But I have to emphasize that getting there was a bumpy ride. I first tried to do
the conversion in a `const` function, which resulted in an error message. I had
bigger fish to fry, so for a while thereafter, I was using floating point
literals. Then one day, I got really frustrated. Since C allows floating point
arithmetic in constant expressions, the fact that Rust would not just seemed
nuts. So, I tried floating point arithmetic in a `const` expression and it
worked. Then I tried floating point arithmetic in a `const` function again and
it didn't work. Then it dawned on me that there really was a difference between
the two. Then I searched the GitHub issues. Finally, while reading through [the
corresponding issue](https://github.com/rust-lang/rust/issues/57241), I had the
idea for the macro.

Unfortunately, the Rust compiler doesn't tell us about these work-arounds.
Neither do most books about Rust. For instance, David Drysdale's otherwise
excellent *Effective Rust* is so focused on being [fair and
balanced](https://www.lurklurk.org/effective-rust/macros.html), I can't but
wonder if he appreciates how important Rust macros are to the success of Rust.
The one exception I can think of is [The Little Book of
Macros](https://veykril.github.io/tlborm/). I for one am deeply appreciative of
Lukas Wirth's and Daniel Keep's efforts, in 2024 as well as in 2019. There also
is David Tolnay's [procedural macro
workshop](https://github.com/dtolnay/proc-macro-workshop), but I haven't worked
through the material yet. Any other recommendations?


## PyO3: The Bad

Let's get back to PyO3.




## PyO3: The Ugly

