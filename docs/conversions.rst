Color Conversions
=================

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

In practice, however, there are a few complications:

 1. The extended ANSI colors have no widely accepted mapping to high-resolution
    colors and usually are configured through themes. Hence prettypretty also
    supports color themes. The colors of VGA text mode provide the default.
 2. With theme support in place, converting low-resolution colors to
    higher-resolution RGB256 is straightforward. In particular, xterm's formulae
    for converting the 6x6x6 RGB cube and 24-step gray gradient embedded in
    8-bit color are widely used.
 3. However, there are no good formulae for converting RGB256 back to
    low-resolution. To yield good results, prettypretty performs the
    downconversion in Oklab, finding the closest color amongst the 16 extended
    ANSI colors when converting to ANSI, the 240 RGB6 and gray gradient colors
    when converting to 8-bit, and the 216 RGB6 colors when converting to RGB6.
    The conversion to 8-bit colors ignores the 16 ANSI colors because they stick
    out when converting gradients of color.
 4. While not excessively large, the look-up tables for downconversion are
    created only on demand and thereafter cached.
 5. Similarly, the modules for low-resolution color conversions and color themes
    aren't particularly large. Still, the module implementing generic
    conversions only loads the module implementing low-resolution color
    operations when strictly needed. That nicely avoids an error due to circular
    module dependencies as well.
