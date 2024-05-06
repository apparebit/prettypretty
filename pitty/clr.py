"""
This module provides color support for terminals.


# Color Formats, Spaces, and Algorithms

This module recognizes the following written color formats:

  * The __hashed hexadecimal__ format familiar from the web, e.g., `#abc` or
    `#123456`, encodes a 24-bit RGB value, i.e., a triplet of byte-sized
    components. This module calls such color values `RGB256`.
  * The __XParseColor__ format, e.g., `rgb:0/12/3456` or `rgbi:+0.3/0.6/90e-2`,
    encodes a triplet of floating point numbers. Both variations can encode
    higher precision component values, hence the floating point representation.
    This module calls such color values `sRGB`.

It further supports the following in-memory color formats and color spaces:

  * `ANSI`: the sixteen extended ANSI colors
  * `8-bit`: incorporates the ANSI colors from 0--15, a 6x6x6 RGB cube from
    16--231, and a 24-step gradient from black to white from 232--255.
  * `RGB6`: an RGB triplet decoded from an 8-bit color
  * `RGB256`: an RGB triplet with 8-bit components
  * `sRGB`: the corresponding floating point representation
  * `linear-sRGB`: the linearized version of sRGB
  * `XYZ`: the base color space when converting between color spaces but only
    for whitepoint D65, not D50
  * `Oklab`: a color space that is more perceptually uniform than CIELab
  * `Oklch`: the cylindrical version thereof

Like sRGB, colors in linear-sRGB, XYZ, Oklab, and Oklch are represented as
floating point triplets.

This module also supports the following algorithms:

  * Compute distance ΔE between two colors in Oklab color space
  * Compute the APCA contrast between two colors in sRGB color space


# Color Representation

Since colors are just integers, integer triplets, or floating point triplets,
all functionality is implemented by fairly simple functions that can be easily
composed with each other. That provides for a simple and flexible
implementation, but also results in a fairly low-level interface that is a bit
cumbersome to use. Hence, for a better DevX, this module also provides
higher-level functions that cover common usage patterns. Only those functions
are exported from the pitty package.

The following table summarizes, which color formats can be directly converted
into which other color formats and which color formats are accepted by APCA for
computing contrast and by Oklab's ΔE for computing distance.

```txt
==================================================================================
from:  ANSI   RGB6  8-bit RGB256  sRGB  linear  XYZ   Oklab  Oklch   APCA    ΔE
----------------------------------------------------------------------------------
ANSI    --     XX                                       XX
RGB6           --    **     XX
8-bit          **    --                                               **     **
RGB256  XX     XX    **     --     XX
sRGB                 **     XX     --     XX                          XX
linear                             XX     --     XX
XYZ                                       XX     --     XX
Oklab                **            XX            XX     --     XX            XX
Oklch                                                   XX     --
=================================================================================
```

Since most terminals support 8-bit colors but not necessarily 24-bit colors and
8-bit colors incorporate the extended ANSI colors, 8-bit colors probably are the
best choice for terminal applications. The double stars in the table indicate
all functions that either accept or produce 8-bit colors.
"""
import math
import re
from typing import Callable, cast, Literal, overload, TypeVar


T = TypeVar("T")
Triple = tuple[T, T, T]
RGB = Triple[int]
TrueColor = Triple[float]
Matrix = Triple[Triple[float]]


# --------------------------------------------------------------------------------------


_ANSI_TO_RGB256 = (
    (0, 0, 0),
    (0x99, 0, 0),
    (0, 0x99, 0),
    (0x99, 0x99, 0),
    (0, 0, 0x99),
    (0x99, 0, 0x99),
    (0, 0x99, 0x99),
    (0xcc, 0xcc, 0xcc),
    (0x99, 0x99, 0x99),
    (0xff, 0, 0),
    (0, 0xff, 0),
    (0xff, 0xff, 0),
    (0, 0, 0xff),
    (0xff, 0, 0xff),
    (0, 0xff, 0xff),
    (0xff, 0xff, 0xff),
)

# See below for _ANSI_TO_OKLAB

_RGB6_TO_RGB256 = (0, 0x5F, 0x87, 0xAF, 0xD7, 0xFF)

