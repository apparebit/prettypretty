# Accommodating All Colors

To reap the benefits of 2020s color science for 1970s terminal colors, we need
to be able to translate between terminal and high-resolution colors at will, in
both directions. I see three major reasons why doing just that is difficult:

 1. Whereas all high-resolution colors fit into a uniform model of coordinates
    tagged by their color spaces, different kinds of terminal colors have
    different representations from each other and from high-resolution colors.
    In other words, there is little uniformity amongst terminal colors.
 2. Some of the differences between terminal colors are not just differences of
    representation but rather radically different conceptualizations of color.
    In particular, ANSI colors have no intrinsic color values. On top of that,
    the default colors are also context-sensitive and hence are of limited use.
 3. There are huge differences in the number of available colors: 16 ANSI colors
    versus 256 indexed colors versus 16 million true colors. Curiously, the
    bigger difference when it comes to translating colors is not the one from 16
    million down to 256 colors but the one from 256 down to 16 colors.

Despite these substantial differences, many terminal color representations can
at least be partially and losslessly converted into a few other color
representations. For example, with exception of the ANSI colors, 8-bit colors,
as represented by [`EmbeddedRgb`] and [`GrayGradient`], have well-defined
formulas for converting to index values, i.e., `u8`, as well as 24-bit color,
i.e., [`TrueColor`]. True colors, in turn, are easily convertible to
high-resolution [`Color`]. Meanwhile, [`DefaultColor`], [`AnsiColor`],
[`EmbeddedRgb`], [`GrayGradient`], and [`TrueColor`] are all trivially
convertible to [`TerminalColor`]. They should be because [`TerminalColor`]'s
very purpose is being the unifying wrapper type. Prettypretty accommodates the
stateless conversions by implementations of the `From<T>` and `TryFrom<T>`
traits in Rust and static methods in Python. Alas, those can't help translate
default and ANSI colors.


## Translation Is Necessarily Stateful

Since the default and ANSI colors are abstract, translation to high-resolution
colors necessarily requires some form of lookup table, i.e., a color theme.
Prettypretty relies on the same abstraction to store that table as well as the
derived state for translating high-resolution colors to terminal colors again:

  * [`Translator`] provides the logic and state for translating between
    terminal and high-resolution colors.

