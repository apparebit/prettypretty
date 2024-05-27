How to Pretty-Pretty
====================

Let's see what prettypretty can do for your command line tools. I picked the
implementation of a progress bar for several reasons. First, I've been itching
to write one myself for quite a while now. Second, animated anything is nice and
shiny, i.e., makes for a good demo. Third, the script is simple enough to fit
into less than 100 lines of Python, yet complex enough to show off most major
features. The `complete script
<https://github.com/apparebit/prettypretty/blob/main/prettypretty/progress.py>`_
is part of prettypretty's distribution.


Visualizing Progress
--------------------

So, a good start for you is to create a new virtual environment, install
prettypretty into it, and run the progress bar demo:

.. code-block:: console

   $ mkdir progress
   $ cd progress
   $ python -m venv .venv
   $ source .venv/bin/activate
   $ python -m pip install prettypretty
   Downloading prettypretty-0.3.0-py3-none-any.whl (64 kB)
   Installing collected packages: prettypretty
   Successfully installed prettypretty-0.3.0
   $ python -m prettypretty.progress

(The above command line incantations work just as written on Linux and macOS.
But you may have to adjust them somewhat, if you use a package manager other
than pip or are running Windows. I trust you know what to do differently.)

That last command actually executes the demo script. You should see a progress
bar rapidly go from 0% to 100%. It may end up looking like this:

.. image:: figures/progress-bar-light.png
   :alt: A bright green progress bar at 100% against a white background

Or, if your terminal's color theme is a dark theme, it may end up looking more
like this:

.. image:: figures/progress-bar-dark.png
   :alt: A medium green progress bar at 100% against a black background


P3, sRGB, 8-bit Color, Oh My
----------------------------

If you compare the two screenshots, you may notice that the progress bars have
different shades of green, with the dark mode version less bright and containing
some blue. That difference is very much intentional. To show off prettypretty's
color conversions, I picked an aggressively green green for the light mode, the
primary green for the `Display P3 <https://en.wikipedia.org/wiki/DCI-P3>`_ color
space, i.e., the color with tag ``p3`` and coordinates ``0, 1, 0`` when using
prettypretty. Since bright colors seem even brighter against a dark background,
that green most certainly won't do for dark mode and I picked a second, darker
green as well, i.e., the color with tag ``rgb256`` and coordinates ``3, 151,
49``, which is three divisions by 255 away from the color with tag ``srgb`` and
coordinates ``0.01176, 0.59216, 0.19216`` (rounded to five decimals).

As the examples suggest, prettypretty's color representation includes a tag—to
identify the color format or space—and the coordinates. Supported formats,
including ``ansi``, ``eight_bit``, ``rgb6``, and ``rgb256``, have one or three
integer coordinates, whereas supported color spaces, including ``srgb``, ``p3``,
``oklab``, and ``oklch``, have three floating point coordinates. For the RGB
color spaces, including ``srgb`` and ``p3``, the coordinates are normalized,
i.e., range from 0 to 1, inclusive. Prettypretty can convert between all of
these formats and color spaces, though some of the conversions are inherently
lossy.

Prettypretty's basic color abstraction, :class:`.ColorSpec`, is just a record
with a ``tag`` and ``coordinates``. Prettypretty also has a fully featured color
class, :class:`.Color`, that adds a good number of methods to the basic color
specification record. To actually write out colors, you have a number of
options:

 1. Invoke :class:`.ColorSpec` on a tag and coordinates tuple;
 2. Use the :meth:`.ColorSpec.of` helper method, which gets rid of extra
    parentheses by accepting coordinates inline, as arguments;
 3. Treat prettypretty's main methods expecting colors, :meth:`.StyleSpec.fg`,
    :meth:`.StyleSpec.bg`, :meth:`.Terminal.fg`, and :meth:`.Terminal.bg`, as if
    they were :meth:`.ColorSpec.of`;
 4. Invoke :class:`.Color` on a string literal with the color in hexadecimal, X
    Windows, or functional notation.

The code below illustrates all four options on the example of setting a
terminal's foreground color to the primary greens of 8-bit and 24-bit colors,
which really are one and the same color.

.. code-block:: python

   from prettypretty.color.spec import ColorSpec
   from prettypretty.color.object import Color
   from prettypretty.terminal import Terminal

   # Create terminal, don't let any styles leak
   with Terminal().scoped_style() as term:

      # 8-bit color 46 is primary green of embedded 6x6x6 RGB cube
      term.fg(ColorSpec('eight_bit', (46,)))
      term.fg(ColorSpec.of(46))
      term.fg(ColorSpec.of('eight_bit', 46))
      term.fg(46)
      term.fg('rgb6', 0, 5, 0)

      # '#00FF00' is the primary green of sRGB
      term.fg('srgb', 0, 1, 0)
      term.fg('rgb256', 0, 255, 0)
      term.fg(Color('#00ff00'))
      term.fg(Color('rgb:0000/ffff/0000'))
      term.fg(Color('srgb(0, 1, 0)'))

