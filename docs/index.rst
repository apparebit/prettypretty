Pretty 🌸 Pretty
================

Prettypretty helps build awesome terminal user interfaces in Python. Notably, it
incorporates a powerful and general color library. The resulting, near seemless
integration of 1970s archaic but beloved ANSI escape codes for terminal styling
with 2020s color science, notably via the `Oklab perceptual color space
<https://bottosson.github.io/posts/oklab/>`_, is unique to prettypretty and
helps your application automatically adapt its styles to a user's current color
theme, dark or light mode, and color preferences. So, what are you waiting for?
Switch to prettypretty for all your terminal styling needs. Prettypretty is
awesome!

The Rust version of prettypretty has its own package, uhm, `crate
<https://crates.io/crates/prettypretty>`_ and its own `documentation
<https://docs.rs/prettypretty/latest/prettypretty/index.html>`_. The latter even
includes color swatches with the example code.

.. image:: https://github.com/apparebit/prettypretty/actions/workflows/ci.yml/badge.svg

.. image:: https://github.com/apparebit/prettypretty/actions/workflows/gh-pages.yml/badge.svg


.. toctree::
   :maxdepth: 1
   :hidden:

   self

.. toctree::
   :maxdepth: 1
   :caption: How to Pretty-Pretty

   Style <howto-style>
   Color <howto-color>
   Mo' Color <hires-slices>

.. toctree::
   :maxdepth: 1
   :caption: API

   apidocs/prettypretty
   apidocs/tools

.. toctree::
   :maxdepth: 1
   :caption: Links
   :hidden:

   Repository <https://github.com/apparebit/prettypretty>
   Package <https://pypi.org/project/prettypretty/>
   Documentation <https://apparebit.github.io/prettypretty/>


A Drab Reality
--------------

Building delightful terminal user interfaces requires coping with significant
variability. Any given terminal may support only 16 or 256 colors. Furthermore,
its appearance has probably been customized, often using a dark theme even if
the OS isn't in dark mode. Such deep configurability is a must. That's because,
in part, terminals' primary users like to tinker. Also, some of those users have
strongly held convictions about their tools' appearances, with some favoring a
low-contrast monochrome and others a psychedelic explosion of rainbows.

So, determining the terminal's level of color support is only a first step out
of many. A command line application also needs to take the current color theme
into account, whether that theme is dark or light, and whether the user approves
of color or not. Assembling all that information is three more steps in the
right direction, but an application still needs *to do something* with that
information, i.e., accordingly adjust its styles. Or, it can keep looking drab
and sad.


Enter Pretty 🌸 Pretty
----------------------

Prettypretty helps you bring awesome color to this drab world: The library takes
care of determining color support, color theme, and the polarity of the theme.
It also supports user-directed overrides. Once assembled, prettypretty leverages
this information to automatically adjust your application's styles to the
current runtime context. And for that, it uses the latest in color spaces and
algorithms, bringing the benefits of the Oklab perceptual color space and CSS
Color algorithms for contrast and color adjustment to terminals.


Pretty 🌸 Pretty Illustrated
----------------------------

The following screen shots illustrate some of the benefits of prettypretty's
color management, showing off its algorithms for contrast and color adjustment
for the 6x6x6 RGB cube embedded inside 8-bit terminal colors. The first
screenshot demonstrates prettypretty's support for maximizing text contrast by
comparing against backgrounds in all 216 colors from the 6x6x6 RGB cube of 8-bit
terminal colors.

.. image:: figures/rgb6-background.png
   :alt: The 6x6x6 RGB cube used for background colors


The second screenshot illustrates the reverse challenge, with prettypretty
picking the background color to maximize contrast for text in all 216 colors
from the 6x6x6 RGB cube. If you compare with the previous screenshot, you may
notice that prettypretty's contrast metric, a perceptual contrast metric
surprisingly similar to `APCA <https://github.com/Myndex/apca-w3>`_, is *not*
symmetric. That's just why it is more accurate than, say, the WCAG 2.0 formula.

.. image:: figures/rgb6-text.png
   :alt: The 6x6x6 RGB cube used for text colors


The third screenshot illustrates prettypretty's support for finding the
perceptually closest color out of several colors. That's just how prettypretty
performs high-quality downsampling, in this case turning the 216 colors from the
6x6x6 RGB cube into 16 extended ANSI colors.

.. image:: figures/rgb6-ansi-macos.png
   :alt: The 6x6x6 RGB cube used for text colors


Since almost all terminals have robust support for theming just those 16
extended ANSI colors, prettypretty doesn't just use some hardcoded set of colors
but has built-in support for color themes. You can of course configure and
reconfigure the current colors as you please. But prettypretty can do one
better: It can automatically query a terminal for the current theme colors.
The fourth screenshot illustrates the impact. When running in iTerm2 instead of
macOS Terminal, prettypretty makes good use of the brighter colors in one of
iTerm's builtin themes and generates a substantially different grid!

.. image:: figures/rgb6-ansi-iterm2.png
   :alt: The 6x6x6 RGB cube used for text colors


Here is the same grid, this time running in Gnome Terminal in a Linux virtual
machine with one of the default light themes. As you can clearly see, colors are
very much different again.

.. image:: figures/rgb6-ansi-ubuntu.png
   :alt: The 6x6x6 RGB cube downsampled to ANSI


Overall, prettypretty has robust support for:

  * Automatically determining a terminal's level of color support;
  * Automatically determining a terminal's color theme;
  * Automatically determining whether a color theme is light or dark;
  * Automatically determining whether the OS is in light or dark mode;
  * Automatically adjusting terminal styles to terminal capabilities;
  * Finding the closest color out of several;
  * Using that search to perform high-quality downsampling to 8-bit
    and ANSI colors;
  * Maximizing label contrast for a given background color;
  * Maximizing background contrast for a given text color;
  * Converting colors between sRGB, Display P3, Oklab, Oklch, and a
    few other color spaces;
  * Gamut mapping out-of-gamut colors;
  * Finding the closest color out of several;
  * Using that search to perform high-quality downsampling to 8-bit
    and ANSI colors;

Are you still using chalk or other, poor substitutes for real terminal color?
It's time to switch to prettypretty!


Acknowledgements
----------------

Implementing this package's color support was a breeze. In part, that was
because I had built a prototype before and knew exactly what I was going for. In
part, that was because I copied many of the nitty-gritty color algorithms and
conversion matrices from the most excellent `Color.js <https://colorjs.io>`_
library by `Lea Verou <http://lea.verou.me/>`_ and `Chris Lilley
<https://svgees.us/>`_. Theirs being a JavaScript library and mine being a
Python package, there are many differences, small and not so small. But without
Color.js, I could not have implemented color support in less than a week. Thank
you!

Now how about bringing terminal colors to `Color.js <https://colorjs.io>`_?
