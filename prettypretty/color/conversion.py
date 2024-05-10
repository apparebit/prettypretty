"""
Conversion between common color formats and spaces. With exception of color
formats that explicitly model integral components, all colors are represented by
triples with floating point components.
"""
from collections import deque
import math
from typing import Callable, cast

from .difference import closest_oklab
from .theme import current_theme, Theme


_RGB6_TO_RGB256 = (0, 0x5F, 0x87, 0xAF, 0xD7, 0xFF)

# See https://github.com/color-js/color.js/blob/a77e080a070039c534dda3965a769675aac5f75e/src/spaces/srgb-linear.js

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

# See https://github.com/color-js/color.js/blob/a77e080a070039c534dda3965a769675aac5f75e/src/spaces/p3-linear.js

_XYZ_TO_LINEAR_P3 = (
	(  2.493496911941425,   -0.9313836179191239,  -0.40271078445071684  ),
	( -0.8294889695615747,   1.7626640603183463,   0.023624685841943577 ),
	(  0.03584583024378447, -0.07617238926804182,  0.9568845240076872   ),
)

_LINEAR_P3_TO_XYZ = (
	( 0.4865709486482162, 0.26566769316909306, 0.1982172852343625 ),
	( 0.2289745640697488, 0.6917385218365064,  0.079286914093745  ),
	( 0.0000000000000000, 0.04511338185890264, 1.043944368900976  ),
)

# See https://github.com/color-js/color.js/blob/a77e080a070039c534dda3965a769675aac5f75e/src/spaces/oklab.js

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

# See https://bottosson.github.io/posts/oklab/ for next four matrices

_LINEAR_SRGB_TO_SRGB_LMS = (
    ( 0.4122214708, 0.5363325363, 0.0514459929 ),
	( 0.2119034982, 0.6806995451, 0.1073969566 ),
	( 0.0883024619, 0.2817188376, 0.6299787005 ),
)

_SRGB_LMS_TO_OKLAB = (
    ( 0.2104542553,  0.7936177850, -0.0040720468 ),
    ( 1.9779984951, -2.4285922050,  0.4505937099 ),
    ( 0.0259040371,  0.7827717662, -0.8086757660 ),
)

_OKLAB_TO_SRGB_LMS = (
    ( 1.0,  0.3963377774,  0.2158037573 ),
    ( 1.0, -0.1055613458, -0.0638541728 ),
    ( 1.0, -0.0894841775, -1.2914855480 ),
)

_SRGB_LMS_TO_LINEAR_SRGB = (
    ( 4.0767416621, -3.3077115913,  0.2309699292 ),
    ( 1.2684380046,  2.6097574011, -0.3413193965 ),
    ( 0.0041960863, -0.7034186147,  1.7076147010 ),
)

_IntVector = tuple[int, int, int]
_Vector = tuple[float, float, float]
_Matrix = tuple[_Vector, _Vector, _Vector]

def _multiply(matrix: _Matrix, vector: _Vector) -> _Vector:
    return cast(
        _Vector,
        tuple(sum(r * c for r, c in zip(row, vector)) for row in matrix)
    )


# --------------------------------------------------------------------------------------
# 8-bit terminal colors


def eight_bit_cube_to_rgb6(color: int) -> tuple[int, int, int]:
    """
    Convert the given eight-bit color to the three components of the 6x6x6 RGB
    cube. The color value must be between 16 and 231, inclusive.
    """
    assert 16 <= color <= 231

    b = color - 16
    r = b // 36
    b -= 36 * r
    g = b // 6
    b -= 6 * g
    return r, g, b


def rgb6_to_eight_bit_cube(r: int, g: int, b: int) -> tuple[int]:
    """
    Convert the given color from the 6x6x6 RGB cube of 8-bit terminal colors to
    an actual 8-bit terminal color.
    """
    return 16 + 36 * r + 16 * g + b,


def rgb6_to_rgb256(r: int, g: int, b: int) -> tuple[int, int, int]:
    """
    Convert the given color from the 6x6x6 RGB cube of 8-bit terminal colors to
    24-bit RGB. Each component must be between 0 and 5, inclusive.
    """
    return cast(_IntVector, tuple(map(lambda c: _RGB6_TO_RGB256[c], (r, g, b))))


