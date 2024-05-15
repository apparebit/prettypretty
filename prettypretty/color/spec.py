import dataclasses
from typing import TypeAlias


IntCoordinateSpec: TypeAlias = tuple[int, int, int]
FloatCoordinateSpec: TypeAlias = tuple[float, float, float]
CoordinateSpec: TypeAlias = tuple[int] | FloatCoordinateSpec


@dataclasses.dataclass(frozen=True, slots=True)
class ColorSpec:
    """
    An immutable color specification.

    Attributes:
        tag: identifies the color format or space
        coordinates: are the color's components
    """
    tag: str
    coordinates: CoordinateSpec
