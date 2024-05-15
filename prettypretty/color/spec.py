from typing import Callable, TypeAlias

IntCoordinateSpec: TypeAlias = tuple[int, int, int]
FloatCoordinateSpec: TypeAlias = tuple[float, float, float]
CoordinateSpec: TypeAlias = tuple[int] | FloatCoordinateSpec

CoordinateVectorSpec: TypeAlias = tuple[FloatCoordinateSpec, ...]

ConverterSpec: TypeAlias = (
    Callable[[float], CoordinateSpec]
    | Callable[[float, float, float], CoordinateSpec]
)