It appears that Kermit was wrong. It's pretty easy being green after all.

What isn't so easy is locking down the exact shade of green being displayed. In
fact, that's pretty much out of our hands. If you have done any web development,
then this should be familiar: You can express an aspirational goal for the
appearance of your web pages, but the actual rendered result very much depends
on the current device, web browser, and network connectivity. It works pretty
much the same way when it comes to color and terminals—except terminals don't do
graceful degradation, let alone progressive enhancement. Prettypretty does that
for you!

Against that background, it won't come as too much of a surprise for you when I
tell you that the above screenshots do *not* show the green primary of Display
P3 nor the color we now know to write as this:

.. code-block:: python

   >>> from prettypretty.color.spec import ColorSpec
   >>> from prettypretty.color.object import Color
   >>> ColorSpec('rgb256', (3, 151, 49))
   ColorSpec(tag='rgb256', coordinates=(3, 151, 49))
   >>> ColorSpec.of('rgb256', 0x03, 0x97, 0x31)
   ColorSpec(tag='rgb256', coordinates=(3, 151, 49))
   >>> Color('#039731')
   Color(tag='rgb256', coordinates=(3, 151, 49))

Instead, the first screenshot shows the primary green of sRGB and the second
screenshot shows the color we now know to write as this:

.. code-block:: python

   >>> from prettypretty.color.spec import ColorSpec
   >>> from prettypretty.color.object import Color
   >>> ColorSpec.of(28)
   ColorSpec(tag='eight_bit', coordinates=(28,))
   >>> ColorSpec.of('rgb6', 0, 2, 0)
   ColorSpec(tag='rgb6', coordinates=(0, 2, 0))
   >>> ColorSpec.of('rgb256', 0, 135, 0)
   ColorSpec(tag='rgb256', coordinates=(0, 135, 0))
   >>> Color('#008700')
   Color(tag='rgb256', coordinates=(0, 135, 0))
   >>>
   >>> # They all are the same color:
   >>> import prettypretty.color.lores as lores
   >>> lores.eight_bit_to_rgb6(28)
   (0, 2, 0)
   >>> lores.eight_bit_to_rgb256(28)
   (0, 135, 0)

The last several lines above use the :mod:`prettypretty.color.lores` module,
which contains functions for handling low-resolution colors including for
converting them.

How did we get there? Conceptually, it's pretty straight-forward. Upon
initialization of its :class:`.Terminal` abstraction, prettypretty makes an
educated guess about the terminal's color capabilities and, from then on out, it
automatically checks every color before using it. If a color *cannot* be
displayed on the current terminal, prettypretty first converts it to the next
best matching color that *can* be displayed.


Making Colors Renderable
------------------------

In practice, it's quite a bit more involved. To begin with, terminals support
either ANSI colors, 8-bit colors, or truecolor, which is the same as 24-bit RGB,
tagged ``rgb256`` in prettypretty. Next, prettypretty uses different techniques
for converting colors from arbitrary color spaces such as Display P3 to
sRGB/RGB256 and for converting sRGB colors to 8-bit or ANSI colors. Of course,
if it needs to convert colors from an arbitrary color space to 8-bit or ANSI
colors, it successively employs both techniques.

**To convert to sRGB**, prettypretty first performs the actual conversion
between color spaces and then checks whether the result is in gamut, i.e.,
whether the color is part of the sRGB color space. For example, the green
primary for Display P3 converts to the coordinates -0.5116, 1.01827, -0.31067 in
sRGB (rounded to 5 decimals). Since RGB color space coordinates need to fit into
the normal range between 0 and 1, these coordinates are pretty glaringly out of
gamut.

If the coordinates are out of gamut, as in the example, prettypretty uses the
`gamut mapping algorithm <https://www.w3.org/TR/css-color-4/#gamut-mapping>`_
from CSS Color 4 to find the next best color in sRGB. In the example, that color
has the sRGB coordinates 0, 0.98576, 0.15974 (again rounded to 5 decimals). In
other words, Display P3's green primary doesn't even map to sRGB's green
primary, but to a color with a small but non-negligible blue component. The
reason the first screenshot nonetheless displays sRGB's green primary is the
next conversion.

**To convert to ANSI or 8-bit color**, prettypretty exhaustively compares the
color to be converted against all of the 16 extended ANSI colors or 240 of the
256 8-bit colors and picks the color that is closest. Doing so requires a shared
color space and a meaningful distance metrics. Prettypretty uses the
perceptually uniform Oklab color space and its ΔE metric, which is just the
Euclidian distance between coordinates.

My default terminal, Apple's Terminal.app, only supports 8-bit color, not
truecolor. Hence, the above conversion to a gamut-mapped sRGB color is
insufficient and prettypretty needs to further convert that color to an 8-bit
color. The result of the attendant search across 8-bit colors is color 46, which
corresponds to the green primary of the 6x6x6 RGB cube embedded in 8-bit color
as well as the green primary of sRGB. You can try this out yourself:

