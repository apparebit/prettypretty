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
preprocessor amounts to a somewhat confounding template engine only. In other
words, the key difference between Rust's and Java's attributes is that Rust has
a language-supported means for acting on them.


### The Popularity of Rust Macros

<img src=macropower/most-downloaded.png
     alt="screenshot of crates.io's most downloaded list"
     width=200px
     style="float:right;">

The thing is, PyO3 is not the only Rust-based library and tool to make heavy use
of macros. At the time of this writing, `bitflags` is the fourth most downloaded
crate on crates.io. Its claim to fame is extending Rust with support for bit
flags, which are ubiquitous in C APIs and network protocols. Astonishingly, the
top three most downloaded crates as of early morning August 20, 2024, `syn`,
`proc-macro2`, and `quote`, are designed to aid the implementation of procedural
macros. Out of the remaining six in the top ten most downloaded crates, one more
crate, `cfg-if`, exists to wrangle syntax, two more crates, `rand_core` and
`rand`, make liberal use of macros for their implementation, two more crates,
`libc` and `hashbrown`, make use of a few macros, notably their own vendored
versions of `cfg-if`, and only one crate, `base64` appears to neither contain
nor use any macros.

The fact that macro-related crates are the most popular crates period points
towards a truth about Rust that doesn't get nearly enough attention: The
language's underappreciated superpower aren't affine types and the borrow
checker. They *are* the language's marquee features and hence the primary
reasons developers try out Rust. But they probably aren't enough to make them
stay. In fact, why would anyone stick with Rust when the developer experience
invariably entails needing something that isn't quite available in stable Rust
but has a GitHub issue indicating that the feature is fully implemented but
stuck somewhere just before or in stabilization. It happens less frequently in
2024 than in 2019, but it still happens with reliability.

