"""
Support for low-resolution terminal colors
"""
from itertools import chain
from typing import TypeAlias

from .conversion import get_converter
from .difference import closest_oklab
from .theme import current_theme, Theme


_CoordinateVector: TypeAlias = tuple[tuple[float, float, float], ...]

_RGB6_TO_RGB256 = (0, 0x5F, 0x87, 0xAF, 0xD7, 0xFF)


def rgb6_to_eight_bit(r: int, g: int, b: int) -> int:
    """
    Convert the given color from the 6x6x6 RGB cube of 8-bit terminal colors to
    an actual 8-bit terminal color.
    """
    assert 0 <= r <= 5 and 0 <= g <= 5 and 0 <= b <= 5
    return 16 + 36 * r + 6 * g + b


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


def rgb256_to_rgb6(r: int, g: int, b: int) -> tuple[int, int, int]:
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


def _eight_bit_gray_to_rgb256(color: int) -> tuple[int, int, int]:
    """Convert the given 8-bit gray to RGB256 format."""
    assert 232 <= color <= 255
    c = 10 * (color - 232) + 8
    return c, c, c


def eight_bit_to_rgb256(color: int) -> tuple[int, int, int]:
    """
    Convert the given 8-bit terminal color to 24-bit RGB.

    .. warning::
        The result of this function may depend on the current color theme.
        It provides RGB256 color values for 8-bit colors 0–15, i.e., the
        extended ANSI colors.
    """
    if 0 <= color <= 15:
        return current_theme().ansi(color)
    if 16 <= color <= 231:
        return rgb6_to_rgb256(*eight_bit_to_rgb6(color))
    if 232 <= color <= 255:
        return _eight_bit_gray_to_rgb256(color)

    raise ValueError(f'{color} is not a valid 8-bit terminal color')


class _LUT:

    def __init__(self) -> None:
        self._ansi: dict[Theme, _CoordinateVector] = {}
        self._rgb: None | _CoordinateVector = None
        self._gray: None | _CoordinateVector = None

        self.convert = get_converter('rgb256', 'oklab')

    @property
    def ansi(self) -> _CoordinateVector:
        theme = current_theme()
        if theme not in self._ansi:
            self._ansi[theme] = tuple(
                self.convert(*c)
                for n, c in theme.colors() if n not in ('text', 'background')
            )
        return self._ansi[theme]

    @property
    def rgb(self) -> _CoordinateVector:
        if self._rgb is None:
            self._rgb = tuple(
                self.convert(*rgb6_to_rgb256(r, g, b))
                for r in range(6) for g in range(6) for b in range(6)
            )
        return self._rgb

    @property
    def gray(self) -> _CoordinateVector:
        if self._gray is None:
            self._gray = tuple(
                self.convert(*_eight_bit_gray_to_rgb256(c))
                for c in range(232, 256)
            )
        return self._gray

_look_up_table = _LUT()


def oklab_to_eight_bit(L: float, a: float, b: float) -> int:
    """
    :bdg-warning:`Lossy conversion` Convert the given color from Oklab to an
    8-bit terminal color.
    """
    index, _ = closest_oklab((L, a, b), chain(_look_up_table.rgb, _look_up_table.gray))
    return 16 + index


def oklab_to_rgb6(L: float, a: float, b: float) -> tuple[int, int, int]:
    """
    :bdg-warning:`Lossy conversion` Convert the given color from Oklab to RGB6.
    """
    index, _ = closest_oklab((L, a, b), _look_up_table.rgb)
    return eight_bit_to_rgb6(16 + index)


def oklab_to_ansi(L: float, a: float, b: float) -> int:
    """
    :bdg-warning:`Lossy conversion` Convert the given color from Oklab to the
    extended sixteen ANSI colors.

    .. warning::
        The result of this function critically depends on the current color
        theme. It provides an implicit input in addition to the arguments.
    """
    index, _ = closest_oklab((L, a, b), _look_up_table.ansi)
    return index


def lores_to_rgb256(source: str, *coordinates: int) -> tuple[int, int, int]:
    fn = rgb6_to_rgb256 if source == 'rgb6' else eight_bit_to_rgb256
    return fn(*coordinates)

def oklab_to_lores(target: str, *coordinates: float) -> int | tuple[int, int, int]:
    return globals()[f'oklab_to_{target}'](*coordinates)
