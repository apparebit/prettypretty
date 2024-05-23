"""
Equality of colors.

Equality comparison for colors is complicated by four orthogonal problems:

 1. Coordinate values may be not-a-numbers, which do not equal themselves.
 2. Angular coordinates may have values outside the range of 0 to 360 but
    still denote the same angle, albeit with additional rotations.
 3. Conversion between color spaces may accrue some amount of floating point
    error. That amount may further differ between processors and operating
    systems.
 4. Python requires that, if a class redefines ``__eq__()`` it must also
    redefine ``__hash__()``, so that two equal instances also have the same
    hash codes.

Addressing the first two problems fundamentally requires customizing equality
comparison to correctly handle not-a-numbers and angles. The third problem could
be addressed by testing whether the difference between two coordinates is
smaller than some epsilon. But that defines color equality in terms of the
difference between two instances and hence makes computing the hash even harder,
if not impossible. A more productive strategy is to normalize a color's
coordinates to some canonical representation that is then used for both hash
computation and equality comparison.

This module implements just that normalization. Correct normalization requires
knowing which coordinates are angles. But I am not aware of any color space that
has more than one angle—for the hue.
"""
import math


PRECISION = 14
"""
The default precision for rounding coordinates during normalization.
"""


def normalize(
    coordinates: tuple[float, ...],
    *,
    angular_index: int = -1,
    integral: bool = False,
    precision: int = PRECISION,
) -> tuple[None | float, ...]:
    """
    Normalize the coordinates.

    Args:
        coordinates: are the color's components.
        angle_index: is the index of the angular coordinate, if there is one.
        integral: indicates that the color has integral components.
        precision: is the number of decimals to round to.
    Returns:
        The normalized coordinates.

    This function normalizes the coordinates as follows:

      * It replaces not-a-numbers with ``None``, which equals itself;
      * It coerces integral coordinates to integers;
      * It maps angles to 0–360, coerces them to floating point numbers,
        and rounds them to two decimal digits less than precision;
      * It coerces all other coordinates to floating point numbers and
        rounds them to as many decimal digits as precision.

    The default precision of 14 digits for arbitrary coordinates and 12 digits
    for angles was experimentally determined by running tests on macOS and
    Linux.

    The resulting tuple accounts for all of the problems identified above and
    hence is trivially suitable for hashing and equality testing.

    It may be possible to speed up equality testing in case of coordinates
    already being equal. But it is doubtful that this will lead to noticeable
    speed gains in applications because equality testing for colors really only
    matters when testing color manipulation. In general, color distance is the
    far more relevant comparison.
    """
    result: list[None | float] = []

    for index, value in enumerate(coordinates):
        if math.isnan(value):
            result.append(None)
            continue

        if integral:
            result.append(int(value))
            continue

        if index == angular_index:
            value = value % 360
            effective_precision = precision - 2
        else:
            effective_precision = precision

        result.append(round(value, effective_precision))

    return tuple(result)
