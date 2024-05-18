import math


EQUALITY_PRECISION = 14

def same_coordinates(
    coordinates1: tuple[float, float, float],
    coordinates2: tuple[float, float, float],
    *,
    precision: int = EQUALITY_PRECISION,
    hue: int = -1,
) -> bool:
    """
    Determine whether the coordinates are the same.

    Args:
        coordinates1: are the first color's coordinates
        coordinates2: are the second color's coordinates
        epsilon: is the tolerance for the comparison
        hue: is the index of the hue or -1 if there is no hue
    Returns:
        ``True`` if the coordinates are identical after rounding to the given
        precision or, for the hue, if both coordinates are not-a-number.

    The two coordinates must be in the same color space. Furthermore, since this
    function uses a single precision for all three coordinate axes, the ranges
    of the three axes should be reasonably similar.

    The comparison allows for the hue to be not-a-number, but this function does
    not enforce related constraints, such as the chroma being very close to zero
    or one.

    Python objects must have the same hash if they compare as equal. It is
    impossible to implement that behavior in a color class if equality is based
    on the magnitude of the (relative) difference between coordinates. But it
    *is* possible to implement that behavior when each color's coordinates can
    be independently prepared for the comparison, notably by rounding.
    """
    assert -1 <= hue <= 2

    for index in range(3):
        c1 = coordinates1[index]
        c2 = coordinates2[index]

        if index == hue:
            # Account for hue being not-a-number
            if math.isnan(c1) != math.isnan(c2):
                return False
            if math.isnan(c1):
                # Both coordinates are not-a-number
                continue
            # We still need to compare two hue numbers

        if c1 != c2 and round(c1, precision) != round(c2, precision):
            return False

    return True
