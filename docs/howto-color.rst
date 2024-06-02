How to Pretty-Pretty: Color
===========================

Let's see what prettypretty can do for your command line tools. I picked the
implementation of a progress bar for several reasons. First, I've been itching
to write one myself for quite a while now. Second, animated anything is nice and
shiny, i.e., makes for a good demo. Third, the script is simple enough to fit
into less than 100 lines of Python, yet complex enough to show off most major
features. The `complete script
<https://github.com/apparebit/prettypretty/blob/main/prettypretty/progress.py>`_
is part of prettypretty's distribution.

*How to Pretty-Pretty* has two parts. This part focuses on prettypretty's color
support. That includes a discussion of how prettypretty adjusts colors to
terminal capabilities and plenty of examples for manipulating colors. The other
part focuses on prettypretty-specific code in the progress bar script. You
probably want to start by working through the other part to get a good overview.
But if your learning styles favors fundamentals first, then this part probably
is a better start.


Visualizing Progress
--------------------

In either case, you probably want to get started by running the progress bar
script yourself. So please go through the usual incantations for creating a new
virtual environment, installing prettypretty into it, and running the progress
bar demo:

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

That last command actually executes the progress bar script. You should see a
green bar rapidly go from 0% to 100%. It may end up looking like this:

.. image:: figures/progress-bar-light.png
   :alt: A bright green progress bar at 100% against a white background
   :scale: 50 %

Or, if your terminal's color theme is a dark theme, it may end up looking more
like this:

.. image:: figures/progress-bar-dark.png
   :alt: A medium green progress bar at 100% against a black background
   :scale: 50 %


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
49``, which is three divisions by 255 away from the `sRGB
<https://en.wikipedia.org/wiki/SRGB>`_ color with tag ``srgb`` and coordinates
``0.01176, 0.59216, 0.19216`` (rounded to five decimals). Between the two color
spaces sRGB and Display P3, sRGB is the older and smaller one. It also has been
the default color space for monitors and the web for the longest time.

As the examples suggest, prettypretty's color representation includes a tag—to
identify the color format or space—and the coordinates. Supported formats,
including ``ansi``, ``eight_bit``, ``rgb6``, and ``rgb256``, have one or three
integer coordinates, whereas supported color spaces, including ``srgb``, ``p3``,
``oklab``, and ``oklch``, have three floating point coordinates. For RGB color
spaces such as ``srgb`` and ``p3`` the coordinates are normalized, i.e., range
from 0 to 1, inclusive. Prettypretty can convert between all of these formats
and color spaces, though some of the conversions are inherently lossy.

Prettypretty's basic color abstraction, :class:`.ColorSpec`, is just a record
with a ``tag`` and ``coordinates``. In addition, prettypretty has a fully
featured color class, :class:`.Color`, that adds a good number of methods to the
basic color record. To actually write out colors, you have a number of options:

 1. Invoke :class:`.ColorSpec` on a tag and coordinates tuple;
 2. Invoke :meth:`.ColorSpec.of` on a tag and coordinates tuple;
 3. Invoke :meth:`.ColorSpec.of` on a tag and coordinates but with the
    coordinates specified inline, thus avoiding the extra parentheses;
 4. Invoke :meth:`.ColorSpec.of` on an integer representing an ANSI or 8-bit
    color;
 5. Invoke :meth:`.ColorSpec.of` on three integers representing a 24-bit RGB
    color;
 6. Treat prettypretty's main methods expecting colors, :meth:`.StyleSpec.fg`,
    :meth:`.StyleSpec.bg`, :meth:`.Terminal.fg`, and :meth:`.Terminal.bg`, as if
    they were :meth:`.ColorSpec.of`;
 7. Treat :class:`.Color`'s constructor as if it was :meth:`.ColorSpec.of`;
 8. Invoke :class:`.Color` on a string literal with the color in hexadecimal, X
    Windows, or functional notation.

To make this all work consistently, the implementations of :class:`.Color`,
:meth:`.StyleSpec.fg`, :meth:`.StyleSpec.bg`, :meth:`.Terminal.fg`, and
:meth:`.Terminal.bg` all delegate to :meth:`.ColorSpec.of`. The code below
illustrates these options on the example of setting a terminal's foreground
color to the primary greens of 8-bit and 24-bit colors, which really are one and
the same.

