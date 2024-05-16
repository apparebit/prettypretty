"""
Basic type declarations for coordinates, colors, and conversions:

  * ``IntCoordinateSpec`` and ``FloatCoordinateSpec`` are triples of integers
    and floating point values, respectively
  * ``CoordinateSpec`` combines the two types with a tuple of a single integer
  * ``ConverterSpec`` describes a function that converts from one color format
    or space into another
  * ``ColorSpec`` is a dataclass that associates a color's components with
    the tag identifying the color format or space.

All container types are immutable.
"""
import dataclasses
from typing import overload, Protocol, TypeAlias

IntCoordinateSpec: TypeAlias = tuple[int, int, int]
FloatCoordinateSpec: TypeAlias = tuple[float, float, float]
CoordinateSpec: TypeAlias = (
    tuple[int] | tuple[int, int, int] | tuple[float, float, float]
)

CoordinateVectorSpec: TypeAlias = tuple[FloatCoordinateSpec, ...]

class ConverterSpec(Protocol):
    @overload
    def __call__(
        self, __c: int
    ) -> tuple[int] | tuple[int, int, int]: ...
    @overload
    def __call__(
        self, __c1: int, __c2: int, __c3: int
    ) -> tuple[int, int, int]: ...
    @overload
    def __call__(
        self, __c1: float, __c2: float, __c3: float
    ) -> tuple[float, float, float]: ...

    def __call__(
        self,
        __c1: int | float,
        __c2: None | int | float = None,
        __c3: None | int | float = None,
    ) -> CoordinateSpec:
        ...


_TAGS = {
    'ansi': '1i',
    'eight_bit': '1i',
    'linear_p3': '3f',
    'linear_srgb': '3f',
    'oklab': '3f',
    'oklch': '3f',
    'p3': '3f',
    'rgb6': '3i',
    'rgb256': '3i',
    'srgb': '3f',
    'xyz': '3f',
}


@dataclasses.dataclass(frozen=True, slots=True)
class ColorSpec:
    """
    A color specification.

    Attributes:
        tag: identifies the coordinates' color format or space
        coordinates: are the numeric components of the color

    With exception of the ``ansi`` and ``eight_bit`` terminal color formats,
    which have one components, all other color formats and spaces have three
    components. In particular, the ``rgb6`` and ``rgb256`` formats have three
    integer components, whereas the color spaces all have three floating point
    components.

    This class does validate the tag and number of coordinates upon creation.

    Instance of this class are immutable.
    """
    tag: str
    coordinates: CoordinateSpec

    def __post_init__(self) -> None:
        code = _TAGS.get(self.tag)
        if code is None:
            raise ValueError(f'{self.tag} is not a valid color format or space')
        count = 1 if code == '1i' else 3
        if (l := len(self.coordinates)) != count:
            raise ValueError(f'{self.tag} should have {count} coordinates, not {l}')
