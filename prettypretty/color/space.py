"""
Metadata about color spaces.

Note that the ``conversion`` and ``serde`` modules have local copies of some of
the same data. If you update this module, be sure to update the other two
modules, too.
"""
import dataclasses
import math
from typing import cast, Literal, Self

from .spec import CoordinateSpec


EPSILON = 0.000075


@dataclasses.dataclass(frozen=True, slots=True)
class Coordinate:
    """
    A color space coordinate.

    Attributes:
        name: the single-letter name of the coordinate or an empty string
            for nameless coordinates
        min: the optional minimum value for the coordinate
        max: the optional maximum value for the coordinate
        type: the optional type for common coordinate semantics

    The following three types are recognized:

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
            raise ValueError('coordinate must have name with at most 1 character')
        elif self.type == 'angle':
            if self.min != 0 or self.max != 360:
                raise ValueError('angle coordinate must have range from 0 to 360')
        elif self.type == 'normal':
            if self.min != 0 or self.max != 1:
                raise ValueError('normal coordinate must have range from 0 to 1')

    def is_in_range(self, value: float, *, epsilon: float = EPSILON) -> bool:
        """
        Determine whether the value is within the range set by this coordinate's
        ``min`` and ``max`` attributes with an ``epsilon`` tolerance. If either
        limit is ``None``, that limit is not tested.
        """
        if math.isnan(value):
            return self.type == 'angle'
        if self.type == 'int' and not isinstance(value, int):
            return False
        return (
            (self.min is None or self.min - epsilon <= value)
            and (self.max is None or value <= self.max + epsilon)
        )

    def clip(self, value: float, *, epsilon: float = 0) -> float:
        """
        Clip the value to the range set by this coordinate's ``min`` and ``max``
        attributes with an ``epsilon`` tolerance. If either limits is ``None``,
        that limit has no impact on the value.
        """
        if self.min is not None and value < self.min - epsilon:
            return self.min
        if self.max is not None and value > self.max + epsilon:
            return self.max
        return value


@dataclasses.dataclass(frozen=True, slots=True)
class Space:
    """
    A color format or space.

    Attributes:
        tag: is a lower-case Python identifier
        name: is a human-readable, descriptive label
        base: is the optional base color space
        coordinates: are the coordinates
        css_format: is the optional CSS format
        lores: is the flag for a low-resolution format
    """
    tag: str
    label: str
    base: None | Self
    coordinates: tuple[Coordinate, ...]
    css_format: None | str = None
    lores: bool = False

    def is_in_gamut(self, *coordinates: float, epsilon: float = EPSILON) -> bool:
        """
        Determine whether the coordinates are in gamut for this color space with
        an ``epsilon`` tolerance.
        """
        assert len(self.coordinates) == len(coordinates)
        for coordinate, value in zip(self.coordinates, coordinates):
            if not coordinate.is_in_range(value, epsilon=epsilon):
                return False
        return True

    def clip(self, *coordinates: float, epsilon: float = 0) -> CoordinateSpec:
        """
        Clip the coordinates to this color space's gamut with an ``epsilon``
        tolerance.
        """
        return cast(
            CoordinateSpec,
            tuple(
                c.clip(v, epsilon=epsilon)
                for c, v in zip(self.coordinates, coordinates)
            ),
        )


_RGB_COORDINATES = (
    Coordinate('r', 0, 1, 'normal'),
    Coordinate('g', 0, 1, 'normal'),
    Coordinate('b', 0, 1, 'normal'),
)

XYZ = XYZ_D65 = Space(
    tag='xyz',
    label='XYZ D65',
    base=None,
    coordinates=(
        Coordinate('X'),
        Coordinate('Y'),
        Coordinate('Z'),
    ),
    css_format='color(xyz {})',
)

LINEAR_SRGB = Space(
    tag='linear_srgb',
    label='Linear sRGB',
    base=XYZ,
    coordinates=_RGB_COORDINATES,
    css_format='color(srgb-linear {})',
)

SRGB = Space(
    tag='srgb',
    label='sRGB',
    base=LINEAR_SRGB,
    coordinates=_RGB_COORDINATES,
    css_format='color(srgb {})',
)

RGB256 = Space(
    tag='rgb256',
    label='24-bit RGB',
    base=SRGB,
    coordinates=(
        Coordinate('r', 0, 255, 'int'),
        Coordinate('g', 0, 255, 'int'),
        Coordinate('b', 0, 255, 'int'),
    ),
    css_format='rgb({})',
)

LINEAR_P3 = Space(
    tag='linear_p3',
    label='Linear P3',
    base=XYZ,
    coordinates=_RGB_COORDINATES,
    css_format=None,
)

P3 = Space(
    tag='p3',
    label='P3',
    base=LINEAR_P3,
    coordinates=_RGB_COORDINATES,
    css_format='color(display-p3 {})',
)

OKLAB = Space(
    tag='oklab',
    label='Oklab',
    base=XYZ,
    coordinates=(
        Coordinate('L', 0, 1, 'normal'),
        Coordinate('a', -0.4, 0.4),
        Coordinate('b', -0.4, 0.4),
    ),
    css_format='oklab({})',
)

OKLCH = Space(
    tag='oklch',
    label='Oklch',
    base=OKLAB,
    coordinates=(
        Coordinate('L', 0, 1, 'normal'),
        Coordinate('C', 0, 0.4),
        Coordinate('h', 0, 360, 'angle'),
    ),
    css_format='oklch({})',
)

EIGHT_BIT = Space(
    tag='eight_bit',
    label='8-bit terminal color',
    base=RGB256,
    coordinates=(
        Coordinate('', 0, 255, 'int'),
    ),
    css_format=None,
    lores=True,
)

RGB6 = Space(
    tag='rgb6',
    label='8-bit terminal color RGB',
    base=EIGHT_BIT,
    coordinates=(
        Coordinate('r', 0, 5, 'int'),
        Coordinate('g', 0, 5, 'int'),
        Coordinate('b', 0, 5, 'int'),
    ),
    css_format=None,
    lores=True,
)

ANSI = Space(
    tag='ansi',
    label='ANSI terminal color',
    base=EIGHT_BIT,
    coordinates=(
        Coordinate('', 0, 15, 'int'),
    ),
    css_format=None,
    lores=True,
)

UNIVERSE = (
    # Color spaces
    XYZ,
    LINEAR_SRGB,
    SRGB,
    LINEAR_P3,
    P3,
    OKLAB,
    OKLCH,

    # In-memory format
    RGB256,

    # Terminal-specific in-memory format
    RGB6,
    EIGHT_BIT,
    ANSI,
)

_TAG_TO_SPACE = { space.tag: space for space in UNIVERSE }

def resolve(tag: str) -> Space:
    """Resolve the given tag into the corresponding color space metadata."""
    return _TAG_TO_SPACE[tag]

def is_tag(tag: str) -> bool:
    """
    Determine whether the given string is a valid tag for a color format or
    space.
    """
    return tag in _TAG_TO_SPACE
