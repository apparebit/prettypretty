# Summary

Prettypretty's abstractions fall into four major categories:

  * High-resolution colors
  * Terminal colors
  * Direct conversion between color representations
  * Translation between colors

In more detail:

  * [`Color`] is prettypretty's high-resolution color object. It combines a
    [`ColorSpace`] with three [`Float`] coordinates. Its methods expose much
    of prettypretty's functionality, including conversion between color
    spaces, interpolation between colors, calculation of perceptual
    contrast, as well as gamut testing, clipping, and mapping.
  * [`TerminalColor`] combines [`DefaultColor`], [`AnsiColor`],
    [`EmbeddedRgb`], [`GrayGradient`], and [`TrueColor`] to represent the
    different kinds of terminal colors.
  * A fair number of `From<T>` and `TryFrom<T>` implementations cover
    lossless and partial conversions between color representations
    including, for example, conversion from EmbeddedRgb to `u8` index values
    as well true, terminal, and high-resolution colors.
  * [`Translator`] performs the more difficult translation from ANSI to
    high-resolution colors, from high-resolution to 8-bit or ANSI colors, and
    the downgrading of terminal colors based on terminal capabilities and user
    preferences.


{{#include ../links.md}}
