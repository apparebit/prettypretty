Color Conversions
=================

.. warning::

   Converting colors between color spaces is not guaranteed to result in colors
   that are within the target color space's gamut. For example, ``p3(0, 1, 0)``
   (to use prettypretty's function syntax for colors) is well outside of sRGB's
   gamut. Like Color.js, prettypretty does *not* automatically clip or gamut-map
   colors and instead preserves their out-of-gamut coordinates. Hence a primary
   green in P3 converts to ``srgb(-0.5116, 1.0183, -0.31067)``, with G slightly
   out of gamut and R and B very much out of gamut.

   In other words, if you need in-gamut colors, you *must* explicitly clip or
   gamut-map colors.


For starters, we consider only color formats and spaces that are *not*
terminal-specific.

With exception of the XYZ color space, all other color formats and spaces have a
*base* color space, which may be XYZ. Taking together, they form a tree with XYZ
as root. For every such color format or space, prettypretty also includes
handwritten functions that convert from and to the base color space. For
example, sRGB has linear sRGB as its base color space and
:func:`.srgb_to_linear_srgb` converts to the base whereas
:func:`.linear_srgb_to_srgb` converts from the base. Since the base tree
connects all color formats and spaces with each other and each base edge can be
traversed in both directions, all color formats and spaces can also be converted
into each other, simply by repeatedly performing primitive conversions.

We can automatically determine the shotest sequence of such conversion functions
using `Djikstra's shortest path algorithm
<https://en.wikipedia.org/wiki/Dijkstra%27s_algorithm>`_. Alas, the base
relationship does not form an arbitrary graph but a tree rooted in XYZ. Hence,
we can do better.

In particular, we can trivially precompute the distance of each color format or
space from the root XYZ and store both base pointer and root distance in a
table. Using that table, our optimized version elaborates the paths from the
source and target formats or spaces towards the root. First, it syncs up both
paths by elaborating the path with nodes further from the root until both paths'
last nodes are equidistant from the root. Second, it elaborates both paths in
lockstep until their last nodes are the same.

Since we are moving inwards along the branches of a tree, we are guaranteed to
converge, i.e., at the root of the tree. However, since we are also moving
inwards while maintaining the same distance from the root, we are guaranteed to
converge on the shared node furthest from the root and hence the shortest path
between the source and target.

The :func:`.converter` function implements just that algorithm to determine the
necessary sequence of to/from base conversions but packages that sequence in a
closure for easier application. Furthermore, since the number of color formats
and spaces is relatively small, i.e., currently 8 and unlikely to grow beyond
12, ``converter`` does one better and caches each closure upon creation. That
way, very conversion needs to be synthesized only once.

The following table summarizes the handwritten conversion functions:

===============  ======  ====  =========  ==  =======  =====  =====  ===
▼ From/To ►      RGB256  sRGB  lin. sRGB  P3  lin. P3  Oklch  Oklab  XYZ
===============  ======  ====  =========  ==  =======  =====  =====  ===
**RGB256**            ━     1
**sRGB**              1     ━          1
**Linear sRGB**             1          ━                               1
**P3**                                     ━       1
**Linear P3**                              1       ━                   1
**Oklch**                                                  ━      1
**Oklab**                                                  1      ━    1
**XYZ**                                1           1              1    ━
===============  ======  ====  =========  ==  =======  =====  =====  ===


Low-Resolution Formats
----------------------

The basic approach to integrating the three terminal-specific color formats,
``ansi``, ``eight_bit``, and ``rgb6``, is the same as for the other color
formats and spaces: For each terminal-specific color format, we write a pair of
functions that convert to and from higher-resolution color. By putting those
functions first and/or last in the sequence of conversions and otherwise routing
as described above, we can convert between all color formats and spaces,
terminal-specific formats included.

In practice, however, there are several complications:

 1. The 16 extended ANSI colors have no widely accepted mapping to
    high-resolution colors and usually are configured through themes. Colors are
    usually specified in ``#<hexdigits>`` notation, i.e., RGB256.
 2. Xterm's formulae for converting the 6x6x6 RGB cube and 24-step gray gradient
    *to* RGB256 are widely accepted. But there are no established techniques for
    converting back.
 3. Prettypretty's color themes are a form of state duplication, with all
    attendant problems. Notably, if a manually configured theme does not match
    the terminal's current theme, the results of color manipulation may just
    look awful.
 4. ANSI escape sequences support querying the terminal for currently configured
    colors. But the results are in xterm's ``rgb:`` notation with *four*
    hexadecimal digits per coordinate, which is well beyond RGB256's resolution.

Prettypretty indeed addresses the fundamental conversion challenge for the 16
extended ANSI colors by supporting color themes, with each color having an
arbitrary color format or space. Since manual color theme configuration is
problematic, it also supports dynamically querying terminals for configured
colors and preserves the resolution of the responses as sRGB colors.

To convert low-resolution to high-resolution colors, prettypretty currently
targets sRGB as the "base" color space for all low-resolution colors. For the
6x6x6 RGB cube and 24-step gray gradient, it simply uses the established
formulae to convert to RGB256 and then performs the trivial conversion to sRGB.
For the 16 extended ANSI colors, it looks up the target color in the current
color theme and, if it is not RGB256 or sRGB, does a high-resolution conversion
to sRGB. Since high-resolution conversion does not clip or gamut-map colors,
this conversion does preserve the represented color. The result should probably
be cached, but it currently is not.

For converting high-resolution to low-resolution colors, there is not
established algorithm. Since the low-resolution colors, by definition, do not
comprise that many colors, exhaustive search becomes a realistic possibility and
is just the technique used by prettypretty. In particular, prettypretty compares
the high-resolution source color with high-resolution versions of all
low-resolution colors (thus assuming that the conversion from low-resolution to
high-resolution is functional) and returns the color with the smallest distance.
All comparisons are performed in Oklab, which is perceptually uniform.

To speed up conversion to low-resolution color, prettypretty uses look-up tables
for the Oklab-equivalent colors. Those tables are initialized lazily, on demand.
When targeting 8-bit color, prettypretty does not include the 16 extended ANSI
colors as candidates because experiments with color gradients resulted in
low-resolution gradients disrupted by one of those 16 colors, which just
happened to be closer.

The exhaustive search for closest matching low-resolution color may still
produce suboptimal results because it optimizes for Euclidian distance in Oklab.
In other words, it treats a difference in lightness just as it treats the same
difference in one of the two color axes. That may indeed be the right approach
for small color differences but it's not clear that is the right approach at the
granularity of 8-bit colors.
