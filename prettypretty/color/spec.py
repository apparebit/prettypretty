from typing import Callable, TypeAlias

LoresCoordinateSpec: TypeAlias = tuple[int]
IntCoordinateSpec: TypeAlias = tuple[int, int, int]
FloatCoordinateSpec: TypeAlias = tuple[float, float, float]
CoordinateSpec: TypeAlias = (
    LoresCoordinateSpec | IntCoordinateSpec | FloatCoordinateSpec
)

CoordinateVectorSpec: TypeAlias = tuple[FloatCoordinateSpec, ...]

ConverterSpec: TypeAlias = (
    Callable[[int], CoordinateSpec]
    | Callable[[int, int, int], CoordinateSpec]
    | Callable[[float, float, float], CoordinateSpec]
)