The other feature frequently mentioned in the context of Rust is async. But the
state of async in Rust is such that async is closer to being Rust's kryptonite
than a superpower. As evidenced by the [project goals for the second half of
2024](https://github.com/rust-lang/rfcs/blob/master/text/3672-Project-Goals-2024h2.md),
that isn't news to the Rust project. But as a Rust user, I am disappointed that
generators, no matter their color, continue to be one feature seemingly stuck in
nightly, in 2024 as much as in 2019. Still, the fact that Rust 2024 will reserve
the `gen` keyword for just that use case is welcome news.

Like procedural macros, async depends on ecosystem crates for practical use. So,
I was surprised when I had to go down the most downloaded list until #57 to find
a crate dedicated to async, `mio`, with `tokyo` following at #59. A closer look
at the two language features and the most popular supporting crates helps to put
that disparity in perspective:

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

With `syn` having 2.6× the downloads of `mio`, the *Download Count* does indeed
suggest that procedural macros are more frequently used than async. But with
macros more generally useful than async, the fact that there is a difference
shouldn't be surprising. Though the magnitude of the difference still strikes me
as high. At the same time, there seem to be significant commonalities: As the
*First Commit* and *Stable Rust* columns make clear, both features had been in
development for a long time before initial stabilization. Furthermore, ecosystem
demand and uptake started long before stabilization. In other words, they both
are very much consistent with a community-based project that takes a
deliberately incremental approach to programming language evolution.


### The Uniqueness of Rust Macros

It is safe to conclude that Rust macros are very very popular. And that
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
information*, then that's ok. Just stick to declarative macros for now. They
aren't all that complicated and can be very effective.


### Rust Macros Save Rust From Itself

<div class=color-swatch style="float: right;"><div style="background: rgb(254 156 201);"></div></div>

In fact, they can be very effective at helping cope with papercuts resulting
from Rust's incremental evolution. Here's an example from prettypretty. The
issue is creating compile-time color constants in code. To facilitate conversion
between color spaces, prettypretty's representation for high-resolution colors,
`Color`, uses normalized floating point coordinates. While that is an
appropriate representation for processing colors, (hexadecimal) integers are the
more familiar and ergonomic notation for us humans. Nobody prefers
```rust,ignore
Color::srgb(0.996078431372549, 0.611764705882353, 0.788235294117647)
```
over
```rust,ignore
Color::from_24bit(0xfe, 0x9c, 0xc9)
```
for creating the same color object representing the same prettypretty pink shown
in the color swatch floating to the right.

Alas, `Color::from_24bit` cannot be used for creating compile-time constants in
current Rust (1.80.1), since `const` functions must not perform floating point
arithmetic. Worse, there is no support for `const` traits at all. Somewhat
strangely, however, floating point arithmetic *is* supported in `const`
expressions. This will get better in the near future. Stabilization for floating
point arithmetic in `const` functions [started a few weeks
ago](https://github.com/rust-lang/rust/pull/128596). And `const` traits are [on
the list of project
goals](https://github.com/rust-lang/rust-project-goals/issues/106) for the
second half of 2014. That doesn't mean they will be supported within that time
frame. But it does mean they are on the Rust team's radar screen.

So does that mean writing obscure floating point numbers for color constants for
now? As it turns out, stable Rust's support for floating point arithmetic in
expressions isn't only painful reminder of what could (or will) be, it also is
key to solving the problem *today*—no unsafe code or nightly Rust necessary!
Consider the following declarative macro, straight from prettypretty's sources:

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
to create a color constant for that prettypretty pink. 🌸

That's one real-world example of Rust macros [making the future
today](https://www.youtube.com/watch?v=Wlh6yFSJEms) and thereby making Rust more
usable. But I have to emphasize that getting there was a bumpy ride. I first
tried to do the conversion in a `const` function, which resulted in an error
message. I had bigger fish to fry, so for a while thereafter, I was using
floating point literals. Then one day, I got really frustrated. Since C allows
floating point arithmetic in constant expressions, the fact that Rust would not
just seemed nuts. So, I tried floating point arithmetic in a `const` expression
and it worked. Then I tried floating point arithmetic in a `const` function
again and it didn't work. Then it dawned on me that there really was a
difference between the two. Then I searched the GitHub issues. Finally, while
reading through [the corresponding
issue](https://github.com/rust-lang/rust/issues/57241), I had the idea for the
macro.

Unfortunately, the Rust compiler doesn't tell us about these work-arounds.
Neither do most books about Rust. For instance, David Drysdale's otherwise
excellent *Effective Rust* is so focused on being [fair and
balanced](https://www.lurklurk.org/effective-rust/macros.html), I can't but
wonder if he appreciates how important Rust macros are to the success of Rust.
The one exception I can think of is [The Little Book of
Macros](https://veykril.github.io/tlborm/). I for one am deeply appreciative of
Daniel Keep's and Lukas Wirth's efforts, in 2024 as much as in 2019. There also
is David Tolnay's [procedural macro
workshop](https://github.com/dtolnay/proc-macro-workshop), but I haven't worked
through the material yet. Any other recommendations?


## PyO3: The Bad

Let's get back to PyO3. While its deep integration between Rust and Python is
impressive, it doesn't always succeed. At the same time, even PyO3's failures
are instructive. First, I'm going to discuss two issues where integration with
Python is less than seamless. Second, in the next section, I'll discuss a major
pain point that results from a poor interaction between Rust features. In all
three cases, I also describe prettypretty's pragmatic workarounds and point
towards more principled and general solutions.

The first issue is that PyO3 exposes misleading or incorrect module and package
names to Python. First, by default, it identifies the `__module__` for functions
and classes as `builtins`. While I can see an associative connection, Python's
`builtins` are also written in native code, having accurate information on the
provenance of objects is critical for debugging. Second, PyO3 supports
submodules implemented by the same dynamically linked library. But it doesn't
set the `__package__` attribute and incorrectly sets the `__name__` attribute to
the unqualified name instead of the qualified one. Neither does it install
submodules in Python's `sys.modules` registry of loaded modules, thus preventing
Python from importing them.

PyO3 does offer partial work-arounds with declarative modules and the
`append_to_inittab` macro. But neither is a complete solution. To present
correct provenance, I had to add `module = "prettypretty.color"` (and similar)
arguments to all `#[pyclass]` attributes. To correctly set up the extension
module and its submodules, prettypretty's library initialization function
follows this outline:

```rust,ignore
#[pymodule]
pub fn color(mod_color: &Bound<'_, PyModule>) -> PyResult<()> {
    // Get fully qualified module name:
    let base_name = m.name()?;
    let base_name = base_name.to_str()?;

    // Populate extension module:
    mod_color.add_class::<Color>()?;

    // Create submodule, format its name, fix __package__:
    let mod_spectrum = PyModule::new_bound(mod_color.py(), "spectrum")?;
    let mod_spectrum_name = format!("{}.spectrum", base_name)
    mod_spectrum.add("__package__", base_name)?;

    // Populate submodule:
    mod_spectrum.add_class::<Observer>()?;

    // Add submodule to parent:
    mod_color.add_submodule(&mod_spectrum)?;
    // Only after add_submodule(), fix __name__:
    mod_spectrum.setattr("__name__", &mod_spectrum_name)?;

    // Patch sys.modules:
    let sysmodules: Bound<'_, PyDict> =
        PyModule::import_bound(mod_color.py(), "sys")?
        .getattr("modules")?
        .downcase_into()?;
    sysmodules.set_item(&mod_spectrum_name, mod_spectrum)?;

    Ok(()) // Ok?!
}
```

None of the was particularly hard. In fact, PyO3's user guide mentions the
`builtins` issue and its solution. It also provides example code for updating
`sys.modules`. But that still adds to the cognitive load when picking up PyO3
and imposes busy work on all users, well, those who care about playing well with
Python. Given PyO3's otherwise excellent integration with Python, I am a bit
puzzled by this apparent lack of care for proper naming. As prettypretty's
initialization function demonstrates, it is possible to do proper module setup
without even hardcoding the extension module's name. Since that function also
populates the extension module with its contents, that should suffice for fixing
all naming issues and hence seems to preclude technical limitations as an
explanation. That leaves me wondering whether the lack of immediate
consequences, whether positive (e.g., additional features) or negative (e.g.,
Python crashing), also removes a potent motivation to do better.

By contrast, the second issue is well beyond the control of the PyO3 project. In
fact, PyO3 does its part by exporting Rust documentation attributes to Python as
`__doc__` comments. The problem is a complete lack of tools to process that
text. By largely vanquishing the foreign function interface, PyO3 also
eliminates the need for a separate interface layer dedicated to Python. Instead,
we sprinkle some attribute dust over the code and the same abstractions, by and
large, become accessible from both Rust and Python. That has consequences for
API documentation: Whereas embedded comments and tools were monolingual before,
they now need to be bilingual, accounting for use case in both.

Alas, to the best of my knowledge, no such tools exist. It's also unclear how to
best write and present such bilingual documentation. To begin with, independent
views for each programming language have the advantage of hiding unnecessary
complexity from developers who only care about one of the languages. But the
need for producing two high-quality, independent views also runs the real risk
of doubling the documentation effort, which is neither economic nor ergonomic.
Documentation processors for the [OpenAPI](https://www.openapis.org) interface
definition language point to a more unified approach, where embedded tabs
feature language-specific examples.

Since I am partial to Rustdoc's clean and well-structured style, prettypretty's
documentation is closer to the latter approach, though it is possible to build a
version that covers Rust only. Where the two language APIs substantially
diverge, I try to cover the differences in the embedded comments. I also use <i
class=python-only>Python only!</i> or <i class=rust-only>Rust only!</i> to
annotate features that are available in one language only.



 When
provided with manually written Python typing stubs,
[pdoc](https://github.com/mitmproxy/pdoc) can generate API documentation. Though
it [doesn't seem very robust](https://github.com/mitmproxy/pdoc/issues/731) and
does not support Rustdoc's conventions for linking types. The need for the
latter is a side-effect of vanquishing the foreign function interface. Now the
same data type definition in Rust may serve as a Rust `struct` or `enum` or a
Python `class`. That also means that the API documentation needs to cover both
languages. As far as I know, there are no API documentation tools that support


When the foreign function
interface is largely va


The latter is necessary
because the same code is


The latter is the
result of Rust code being exposed to Python


The latter is
the fundamental challenge for any API documentation

More importantly, it
doesn't know what to do with Rustdoc's linking




In this section, I'm
going to highlight two pain points related to the integration with Python. In
the next section, I'm going to highlight one pain point related to Rust's
support for macros that also is more severe than the previous two. Prettypretty
works around all three, but especially the solution to the third ain't pretty at
all. While




## PyO3: The Ugly

