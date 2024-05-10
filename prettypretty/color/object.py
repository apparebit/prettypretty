from typing import cast, Literal, NoReturn, overload, Self

from .apca import contrast, use_black_text, use_black_background
from .conversion import convert
from .space import resolve, Space
from .theme import ColorSpec


@overload
def _check(
    is_valid: Literal[False], entity: str, value: object, deficiency: str = ...
) -> NoReturn:
    ...
@overload
def _check(
    is_valid: bool, entity: str, value: object, deficiency: str = ...
) -> None | NoReturn:
    ...
def _check(
    is_valid: bool, entity: str, value: object, deficiency: str = 'is malformed'
) -> None | NoReturn:
    if not is_valid:
        raise SyntaxError(f'{entity} "{value}" {deficiency}')
    return


def parse_hex(color: str) -> tuple[str, tuple[int, ...]]:
    entity = 'hex web color'

    try:
        _check(color.startswith('#'), entity, color, 'does not start with "#"')
        color = color[1:]
        digits = len(color)
        _check(digits in (3, 6), entity, color, 'does not have 3 or 6 digits')
        if digits == 3:
            color = ''.join(f'{d}{d}' for d in color)
        return 'rgb256', tuple(int(color[n:n+2], base=16) for n in range(0, 6, 2))
    except SyntaxError:
        raise
    except:
        _check(False, entity, color)


def parse_x_rgb(color: str) -> tuple[str, tuple[float, ...]]:
    entity = 'X rgb color'

    try:
        _check(color.startswith('rgb:'), entity, color, 'does not start with "rgb:"')
        hexes = [(f'{h}{h}' if len(h) == 1 else h) for h in color[4:].split('/')]
        _check(len(hexes) == 3, entity, color, 'does not have three components')
        if max(len(h) for h in hexes) == 2:
            return 'rgb256', tuple(int(h, base=16) for h in hexes)
        else:
            return 'srgb', tuple(int(h, base=16) / 16 ** len(h) for h in hexes)
    except SyntaxError:
        raise
    except:
        _check(False, entity, color)


def parse_x_rgbi(color: str) -> tuple[str, tuple[float, ...]]:
    entity = 'X rgbi color'

    try:
        _check(color.startswith('rgbi:'), entity, color, 'does not start with "rgbi:"')
        cs = [float(c) for c in color[5:].split('/')]
        _check(len(cs) == 3, entity, color, 'does not have three components')
        for c in cs:
            _check(0 <= c <= 1, entity, color, 'has non-normal component')
        return 'srgb', tuple(cs)
    except SyntaxError:
        raise
    except:
        _check(False, entity, color)


type CoordinateValues = tuple[float, float, float]


class Color(ColorSpec):
    """
    A color.

    Like its baseclass :class:`prettypretty.color.theme.ColorSpec`, this class
    is immutable. Consequently, all methods that compute a different color
    value, e.g., by converting to another color space or clipping to its own
    color space, also return a new color instance.
    """

    __slots__ = ()

    @property
    def space(self) -> Space:
        """This color's color space."""
        return resolve(self.tag)

    @classmethod
    def of(cls, s: str) -> Self:
        """Create a new color by parsing the given string."""
        if s.startswith('#'):
            return cls(*parse_hex(s))
        elif s.startswith('rgb:'):
            return cls(*parse_x_rgb(s))
        elif s.startswith('rgbi:'):
            return cls(*parse_x_rgbi(s))
        else:
            raise ValueError(f'unable to parse "{s}"')

    def to(self, target: str) -> Self:
        """Convert this color to the color format or space with the given tag."""
        if self.tag == target:
            return self
        return type(self)(target, convert(self.coordinates, self.tag, target))

    def is_in_gamut(self) -> bool:
        """Check whether this color is within the gamut of its color space."""
        return self.space.is_in_gamut(*self.coordinates)

    def clip(self) -> Self:
        """Clip this color to the gamut of its color space."""
        return type(self)(self.tag, self.space.clip(*self.coordinates))

    def contrast_against(self, background: Self) -> float:
        """
        Determine the contrast for text with this color against the given
        background color.
        """
        return contrast(
            cast(CoordinateValues, self.to('srgb').coordinates),
            cast(CoordinateValues, background.to('srgb').coordinates),
        )

    def has_better_contrast_with_black_text(self) -> bool:
        """
        Determine whether a background with this color has better contrast with
        black or white text.
        """
        return use_black_text(*self.to('srgb').coordinates)

    def has_better_contrast_with_black_background(self) -> bool:
        """
        Determine whether text with this color has better contrast against a
        black or white background.
        """
        return use_black_background(*self.to('srgb').coordinates)

    def __getattr__(self, name: str) -> float | Self:
        if len(name) > 1:
            return self.to(name)

        for coordinate, value in zip(self.space.coordinates, self.coordinates):
            if coordinate.name == name:
                return value
        raise AttributeError(f'color {self} has no attribute named "{name}"')

    # FIXME: add support for the CSS method of gamut mapping
    # FIXME: add support for color ranges
