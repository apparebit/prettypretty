"""
Support for gamut mapping.

Gamut mapping makes extensive use of other color algorithms, constantly
converting betwee color spaces, computing the distance between colors, checking
whether colors are in gamut, and clipping colors. As a result, this module has
more dependencies than most of the color modules, importing symbols from
:mod:`.conversion`, :mod:`.difference`, :mod:`.space`, and :mod:`.spec`.
"""
from typing import cast

from .conversion import get_converter, oklch_to_oklab
from .difference import deltaE_oklab
from .space import resolve
from .spec import CoordinateSpec, FloatCoordinateSpec


JND = 0.02
EPSILON = 0.0001

def map_into_gamut(
    target: str,
    coordinates: CoordinateSpec,
) -> FloatCoordinateSpec:
    """
    Map the coordinates into gamut by using the `CSS Color 4 algorithm
    <https://drafts.csswg.org/css-color/#css-gamut-mapping>`_.

    The algorithm performs a binary search across the chroma range between zero
    and the chroma of the original, out-of-gamut color in Oklch. It stops the
    search once the chroma-adjusted color is within the just noticeable
    difference (JND) of its clipped version as measured by deltaEOK and uses
    that clipped version as result. As such, the algorithm manipulates colors
    across three color spaces: It uses the coordinates' color space for gamut
    testing and clipping, Oklch for producing candidate colors, and Oklab for
    measuring distance.
    """
    target_space = resolve(target)
    if target_space.is_integral():
        raise ValueError(f'color format {target} cannot be gamut mapped')

    # Oklab, Oklch, and XYZ are effectively unbounded color spaces
    if target in ('oklab', 'oklch', 'xyz'):
        return cast(FloatCoordinateSpec, coordinates)

    # We'll be using these converters a lot
    oklch_to_target = get_converter('oklch', target)
    target_to_oklab = get_converter(target, 'oklab')

    # 1. Preliminary: Check lightness
    origin_as_oklch = get_converter(target, 'oklch')(*coordinates)
    L = origin_as_oklch[0]
    if L >= 1.0:
        return oklch_to_target(1, 0, 0)
    if L <= 0.0:
        return oklch_to_target(0, 0, 0)

    # 2. Preliminary: Check gamut
    if target_space.in_gamut(*coordinates, epsilon=0):
        return cast(FloatCoordinateSpec, coordinates)

    # Minimize just noticeable difference between current and clipped colors
    current_as_oklch = origin_as_oklch
    clipped_as_target = target_space.clip(
        *oklch_to_target(*current_as_oklch), epsilon=0
    )
    diff = deltaE_oklab(
        *target_to_oklab(*clipped_as_target), *oklch_to_oklab(*current_as_oklch)
    )

    if diff < JND:
        return cast(FloatCoordinateSpec, clipped_as_target)

    # Perform a binary search by adjusting chroma in Oklch
    min = 0
    max = origin_as_oklch[1]
    min_in_gamut = True

    while max - min > EPSILON:
        chroma = (min + max) / 2
        current_as_oklch = current_as_oklch[0], chroma, current_as_oklch[2]
        current_as_target = oklch_to_target(*current_as_oklch)
        if min_in_gamut and target_space.in_gamut(*current_as_target, epsilon=0):
            min = chroma
            continue

        clipped_as_target = target_space.clip(*current_as_target, epsilon=0)
        diff = deltaE_oklab(
            *target_to_oklab(*clipped_as_target), *oklch_to_oklab(*current_as_oklch)
        )
        if diff < JND:
            if JND - diff < EPSILON:
                return cast(FloatCoordinateSpec, clipped_as_target)
            min_in_gamut = False
            min = chroma
        else:
            max = chroma

    return cast(FloatCoordinateSpec, clipped_as_target)
