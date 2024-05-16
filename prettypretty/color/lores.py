"""
Support for low-resolution terminal colors
"""
from itertools import chain
from typing import Callable, cast

# Rename basic conversion to prevent it from tripping up module scan
from .conversion import get_converter, rgb256_to_srgb as rgb256_into_srgb
from .difference import closest_oklab
from .spec import CoordinateVectorSpec
from .theme import current_theme, Theme


_RGB6_TO_RGB256 = (0, 0x5F, 0x87, 0xAF, 0xD7, 0xFF)


def rgb6_to_eight_bit(r: int, g: int, b: int) -> tuple[int]:
    """
    Convert the given color from the 6x6x6 RGB cube of 8-bit terminal colors to
    an actual 8-bit terminal color.
    """
    assert 0 <= r <= 5 and 0 <= g <= 5 and 0 <= b <= 5
    return 16 + 36 * r + 6 * g + b,


def eight_bit_to_rgb6(color: int) -> tuple[int, int, int]:
    """
    Convert the given 8-bit color to the three components of the 6x6x6 RGB cube.
    The color value must be between 16 and 231, inclusive.
    """
    assert 16 <= color <= 231

    b = color - 16
    r = b // 36
    b -= 36 * r
    g = b // 6
    b -= 6 * g
    return r, g, b


def rgb6_to_rgb256(r: int, g: int, b: int) -> tuple[int, int, int]:
    """Convert the given color in RGB6 format to RGB256 format."""
    assert 0 <= r <= 5 and 0 <= g <= 5 and 0 <= b <= 5
    return _RGB6_TO_RGB256[r], _RGB6_TO_RGB256[g], _RGB6_TO_RGB256[b]


def approximate_rgb256_with_rgb6(r: int, g: int, b: int) -> tuple[int, int, int]:
    """
    :bdg-warning:`Lossy conversion` Convert the given color from RGB256 to RGB6.

    This function effectively reverses the conversion from RGB6 to RGB256: It
    compares each RGB256 coordinate with the RGB256 values used for the inverse
    and picks the RGB6 with the closest RGB256 value for the inverse.

    The correctness of this particular implementation depends on the inverse
    mapping the extrema of the domain to the extrema of the codomain, i.e., 0 to
    0 and 5 to 255.
    """
    assert 0 <= r <= 255 and 0 <= g <= 255 and 0 <= b <= 255

    def convert(value: int) -> int:
        for index, level in enumerate(_RGB6_TO_RGB256):
            if value == level:
                return index
            if value > level:
                continue

            # The RGB256 value is between two RGB6 values. Pick the closer one.
            previous_level = _RGB6_TO_RGB256[index - 1]
            return index if level - value < value - previous_level else index - 1

        assert False, 'unreachable statement'

    return convert(r), convert(g), convert(b)


def ansi_to_eight_bit(color: int) -> tuple[int]:
    """Convert the given ANSI color to 8-bit format."""
    return color,


def ansi_to_rgb256(color: int) -> tuple[int, int, int]:
    """
    :bdg-warning:`Lossy conversion` Convert the given ANSI color to RGB256
    format.

    .. warning::
        The result of this function critically depends on the current color
        theme. After all, the current theme determines the RGB256 values for all
        extended ANSI colors.
    """
    assert 0 <= color <= 15
    c = current_theme().ansi(color)

    assert c.tag == 'rgb256'
    return c.coordinates  # type: ignore


def ansi_to_srgb(color: int) -> tuple[float, float, float]:
    """
    Convert the ANSI color to sRGB. Directly converting to sRGB and avoiding
    RGB256 is the more conservative conversion because most terminals, when
    queried with OSC-4, report color values with four hexadecimal digits per
    coordinate. RGB256 obviously cannot preserve that resolution, though sRGB
    can.

    .. warning::
        The result of this function critically depends on the current color
        theme. After all, the current theme determines the RGB256 values for all
        extended ANSI colors.
    """
    assert 0 <= color <= 15
    c = current_theme().ansi(color)

    if c.tag == 'rg256':
        coordinates = rgb256_into_srgb(*cast(tuple[int, int, int], c.coordinates))
    elif c.tag == 'srgb':
        coordinates = c.coordinates
    else:
        coordinates = get_converter(c.tag, 'srgb')(*c.coordinates)

    return cast(tuple[float, float, float], coordinates)


