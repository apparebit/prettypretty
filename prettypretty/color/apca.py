"""
Support for computing color contrast with the `accessible perceptual contrast
algorithm <https://github.com/Myndex/apca-w3>`_ (APCA), version 0.1.9, dated
July 3, 2022. Note that APCA is asymmetric, i.e., the order of text and
background luminance matters. Furthermore, it is designed for text on a solidly
colored background only and clamps small contrast values to zero.
"""
import math


# Switching from naive floating point math to math.sumprod caused one test to
# have -1e-16 for blue component, which triggers a math domain error in
# srgb_to_luminance. The work-around is to clamp slightly negative values to 0.
# FIXME: Consider doing the same when converting srgb or p3 to their linear
# versions.
_NEGATIVE_SRGB_TOLERANCE = -1e-15

_APCA_EXPONENT = 2.4
_APCA_SRGB_COEFFICIENTS = (0.2126729, 0.7151522, 0.0721750)
_APCA_P3_COEFFICIENTS = (
    0.2289829594805780,
    0.6917492625852380,
    0.0792677779341829,
)

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

    Conceptually, this function performs a conversion similar to that from sRGB
    to XYZ. That means undoing the gamma correction, which is impossible for
    out-of-gamut, negative sRGB coordinates. To prevent spurious exceptions
    caused by negative coordinates that are well within the floating point error
    margins for color conversion, this function clamps values greater than
    -1e-15 to zero.
    """
    if not all(_NEGATIVE_SRGB_TOLERANCE < c for c in (r, g, b)):
        raise ValueError(f'Negative sRGB coordinate for {r}, {g}, {b}')

    # The max(...) clamps c to a minimum of zero.
    linear_srgb = (math.pow(max(c, 0.0), _APCA_EXPONENT) for c in (r, g, b))
    return math.sumprod(_APCA_SRGB_COEFFICIENTS, linear_srgb)


def p3_to_luminance(r: float, g: float, b: float) -> float:
    """
    :bdg-warning:`Internal API` Determine the non-standard APCA luminance for
    the given P3 color.
    """
    linear_p3 = (math.pow(c, _APCA_EXPONENT) for c in (r, g, b))
    return math.sumprod(_APCA_P3_COEFFICIENTS, linear_p3)


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


def srgb_contrast(
    text_color: tuple[float, float, float],
    background_color: tuple[float, float, float],
) -> float:
    """
    Compute the contrast between the given text and background colors in the
    sRGB color space.

    Both text and background colors must be in gamut for sRGB.
    """
    text_luminance = srgb_to_luminance(*text_color)
    background_luminance = srgb_to_luminance(*background_color)
    return luminance_to_contrast(text_luminance, background_luminance)


def p3_contrast(
    text_color: tuple[float, float, float],
    background_color: tuple[float, float, float],
) -> float:
    """
    Compute the contrast between the given text and background colors in the
    Display P3 color space.

    Both text and background color must in gamut for Display P3.
    """
    text_luminance = p3_to_luminance(*text_color)
    background_luminance = p3_to_luminance(*background_color)
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
