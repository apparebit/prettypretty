"""Metadata about color spaces"""
import dataclasses
import math
from typing import Literal, Self


@dataclasses.dataclass(frozen=True, slots=True)
class Coordinate:
    """
    Representation of a color space coordinate.

    Attributes:
        name: the single-letter name of the coordinate or an empty string
            for coordinates that have no name
        min: the optional minimum value for the coordinate
        max: the optional maximum value for the coordinate
        type: the optional type to further clarify coordinate semantics

    More specifically, the ``type`` can identify one of three common classes of
    coordinates:

      * An **angle** is a floating point number representing a rotation between
        0 and 360 degrees, inclusive; used for the *hue* in polar color spaces.
      * A **normal** is a floating point number between 0 and 1, inclusive.
      * An **int** may have arbitrary range but all its values must be integral.

    """
    name: str
    min: None | float = None
    max: None | float = None
    type: None | Literal['angle', 'int', 'normal'] = None

    def __post_init__(self) -> None:
        if len(self.name) > 1:
            raise ValueError(
                f'Color space coordinate name "{self.name}" is not single-letter'
            )

    @classmethod
    def for_normal(cls, name: str) -> Self:
        return cls(name, 0, 1, 'normal')

    @classmethod
    def for_angle(cls, name: str) -> Self:
        return cls(name, 0, 360, 'angle')

    def is_in_range(self, value: float, *, epsilon: float = .000075) -> bool:
        if math.isnan(value):
            return self.type == 'angle'
        if self.type == 'int' and not isinstance(value, int):
            return False
        return (
            (self.min is None or self.min - epsilon <= value)
            and (self.max is None or value <= self.max + epsilon)
        )

    def clip(self, value: float) -> float:
        if self.min is not None and value < self.min:
            return self.min
        if self.max is not None and value > self.max:
            return self.max
        return value


@dataclasses.dataclass(frozen=True, slots=True)
class Space:
    """
    Representation of a color space.

    Attributes:
        tag: a valid Python name used to uniquely identify the color space
        name: a human-readable, descriptive name
        base: the optional base color space
        coordinates: the coordinates of the color space
    """
    tag: str
    name: str
    base: None | Self
    coordinates: tuple[Coordinate, ...]
    css_format: None | str

    def is_in_gamut(self, *values: float, epsilon: float = .000075) -> bool:
        assert len(self.coordinates) == len(values)
        for coordinate, value in zip(self.coordinates, values):
            if not coordinate.is_in_range(value, epsilon=epsilon):
                return False
        return True

    def clip(self, *values: float) -> tuple[float, ...]:
        return tuple(c.clip(v) for c, v in zip(self.coordinates, values))


_RGB_COORDINATES = (
    Coordinate.for_normal('r'),
    Coordinate.for_normal('g'),
    Coordinate.for_normal('b'),
)

XYZ = XYZ_D65 = Space(
    'xyz',
    'XYZ D65',
    None,
    (
        Coordinate('X'),
        Coordinate('Y'),
        Coordinate('Z'),
    ),
    'color(xyz {})',
)

LINEAR_SRGB = Space(
    'linear_srgb',
    'Linear sRGB',
    XYZ,
    _RGB_COORDINATES,
    'color(srgb-linear {})',
)

SRGB = Space(
    'srgb',
    'sRGB',
    LINEAR_SRGB,
    _RGB_COORDINATES,
    'color(srgb {})',
)

RGB256 = Space(
    'rgb256',
    '24-bit RGB',
    SRGB,
    (
        Coordinate('r', 0, 255, 'int'),
        Coordinate('g', 0, 255, 'int'),
        Coordinate('b', 0, 255, 'int'),
    ),
    'rgb({})',
)

RGB6 = Space(
    'rgb6',
    '8-bit terminal color RGB',
    RGB256,
    (
        Coordinate('r', 0, 6, 'int'),
        Coordinate('g', 0, 6, 'int'),
        Coordinate('b', 0, 6, 'int'),
    ),
    None,
)

EIGHT_BIT = Space(
    'eight_bit',
    '8-bit terminal color',
    None,
    (
        Coordinate('', 0, 255, 'int'),
    ),
    None,
)

ANSI = Space(
    'ansi',
    'ANSI terminal color',
    EIGHT_BIT,
    (
        Coordinate('', 0, 15, 'int'),
    ),
    None,
)

LINEAR_P3 = Space(
    'linear_p3',
    'Linear P3',
    XYZ,
    _RGB_COORDINATES,
    None,
)

P3 = Space(
    'p3',
    'P3',
    LINEAR_P3,
    _RGB_COORDINATES,
    'color(display-p3 {})',
)

OKLAB = Space(
    'oklab',
    'Oklab',
    XYZ,
    (
        Coordinate.for_normal('L'),
        Coordinate('a', -0.4, 0.4),
        Coordinate('b', -0.4, 0.4),
    ),
    'oklab({})',
)

OKLCH = Space(
    'oklch',
    'Oklch',
    OKLAB,
    (
        Coordinate.for_normal('L'),
        Coordinate('C', 0, 0.4),
        Coordinate.for_angle('h'),
    ),
    'oklch({})',
)


def resolve(tag: str) -> Space:
    """Resolve the given tag into the corresponding color space."""
    space = globals()[tag.upper()]
    if isinstance(space, Space):
        return space
    raise AttributeError(f"module {__name__} has no color space with tag '{tag}'")


def is_tag(tag: str) -> bool:
    """
    Determine whether the given string is a valid tag for color format or space.
    """
    try:
        resolve(tag)
    except:
        return False
    else:
        return True
