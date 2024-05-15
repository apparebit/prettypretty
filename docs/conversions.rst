Color Conversions
=================

We first consider only color formats and spaces that are *not*
terminal-specific. With exception of the XYZ color space, all other color
formats and spaces have exactly one base color space, which may be XYZ. For
every such color format or space, prettypretty includes handwritten functions
that convert from and to the base color space. For example, sRGB has linear sRGB
as its base color space and :func:`.srgb_to_linear_srgb` converts to the base
and :func:`.linear_srgb_to_srgb` converts from the base. Since following from
base to base invariably ends in the XYZ color space, all color formats and
spaces that are not terminal-specific can be converted into each other by
repeatedly applying one of the handwritten conversions.

`Djikstra's shortest path algorithm
<https://en.wikipedia.org/wiki/Dijkstra%27s_algorithm>`_ could be used to
automatically determine the shortest sequence of such conversion functions. But
the base relationship does not form an arbitrary graph but rather a tree rooted
in XYZ. Hence we can trivially precompute the distance of each format or space
from XYZ and store base and distance in a look-up table (LUT). The optimized
algorithm elaborates the paths from source and target formats or spaces to the
root by using this LUT. First, it syncs up both paths by elaborating the path
whose last node is further from the root. Second, once synced up, it keeps
elaborating one node for each path until both paths share the last node, which
is the shared node closest to both source and target.

The :func:`.converter` function implements just that algorithm to determine the
necessary sequence of to/from base conversions but returns a closure for easier
application. Since the number of color formats and spaces is relatively small
(currently 8 and unlikely to grow beyond 11), :func:`.converter` does one better
and caches each closure upon creation. That way, every conversion needs to be
synthesized only once.

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


Low-Resolution formats
----------------------

Prettypretty supports three more, terminal-specific color formats, which use the
``ansi``, ``eight_bit``, and ``rgb6`` tags. Since the three terminal-specific
color formats also are low-resolution formats, conversions to high-resolution
colors are far simpler than conversions from high-resolution colors, which are
inherently lossy and harder to get right. This is directly reflected in
prettypretty's API: All three low-resolution formats convert to RGB256, but only
Oklab converts to the three low-resolution formats.

For ANSI colors, conversions in either direction depend on the current color
theme. The default is VGA.

Conversion to low-resolution colors works by finding the closes color in Oklab
that corresponds to the 16 extended ANSI colors when converting to ANSI, the 240
RGB6 and gray gradient colors when converting to 8-bit, and the 216 RGB6 colors
when converting to RGB6. The implementation creates the necessary look-up tables
lazily, on demand, and caches them thereafter. The conversion to 8-bit colors
ignores the 16 ANSI colors because those 16 colors unpleasantly stick out when
converting an entire gradient of colors.