def _eight_bit_gray_to_rgb256(color: int) -> tuple[int, int, int]:
    """Convert the given 8-bit gray to RGB256 format."""
    assert 232 <= color <= 255
    c = 10 * (color - 232) + 8
    return c, c, c


def eight_bit_to_rgb256(color: int) -> tuple[int, int, int]:
    """
    :bdg-warning:`Lossy conversion` Convert the given 8-bit terminal color to
    24-bit RGB.

    .. warning::
        The result of this function may depend on the current color theme.
        It provides RGB256 color values for 8-bit colors 0–15, i.e., the
        extended ANSI colors.
    """
    if 0 <= color <= 15:
        return ansi_to_rgb256(color)
    if 16 <= color <= 231:
        return rgb6_to_rgb256(*eight_bit_to_rgb6(color))
    if 232 <= color <= 255:
        return _eight_bit_gray_to_rgb256(color)

    raise ValueError(f'{color} is not a valid 8-bit terminal color')


def eight_bit_to_srgb(color: int) -> tuple[float, float, float]:
    """
    Convert the given 8-bit terminal color to sRGB.

    .. warning::
        The result of this function may depend on the current color theme.
        It provides RGB256 color values for 8-bit colors 0–15, i.e., the
        extended ANSI colors.
    """
    if 0 <= color <= 15:
        return ansi_to_srgb(color)
    if 16 <= color <= 231:
        return rgb256_into_srgb(*rgb6_to_rgb256(*eight_bit_to_rgb6(color)))
    if 232 <= color <= 255:
        return rgb256_into_srgb(*_eight_bit_gray_to_rgb256(color))

    raise ValueError(f'{color} is not a valid 8-bit terminal color')


# --------------------------------------------------------------------------------------


_RGB256_TO_OKLAB = Callable[[int, int, int], tuple[float, float, float]]

class _LUT:

    def __init__(self) -> None:
        self._ansi: dict[Theme, CoordinateVectorSpec] = {}
        self._rgb: None | CoordinateVectorSpec = None
        self._gray: None | CoordinateVectorSpec = None
        self._convert: None | _RGB256_TO_OKLAB = None

    @property
    def convert(self) -> _RGB256_TO_OKLAB:
        if self._convert is None:
            setattr(self, '_convert', get_converter('rgb256', 'oklab'))
        assert self._convert is not None
        return self._convert

    @property
    def ansi(self) -> CoordinateVectorSpec:
        theme = current_theme()
        if theme not in self._ansi:
            self._ansi[theme] = tuple(
                (self.convert if c.tag == 'srgb256' else get_converter(c.tag, 'oklab'))
                (*c.coordinates)  # type: ignore
                for n, c in theme.colors() if n not in ('text', 'background')
            )
        return self._ansi[theme]

    @property
    def rgb(self) -> CoordinateVectorSpec:
        if self._rgb is None:
            self._rgb = tuple(
                self.convert(*rgb6_to_rgb256(r, g, b))
                for r in range(6) for g in range(6) for b in range(6)
            )
        return self._rgb

    @property
    def gray(self) -> CoordinateVectorSpec:
        if self._gray is None:
            self._gray = tuple(
                self.convert(*_eight_bit_gray_to_rgb256(c))
                for c in range(232, 256)
            )
        return self._gray

_look_up_table = _LUT()


def oklab_to_eight_bit(L: float, a: float, b: float) -> tuple[int]:
    """
    :bdg-warning:`Lossy conversion` Convert the given color from Oklab to an
    8-bit terminal color.
    """
    index, _ = closest_oklab((L, a, b), chain(_look_up_table.rgb, _look_up_table.gray))
    return 16 + index,


def oklab_to_rgb6(L: float, a: float, b: float) -> tuple[int, int, int]:
    """
    :bdg-warning:`Lossy conversion` Convert the given color from Oklab to RGB6.
    """
    index, _ = closest_oklab((L, a, b), _look_up_table.rgb)
    return eight_bit_to_rgb6(16 + index)


def oklab_to_ansi(L: float, a: float, b: float) -> tuple[int]:
    """
    :bdg-warning:`Lossy conversion` Convert the given color from Oklab to the
    extended sixteen ANSI colors.

    .. warning::
        The result of this function critically depends on the current color
        theme. It provides an implicit input in addition to the arguments.
    """
    index, _ = closest_oklab((L, a, b), _look_up_table.ansi)
    return index,
