from collections.abc import Iterable
from dataclasses import dataclass
from typing import cast, Literal, Self

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
from ..color.spec import CoordinateSpec, FloatCoordinateSpec


@dataclass(frozen=True, slots=True)
class Color:
    """
    A color object.

    Attributes:
        tag: identifies the color format or space
        coordinates: are the numerical components of the color

    You can create a new color object either by passing a string that starts
    with ``#``, ``rgb:``, or ``rgbi:`` and is followed by valid color components
    or by passing both the color format or space tag and the coordinates. The
    resulting color object is immutable.
    """
    tag: str
    coordinates: CoordinateSpec

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

    def __init__(
        self,
        tag: str,
        coordinates: None | CoordinateSpec = None
    ) -> None:
        if coordinates is None:
            if tag.startswith('#'):
                tag, coordinates = parse_hex(tag)
            elif tag.startswith('rgb:'):
                tag, coordinates = parse_x_rgb(tag)
            elif tag.startswith('rgbi:'):
                tag, coordinates = parse_x_rgbi(tag)
            else:
                raise ValueError(f'unknown color format "{tag}"')

        object.__setattr__(self, 'tag', tag)
        object.__setattr__(self, 'coordinates', coordinates)

    def __post_init__(self) -> None:
        if not is_tag(self.tag):
            raise ValueError(f'{self.tag} does not identify a color format or space')

    # ----------------------------------------------------------------------------------

    def is_in_gamut(self, epsilon: float = EPSILON) -> bool:
        """Determine whether this color is within gamut for its color space."""
        return self.space.is_in_gamut(*self.coordinates, epsilon)

    def clip(self, epsilon: float = 0) -> Self:
        """Clip this color to its color space's gamut."""
        if self.is_in_gamut(epsilon):
            return self
        return type(self)(self.tag, self.space.clip(*self.coordinates))

    # ----------------------------------------------------------------------------------

    def to(self, target: str) -> Self:
        """Convert this color to the specified color format or space."""
        if self.tag == target:
            return self
        return type(self)(target, get_converter(self.tag, target)(*self.coordinates))

    # ----------------------------------------------------------------------------------

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

    def __format__(self, format_spec: str) -> str:
        fmt, precision = parse_format_spec(format_spec)
        return stringify(self.tag, self.coordinates, fmt, precision)

    def __str__(self) -> str:
        return stringify(self.tag, self.coordinates)