def eight_bit_grey_to_rgb256(color: int) -> tuple[int, int, int]:
    """
    Convert the given color from the 24-step grey gradient of 8-bit terminal
    colors to 24-bit RGB. The color value must be between 232 and 255,
    inclusive.
    """
    assert 232 <= color <= 255

    c = 10 * (color - 232) + 8
    return c, c, c


# --------------------------------------------------------------------------------------
# 24-bit RGB


def rgb256_to_srgb(r: int, g: int, b: int) -> tuple[float, float, float]:
    """Convert the given color from 24-bit RGB to sRGB."""
    return cast(_Vector, tuple(map(lambda c: c / 255.0, (r, g, b))))


def srgb_to_rgb256(r: float, g: float, b: float) -> tuple[int, int, int]:
    """
    :bdg-warning:`Lossy conversion` Convert the given color from sRGB to 24-bit
    RGB.
    """
    return cast(tuple[int, int, int], tuple(map(lambda c: round(c * 255), (r, g, b))))


# --------------------------------------------------------------------------------------
# sRGB and Linear sRGB
# See https://github.com/color-js/color.js/blob/a77e080a070039c534dda3965a769675aac5f75e/src/spaces/srgb.js

def srgb_to_linear_srgb(r: float, g: float, b: float) -> tuple[float, float, float]:
    """Convert the given color from sRGB to linear sRGB."""
    def convert(value: float) -> float:
        magnitude = math.fabs(value)

        if magnitude <= 0.04045:
            return value / 12.92

        return math.copysign(math.pow((magnitude + 0.055) / 1.055, 2.4), value)

    return convert(r), convert(g), convert(b)


def linear_srgb_to_srgb(r: float, g: float, b: float) -> tuple[float, float, float]:
    """Convert the given color from linear sRGB to sRGB."""
    def convert(value: float) -> float:
        magnitude = math.fabs(value)

        if magnitude <= 0.0031308:
            return value * 12.92

        return math.copysign(math.pow(magnitude, 1/2.4) * 1.055 - 0.055, value)

    return convert(r), convert(g), convert(b)


def linear_srgb_to_xyz(r: float, g: float, b: float) -> tuple[float, float, float]:
    """Convert the given color from linear sRGB to XYZ."""
    return _multiply(_LINEAR_SRGB_TO_XYZ, (r, g, b))


# --------------------------------------------------------------------------------------
# P3 and Linear P3

p3_to_linear_p3 = srgb_to_linear_srgb
linear_p3_to_p3 = linear_srgb_to_srgb


def linear_p3_to_xyz(r: float, g: float, b: float) -> tuple[float, float, float]:
    """Convert the given color from linear P3 to XYZ."""
    return _multiply(_LINEAR_P3_TO_XYZ, (r, g, b))


# --------------------------------------------------------------------------------------
# Oklab and Oklch:
# See https://github.com/color-js/color.js/blob/a77e080a070039c534dda3965a769675aac5f75e/src/spaces/oklch.js


def oklch_to_oklab(L: float, C: float, h: float) -> tuple[float, float, float]:
    """Convert the given color from Oklch to Oklab."""
    if math.isnan(h):
        a = b = 0.0
    else:
        a = C * math.cos(h * math.pi / 180)
        b = C * math.sin(h * math.pi / 180)

    return L, a, b


def oklab_to_oklch(L: float, a: float, b: float) -> tuple[float, float, float]:
    """Convert the given color from Oklab to Oklch."""
    ε = 0.0002

    if math.fabs(a) < ε and math.fabs(b) < ε:
        h = math.nan
    else:
        h = math.atan2(b, a) * 180 / math.pi

    return L, math.sqrt(math.pow(a, 2) + math.pow(b, 2)), math.fmod(h + 360, 360)


def oklab_to_xyz(L: float, a: float, b: float) -> tuple[float, float, float]:
    """Convert the given color from Oklab to XYZ."""
    LMSg = _multiply(_OKLAB_TO_LMS, (L, a, b))
    LMS = cast(_Vector, tuple(map(lambda c: math.pow(c, 3), LMSg)))
    return _multiply(_LMS_TO_XYZ, LMS)


