from dataclasses import dataclass
import math
from typing import Literal, Self


@dataclass(frozen=True, slots=True)
class Coordinate:
    """
    Representation of a color space coordinate.

    Every coordinate has a unique ``name``. If its domain is restricted, ``min``
    and ``max`` define the range of valid values. If the coordinate meets the
    requirements of one of the three types, it also declares that type:

      * An **angle** is a floating point number between 0 and 360, inclusive.
      * A **normal** is a floating point number between 0 and 1, inclusive.
      * An **int** may have arbitrary range but all its values must be integral.
    """

    name: str
    min: None | float = None
    max: None | float = None
    type: None | Literal['angle', 'int', 'normal'] = None

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


@dataclass(frozen=True, slots=True)
class Space:
    """
    Representation of a color space.


    """

    tag: str
    name: str
    base: None | Self
    coordinates: tuple[Coordinate, ...]

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
)

LINEAR_SRGB = Space(
    'linear_srgb',
    'Linear sRGB',
    XYZ,
    _RGB_COORDINATES,
)

SRGB = Space(
    'srgb',
    'sRGB',
    LINEAR_SRGB,
    _RGB_COORDINATES,
)

RGB256 = Space(
    'rgb256',
    '24-bit RGB',
    SRGB,
    (
        Coordinate('r', 0, 255, 'int'),
        Coordinate('g', 0, 255, 'int'),
        Coordinate('b', 0, 255, 'int'),
    )
)

RGB6 = Space(
    'rgb6',
    '8-bit terminal color RGB',
    RGB256,
    (
        Coordinate('r', 0, 6, 'int'),
        Coordinate('g', 0, 6, 'int'),
        Coordinate('b', 0, 6, 'int'),
    )
)

EIGHT_BIT_CUBE = Space(
    'eight_bit_cube',
    '8-bit terminal 6x6x6 cube',
    RGB6,
    (
        Coordinate('value', 16, 231, 'int'),
    )
)

EIGHT_BIT_GREY = Space(
    'eight_bit_grey',
    '8-bit terminal grey',
    RGB256,
    (
        Coordinate('value', 232, 255, 'int'),
    )
)

LINEAR_P3 = Space(
    'linear_p3',
    'Linear P3',
    XYZ,
    _RGB_COORDINATES,
)

P3 = Space(
    'p3',
    'P3',
    LINEAR_P3,
    _RGB_COORDINATES,
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
)


def resolve(tag: str) -> Space:
    """Resolve the given tag into the corresponding color space."""
    space = globals()[tag.upper()]
    if isinstance(space, Space):
        return space
    raise AttributeError(f"module {__name__} has no color space with tag '{tag}'")