There is ample precedent for the use of color themes to provide concrete values
for abstract colors. In fact, most terminal emulators feature robust support for
the [plethora of such themes](https://gogh-co.github.io/Gogh/) readily available
on the web. However, asking users to configure theme colors after they already
configured their terminals decidedly is the wrong approach. Luckily, ANSI escape
codes include sequences for querying a terminal for its current theme colors,
making it possible to automatically and transparently adjust to the runtime
environment.


## The Fall From High-Resolution

Theme colors turn the translation of terminal to high-resolution colors into a
simple lookup. The level of difficulty when translating in the other direction,
from high-resolution to terminal colors, very much depends on the target colors:


### 24-Bit Colors

In the best case, when the source color is in-gamut for sRGB and the target are
24-bit "true" colors, a loss of numeric resolution is the only concern. It
probably is imperceptible as well. However, if the source color is out of sRGB
gamut, even when still targeting 24-bit colors and, like [`Translator`], using
gamut-mapping, the difference between source and target colors may become
clearly noticeable. It only becomes more obvious when targeting 8-bit or ANSI
colors.


### 8-Bit Colors

While accuracy necessarily suffers when targeting less than 24-bit colors,
translation to 8-bit colors actually isn't particularly difficult. The reasons
are twofold: First, there *few enough* colors that brute force search for the
closest matching color becomes practical. Second, there are *many enough* colors
that brute force search is bound to find a reasonable match. Critically, the
embedded 6x6x6 RGB cube provides variations that go well beyond the primary and
secondary colors.

Since the brute force search compares colors for their distance, two convenient
color spaces for conducting that comparison are Oklab and Oklrab. The trick for
achieving consistently good results, especially when translating more than one
color, is to omit the ANSI colors from the set of candidate colors. Color themes
are not designed for regular placement within any color spaces. So ANSI colors
are bound to stick out amongst the more homogeneous embedded RGB and gray
gradient colors. Besides, they only make up 1/16 of all 8-bit colors and hence
don't add much compared to other 8-bit colors.


### ANSI Colors

Omitting ANSI colors is, of course, not feasible when targeting ANSI colors.
Still, brute force search over the ANSI colors works well enough *most of the
time*. But because there are so few candidates, the closest matching color may
just violate basic human expectations about what is a match, e.g., that warm
tones remain warm, cold tones remain cold, light tones remain light, dark tones
remain dark, and last but not least color remains color.
[`Translator::to_closest_ansi`](https://apparebit.github.io/prettypretty/prettypretty/struct.Translator.html#method.to_closest_ansi)'s
documentation provides an example that violates the latter expectation, with a
light orange tone turning into a light gray. That is jarring, especially in
context of other colors that are *not* mapped to gray.

Hence, I developed a more robust algorithm for downsampling to ANSI colors. It
leverages not only uses color pragmatics, i.e., the coordinates of theme colors,
but also color semantics, i.e., their intended appearance. In other words, the
algorithm leverages the very fact that ANSI colors are abstract colors to
improve the quality of matches. As implemented by
[`Translator::to_ansi_hue_lightness`](https://apparebit.github.io/prettypretty/prettypretty/struct.Translator.html#method.to_ansi_hue_lightness),
the algorithm first uses hue in Oklrch to find a pair of regular and bright
colors and second uses lightness to pick the closer one. In my evaluation so
far, it is indeed more robust than brute force search. But it also won't work if
the theme colors themselves are inconsistent with theme semantics. Since that
can be automatically checked,
[`Translator::to_ansi`](https://apparebit.github.io/prettypretty/prettypretty/struct.Translator.html#method.to_ansi)
transparently picks the best possible method.


## Translator Methods

Now that we understand the challenges and the algorithms for overcoming them, we
turn to [`Translator`]'s interface. We group its method by task:

 1. [`Translator::resolve`](https://apparebit.github.io/prettypretty/prettypretty/struct.Translator.html#method.resolve)
    translates terminal colors to high-resolution colors. Thanks to the
    `Into<TerminalColor>` trait, Rust code can invoke the method with an
    instance of `u8`, [`DefaultColor`], [`AnsiColor`], [`EmbeddedRgb`],
    [`GrayGradient`], [`TrueColor`], or [`TerminalColor`]. Thanks to a custom
    PyO3 conversion function, Python code can do the exact same.
 2. [`Translator::to_closest_8bit`](https://apparebit.github.io/prettypretty/prettypretty/struct.Translator.html#method.to_closest_8bit)
    and
    [`Translator::to_ansi`](https://apparebit.github.io/prettypretty/prettypretty/struct.Translator.html#method.to_ansi)
    translate high-resolution colors to low-resolution terminal colors.
    Prettypretty does not support conversion to the default colors and
    high-resolution colors can be directly converted to true colors, without
    requiring mediation through [`Translator`].

    The
    [`Translator::supports_hue_lightness`](https://apparebit.github.io/prettypretty/prettypretty/struct.Translator.html#method.supports_hue_lightness),
    [`Translator::to_ansi_hue_lightness`](https://apparebit.github.io/prettypretty/prettypretty/struct.Translator.html#method.to_ansi_hue_lightness),
    [`Translator::to_closest_ansi`](https://apparebit.github.io/prettypretty/prettypretty/struct.Translator.html#method.to_closest_ansi),
    and
    [`Translator::to_ansi_rgb`](https://apparebit.github.io/prettypretty/prettypretty/struct.Translator.html#method.to_ansi_rgb)
    methods provide direct access to individual algorithms for converting to
    ANSI colors. For instance, I use these methods for comparing the
    effectiveness of different approaches. But your code is probably better off
    using
    [`Translator::to_ansi`](https://apparebit.github.io/prettypretty/prettypretty/struct.Translator.html#method.to_ansi),
    which automatically picks `to_ansi_hue_lightness` or `to_closest_ansi`. In
    any case, I strongly recommend avoiding `to_ansi_rgb`. It only exists to
    evaluate the approach taken by the popular JavaScript library
    [Chalk](https://github.com/chalk/chalk) and reliably produces subpar
    results. Ironically, Chalk's tagline is "Terminal string styling done
    right."
 3. [`Translator::cap`](https://apparebit.github.io/prettypretty/prettypretty/struct.Translator.html#method.cap)
    tanslates terminal colors to terminal colors. Under the hood, it may very
    well translate a terminal color to a high-resolution color and then match
    against that color to produce a terminal color again. Use this method to
    adjust terminal colors to the runtime environment and user preferences,
    which can be concisely expressed by a [`Fidelity`] level.
 4. [`Translator::is_dark_theme`]() determines whether the color theme used by this
    translator instance is a dark theme.


[`Translator`] eagerly creates the necessary tables with colors for brute force and
hue-lightness search in the constructor. Altogether, an instance of this struct
owns 306 colors, which take up 7,160 bytes on macOS. As long as the terminal
color theme doesn't change, a translator need not be regenerated. That also means
that it can be used concurrently without locking—as long as threads have their
own references.


## Translator

The example code below illustrates the use of each major entry point besides
`to_closest_8bit`, which isn't that different from `to_ansi`:

```rust
# extern crate prettypretty;
# use prettypretty::{AnsiColor, Color, ColorFormatError, Translator, VGA_COLORS};
# use prettypretty::{OkVersion, TrueColor, Fidelity, EmbeddedRgb};
# use std::str::FromStr;
let red = &VGA_COLORS[AnsiColor::BrightRed as usize + 2];
assert_eq!(red, &Color::srgb(1.0, 0.333333333333333, 0.333333333333333));

let translator = Translator::new(OkVersion::Revised, VGA_COLORS.clone());
let also_red = &translator.resolve(AnsiColor::BrightRed);
assert_eq!(red, also_red);

let black = translator.to_ansi(&Color::srgb(0.15, 0.15, 0.15));
assert_eq!(black, AnsiColor::Black);

let maroon = translator.cap(TrueColor::new(148, 23, 81), Fidelity::EightBit);
assert_eq!(maroon, Some(EmbeddedRgb::new(2,0,1).unwrap().into()));
# Ok::<(), ColorFormatError>(())
```
<div class=color-swatch>
<div style="background-color: #f55;"></div>
<div style="background-color: #262626;"></div>
<div style="background-color: #000;"></div>
<div style="background-color: #941751;"></div>
<div style="background-color: #87005f;"></div>
</div>

The Python version is a close match:

```python
~from prettypretty.color import AnsiColor, Color, OkVersion, Translator, Fidelity
~from prettypretty.theme import VGA
red = VGA[AnsiColor.BrightRed.to_8bit() + 2]
assert red == Color.srgb(1.0, 0.333333333333333, 0.333333333333333)

translator = Translator(OkVersion.Revised, VGA)
also_red = translator.resolve(AnsiColor.BrightRed)
assert red == also_red

black = translator.to_ansi(Color.srgb(0.15, 0.15, 0.15))
assert black == AnsiColor.Black

maroon = translator.cap(TrueColor(148, 23, 81), Fidelity.EightBit)
assert maroon == TerminalColor.Rgb6(EmbeddedRgb(2, 0, 1))
```
<div class=color-swatch>
<div style="background-color: #f55;"></div>
<div style="background-color: #262626;"></div>
<div style="background-color: #000;"></div>
<div style="background-color: #941751;"></div>
<div style="background-color: #87005f;"></div>
</div>


{{#include ../links.md}}