_XYZ_TO_LINEAR_SRGB = (
	(  3.2409699419045226,  -1.537383177570094,   -0.4986107602930034  ),
	( -0.9692436362808796,   1.8759675015077202,   0.04155505740717559 ),
	(  0.05563007969699366, -0.20397695888897652,  1.0569715142428786  ),
)

_LINEAR_SRGB_TO_XYZ = (
	( 0.41239079926595934, 0.357584339383878,   0.1804807884018343  ),
	( 0.21263900587151027, 0.715168678767756,   0.07219231536073371 ),
	( 0.01933081871559182, 0.11919477979462598, 0.9505321522496607  ),
)

_XYZ_TO_LMS = (
	( 0.8190224379967030, 0.3619062600528904, -0.1288737815209879 ),
	( 0.0329836539323885, 0.9292868615863434,  0.0361446663506424 ),
	( 0.0481771893596242, 0.2642395317527308,  0.6335478284694309 ),
)

_LMS_TO_XYZ = (
	(  1.2268798758459243, -0.5578149944602171,  0.2813910456659647 ),
	( -0.0405757452148008,  1.1122868032803170, -0.0717110580655164 ),
	( -0.0763729366746601, -0.4214933324022432,  1.5869240198367816 ),
)

_LMS_TO_OKLAB = (
	( 0.2104542683093140,  0.7936177747023054, -0.0040720430116193 ),
	( 1.9779985324311684, -2.4285922420485799,  0.4505937096174110 ),
	( 0.0259040424655478,  0.7827717124575296, -0.8086757549230774 ),
)

_OKLAB_TO_LMS = (
    ( 1.0000000000000000,  0.3963377773761749,  0.2158037573099136 ),
	( 1.0000000000000000, -0.1055613458156586, -0.0638541728258133 ),
	( 1.0000000000000000, -0.0894841775298119, -1.2914855480194092 ),
)


# --------------------------------------------------------------------------------------


def hex_string_to_rgb256(color: str) -> RGB:
    """
    Convert the hexadecimal web color in `#xxx` or `#xxxxxx` format to RGB with
    8-bit per component value.
    """
    if (length := len(color)) not in (4, 7) or color[0] != '#':
        raise ValueError(f'"{color}" is not a valid color in #hex notation')

    digits = color[1:]
    if length == 4:
        digits = ''.join(a for b in ((c, c) for c in digits) for a in b)

    return cast(RGB, tuple(int(digits[n:n+2], base=16) for n in range(0, 6, 2)))


_X_PARSE_HEX = re.compile(r'[0-9a-fA-F]{1,4}')
_X_PARSE_FLOAT = re.compile(r'[+-]?\d+(?:[.]\d+)?(?:[eE][+-]?\d+)?')

def x_parse_color_to_srgb(color: str) -> TrueColor:

    def convert_hex(value: str) -> float:
        if not _X_PARSE_HEX.match(value):
            raise ValueError(f'"{color}" component not in hex format')
        return int(value, base=16) / 2**(4 * len(value))

    def convert_float(value: str) -> float:
        if not _X_PARSE_FLOAT.match(value):
            raise ValueError(f'"{color}" component not in floating point format')
        return float(value)

    try:
        tag, _, components = color.partition(':')
        r, g, b = components.split('/')
    except:
        raise ValueError(
            f'"{color}" does not have XParseColor "tag:<c1>/<c2>/<c3>" format'
        )

    if tag == 'rgb':
        return convert_hex(r), convert_hex(g), convert_hex(b)
    elif tag == 'rgbi':
        return convert_float(r), convert_float(g), convert_float(b)
    else:
        # CIEXYZ, CIEuvY, CIExyY, CIELab, CIELuv, TekHVC
        raise ValueError(f'Unsupported tag {tag} in XParseColor format')


# --------------------------------------------------------------------------------------
# Conversions to colors with integer components


def ansi_to_rgb256(color: int) -> RGB:
    """Convert the extended ANSI color to RGB with 8-bit component values."""
    assert 0 <= color <= 15
    return _ANSI_TO_RGB256[color]


def eight_bit_cube_to_rgb6(color: int) -> RGB:
    """Convert the 8-bit terminal color to RGB with six levels per component."""
    assert 16 <= color <= 231

    b = color - 16
    r = b // 36
    b -= 36 * r
    g = b // 6
    b -= 6 * g
    return r, g, b


