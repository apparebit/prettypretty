"""
Support for computing color contrast. This module implements a perceptual
contrast metric that is surprisingly similar to the `accessible perceptual
contrast algorithm <https://github.com/Myndex/apca-w3>`_ (APCA), version 0.1.9,
dated July 3, 2022, but makes

Note that the perceptual contrast metric is asymmetric, i.e., the order of text
and background luminance matters. Furthermore, the metric is designed for text
on solidly colored backgrounds only and clamps small contrast values to zero.
"""
import math


_EXPONENT = 2.4
_SRGB_COEFFICIENTS = (0.2126729, 0.7151522, 0.0721750)
_P3_COEFFICIENTS = (
    0.2289829594805780,
    0.6917492625852380,
    0.0792677779341829,
)

_BLACK_THRESHOLD = 0.022
_BLACK_EXPONENT = 1.414
_INPUT_CLAMP = 0.0005

_BoW_TEXT = 0.57
_BoW_BACKGROUND = 0.56
_WoB_TEXT = 0.62
_WoB_BACKGROUND = 0.65

_SCALE = 1.14
_OUTPUT_CLAMP = 0.1
_OFFSET = 0.027


def srgb_to_luminance(r: float, g: float, b: float) -> float:
    """
    Determine the contrast luminance for the given sRGB color.

    The color must be in-gamut for sRGB with some error tolerance.
    """
    def linearize(value: float) -> float:
        magnitude = math.fabs(value)
        return math.copysign(math.pow(magnitude, _EXPONENT), value)

    linear_srgb = (linearize(c) for c in (r, g, b))
    return math.sumprod(_SRGB_COEFFICIENTS, linear_srgb)


def p3_to_luminance(r: float, g: float, b: float) -> float:
    """
    Determine the contrast luminance for the given Display P3 color.

    The color must be in-gamut for Display P3 with some error tolerance. Also,
    when colors are in-gamut for sRGB, prefer :func:`srgb_to_luminance` instead.
    """
    def linearize(value: float) -> float:
        magnitude = math.fabs(value)
        return math.copysign(math.pow(magnitude, _EXPONENT), value)

    linear_p3 = (linearize(c) for c in (r, g, b))
    return math.sumprod(_P3_COEFFICIENTS, linear_p3)


def luminance_to_contrast(
    text_luminance: float,
    background_luminance: float,
) -> float:
    """
    Determine the contrast between the text and background luminance values.
    """
    if (
        math.isnan(text_luminance)
        or not 0.0 <= text_luminance <= 1.1
        or math.isnan(background_luminance)
        or not 0.0 <= background_luminance <= 1.1
    ):
        return 0.0

    # Soft-clip black
    if text_luminance < _BLACK_THRESHOLD:
        text_luminance += math.pow(
            _BLACK_THRESHOLD - text_luminance,
            _BLACK_EXPONENT,
        )
    if background_luminance < _BLACK_THRESHOLD:
        background_luminance += math.pow(
            _BLACK_THRESHOLD - background_luminance,
            _BLACK_EXPONENT,
        )

    # Small ΔY have too little contrast. Clamp result to zero.
    if abs(text_luminance - background_luminance) < _INPUT_CLAMP:
        return 0.0

    # Computer Lc
    if background_luminance > text_luminance:
        contrast = (
            math.pow(background_luminance, _BoW_BACKGROUND)
            - math.pow(text_luminance, _BoW_TEXT)
        ) * _SCALE

        if contrast < _OUTPUT_CLAMP:
            return 0.0
        return contrast - _OFFSET

    else:
        contrast = (
            math.pow(background_luminance, _WoB_BACKGROUND)
            - math.pow(text_luminance, _WoB_TEXT)
        ) * _SCALE

        if contrast > -_OUTPUT_CLAMP:
            return 0.0
        return contrast + _OFFSET


def use_black_text(luminance: float) -> bool:
    """
    Determine whether black or white text maximizes contrast against the given
    background luminance.
    """
    return (
        luminance_to_contrast(0.0, luminance) >= -luminance_to_contrast(1.0, luminance)
    )


def use_black_background(luminance: float) -> bool:
    """
    Determine whether a black or white background maximizes contrast for the
    given text luminance.
    """
    return (
        luminance_to_contrast(luminance, 0.0) <= -luminance_to_contrast(luminance, 1.0)
    )
