"""
Support for computing color contrast with the `accessible perceptual contrast
algorithm <https://github.com/Myndex/apca-w3>`_ (APCA), version 0.1.9, dated
July 3, 2022. Note that APCA is asymmetric, i.e., the order of text and
background luminance matters. Furthermore, it is designed for text on a solidly
colored background only and clamps small contrast values to zero.
"""
import math


_APCA_EXPONENT = 2.4
_APCA_COEFFICIENTS = (0.2126729, 0.7151522, 0.0721750)

_APCA_BLACK_THRESHOLD = 0.022
_APCA_BLACK_CLIP = 1.414
_APCA_DELTA_Y_MIN = 0.0005

_APCA_BoW_TEXT = 0.57
_APCA_BoW_BACKGROUND = 0.56
_APCA_WoB_TEXT = 0.62
_APCA_WoB_BACKGROUND = 0.65

_APCA_SCALE = 1.14
_APCA_CLAMP = 0.1
_APCA_OFFSET = 0.027


def srgb_to_luminance(r: float, g: float, b: float) -> float:
    """
    :bdg-warning:`Internal API` Determine the non-standard APCA luminance for
    the given sRGB color.
    """
    linear_srgb = (math.pow(c, _APCA_EXPONENT) for c in (r, g, b))
    return sum(c1 * c2 for c1, c2 in zip(_APCA_COEFFICIENTS, linear_srgb))


def luminance_to_contrast(
    text_luminance: float,
    background_luminance: float,
) -> float:
    """
    :bdg-warning:`Internal API` Determine the contrast between the text and
    background luminance values."""
    assert 0.0 <= text_luminance <= 1.1 and 0.0 <= background_luminance <= 1.1

    # Soft-clip and clamp black
    if text_luminance < _APCA_BLACK_THRESHOLD:
        text_luminance += math.pow(
            _APCA_BLACK_THRESHOLD - text_luminance,
            _APCA_BLACK_CLIP,
        )
    if background_luminance < _APCA_BLACK_THRESHOLD:
        background_luminance += math.pow(
            _APCA_BLACK_THRESHOLD - background_luminance,
            _APCA_BLACK_CLIP,
        )

    # Small ΔY have too little contrast. Clamp result to zero.
    if abs(text_luminance - background_luminance) < _APCA_DELTA_Y_MIN:
        return 0.0

    # Computer Lc
    if background_luminance > text_luminance:
        contrast = (
            math.pow(background_luminance, _APCA_BoW_BACKGROUND)
            - math.pow(text_luminance, _APCA_BoW_TEXT)
        ) * _APCA_SCALE

        if contrast < _APCA_CLAMP:
            return 0.0
        return contrast - _APCA_OFFSET

    else:
        contrast = (
            math.pow(background_luminance, _APCA_WoB_BACKGROUND)
            - math.pow(text_luminance, _APCA_WoB_TEXT)
        ) * _APCA_SCALE

        if contrast > -_APCA_CLAMP:
            return 0.0
        return contrast + _APCA_OFFSET


def contrast(
    text_color: tuple[float, float, float],
    background_color: tuple[float, float, float],
) -> float:
    """
    Compute the contrast between the given text and background colors in the
    sRGB color space.
    """
    text_luminance = srgb_to_luminance(*text_color)
    background_luminance = srgb_to_luminance(*background_color)
    return luminance_to_contrast(text_luminance, background_luminance)


def use_black_text(r: float, g: float, b: float) -> bool:
    """
    Determine whether text should be black or white to maximize its contrast
    against a background with the given sRGB color.
    """
    luminance = srgb_to_luminance(r, g, b)

    return (
        luminance_to_contrast(0.0, luminance) >= -luminance_to_contrast(1.0, luminance)
    )


def use_black_background(r: float, g: float, b: float) -> bool:
    """
    Determine whether the background should be black or white to maximize
    its contrast against text with the given sRGB color.
    """
    luminance = srgb_to_luminance(r, g, b)

    return (
        luminance_to_contrast(luminance, 0.0) <= -luminance_to_contrast(luminance, 1.0)
    )