def rgb6_to_eight_bit(r: int, g: int, b: int) -> int:
    """Convert RGB with 6 levels per component to an 8-bit terminal color."""
    assert 0 <= r <= 5 and 0 <= g <= 5 and 0 <= b <= 5
    return 16 + r * 36 + g * 6 + b


def rgb6_to_rgb256(r: int, g: int, b: int) -> RGB:
    """"
    Convert RGB with 6 levels per component to RGB with 8-bit per component.
    """
    assert 0 <= r <= 5 and 0 <= g <= 5 and 0 <= b <= 5
    return _RGB6_TO_RGB256[r], _RGB6_TO_RGB256[g], _RGB6_TO_RGB256[b]


def rgb256_to_rgb6(r: int, g: int, b: int) -> RGB:
    """
    Convert RGB with 8-bit per component to RGB with levels per component.
    """
    assert 0 <= r <= 255 and 0 <= g <= 255 and 0 <= b <= 255

    def convert(value: int) -> int:
        for index, level in enumerate(_RGB6_TO_RGB256):
            if value == level:
                return index
            if value > level:
                continue

            previous_level = _RGB6_TO_RGB256[index - 1]
            return index if level - value < value - previous_level else index - 1

        assert False, "unreachable statement"

    return convert(r), convert(g), convert(b)


def eight_bit_grey_to_rgb256(color: int) -> RGB:
    """
    Convert the 8-bit terminal color representing a grey value to RGB with 8-bit
    per component value.
    """
    assert 232 <= color <= 255

    c = (color - 232) * 10 + 8
    return c, c, c


def eight_bit_to_rgb256(
    color: int,
    convert_ansi: Callable[[int], RGB] = ansi_to_rgb256
) -> RGB:
    """Convert the 8-bit terminal color to RGB with 8-bit per component value."""
    if 0 <= color <= 15:
        return convert_ansi(color)
    elif 16 <= color <= 231:
        return rgb6_to_rgb256(*eight_bit_cube_to_rgb6(color))
    else:
        return eight_bit_grey_to_rgb256(color)


def srgb_to_rgb256(r: float, g: float, b: float) -> RGB:
    """
    Convert high resolution color to the more compact conventional representation
    with 8-bit per component.
    """
    def convert(value: float) -> int:
        return int(value * 255)
    return convert(r), convert(g), convert(b)


_ANSI_TO_OKLAB: None | tuple[TrueColor, ...] = None

def oklab_to_ansi(L: float, a: float, b: float) -> tuple[int, TrueColor]:
    """
    Determine the extended ANSI color closest to the given Oklab color. This
    function uses the default RGB colors returned by `ansi_to_rgb256()` as
    candidates, after converting them to Oklab of course. If you want to use a
    different mapping, use `closest()` with the sixteen candidate colors
    instead.
    """
    global _ANSI_TO_OKLAB

    if _ANSI_TO_OKLAB is None:
        _ANSI_TO_OKLAB = tuple(  # type: ignore
            srgb_to_oklab(*rgb256_to_srgb(*ansi_to_rgb256(ansi))) for ansi in range(16)
        )

    return closest((L, a, b), *_ANSI_TO_OKLAB)


def rgb6_to_ansi(r: int, g: int, b: int) -> int:
    """Convert RGB with 6 levels per component to extended ANSI color."""
    ansi, _ = oklab_to_ansi(
        *srgb_to_oklab(
            *rgb256_to_srgb(
                *rgb6_to_rgb256(r, g, b)
            )
        )
    )
    return ansi


# --------------------------------------------------------------------------------------
# Conversions to colors with float components


def rgb256_to_srgb(r: int, g: int, b: int) -> TrueColor:
    """
    Convert conventional RGB with 8-bit component values to high resolution
    color with floating point component values.
    """
    def convert(value: int) -> float:
        assert 0 <= value <= 255
        return value / 255.0

    return convert(r), convert(g), convert(b)


def eight_bit_to_srgb(color: int) -> TrueColor:
    """Convert 8-bit terminal color to sRGB."""
    return rgb256_to_srgb(*eight_bit_to_rgb256(color))


def _multiply(matrix: Matrix, color: TrueColor) -> TrueColor:
    return cast(
        Triple[float],
        tuple(sum(r * c for r, c in zip(row, color)) for row in matrix)
    )


