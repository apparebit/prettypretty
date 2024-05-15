"""Support for computing the difference between two or more colors"""
from collections.abc import Iterable
import math
from typing import Literal


def deltaE_oklab(
    L1: float, a1: float, b1: float,
    L2: float, a2: float, b2: float,
    *,
    version: Literal[1, 2] = 1,
) -> float:
    """
    Determine the difference between two Oklab colors.

    For the first version, that difference is just the Euclidian distance
    between the coordinates.

    For the second version, the difference between the *a* and *b* coordinates
    is scaled by a constant factor before computing the Euclidian distance. For
    now that factor is 2, even though `it probably is closer to 2.1
    <https://github.com/w3c/csswg-drafts/issues/6642#issuecomment-945714988>`_.
    """
    ΔL = L1 - L2
    Δa = version * (a1 - a2)
    Δb = version * (b1 - b2)
    return math.sqrt(ΔL * ΔL + Δa * Δa + Δb * Δb)


def closest_oklab(
    origin: tuple[float, float, float],
    candidates: Iterable[tuple[float, float, float]],
) -> tuple[int, tuple[float, float, float]]:
    """
    Find the color closest to the origin amongst candidate colors.

    Args:
        origin: is the reference color in Oklab coordinates
        candidates: are the colors to compare to, also in Oklab coordinates
    Returns:
        the index and coordinates of the candidate color closest to the origin,
        which are -1 and ``origin`` if the iterable is empty.

    This function iterates over the candidates only once and hence the iterable
    may also be an iterator.
    """
    min_ΔE = math.inf
    min_index = -1
    min_color = origin

    for index, color in enumerate(candidates):
        ΔE = deltaE_oklab(*origin, *color)
        if ΔE < min_ΔE:
            min_ΔE = ΔE
            min_index = index
            min_color = color

    return min_index, min_color
