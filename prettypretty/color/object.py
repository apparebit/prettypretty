from typing import cast, Literal, NoReturn, overload, Self

from .apca import contrast, use_black_text, use_black_background
from .conversion import convert
from .difference import deltaE_oklab, closest_oklab
from .space import is_tag, resolve, Space
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

    To convert a color to another format or color space, you can invoke the
    :meth:`to` method or access the eponymous attribute. Similarly, you can
    access a specific coordinate through the ``coordinates`` tuple as well as
    the corresponding attribute. For example:

    >>> red = Color.of('#f00')
    >>> red
    Color(tag='rgb256', coordinates=(255, 0, 0))
    >>> red.srgb
    Color(tag='srgb', coordinates=(1.0, 0.0, 0.0))
    >>> red.coordinates[0]
    255
    >>> red.r
    255

    """
    # FIXME: add support for the CSS method of gamut mapping
    # FIXME: add support for color ranges

    __slots__ = ()

    @property
    def space(self) -> Space:
        """This color's color space."""
        return resolve(self.tag)

    def __getattr__(self, name: str) -> float | Self:
        """
        Enable access to color coordinate values through coordinate names as
        well as conversion to color formats and spaces through the corresponding
        tags.
        """
        if len(name) == 1:
            for coordinate, value in zip(self.space.coordinates, self.coordinates):
                if coordinate.name == name:
                    return value
        elif is_tag(name):
            return self.to(name)

        raise AttributeError(f'color {self} has no attribute named "{name}"')

    # ----------------------------------------------------------------------------------
    # Conversion to colors

    @classmethod
    def of(cls, color: str| ColorSpec | Self) -> Self:
        """
        Turn the color argument into a color object.

        If the argument is a color object already, this method simply returns
        it. If the argument is a ``ColorSpec``, this method upgrades it to a
        color object. Finally, if the argument is a string, this method parses
        it.
        """
        if isinstance(color, cls):
            return color
        elif isinstance(color, ColorSpec):
            return cls(color.tag, color.coordinates)
        elif color.startswith('#'):
            return cls(*parse_hex(color))
        elif color.startswith('rgb:'):
            return cls(*parse_x_rgb(color))
        elif color.startswith('rgbi:'):
            return cls(*parse_x_rgbi(color))
        else:
            raise ValueError(f'unable to parse "{color}"')

    # ----------------------------------------------------------------------------------
    # Conversion to other formats and spaces

    def to(self, target: str) -> Self:
        """Convert this color to the color format or space with the given tag."""
        if self.tag == target:
            # Avoid recreating an immutable object
            return self
        return type(self)(target, convert(self.coordinates, self.tag, target))

    # ----------------------------------------------------------------------------------
    # Gamut

    def is_in_gamut(self) -> bool:
        """Check whether this color is within the gamut of its color space."""
        return self.space.is_in_gamut(*self.coordinates)

    def clip(self) -> Self:
        """Clip this color to the gamut of its color space."""
        return type(self)(self.tag, self.space.clip(*self.coordinates))

    # ----------------------------------------------------------------------------------
    # Difference

    def difference(self, other: ColorSpec) -> float:
        """Determine the difference ΔE between this and the other color."""
        c1 = self.to('oklab').coordinates
        c2 = convert(other.coordinates, other.tag, 'oklab')
        return deltaE_oklab(*c1, *c2)

    def closest(self, *candidates: ColorSpec) -> tuple[int, Self]:
        """
        Find the closest color.

        Args:
            candidates: to compare with this color

        Returns:
            the one-based index and color object for the candidate with the
            smallest difference; if there are no candidates, the index is zero
            and the color is this color.
        """
        if len(candidates) == 0:
            return 0, self

        c0 = cast(CoordinateValues, self.to('oklab').coordinates)
        cs = [
            cast(
                CoordinateValues,
                convert(candidate.coordinates, candidate.tag, 'oklab'),
            ) for candidate in candidates
        ]
        index, _ = closest_oklab(c0, *cs)
        return index, type(self).of(candidates[index - 1])

    # ----------------------------------------------------------------------------------
    # Contrast

    def contrast_against(self, background: Self) -> float:
        """
        Determine the contrast for text with this color against the given
        background color. Note that this method is *not* symmetric. The result
        of ``text.contrast_against(background)`` is likely different from
        ``background.contrast_against(text)`` and only the first invocation is
        semantically correct.
        """
        return contrast(
            cast(CoordinateValues, self.to('srgb').coordinates),
            cast(CoordinateValues, background.to('srgb').coordinates),
        )

    def use_with_black_text(self) -> bool:
        """
        Determine whether a background with this color has better contrast with
        black text than with white text.
        """
        return use_black_text(*self.to('srgb').coordinates)

    def use_with_black_background(self) -> bool:
        """
        Determine whether text with this color has better contrast against a
        black background than against a white background.
        """
        return use_black_background(*self.to('srgb').coordinates)

    # ----------------------------------------------------------------------------------
    # String formatting

    def __format__(self, format_spec: str) -> str:
        """
        Format this color according to the given specification. This method only
        recognizes one term from Python's format string mini-language, the
        precision. Just as for the mini-language, it is prefixed by a period.
        """
        precision = int(format_spec[1:]) if format_spec.startswith('.') else 4
        coordinates = ', '.join(
            f'{c}' if isinstance(c, int) else f'{c:.{precision}}'
            for c in self.coordinates
        )
        return f'{self.tag}({coordinates})'

    def __str__(self) -> str:
        return self.__format__('.4')

    def css(self, precision: int = 4) -> str:
        """
        Convert the color to its CSS serialization.

        Out of prettypretty's color formats and spaces, ``ansi``, ``eight_bit``,
        ``rgb6``, and ``linear_p3`` do *not* have a CSS serialization. If this
        color is in one of these color formats or spaces, this function raises
        an exception. Convert to a different format or space first.
        """
        css_format = self.space.css_format
        if css_format is None:
            raise ValueError(f'{self!r} does not have a CSS serialization')

        return css_format.format(' '.join(
            f'{c}' if isinstance(c, int) else f'{c:.{precision}}'
            for c in self.coordinates
        ))
