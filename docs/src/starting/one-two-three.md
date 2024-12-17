# Prettypretty's One-Two-Three

Prettypretty's integration of 2020s color science with 1970s terminal output
enables a fresh take on styling the output of command line applications. Similar
to CSS on the web, that approach treats styles as dynamic, with rendered results
looking differently on different terminals. However, while CSS favors
progressive enhancement, i.e., starting with a more basic design and adding
features and flourishes as screen sizes and computing power increase,
prettypretty builds on graceful degradation, i.e., starting with expressive
styles and automatically adjusting them to user preferences and less capable
terminals. Very much like CSS, prettypretty delivers better results, when you
author for variability. Prettypretty's workflow is based on three major steps.
This chapter provides an overview of prettypretty's one-two-three. The [deep
dive on the progress bar script](../deepdive/progressbar.md) adds, ahem, depth.


## 1. Fluently Assembling Styles

The first step is the fluent assembly of [`Style`] objects with [`stylist`]s,
i.e., style builders. Each such style can optionally reset terminal appearance,
may include a text [`Format`], a foreground color using any supported color
format, and a background color using any supported color format.

The following example fluently assembles a style for bold black text on yellow
background:

```rust
# extern crate prettypretty;
# use prettypretty::{Color, ColorSpace};
# use prettypretty::style::{stylist, Colorant, format::Format, TrueColor};
let style = stylist()
    .bold()
    .foreground(Color::default())
    .background(TrueColor::new(0xff, 0xe0, 0x6c))
    .et_voila();

assert_eq!(style.format(), Some(Format::new().bold()));
assert_eq!(style.foreground(),
    Some(Colorant::HiRes(Color::new(
        ColorSpace::Xyz, [0.0, 0.0, 0.0]))).as_ref());
assert_eq!(style.background(),
    Some(Colorant::Rgb(TrueColor::new(255, 224, 108))).as_ref());
```
<div class=color-swatch>
<div style="background-color: color(xyz 0 0 0);"></div>
<div style="background-color: #ffe06c;"></div>
</div>
<br>

Best practice is to define a complimentary style for dark mode as well. Since
colors tend to appear more saturated in dark mode, simply switching foreground
and background colors doesn't work. Instead pick a less saturated yellow and
also a less dark black. Once you picked those colors, how about assembling the
corresponding style in Rust?


## 2. Adjusting the Fidelity of Styles

The second step is adjusting those pretty assembled styles to the capabilities
of the current terminal and to user preferences, e.g., `NO_COLOR`, as captured
by [`Fidelity`] levels. [`Fidelity::from_environment`] determines likely
terminal capabilities based on environment variables. Meanwhile, [`Translator`]
performs the actual conversion. You instantiate a translator with the colors for
the current color theme.

Once you have the right fidelity level and a translator, you pick between
styles for light or dark mode with the [`Translator::is_dark_theme`]. And you
adjust the selected styles with [`Style::cap`].

The example code shows how to adjust the style from the previous example for a
terminal that renders 8-bit colorts only.

```rust
# extern crate prettypretty;
# use prettypretty::{Color, OkVersion, Translator};
# use prettypretty::style::{stylist, AnsiColor, Colorant, Fidelity, TrueColor};
# use prettypretty::theme::VGA_COLORS;
# let style = stylist()
#     .bold()
#     .foreground(Color::default())
#     .background(TrueColor::new(0xff, 0xe0, 0x6c))
#     .et_voila();
let translator = Translator::new(
    OkVersion::Revised, VGA_COLORS.clone());
let style = style.cap(Fidelity::Ansi, &translator);

assert_eq!(style.foreground(),
    Some(Colorant::Ansi(AnsiColor::Black)).as_ref());
assert_eq!(style.background(),
    Some(Colorant::Ansi(AnsiColor::BrightYellow)).as_ref());
```
<div class=color-swatch>
<div style="background-color: rgb(0 0 0);"></div>
<div style="background-color: rgb(255 255 85);"></div>
</div>


## 3. Applying and Reverting Styles

The third step is actually using the assembled and adjusted styles. Applying a
style to text, say, `prettypretty`, is as simple as writing its display to the
terminal. Reverting the style again takes nothing more than writing the display
of the negation to the terminal.

The example illustrates how to apply the style from the example above to this
package's name, `prettypretty`.

```rust
# extern crate prettypretty;
# use prettypretty::{Color, OkVersion, Translator};
# use prettypretty::style::{stylist, AnsiColor, Colorant, Fidelity, TrueColor};
# use prettypretty::theme::VGA_COLORS;
# let style = stylist()
#     .bold()
#     .foreground(Color::default())
#     .background(TrueColor::new(0xff, 0xe0, 0x6c))
#     .et_voila();
# let translator = Translator::new(
#     OkVersion::Revised, VGA_COLORS.clone());
# let style = style.cap(Fidelity::Ansi, &translator);
let s = format!("{}prettypretty{}", style, -&style);

assert_eq!(s, "\x1b[1;30;103mprettypretty\x1b[22;39;49m")
```

That's all!


{{#include ../links.md}}