.. code-block:: python

   >>> from prettypretty.color.conversion import get_converter
   >>> from prettypretty.color.gamut import map_into_gamut
   >>> srgb = get_converter('p3', 'srgb')(0, 1, 0)
   >>> [round(c, 5) for c in srgb]
   [-0.5116, 1.01827, -0.31067]
   >>> within_srgb_gamut = map_into_gamut('srgb', srgb)
   >>> [round(c, 5) for c in within_srgb_gamut]
   [0, 0.98576, 0.15974]
   >>> eight_bit = get_converter('srgb', 'eight_bit')(*within_srgb_gamut)
   >>> eight_bit
   (46,)
   >>> get_converter('eight_bit', 'rgb6')(*eight_bit)
   (0, 5, 0)

The :func:`.get_converter` function can instantiate a converter for any pair of
color formats and spaces supported by prettypretty. As the last example
illustrates, that includes conversions implemented by the
:mod:`prettypretty.color.lores` module.

Originally, the conversion to 8-bit color compared to all 256 colors. But
:doc:`experiments with color ranges <hires-slices>` showed ugly outliers
corresponding to the 16 extended ANSI colors embedded in 8-bit color. They were
the closest colors at times, but just didn't match the other colors well. To
ensure more harmonious results, I eliminated them as candidates when converting
to 8-bit color.

When converting to ANSI, prettypretty must of course consider the 16 extended
ANSI colors as candidates. But to do so, it must also convert them to Oklab. The
problem is that there is no standard for their color values and, even if there
was, it wouldn't make much of a difference because most terminals modify the
ANSI colors with themes. Prettypretty uses ANSI escape codes to query a terminal
for all color values for the current theme and relies on those values when
converting to ANSI, thus yielding a color that is optimal for the current
terminal.

The progress bar demo includes command line options to further restrict colors.
Try running it with ``--ansi`` or ``--nocolor`` like so:

.. code-block:: console

   $ python -m prettypretty.progress --ansi


Terminal Style
--------------

While color is important, terminals also support a few attributes for styling
text, including making the text appear bold or faint, using italics, or
underlined.



.. code-block:: python

    WARNING = Style.bold.fg(0).bg(220)

Only define complete styles. Don't bother with styles that undo or incrementally
transition a style. You can automatically compute them with Python's negation
``~`` and subtraction ``-`` operators. In particular, the style ``~style`` takes
a terminal in style ``style`` and restores the default style, and ``style2 -
style1`` incrementally transitions from the first to the second style.


The last line of ``format_bar`` illustrates the use of styles. Since their
string representation is the ANSI escape sequence effecting that style, you
could convert it to string. But the more robust option is to simply build a
sequence intermingling text and styles. If you use prettypretty's
:class:`.RichText` may be a little more performant but really any sequence
works.

.. code-block:: python

    return RichText.of('  ┫', style, bar, ~style, '┣', f' {percent:5.1f}%')

As the example nicely illustrates, to undo a style you just invert the style
specification. If you need to go from one style, ``style1``, to another,
``style2``, you could write the inverted ``~style1`` followed by ``style2``. But
that may unnecessarily reset and set terminal attributes. Instead just write
``style2 - style1``, which is the difference between the two styles.

The demo script creates the terminal object, possibly overwriting its color
fidelity, then queries the terminal for its current color theme, hides the
cursor, and commits to resetting all styles at the end of the ``with`` block.

.. code-block:: python

    with (
        Terminal(fidelity=options.fidelity)
        .terminal_theme()
        .hidden_cursor()
        .scoped_style()
    ) as term:

I strongly recommend always reading the terminal theme and always scoping
styles.

Prettypretty can display any sequence of style and text. But to correctly render
colors, it needs to check each style and possibly convert one or both contained
colors. But if styles are really reused, doing the same conversions over and
over again makes little sense. Instead, you can precompute the styles for the
current terminal as shown in the example:

.. code-block:: python

    style = DARK_MODE_BAR if is_dark_theme() else LIGHT_MODE_BAR
    style = style.prepare(term.fidelity)


Much of the demo script should be self-explanatory and is not specific to
prettypretty at all. The two exceptions are the use of :data:`.Style` and
:class:`.Terminal`. A single terminal style collects the stylistic attributes of
text and the two foreground and background colors.

The main loop is amazingly simple. For each percentage value, it first formats
the progress bar from scratch. Then it instructs the terminal to move the
(invisible) cursor back to the start of the line, to display the progress bar,
and to flush the output. Finally, it rests from all the work.

.. code-block:: python

    for percent in progress_reports():
        bar = format_bar(percent, style)
        term.column(0).rich_text(bar).flush()
        time.sleep(random.uniform(1/60, 1/10))