def xyz_to_linear_srgb(X: float, Y: float, Z: float) -> TrueColor:
    """Convert XYZ D65 to linear sRGB."""
    return _multiply(_XYZ_TO_LINEAR_SRGB, (X, Y, Z))


def linear_srgb_to_xyz(r: float, g: float, b: float) -> TrueColor:
    """Convert linear sRGB to XYZ D65."""
    return _multiply(_LINEAR_SRGB_TO_XYZ, (r, g, b))


def srgb_to_linear_srgb(r: float, g: float, b: float) -> TrueColor:
    """Convert sRGB to linear sRGB"""
    def convert(value: float) -> float:
        magnitude = math.fabs(value)

        if magnitude <= 0.04045:
            return value / 12.92

        return math.copysign(math.pow((magnitude + 0.055) / 1.055, 2.4), value)

    return convert(r), convert(g), convert(b)


def linear_srgb_to_srgb(r: float, g: float, b: float) -> TrueColor:
    """Convert linear sRGB to sRGB."""
    def convert(value: float) -> float:
        magnitude = math.fabs(value)

        if magnitude <= 0.0031308:
            return value * 12.92

        return math.copysign(math.pow(magnitude, 1/2.4) * 1.055 - 0.055, value)

    return convert(r), convert(g), convert(b)


def xyz_to_oklab(X: float, Y: float, Z: float) -> TrueColor:
    """Convert XYZ D65 to OkLab"""
    LMS = _multiply(_XYZ_TO_LMS, (X, Y, Z))
    LMSg = tuple(map(lambda c: math.cbrt(c), LMS))
    return _multiply(_LMS_TO_OKLAB, cast(TrueColor, LMSg))


def srgb_to_oklab(r: float, g: float, b: float) -> TrueColor:
    """Convert the sRGB color to Oklab."""
    return xyz_to_oklab(*linear_srgb_to_xyz(*srgb_to_linear_srgb(r, g, b)))


def eight_bit_to_oklab(color: int) -> TrueColor:
    """Convert the 8-bit terminal color to Oklab."""
    return srgb_to_oklab(*rgb256_to_srgb(*eight_bit_to_rgb256(color)))


def oklab_to_xyz(L: float, a: float, b: float) -> TrueColor:
    """Convert OkLab to XYZ D65."""
    LMSg = _multiply(_OKLAB_TO_LMS, (L, a, b))
    LMS = tuple(map(lambda c: math.pow(c, 3), LMSg))
    return _multiply(_LMS_TO_XYZ, cast(TrueColor, LMS))


def oklab_to_oklch(L: float, a: float, b: float) -> TrueColor:
    """Convert OkLab to OkLCh."""
    ε = 0.0002

    if math.fabs(a) < ε and math.fabs(b) < ε:
        h = math.nan
    else:
        h = math.atan2(b, a) * 180 / math.pi

    return L, math.sqrt(math.pow(a, 2) + math.pow(b, 2)), math.fmod(h + 360, 360)


def oklch_to_oklab(L: float, C: float, h: float) -> TrueColor:
    """Convert OkLCh to OkLab."""
    assert math.isnan(h) or 0 <= h <= 360

    if math.isnan(h):
        a = b = 0.0
    else:
        a = C * math.cos(h * math.pi / 180)
        b = C * math.sin(h * math.pi / 180)

    return L, a, b


def ok_deltaE(
    L1: float, a1: float, b1: float,
    L2: float, a2: float, b2: float,
    version: Literal[1, 2] = 2
) -> float:
    """
    Compute the distance between the two Oklab colors. The first version is the
    Euclidian distance, whereas the second version doubles a and b before
    computing the Euclidian distance.
    """
    ΔL = L1 - L2
    Δa = version * (a1 - a2)
    Δb = version * (b1 - b2)
    return math.sqrt(ΔL * ΔL + Δa * Δa + Δb * Δb)


def closest(origin: TrueColor, *candidates: TrueColor) -> tuple[int, TrueColor]:
    """
    Determine the candidate color closest to the origin color. This function
    returns the index of the candidate and its value. If this function is
    invoked without candidates, the index is -1 and the color is the origin
    color.
    """
    closest_index = -1
    closest_color = None
    distance = math.inf

    for index, candidate in enumerate(candidates):
        d = ok_deltaE(*origin, *candidate)
        if d < distance:
            closest_index = index
            closest_color = candidate
            distance = d

    return closest_index, closest_color or origin