_THEME_CACHE: dict[Theme, tuple[tuple[float, ...], ...]] = {}

def oklab_to_ansi(L: float, a: float, b: float) -> tuple[int]:
    """
    :bdg-warning:`Lossy conversion` Convert the given color from Oklab to the
    extended ANSI colors.
    """

    theme = current_theme()
    if theme not in _THEME_CACHE:
        _THEME_CACHE[theme] = tuple(
            convert(c.coordinates, c.tag, 'oklab')
            for n, c in theme.colors() if n not in ('text', 'background')
        )

    ansi = cast(tuple[tuple[float, float, float], ...], _THEME_CACHE[theme])
    index, _ = closest_oklab((L, a, b), *ansi)
    return (index,)


# --------------------------------------------------------------------------------------
# XYZ


def xyz_to_linear_srgb(X: float, Y: float, Z: float) -> tuple[float, float, float]:
    """Convert the given color from XYZ to linear sRGB."""
    return _multiply(_XYZ_TO_LINEAR_SRGB, (X, Y, Z))


def xyz_to_linear_p3(r: float, g: float, b: float) -> tuple[float, float, float]:
    """Convert the given color from XYZ to linear P3."""
    return _multiply(_XYZ_TO_LINEAR_P3, (r, g, b))


def xyz_to_oklab(X: float, Y: float, Z: float) -> tuple[float, float, float]:
    """Convert the given color from XYZ to Oklab."""
    LMS = _multiply(_XYZ_TO_LMS, (X, Y, Z))
    LMSg = cast(_Vector, tuple(map(lambda c: math.cbrt(c), LMS)))
    return _multiply(_LMS_TO_OKLAB, LMSg)


# --------------------------------------------------------------------------------------
# Extras


def extra[C](label: str) -> Callable[[C], C]:
    """Mark a conversion function as optional with the given label."""
    def mark(fn: C) -> C:
        setattr(fn, 'extra', label)
        return fn
    return mark


@extra('ottosson')
def linear_srgb_to_oklab(r: float, g: float, b: float) -> tuple[float, float, float]:
    """
    :bdg-danger:`Do not use!` Convert the given color from linear sRGB to Oklab
    using `Björn Ottosson <https://bottosson.github.io/posts/oklab/>`_'s
    matrices.

    Since those matrices have only single floating point precision, this
    function is far less accurate than the composition of
    :func:`linear_srg_to_xyz` and :func:`xyz_to_oklab`. Use them instead!

    This conversion is marked as an extra conversion with the ``ottosson``
    label. As a result, :func:`route` and :func:`convert` only use this function
    if explicitly requested.
    """
    LMS = _multiply(_LINEAR_SRGB_TO_SRGB_LMS, (r, g, b))
    LMSg = cast(_Vector, tuple(map(lambda c: math.cbrt(c), LMS)))
    return _multiply(_SRGB_LMS_TO_OKLAB, LMSg)


@extra('ottosson')
def oklab_to_linear_srgb(L: float, a: float, b: float) -> tuple[float, float, float]:
    """
    :bdg-danger:`Do not use!` Convert the given color from Oklab to linear sRGB
    using `Björn Ottosson <https://bottosson.github.io/posts/oklab/>`_'s
    matrices.

    Since those matrices have only single floating point precision, this
    function is far less accurate than the composition of :func:`oklab_to_xyz`
    and :func:`xyz_to_linear_srgb`. Use them instead!

    This conversion is marked as an extra conversion with the ``ottosson``
    label. As a result, :func:`route` and :func:`convert` only use this function
    if explicitly requested.
    """
    LMSg = _multiply(_OKLAB_TO_SRGB_LMS, (L, a, b))
    LMS = cast(_Vector, tuple(map(lambda c: math.pow(c, 3), LMSg)))
    return _multiply(_SRGB_LMS_TO_LINEAR_SRGB, LMS)


@extra('fused')
def srgb_to_oklab(r: float, g: float, b: float) -> tuple[float, float, float]:
    """
    Convert the given color from sRGB to Oklab. This conversion is marked as an
    extra conversion with the ``fused`` label.
    """
    return (
        xyz_to_oklab(
            *linear_srgb_to_xyz(
                *srgb_to_linear_srgb(r, g, b)
            )
        )
    )


