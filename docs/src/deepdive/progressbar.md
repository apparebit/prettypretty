# The Appearance of Progress

To explore how command line tools can benefit from a different approach to
terminal styles, this deep dive explores how to present a progress bar. I picked
this topic for a few reasons:

 1. I've been itching to write a progress bar for quite a while now.
 2. An animated demo often is more interesting than a static one.
 3. The script is simple enough to fit into less than 100 lines of Python.
 4. The script provides a feature that's actually useful to terminal apps.

The complete script is, of course, part of [prettypretty's
repository](https://github.com/apparebit/prettypretty/blob/main/prettypretty/progress.py)
and also included in its distribution.


## Visualizing Progress

To get started, you probably want to run the progress bar script yourself. So
please create a new virtual environment, install prettypretty into it, and run
the progress bar script:

```sh
$ mkdir progress
$ cd progress
$ python -m venv .venv
$ source .venv/bin/activate
$ python -m pip install prettypretty
Downloading prettypretty-0.9.0-py3-none-any.whl (64 kB)
Installing collected packages: prettypretty
Successfully installed prettypretty-0.9.0
$ python -m prettypretty.progress
```

Please note that prettypretty requires Python 3.11 or later. Furthermore,
building prettypretty from source requires a locally installed Rust toolchain.
[Rustup](https://rustup.rs) not only installs that toolchain but helps you keep
it up to date.

The last command amongst the shell incantations above actually executes the
progress bar script. You should see a green bar rapidly go from 0% to 100%. If
your terminal uses a light theme, it probably ends up looking like this:

<img src="https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/progress-bar-light.png"
     alt="a complete, green progress bar under light mode" width=293>

If your terminal uses a dark theme, it probably ends up looking more like this:

<img src="https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/progress-bar-dark.png"
     alt="a complete, green progress bar under dark mode" width=298>

Notice that the two progress bars use different tones of green. In particular,
the green tone for the dark theme is considerably less bright and vivid. That is
by design. Human vision adapts to lighting conditions and we tend to perceive
the same colors more intensely when they are presented in a darker context.


## Design Thinking for Terminal Tools

In addition to colors, terminals support a few more attributes for styling text,
including ones that bolden, slant, or underline text. Of course, we could just
write the corresponding ANSI escape sequences to the terminal. We'd still adhere
to best practices and use a nicely encapsulated terminal logger with clearly
distinguishable styles for status updates versus errors. We certainly could. But
considerable experience with the web also reminds us that separating
presentational aspects from content and then treating presentation in a
principled manner contributes towards the design of more consistent user
interfaces that offer a better user experience. It also helps with engineering
by, for example, discouraging code duplication.

Here then is prettypretty's five-step recipe for applying the lessons of the web
to terminal user interfaces. Note that this doesn't necessarily imply using CSS
nor are we required to adhere to web practices only. For instance, whereas
common web practice focuses on progressive enhancement, that is, assuming
limited capabilities and leveraging additional features as they become
available, prettypretty really is in the business of graceful degradation, that
is, assuming standard capabilities and then downgrading as necessary.


### 1. Define Styles

So it makes eminent sense to also isolate terminal styles from content and get
started on developing a language of terminal design tokens and, eventually, also
components. To make this concrete, the progress bar script uses the [`rich`]
fluent builder to declare two styles:

```python
LIGHT_MODE_BAR = rich().fg(Color.p3(0.0, 1.0, 0.0)).style()
DARK_MODE_BAR = rich().fg(3, 151, 49).style()
```
<div class=color-swatch>
<div style="background-color: color(display-p3 0 1 0);"></div>
<div style="background-color: rgb(3 151 49);"></div>
</div>

We could instantiate style objects directly but, depending on the number of
attributes we'd like to set, that won't be very readable. Instead, the [`rich`]
builder provides a fluent interface to declaring styles and more. For instance,
it also supports hyperlinks and cursor movements. But for our progress bar, we
just stick with basic styles. Here's how we might declare a warning style:

```python
WARNING = rich().bold.fg(16).bg(220).style()
```
<div class=color-swatch>
<div style="background-color: #000;"></div>
<div style="background-color: #ffd700;"></div>
</div>

The style sets the bold text attribute (through a property) as well as
foreground and background colors (through methods). It happens to use 8-bit
indexed colors, namely black text on a orange-yellow background.

When declaring styles, only include attributes that you want set and nothing
else. Don't bother with defining styles that undo other styles or incrementally
transition from one style to another. You can easily and automatically compute
them with Python's negation `~`, subtraction `-`, and alternative `|` operators.
In particular, the style `~style` undoes all attributes of `style`, hence
restoring the terminal to its default appearance. The style `style2 - style1`
incrementally transitions from `style1` to `style2` (note the reverse order).
Finally, the style `style1 | style2` lets you recover complete styles from
incremental transitions by combining all attributes from `style1` and `style2`.
Beware that `|` does give precedence to the first operand `style1` if both of
them impact the same attribute.


### 2. Prepare Styled Content

For example, [the last
line](https://github.com/apparebit/prettypretty/blob/da0d1a6d0277dd3a240a1b49037925036f7e8498/prettypretty/progress.py#L55)
of the `format_bar` function in the progress bar script uses negation for its
intended purpose, restoring the default appearance:

```python
return RichText.of('  ┫', style, bar, ~style, '┣', f' {percent:5.1f}%')
```

[`RichText`] is a sequence of strings, styles, and so on that simplifies color
processing during output. It's not required but it may speed up your code a
little bit.


### 3. Set Up Terminal

The progress bar script's [main
function](https://github.com/apparebit/prettypretty/blob/da0d1a6d0277dd3a240a1b49037925036f7e8498/prettypretty/progress.py#L67)
illustrates how to go from style declarations to usable styles and then to
displaying styled text. It starts out by creating a terminal object, possibly
overwriting its color fidelity, querying the terminal for its current color
scheme, hiding the cursor, and scoping all styles. The `with` block ensures that
the cursor reappears and no custom style leaks into your terminal, even if the
application raises an exception. In other words, applications should always use
such a `with` block.

```python
with (
   Terminal(fidelity=options.fidelity)
   .terminal_theme()
   .hidden_cursor()
   .scoped_style()
) as term:
```

Writing such a `with` statement in every script does feel a little boilerplaty.
But that `with` also makes the script more robust by containing style changes.
Furthermore, it makes explicit when the script queries the terminal for its
color theme. That requires substantial I/O, as the application needs to write 18
ANSI escape sequences for requests and then read as many escape sequences for
responses. Since that leaves plenty of opportunities for things to go wrong,
exposing the operation in prettypretty's API simply is prudent interface design.

Prettypretty supports several more contextual operations, including for updating
the [`Terminal.window_title`], using the [`Terminal.alternate_screen`],
performing [`Terminal.batched_output`], and enabling
[`Terminal.bracketed_paste`]. You can perform them individually, each in its own
``with`` statement or, as shown in the above code example, fluently combine them
into a single `with` statement. In my experience, the latter does make context
in Python even more ergonomic and is well worth the extra engineering effort.


### 4. Adjust Styles to Reality

Once the terminal has been set up, the progress bar script uses the
[`current_sampler`]'s [`is_dark_theme`] to pick the attendant style and then
adjusts that style to the terminal's [`Terminal.fidelity`]:

```python
style = DARK_MODE_BAR if current_sampler().is_dark_theme() else LIGHT_MODE_BAR
style = style.prepare(term.fidelity)
```

Doing so once during startup means that the resulting styles are ready for
(repeated) display and incurs the overhead of color conversion only once.
Between [`Style.prepare`] and [`Sampler.adjust`], updating styles and colors to
match a given fidelity level also is positively easy.

In other words, "reality," as far as this progress bar is concerned, has two
dimensions:

 1. Dark or light mode
 2. The fidelity level

We should probably add one more dimension to the production version:

 3. Contrast level: regular or high

Dark/light mode, regular/high contrast should be familiar from the web. The five
fidelity levels combine knowledge about terminal capabilities, i.e., the range
of supported colors, the runtime context, e.g., I/O being redirected, and user
preferences, e.g., an aversion to or hunger for color, all into a simple five
step scale:

 1. Plain
 2. No color
 3. ANSI color
 4. 8-bit color
 5. Full or true color

That definition reflects one distinction that, unfortunately, most terminal
applications ignore: There is a meaningful and important difference between
*plain*, i.e., not styling terminal output and hence not using ANSI escape
sequences, and *no color*, i.e., not using colors but still leveraging ANSI
escape codes for screen management, cursor movement, and text attributes such as
bold. The former helps when running tools non-interactively or when capturing
output as a log. The latter helps when color would be a distraction. Terminal
applications should support both!


### 5. Display Content

With that, the progress bar script is ready for turning randomized progress
events progress bar updates. Each update assembles the rich text for the
progress bar, moves the (invisible) cursor to the beginning of the line, writes
the rich text to terminal output, and flushes the output:

```python
for percent in progress_reports():
   bar = format_bar(percent, style)
   term.column(0).rich_text(bar).flush()
   time.sleep(random.uniform(1/60, 1/10))
```

And that's it.


## What Does It Take?

Well. There still is more code to `prettypretty.progress`. But much of that code
is not specific to prettypretty. The scripts line breakdown looks like this:


| Function         | Lines of Code |
|:---------------- |:--------------|
| Imports          | 9             |
| Argument parser  | 18            |
| Module constants | 7             |
| Progress bar     | 14            |
| Progress reports | 7             |
| `main()`         | 23            |
| Calling `main()` | 2             |
| *Total*          | *80*          |


Only one line out of 14 for formatting the progress bar is specific to
prettypretty. Likewise, `main` requires just one line to write out the progress
bar. Startup has more substance, requiring 8 prettypretty-specific lines of
code. Then again, 6 of them are very generously spaced.

The point: With the right library support, separating styles from content for
terminals isn't very hard. The potential engineering and user benefits are
substantial. Of course, I am partial and believe that prettypretty is that right
library. But even if you disagree on that point, I hope you agree on the larger
one. And if you disagree on prettypretty being that right library, I want to
hear from you. Please do [share your constructive
feedback](https://github.com/apparebit/prettypretty/issues/new).


<div class=warning>

### Change Is in the Air

The previous paragraph requires temporary qualification: Prettypretty's
Rust-based API has been through three major iterations, one for the original
Python-based API, one for the rewrite in Rust, and one to clean up and
consolidate functionality. As a consequence, that part of prettypretty's API is
fairly polished. The current Python-based API for styles and rich text has not
had the benefit of my repeated attention. Fluent styles *feel* like the right
approach to me, yet the current iteration also *feels* a bit clunky to me. So, I
expect that part of the API to change as I port it over to Rust. But it won't be
a complete change of direction.

</div>

[`current_sampler`]: https://apparebit.github.io/prettypretty/python/prettypretty/theme.html#prettypretty.theme.current_sampler
[`is_dark_theme`]: https://apparebit.github.io/prettypretty/python/prettypretty/darkmode.html#prettypretty.darkmode.is_dark_theme
[`rich`]: https://apparebit.github.io/prettypretty/python/prettypretty/style.html#prettypretty.style.rich
[`RichText`]: https://apparebit.github.io/prettypretty/python/prettypretty/style.html#prettypretty.style.RichText
[`Sampler.adjust`]: https://apparebit.github.io/prettypretty/prettypretty/struct.Sampler.html#method.adjust
[`Style.prepare`]: https://apparebit.github.io/prettypretty/python/prettypretty/style.html#prettypretty.style.Style.prepare
[`Terminal`]: https://apparebit.github.io/prettypretty/python/prettypretty/terminal.html#prettypretty.terminal.Terminal
[`Terminal.alternate_screen`]: https://apparebit.github.io/prettypretty/python/prettypretty/terminal.html#prettypretty.terminal.Terminal.alternate_screen
[`Terminal.batched_output`]: https://apparebit.github.io/prettypretty/python/prettypretty/terminal.html#prettypretty.terminal.Terminal.batched_output
[`Terminal.bracketed_paste`]: https://apparebit.github.io/prettypretty/python/prettypretty/terminal.html#prettypretty.terminal.Terminal.bracketed_paste
[`Terminal.fidelity`]: https://apparebit.github.io/prettypretty/python/prettypretty/terminal.html#prettypretty.terminal.Terminal.fidelity
[`Terminal.window_title`]: https://apparebit.github.io/prettypretty/python/prettypretty/terminal.html#prettypretty.terminal.Terminal.window_title