.. code-block:: python

   from prettypretty.color.spec import ColorSpec
   from prettypretty.color.object import Color
   from prettypretty.terminal import Terminal

   # Create terminal, don't let any styles leak
   with Terminal().scoped_style() as term:

      # 8-bit color 46 is primary green of embedded 6x6x6 RGB cube
      term.fg(ColorSpec('eight_bit', (46,)))
      term.fg(ColorSpec.of('eight_bit', (46,)))
      term.fg(ColorSpec.of(46))
      term.fg(ColorSpec.of('eight_bit', 46))
      term.fg(46)
      term.fg('eight_bit', 46)
      term.fg('rgb6', 0, 5, 0)

      # '#00FF00' is the primary green of sRGB
      term.fg('srgb', 0, 1, 0)
      term.fg('rgb256', 0, 255, 0)
      term.fg(0, 255, 0)
      term.fg(ColorSpec.of(0, 255, 0))
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
   >>> rgb256 = get_converter('srgb', 'rgb256')(*within_srgb_gamut)
   >>> rgb256
   (0, 251, 41)
   >>> eight_bit = get_converter('srgb', 'eight_bit')(*within_srgb_gamut)
   >>> eight_bit
   (46,)
   >>> get_converter('eight_bit', 'rgb6')(*eight_bit)
   (0, 5, 0)

The :func:`.get_converter` function in the above example code can instantiate a
converter for any pair of color formats and spaces supported by prettypretty. As
the last example illustrates, that includes conversions implemented by the
:mod:`prettypretty.color.lores` module.

The example shows the 24-bit RGB components for the gamut-mapped color as well.
If your terminal supports truecolor, that should be the color of the progress
bar when running in light mode.

If you use :class:`.Color`, the above becomes a bit more uniform and hence
simpler:

.. code-block:: python

   >>> from prettypretty.color.object import Color
   >>> str(Color("p3(0, 1, 0)"))
   'p3(0.0, 1.0, 0.0)'
   >>> str(Color("p3(0, 1, 0)").to("srgb"))
   'srgb(-0.5116, 1.0183, -0.31067)'
   >>> str(Color("p3(0, 1, 0)").to("srgb").to_gamut())
   'srgb(0.0, 0.98576, 0.15974)'
   >>> str(Color("p3(0, 1, 0)").to("srgb").to_gamut().to("rgb256"))
   'rgb(0, 251, 41)'
   >>> str(Color("p3(0, 1, 0)").to("srgb").to_gamut().to("eight_bit"))
   'eight_bit(46)'

Originally, the conversion to 8-bit colors considered all 256 8-bit colors. But
:doc:`experiments with high-resolution color ranges <hires-slices>` showed ugly
outliers corresponding to the 16 extended ANSI colors embedded in 8-bit color.
They were the closest colors, but just didn't match the results for close-by
colors well, resulting in visually noticeable outliers. To ensure more
harmonious results, I eliminated them as candidates when converting to 8-bit
color.

When converting to ANSI, prettypretty must of course consider the 16 extended
ANSI colors as candidates. But to do so, it must also convert them to Oklab. The
problem is that there is no standard for their RGB color values and, even if
there was, it wouldn't make much of a difference because most terminals modify
the ANSI colors with themes. Therefore, prettypretty uses ANSI escape codes to
query a terminal for the color values for the current theme and then uses those
values when converting to ANSI. That does result in different colors depending
on the terminal and its current theme. But as the :doc:`experiments with 8-bit
color ranges <index>` across different terminals demonstrate, that's actually a
unique strength of prettypretty, resulting in visually more consistent results.

Assuming that your terminal supports at least 8-bit colors, you can use the
``--ansi`` command line option to restrict the progress bar colors to just the
16 extended ANSI colors.

.. code-block:: console

   $ python -m prettypretty.progress --ansi

The progress bar should use ANSI colors 2 or 10, i.e., the regular or bright
green. But the result very much depends on your current terminal theme. If you
are so inclined, you can take this all the way to ``--nocolor``. With that
command line option, the progress bar is a stark black or white (or whatever
color your current terminal theme includes for the default foreground color),
just like the rest of the output.
