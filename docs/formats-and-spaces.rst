Color Formats and Spaces
========================

To maximize accuracy, prettypretty's core  represents color coordinates as
floating point numbers with double precision. It furthermore uses double
precision matrices when converting coordinates between color spaces.


Color Spaces
------------

RGB color spaces are based on an additive color model, with the overall color
being the sum of the colors of the component lights. Every RGB color space has
three coordinates ``r``, ``g``, and ``b``, which range from 0 to 1, inclusive,
for in-gamut colors. The supported RGB color spaces are:

  * **srgb** is the default, gamma-corrected color space of the web, sRGB
  * **linear_srgb** is the linear version of sRGB, without gamma correction
  * **p3** is the larger P3 color space, with gamma correction
  * **linear_p3** is the linear version of P3, without gammar correction

sRGB and P3 use the same gamma correction, hence :func:`.p3_to_linear_p3` is
just an alias to :func:`.srgb_to_linear_srgb` and :func:`.linear_p3_to_p3` is
just an alias to :func:`.linear_srgb_to_srgb`.

Prettypretty supports the following non-RGB color spaces:

  * **xyz** is the XYZ color space with the D65 standard illuminant
  * **oklab** is a perceptually uniform color space, with ``L`` standing for
    lightness and ``a``/``b`` serving as orthogonal color axes
  * **oklch** is a cylindrical version of Oklab, with ``L`` standing for
    lightness, ``C`` for chroma, and ``h`` for hue.

In Oklab, the difference ΔE between two colors is just the Euclidian distance.
Though Oklab's designer, Björn Ottosson, has suggested that a more accurate ΔE
should scale Δa and Δb by a constant factor of around 2.1 before squaring.
Prettypretty implements both versions, like Color.js using a factor of 2 for
version 2 for now.


Color Formats
-------------

In addition to the above color spaces, prettypretty also supports a number of
color formats. There is only one standard format:

  * **rgb256** is a 24-bit RGB color in the sRGB color space, with each
    coordinate being an integer between 0 and 255, inclusive

There is one terminal-specific RGB format:

  * **rgb6** is an RGB color, with each coordinate being an integer between
    0 and 5, inclusive

Finally, there are two more terminal-specific color formats with a single
integer coordinate:

  * **ansi** is an integer between 0 and 15, inclusive, denoting one of the
    sixteen extended ANSI colors
  * **eight_bit** is an integer between 0 and 255, inclusive, denoting one
    of the 8-bit colors:

      * 0–15 are the sixteen extended ANSI colors
      * 16–231 are the 216 RGB6 colors
      * 232–255 is a 24-step grayscale gradient

The RGB6 colors and the 24-step grayscale gradient have a well-defined mapping
to RGB256 and hence sRGB. However, the sixteen extended ANSI colors have only
standardized names but not color values. Furthermore, almost all terminals have
robust support for customizing just those colors through themes. Consequently,
any conversion from and to ANSI colors must take the color theme into account,
too. Prettypretty does just that, even supporting arbitrary color spaces for
theme colors.


Color Serde
-----------

Prettypretty supports two formats for both serialization to and deserialization
from strings:

  * **h** for hexadecimal is the hash-prefixed color format familiar from the
    web
  * **x** for X Windows or xterm uses the `rgb:` and `rgbi:` prefixes followed
    by three slash-separated coordinates in hexadecimal for `rgb:` and floating
    point for `rgbi:`

For serialization to strings, prettypretty supports two additional formats:

  * **f** for function uses one of the above format or color space tags as
    function name and the comma-separated coordinates as arguments
  * **s** for CSS uses ``color()``, ``oklab()``, ``oklch()``, or ``rgb()``
    notation with space-separated coordinates

The four letters serve as format identifiers for colors in Python's f-strings.
However, only RGB256 colors can be serialized in h-format or the ``rgb:``
prefixed x-format; only sRGB colors can be serialized in the ``rgb:`` prefixed
x-format; and the s-format cannot serialize ``linear_p3`` colors.
