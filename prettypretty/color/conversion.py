"""Conversion between color formats and spaces"""
import itertools
import math
from typing import cast, TypeAlias

from .spec import ConverterSpec, CoordinateSpec


_RGB6_TO_RGB256 = (0, 0x5F, 0x87, 0xAF, 0xD7, 0xFF)

# See https://github.com/color-js/color.js/blob/a77e080a070039c534dda3965a769675aac5f75e/src/spaces/srgb-linear.js

_XYZ_TO_LINEAR_SRGB = (
	(  3.2409699419045226,  -1.537383177570094,   -0.4986107602930034  ),
	( -0.9692436362808796,   1.8759675015077202,   0.04155505740717559 ),
	(  0.05563007969699366, -0.20397695888897652,  1.0569715142428786  ),
)

_LINEAR_SRGB_TO_XYZ = (
	( 0.41239079926595934, 0.357584339383878,   0.1804807884018343  ),
	( 0.21263900587151027, 0.715168678767756,   0.07219231536073371 ),
	( 0.01933081871559182, 0.11919477979462598, 0.9505321522496607  ),
)

# See https://github.com/color-js/color.js/blob/a77e080a070039c534dda3965a769675aac5f75e/src/spaces/p3-linear.js

_XYZ_TO_LINEAR_P3 = (
	(  2.493496911941425,   -0.9313836179191239,  -0.40271078445071684  ),
	( -0.8294889695615747,   1.7626640603183463,   0.023624685841943577 ),
	(  0.03584583024378447, -0.07617238926804182,  0.9568845240076872   ),
)

_LINEAR_P3_TO_XYZ = (
	( 0.4865709486482162, 0.26566769316909306, 0.1982172852343625 ),
	( 0.2289745640697488, 0.6917385218365064,  0.079286914093745  ),
	( 0.0000000000000000, 0.04511338185890264, 1.043944368900976  ),
)

# See https://github.com/color-js/color.js/blob/a77e080a070039c534dda3965a769675aac5f75e/src/spaces/oklab.js

_XYZ_TO_LMS = (
	( 0.8190224379967030, 0.3619062600528904, -0.1288737815209879 ),
	( 0.0329836539323885, 0.9292868615863434,  0.0361446663506424 ),
	( 0.0481771893596242, 0.2642395317527308,  0.6335478284694309 ),
)

_LMS_TO_XYZ = (
	(  1.2268798758459243, -0.5578149944602171,  0.2813910456659647 ),
	( -0.0405757452148008,  1.1122868032803170, -0.0717110580655164 ),
	( -0.0763729366746601, -0.4214933324022432,  1.5869240198367816 ),
)

_LMS_TO_OKLAB = (
	( 0.2104542683093140,  0.7936177747023054, -0.0040720430116193 ),
	( 1.9779985324311684, -2.4285922420485799,  0.4505937096174110 ),
	( 0.0259040424655478,  0.7827717124575296, -0.8086757549230774 ),
)

_OKLAB_TO_LMS = (
    ( 1.0000000000000000,  0.3963377773761749,  0.2158037573099136 ),
	( 1.0000000000000000, -0.1055613458156586, -0.0638541728258133 ),
	( 1.0000000000000000, -0.0894841775298119, -1.2914855480194092 ),
)


# --------------------------------------------------------------------------------------


_Vector: TypeAlias = tuple[float, float, float]
_Matrix: TypeAlias = tuple[_Vector, _Vector, _Vector]

def _multiply(matrix: _Matrix, vector: _Vector) -> _Vector:
    return cast(
        _Vector,
        tuple(sum(r * c for r, c in zip(row, vector)) for row in matrix)
    )


# --------------------------------------------------------------------------------------
# 24-bit RGB


def rgb256_to_srgb(r: int, g: int, b: int) -> tuple[float, float, float]:
    """Convert the given color from 24-bit RGB to sRGB."""
    return cast(_Vector, tuple(map(lambda c: c / 255.0, (r, g, b))))


def srgb_to_rgb256(r: float, g: float, b: float) -> tuple[int, int, int]:
    """
    :bdg-warning:`Lossy conversion` Convert the given color from sRGB to 24-bit
    RGB.
    """
    return cast(tuple[int, int, int], tuple(map(lambda c: round(c * 255), (r, g, b))))


# --------------------------------------------------------------------------------------
# sRGB and Linear sRGB
# See https://github.com/color-js/color.js/blob/a77e080a070039c534dda3965a769675aac5f75e/src/spaces/srgb.js