@extra('fused')
def rgb256_to_oklab(r: int, g: int, b: int) -> tuple[float, float, float]:
    """
    Convert the given color from 24-bit RGB to Oklab. This conversion is marked
    as an extra conversion with the ``fused`` label.
    """
    return (
        xyz_to_oklab(
            *linear_srgb_to_xyz(
                *srgb_to_linear_srgb(
                    *rgb256_to_srgb(r, g, b)
                )
            )
        )
    )


# --------------------------------------------------------------------------------------
# Arbitrary Conversions


_ConversionFn = Callable[[float, float, float], tuple[float, float, float]]
_ConversionGraph = dict[str, dict[str, _ConversionFn]]

def collect_conversions(
    conversions: None | _ConversionGraph = None
) -> _ConversionGraph:
    """
    Collect all conversions between color formats and spaces implemented by this
    module.

    Args:
        conversions: If provided, this function fills the dictionary. Otherwise,
            it creates a new dictionary. It always returns the filled
            dictionary.

    Returns:
        A dictionary of dictionaries. Keys for the one outer and the many inner
        dictionaries are tags for color formats and spaces. The value for any
        combination of outer key ``source`` and inner key ``target`` is the
        function that converts the source color into the target color.
    """
    if conversions is None:
        conversions = {}

    for name, value in globals().items():
        if name[0] != '_' and '_to_' in name and callable(value):
            source, _, target = name.partition('_to_')
            targets = conversions.setdefault(source, {})
            targets[target] = value

    return conversions


_all_conversions: _ConversionGraph = {}

def route(source: str, target: str, *, extra: None | str = None) -> list[_ConversionFn]:
    """
    Determine how to convert a color from the source to the target color space.

    Args:
        source: The source color format or space
        target: The target color format or space
        extra: The label for extra conversion functions, ``None`` by default

    Returns:
        A list of functions that perform the desired conversion when applied in
        order from front to back. Each function's arguments should unpack the
        previous function's results.

    Raises:
        ValueError: Indicates that no path from `source` to `target` could be
            found.
    """
    # Handle the first trivial case: No conversion necessary
    if source == target:
        return []

    # We really need the dictionary of dictionaries with all conversions
    if not _all_conversions:
        collect_conversions(_all_conversions)

    # We also need a way of testing for extra compatibility
    def is_suitable_extra(fn: object) -> bool:
        label = getattr(fn, 'extra', None)
        return label is None or label == extra

    # Handle the second trivial case: One function suffices. This also takes
    # care of common cases, which just need a dedicated function.
    fn = _all_conversions[source].get(target)
    if fn is not None and is_suitable_extra(fn):
        return [fn]

    # The general case: Use Dijkstra's algorithm to find shortest path
    previous_spaces: dict[
        str, tuple[None | str, None | _ConversionFn]
    ] = {source: (None, None)}

    todo = deque([source])
    while len(todo) > 0:
        current = todo.popleft()

        if current == target:
            conversions: list[_ConversionFn] = []

            space, fn = previous_spaces[current]
            while space is not None:
                assert fn is not None
                conversions.append(fn)
                space, fn = previous_spaces[space]

            conversions.reverse()
            return conversions

        next_spaces = _all_conversions[current]
        for space, fn in next_spaces.items():
            if space in previous_spaces or not is_suitable_extra(fn):
                continue
            previous_spaces[space] = current, fn
            todo.append(space)

    raise ValueError(f'Cannot convert {source} to {target}')


def convert(
    color: tuple[float, ...],
    source: str,
    target: str,
    *,
    extra: None | str = None,
) -> tuple[float, ...]:
    """
    Convert the given color from its source color space to the target color
    space.

    Args:
        color: The color coordinates to be converted
        source: The color format or space of the coordinates
        target: The target format or color space
        extra: The label for optional conversion functions, ``None`` by default

    Returns:
        The color coordinates in the target format or color space
    """
    conversions = route(source, target, extra=extra)
    for conversion in conversions:
        color = conversion(*color)
    return color
