from collections.abc import Iterable
from typing import cast, Literal, overload, Self

from ..color.apca import contrast, use_black_text, use_black_background
from ..color.conversion import get_converter
from ..color.difference import deltaE_oklab, closest_oklab
from ..color.serde import (
    parse_format_spec,
    parse_hex,
    parse_x_rgb,
    parse_x_rgbi,
    stringify,
)
from ..color.space import EPSILON, is_tag, resolve, Space
from ..color.spec import ColorSpec, CoordinateSpec, FloatCoordinateSpec


class Color(ColorSpec):
    """
    A color object.

    This class implements the high-level, object-oriented API for colors.

    Attributes:
        tag: identifies the color format or space
        coordinates: are the numerical components of the color

    Color's constructor supports a number of options for specifying the color
    and its coordinates:

        * From an existing :class:`.ColorSpec` or ``Color`` object
        * From the textual representation of a color, using the ``#``,
          ``rgb:``, or ``rgbi:`` prefix
        * From a tag and tuple with coordinates
        * From a tag and one or three coordinates

    When invoked on an existing color specification or color object, the
    constructor returns a new color object with the same tag and coordinates.
    Hence, it effectively upgrades color specifications to full featured color
    objects.

    Like its parent class :class:`.ColorSpec`, this class is immutable.
    """
    __slots__ = ()

    @property
    def space(self) -> Space:
        """Get the color space for this color."""
        return resolve(self.tag)


    def __getattr__(self, name: str) -> float | Self:
        """
        Provide access to color space coordinates by single-letter name. Also,
        convert to named color format or space.
        """
        if len(name) == 1:
            for coordinate, value in zip(self.space.coordinates, self.coordinates):
                if coordinate.name == name:
                    return value
        elif is_tag(name):
            return self.to(name)

        raise AttributeError(f'color {self} has no attribute named "{name}"')

    # ----------------------------------------------------------------------------------


    @overload
    def __init__(self, tag: str | ColorSpec | Self) -> None:
        ...

    @overload
    def __init__(self, tag: str, coordinates: CoordinateSpec) -> None:
        ...

    @overload
    def __init__(self, tag: str, coordinates: float, c2: float, c3: float) -> None:
        ...

    def __init__(
        self,
        tag: str | ColorSpec | Self,
        coordinates: None | float | CoordinateSpec = None,
        c2: None | float = None,
        c3: None | float = None,
    ) -> None:
        if isinstance(tag, ColorSpec):
            tag, coordinates = tag.tag, tag.coordinates
        elif coordinates is None:
            if tag.startswith('#'):
                tag, coordinates = parse_hex(tag)
            elif tag.startswith('rgb:'):
                tag, coordinates = parse_x_rgb(tag)
            elif tag.startswith('rgbi:'):
                tag, coordinates = parse_x_rgbi(tag)
            else:
                raise ValueError(f'"{tag}" is not a valid color')
        elif c2 is not None:
            assert isinstance(coordinates, float) and c3 is not None
            coordinates = coordinates, c2, c3

        object.__setattr__(self, 'tag', tag)
        object.__setattr__(self, 'coordinates', coordinates)
        self.__post_init__()


    def update(
        self,
        c1: None | float,
        c2: None | float = None,
        c3: None | float = None,
    ) -> Self:
        """
        Update this color.

        Since color objects are immutable, this method returns a new color. That
        color has the same color format or space as this color. Its coordinates
        are the arguments to this method, except that this color's coordinates
        fill null arguments.
        """
        if len(self.coordinates) == 1:
            assert c2 is None and c3 is None
            return type(self)(self.tag, cast(tuple[int], (c1 or self.coordinates[0],)))

        b1, b2, b3 = self.coordinates
        return type(self)(self.tag, (c1 or b1, c2 or b2, c3 or b3))

    # ----------------------------------------------------------------------------------
    # Gamut and Clipping

    def is_in_gamut(self, epsilon: float = EPSILON) -> bool:
        """Determine whether this color is within gamut for its color space."""
        return self.space.is_in_gamut(*self.coordinates, epsilon)


    def clip(self, epsilon: float = 0) -> Self:
        """Clip this color to its color space's gamut."""
        if self.is_in_gamut(epsilon):
            return self
        return type(self)(self.tag, self.space.clip(*self.coordinates))

    # ----------------------------------------------------------------------------------
    # Conversion to Other Formats and Color Spaces

    def to(self, target: str) -> Self:
        """Convert this color to the specified color format or space."""
        if self.tag == target:
            return self
        return type(self)(target, get_converter(self.tag, target)(*self.coordinates))

    # ----------------------------------------------------------------------------------
    # Distance from Other Colors

    def distance(self, other: Self, *, version: Literal[1, 2] = 1) -> float:
        """
        Determine the symmetric distance ΔE between this color and the given
        color.
        """
        return deltaE_oklab(
            *self.to('oklab').coordinates,
            *other.to('oklab').coordinates,
            version=version,
        )

    def closest(self, colors: Iterable[Self]) -> int:
        """
        Find the color with the smallest symmetric distance from this color and
        return its index.
        """
        index, _ = closest_oklab(
            cast(FloatCoordinateSpec, self.to('oklab').coordinates),
            (cast(FloatCoordinateSpec, c.to('oklab').coordinates) for c in colors),
        )
        return index

    # ----------------------------------------------------------------------------------
    # Contrast Against Other Colors

    def contrast_against(self, background: Self) -> float:
        """
        Determine the asymmetric contrast of text with this color against the
        given background.
        """
        return contrast(
            cast(FloatCoordinateSpec, self.to('srgb').coordinates),
            cast(FloatCoordinateSpec, background.to('srgb').coordinates),
        )

    def use_black_text(self) -> bool:
        """
        Determine whether to use black or white text against a background of
        this color for maximum contrast.
        """
        return use_black_text(*self.to('srgb').coordinates)


    def use_black_background(self) -> bool:
        """
        Determine whether to use a black or white background for text of this
        color for maximum contrast.
        """
        return use_black_background(*self.to('srgb').coordinates)

    # ----------------------------------------------------------------------------------
    # Serialization to Text

    def __format__(self, format_spec: str) -> str:
        fmt, precision = parse_format_spec(format_spec)
        return stringify(self.tag, self.coordinates, fmt, precision)


    def __str__(self) -> str:
        return stringify(self.tag, self.coordinates)