def srgb_to_linear_srgb(r: float, g: float, b: float) -> tuple[float, float, float]:
    """Convert the given color from sRGB to linear sRGB."""
    def convert(value: float) -> float:
        magnitude = math.fabs(value)

        if magnitude <= 0.04045:
            return value / 12.92

        return math.copysign(math.pow((magnitude + 0.055) / 1.055, 2.4), value)

    return convert(r), convert(g), convert(b)


def linear_srgb_to_srgb(r: float, g: float, b: float) -> tuple[float, float, float]:
    """Convert the given color from linear sRGB to sRGB."""
    def convert(value: float) -> float:
        magnitude = math.fabs(value)

        if magnitude <= 0.0031308:
            return value * 12.92

        return math.copysign(math.pow(magnitude, 1/2.4) * 1.055 - 0.055, value)

    return convert(r), convert(g), convert(b)


def linear_srgb_to_xyz(r: float, g: float, b: float) -> tuple[float, float, float]:
    """Convert the given color from linear sRGB to XYZ."""
    return _multiply(_LINEAR_SRGB_TO_XYZ, (r, g, b))


# --------------------------------------------------------------------------------------
# P3 and Linear P3


p3_to_linear_p3 = srgb_to_linear_srgb
linear_p3_to_p3 = linear_srgb_to_srgb


def linear_p3_to_xyz(r: float, g: float, b: float) -> tuple[float, float, float]:
    """Convert the given color from linear P3 to XYZ."""
    return _multiply(_LINEAR_P3_TO_XYZ, (r, g, b))


# --------------------------------------------------------------------------------------
# Oklab and Oklch:
# See https://github.com/color-js/color.js/blob/a77e080a070039c534dda3965a769675aac5f75e/src/spaces/oklch.js


def oklch_to_oklab(L: float, C: float, h: float) -> tuple[float, float, float]:
    """Convert the given color from Oklch to Oklab."""
    if math.isnan(h):
        a = b = 0.0
    else:
        a = C * math.cos(h * math.pi / 180)
        b = C * math.sin(h * math.pi / 180)

    return L, a, b


def oklab_to_oklch(L: float, a: float, b: float) -> tuple[float, float, float]:
    """Convert the given color from Oklab to Oklch."""
    ε = 0.0002

    if math.fabs(a) < ε and math.fabs(b) < ε:
        h = math.nan
    else:
        h = math.atan2(b, a) * 180 / math.pi

    return L, math.sqrt(math.pow(a, 2) + math.pow(b, 2)), math.fmod(h + 360, 360)


def oklab_to_xyz(L: float, a: float, b: float) -> tuple[float, float, float]:
    """Convert the given color from Oklab to XYZ."""
    LMSg = _multiply(_OKLAB_TO_LMS, (L, a, b))
    LMS = cast(_Vector, tuple(map(lambda c: math.pow(c, 3), LMSg)))
    return _multiply(_LMS_TO_XYZ, LMS)


# --------------------------------------------------------------------------------------
# XYZ


def xyz_to_linear_srgb(X: float, Y: float, Z: float) -> tuple[float, float, float]:
    """Convert the given color from XYZ to linear sRGB."""
    return _multiply(_XYZ_TO_LINEAR_SRGB, (X, Y, Z))


def xyz_to_linear_p3(r: float, g: float, b: float) -> tuple[float, float, float]:
    """Convert the given color from XYZ to linear P3."""
    return _multiply(_XYZ_TO_LINEAR_P3, (r, g, b))


def xyz_to_oklab(X: float, Y: float, Z: float) -> tuple[float, float, float]:
    """Convert the given color from XYZ to Oklab."""
    LMS = _multiply(_XYZ_TO_LMS, (X, Y, Z))
    LMSg = cast(_Vector, tuple(map(lambda c: math.cbrt(c), LMS)))
    return _multiply(_LMS_TO_OKLAB, LMSg)


# --------------------------------------------------------------------------------------
# Arbitrary Conversions


def _collect_conversions(
    mod: dict[str, object],
    conversions: dict[str, dict[str, ConverterSpec]]
) -> None:
    for name, value in mod.items():
        if not name.startswith('_') and '_to_' in name and callable(value):
            source, _, target = name.partition('_to_')
            targets = conversions.setdefault(source, {})
            if target in targets:
                raise ValueError(f'duplicate conversion from {source} to {target}')
            targets[target] = value


