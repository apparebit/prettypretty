Color Conversions
=================


To convert pretty much any color format and space into any other color format
and space, we only need a linear number of conversion functions and not ``n *
n`` of such functions.

First, for most color formats and spaces, we implement two pairs of conversions
from and into other color spaces. The resulting graph of conversion functions is
a sequence with directed edges in both directions. In other words, we can compose
conversions to reach spaces that are not directly connected but still part of the
sequence.

Second, we ensure that all such sequences have one color space in common, so
that all color formats and spaces become reachable by composing conversions
functions. In particular, since the conversion graph has cycles, we use
`Djikstra's shortest path algorithm
<https://en.wikipedia.org/wiki/Dijkstra%27s_algorithm>`_ to find the shortest
suitable sequence of primitive conversion functions.

In fact, that's just what :func:`prettypretty.color.conversion.route` does—after
taking care of some trivial cases.

The table below shows the conversions that *are* implemented, using **1** for
conversions that handle related color formats or spaces and **n** for pre-fused
functions designed for optimization.

==============  ====  =====  ====  ======  ====  ======  ===  ==  ======  =====  =====
          To ►  ansi  eight  rgb6  rgb256  srgb  linear  xyz  p3  linear  oklab  oklch
                      bit                        srgb             p3
▼ From                cube
==============  ====  =====  ====  ======  ====  ======  ===  ==  ======  =====  =====
ansi               ━
eight_bit_cube            ━     1
rgb6                      1     ━       1
rgb256                                  ━     1                               n
srgb                                    1     ━       1                       n
linear_srgb                                   1       ━    1
xyz                                                   1    ━           1      1
p3                                                             ━       1
linear_p3                                                  1   1       ━
oklab              1                                       1                  ━      1
oklch                                                                         1      ━
==============  ====  =====  ====  ======  ====  ======  ===  ==  ======  =====  =====
