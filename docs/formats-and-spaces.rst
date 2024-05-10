Color Formats and Spaces
========================

Prettypretty can parse the following text-based color formats:

  * The hashed hexadecimal notation familiar from the web, e.g., ``#123`` or
    ``#abcdef``.
  * The X Parse Color notation familiar from X Windows and xterm, e.g.,
    ``rgb:1/22/4444`` or ``rgbi:0.3/0.6/9e-1``.

It supports the following in-memory RGB color formats and spaces:

  * **rgb6** represents a color from the 6x6x6 RGB cube of 8-bit terminal colors
    by its component values. Each component ranges from 0 to 5, inclusive. The
    implied color space is sRGB, as RGB6 colors convert to RGB256 colors.
  * **eight_bit_cube** also represents a color from the 6x6x6 RGB cube of 8-bit
    terminal colors, but represented as an integer between 16 and 231,
    inclusive.
  * **eight_bit_grey** represents one of the 24-step greyscale colors of 8-bit
    terminal colors represented as an integer between 232 and 255. Like RGB6,
    8-bit grey colors convert to RGB256 colors.
  * **rgb256** represents a color as three 8-bit RGB components. The implied
    color space is sRGB. Parsing the hashed hexadecimal notation or the ``rgb:``
    notation with all components having at most two digits results in this color
    representation.
  * **srgb** represents an sRGB color as three floating point RGB components
    between 0 and 1, inclusive. Parsing the ``rgb:`` notation with at least one
    component having three or four digits or the ``rgbi:`` notation results in
    this color representation.
  * **linear_srgb** represents a *linear* sRGB color as three floating point RGB
    components between 0 and 1, inclusive.
  * **p3** represent a P3 color as three floating point RGB components between 0
    and 1, inclusive.
  * **linear_p3** represents a *linear* P3 color as three floating point RGB
    components between 0 and 1, inclusive.

Finally, it supports the following non-RGB color spaces:

  * **xyz** represents an XYZ color as three floating point components that have
    no pre-defined limits.
  * **oklab** represents an OkLab color as three floating point components. The
    L (lightness) component ranges from 0 to 1, inclusive, whereas the a and b
    components range from -0.4 to +0.4, inclusive.
  * **oklch** represent an OkLCh color as three floating point components. It is
    the polar equivalent of OkLab, with L the same as for OkLab, C (chroma)
    ranging from 0 to 0.4, inclusive, and h (hue) ranging from 0 to 360,
    inclusive (with 0 and 360 being the same hue).

The bold-face tags above are the canonical tags used to identify the
corresponding color formats and spaces. They appear in the names of conversion
functions, serve as arguments to the
:func:`prettypretty.color.conversion.route`,
:func:`prettypretty.color.conversion.convert`, and
:func:`prettypretty.color.space.resolve` functions, and as the first field of
the :class:`prettypretty.color.theme.ColorSpec` and
:class:`prettypretty.color.color.Color` classes.