# --------------------------------------------------------------------------------------
# Color Contrast


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


def srgb_to_apca_luminance(r: float, g: float, b: float) -> float:
    """
    Perform APCA's conversion from sRGB to its non-standard Y. This is one of
    two low-level functions implementing APCA. You may prefer using
    `apca_contrast()`, `apca_use_black_text()`, or
    `apca_use_black_background()`.
    """
    linear_srgb = (math.pow(c, _APCA_EXPONENT) for c in (r, g, b))
    return sum(c1 * c2 for c1, c2 in zip(_APCA_COEFFICIENTS, linear_srgb))


def apca_luminance_to_contrast(
    text_luminance: float,
    background_luminance: float
) -> float:
    """
    Compute the contrast Lc for the nonstandard text and background luminance Y
    using the [Accessible Perceptual Contrast
    Algorithm](https://github.com/Myndex/apca-w3) (APCA), version 0.1.9,
    2022-07-03. While APCA is an improvement on version 2 of the Web Content
    Accessibility Guidelines (WCAG), it still falls short of a general solution:

        * It uses its own conversion from sRGB to a custom version of Y (see
          https://github.com/w3c/silver/issues/643).
        * Hence, it only works with colors that are in gamut for sRGB.
        * Since it defines similar custom conversions for P3 and AdobeRGB,
          chances are they work as well. But they do not seem to be used much.
        * It only works for text on top of a solidly colored background, not
          color against color.
        * It does not work for low contrast, with the implementation clamping
          results smaller than 0.1 to 0.0.

    APCA's author seemed to commit to addressing at least some of these concerns
    in discussions on GitHub. But that was in 2022 and the material about APCA
    still is rather unwieldy. Worse, the author also seems to persue patents for
    this work.

    This is one of two low-level functions implementing APCA. You may prefer
    using `apca_contrast()`, `apca_use_black_text()`, or
    `apca_use_black_background()`.
    """
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


def apca_contrast(
    text_color: int | TrueColor,
    background_color: int | TrueColor
) -> float:
    """
    Compute the APCA contrast for the given text and background colors. Each
    color may be an 8-bit terminal or sRGB color.
    """
    if isinstance(text_color, int):
        text_color = eight_bit_to_srgb(text_color)
    if isinstance(background_color, int):
        background_color = eight_bit_to_srgb(background_color)

    text_luminance = srgb_to_apca_luminance(*text_color)
    background_luminance = srgb_to_apca_luminance(*background_color)
    return apca_luminance_to_contrast(text_luminance, background_luminance)


@overload
def apca_use_black_text(color: int, /) -> bool:
    ...
@overload
def apca_use_black_text(r: float, g: float, b: float, /) -> bool:
    ...
def apca_use_black_text(
    r: int | float,
    g: None | float = None,
    b: None | float = None,
) -> bool:
    """
    Use APCA to determine whether text should be black or white against a
    background with the given 8-bit or sRGB color. This function returns `True`
    as long as the black text has at least the same contrast as white text.
    """
    if isinstance(r, int):
        r, g, b = eight_bit_to_srgb(r)

    assert g is not None and b is not None
    luminance = srgb_to_apca_luminance(r, g, b)

    return (
        apca_luminance_to_contrast(0.0, luminance)
        >= -apca_luminance_to_contrast(1.0, luminance)
    )


@overload
def apca_use_black_background(color: int, /) -> bool:
    ...
@overload
def apca_use_black_background(r: float, g: float, b: float, /) -> bool:
    ...
def apca_use_black_background(
    r: int | float,
    g: None | float = None,
    b: None | float = None,
) -> bool:
    """
    Use APCA to determine whether text with the given 8-bit or sRGB color should
    be on a black or white background. This function returns `True` as long as
    the black background has at least the same contrast as the white background.
    """
    if isinstance(r, int):
        r, g, b = eight_bit_to_srgb(r)

    assert g is not None and b is not None
    luminance = srgb_to_apca_luminance(r, g, b)

    return (
        apca_luminance_to_contrast(luminance, 0.0)
        <= -apca_luminance_to_contrast(luminance, 1.0)
    )