_BASE_TREE = {
    'rgb256': ('srgb', 3),
    'srgb': ('linear_srgb', 2),
    'linear_srgb': ('xyz', 1),
    'p3': ('linear_p3', 2),
    'linear_p3': ('xyz', 1),
    'oklch': ('oklab', 2),
    'oklab': ('xyz', 1),
    'xyz': (None, 0),
}

def _elaborate_route(source: str, target: str) -> tuple[str, ...]:
    """Elaborate the route from the source to the target color format or space."""
    if source not in _BASE_TREE:
        raise ValueError(f'{source} is not a valid color format or space')
    if target not in _BASE_TREE:
        raise ValueError(f'{target} is not a valid color format or space')

    # Trace paths from source and target towards root of base tree
    source_path: list[str] = [source]
    target_path: list[str] = [target]

    def step(path: list[str]) -> bool:
        tag, _ = _BASE_TREE[path[-1]]
        if tag is not None:
            path.append(tag)
        return tag is None

    # Sync up traces, so that both have same distance from root
    _, source_dist = _BASE_TREE[source]
    _, target_dist = _BASE_TREE[target]

    path = source_path if source_dist >= target_dist else target_path
    for _ in range(abs(source_dist - target_dist)):
        done = step(path)
        assert not done

    # Keep tracing in lock step until paths share last node
    while source_path[-1] != target_path[-1]:
        done = step(source_path)
        done |= step(target_path)
        assert not done

    # Assemble complete path
    target_path.pop()
    target_path.reverse()
    return tuple(itertools.chain(source_path, target_path))


def _pass_through(*coordinates: float) -> CoordinateSpec:
    """Pass through the coordinates."""
    return cast(CoordinateSpec, tuple(coordinates))


def _create_converter(conversions: tuple[ConverterSpec, ...]) -> ConverterSpec:
    """
    Instantiate a closure that applies the given conversions. Doing so in a
    dedicated top-level function keeps the closure environment minimal.
    """
    def converter(*coordinates: float) -> CoordinateSpec:
        value = cast(CoordinateSpec, coordinates)
        for fn in conversions:
            value = fn(*value)  # type: ignore
        return value
    return cast(ConverterSpec, converter)


_LORES_FORMAT = {'ansi', 'eight_bit', 'rgb6'}

_converter_cache: dict[str, dict[str, ConverterSpec]] = {}
_collected_mod_lores: bool = False

def get_converter(source: str, target: str) -> ConverterSpec:
    """
    Instantiate a function that converts coordinates from the source color
    format or space to the target color format or space.

    This function factory caches converters to avoid re-instantiating the same
    converter over and over again. Each converter's name is computed as
    ``f"{source}_to_{target}"``.
    """
    # Initialize converter cache with basic conversions
    global _collected_mod_lores

    if not _converter_cache:
        _collect_conversions(globals(), _converter_cache)

        for tag in _LORES_FORMAT:
            _converter_cache.setdefault(tag, {})

    # Handle trivial case
    if source == target:
        return cast(ConverterSpec, _pass_through)

    # Check whether converter already exists
    maybe_converter = _converter_cache[source].get(target)
    if maybe_converter is not None:
        return maybe_converter

    # If either format is lo-res, make sure lo-res conversions have been loaded
    is_source_lores = source in _LORES_FORMAT
    is_target_lores = target in _LORES_FORMAT
    if (is_source_lores or is_target_lores) and not _collected_mod_lores:
        pkg, _, _ = __name__.rpartition('.')
        from importlib import import_module
        mod = import_module('.lores', pkg)
        _collect_conversions(vars(mod), _converter_cache)
        _collected_mod_lores = True

    # Determine route. Only the first and/or last node can be lo-res.
    route: list[str] = [source] if is_source_lores else []

    route.extend(_elaborate_route(
        'rgb256' if is_source_lores else source,
        'oklab' if is_target_lores else target,
    ))

    if is_target_lores:
        route.append(target)

    # Turn list of nodes into list of functions into converter function
    conversions = tuple(
        _converter_cache[t1][t2] for t1, t2 in itertools.pairwise(route)
    )

    # Annotate converter for easy debugability
    converter = _create_converter(conversions)
    name = f'{source}_to_{target}'
    setattr(converter, '__name__', name)
    setattr(converter, '__qualname__', name)
    setattr(converter, 'route', route)
    setattr(converter, 'conversions', conversions)

    _converter_cache[source][target] = converter
    return converter
