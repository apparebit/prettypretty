"""Prettypretty's high-level color API."""

from collections.abc import Iterable
from typing import cast, Literal, overload, Self

from .contrast import (
    p3_to_luminance,
    srgb_to_luminance,
    luminance_to_contrast,
    use_black_text,
    use_black_background,
)
from .conversion import get_converter
from .difference import deltaE_oklab, closest_oklab
from .gamut import map_into_gamut
from .serde import (
    parse_fn,
    parse_format_spec,
    parse_hex,
    parse_x_rgb,
    parse_x_rgbi,
    stringify,
)
from .space import EPSILON, Space
from .spec import ColorSpec, CoordinateSpec, FloatCoordinateSpec


class Color(ColorSpec):
    """
    A color object.

    This class implements the high-level, object-oriented API for colors.

    Attributes:
        tag: identifies the color format or space
        coordinates: are the numerical components of the color

    Color's constructor supports a number of options for specifying the color
    and its coordinates:

        * From an existing :class:`.ColorSpec` or ``Color`` object
        * From the textual representation of a color, using ``#``, ``rgb:``,
          ``rgbi:``, or one of the tags as prefix
        * From a tag and tuple with coordinates
        * From a tag and one or three coordinates

    When invoked on an existing color specification or color object, the
    constructor returns a new color object with the same tag and coordinates.
    Hence, it effectively upgrades color specifications to full featured color
    objects.

    As for the parent class :class:`.ColorSpec`, instances of this class are
    immutable.

    This class implements ``__hash__()`` and ``__eq__()`` so that colors *in the
    same color format or space* with sufficiently close coordinates are treated
    as equal. For color formats with integral coordinates, "sufficiently close"
    means equal coordinates. For color spaces, it means equality after rounding
    to 14 significant digits. Since equal colors must have equal hashes, the
    magnitude of the difference cannot be used for equality.
    """
    __slots__ = ()

    @property
    def space(self) -> Space:
        """Get the color space for this color."""
        return Space.resolve(self.tag)

    def __getattr__(self, name: str) -> float | Self:
        """
        Provide access to color space coordinates by single-letter name. Also,
        convert to named color format or space.
        """
        if len(name) == 1:
            for coordinate, value in zip(self.space.coordinates, self.coordinates):
                if coordinate.name == name:
                    return value
        elif Space.is_tag(name):
            return self.to(name)

        raise AttributeError(f'color {self} has no attribute named "{name}"')

    # ----------------------------------------------------------------------------------

    @overload
    def __init__(self, color: int | str | ColorSpec | Self, /) -> None:
        ...
    @overload
    def __init__(self, c1: int, c2: int, c3: int, /) -> None:
        ...
    @overload
    def __init__(self, tag: str, coordinates: CoordinateSpec, /) -> None:
        ...
    @overload
    def __init__(self, tag: str, c1: float, c2: float, c3: float, /) -> None:
        ...
    def __init__(
        self,
        tag: int | str | ColorSpec | Self,
        coordinates: None | float | CoordinateSpec = None,
        c2: None | float = None,
        c3: None | float = None,
    ) -> None:
        if isinstance(tag, ColorSpec):
            tag, coordinates = tag.tag, tag.coordinates
        elif isinstance(tag, str) and coordinates is None:
            tag = tag.strip()
            if tag.startswith('#'):
                tag, coordinates = parse_hex(tag)
            elif tag.startswith('rgb:'):
                tag, coordinates = parse_x_rgb(tag)
            elif tag.startswith('rgbi:'):
                tag, coordinates = parse_x_rgbi(tag)
            elif '(' in tag and tag.endswith(')'):
                tag, coordinates = parse_fn(tag)
            else:
                raise ValueError(f'"{tag}" is not a valid color')
        else:
            spec = ColorSpec.of(tag, coordinates, c2, c3)
            tag, coordinates = spec.tag, spec.coordinates

        object.__setattr__(self, 'tag', tag)
        object.__setattr__(self, 'coordinates', coordinates)
        # Validate number of coordinates and coerce to int/float
        self.__post_init__()

    def update(
        self,
        c1: None | float,
        c2: None | float = None,
        c3: None | float = None,
    ) -> Self:
        """
        Update this color.

        Since color objects are immutable, this method returns a new color. That
        color has the same color format or space as this color. Its coordinates
        are the arguments to this method, except that this color's coordinates
        fill null arguments.
        """
        if len(self.coordinates) == 1:
            assert c2 is None and c3 is None
            return type(self)(self.tag, cast(tuple[int], (c1 or self.coordinates[0],)))

        b1, b2, b3 = self.coordinates
        return type(self)(self.tag, (c1 or b1, c2 or b2, c3 or b3))

    # ----------------------------------------------------------------------------------
    # Hash and Equality

    def __hash__(self) -> int:
        return hash((self.tag, self.space.normalize(*self.coordinates)))

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, ColorSpec) or self.tag != other.tag:
            return NotImplemented
        space = self.space
        return space.normalize(*self.coordinates) == space.normalize(*other.coordinates)

    # ----------------------------------------------------------------------------------
    # Gamut and Clipping

    def in_gamut(self, epsilon: float = EPSILON) -> bool:
        """Determine whether this color is within gamut for its color space."""
        return self.space.in_gamut(*self.coordinates, epsilon=epsilon)

    def clip(self, epsilon: float = 0) -> Self:
        """Clip this color to its color space's gamut."""
        if self.in_gamut(epsilon):
            return self
        return type(self)(self.tag, self.space.clip(*self.coordinates))

    def to_gamut(self) -> Self:
        """
        Map this color into the gamut of its color space using the `CSS Color 4
        algorithm <https://drafts.csswg.org/css-color/#css-gamut-mapping>`_.

        That algorithm performs a binary search across the chroma range between
        zero and the chroma of the original, out-of-gamut color in Oklch. It
        stops the search once the chroma-adjusted color is within the just
        noticeable difference (JND) of its clipped version as measured by
        deltaEOK and uses that clipped version as result. As such, the algorithm
        simultaneously manipulates colors across three color spaces: It relies
        on the coordinates' color space for gamut testing and clipping, Oklch
        for producing candidate colors, and Oklab for measuring distance.

        Note that only XYZ is unbounded, even if the CSS Color 4 specification
        claims that Oklab and Oklch also are unbounded.        otherwise.
        """
        return type(self)(self.tag, map_into_gamut(self.tag, self.coordinates))

    # ----------------------------------------------------------------------------------
    # Conversion to Other Formats and Color Spaces

    def to(self, target: str) -> Self:
        """Convert this color to the specified color format or space."""
        if self.tag == target:
            return self
        return type(self)(target, get_converter(self.tag, target)(*self.coordinates))

    # ----------------------------------------------------------------------------------
    # Distance from Other Colors

    def distance(self, other: ColorSpec, *, version: Literal[1, 2] = 1) -> float:
        """
        Determine the symmetric distance ΔE between this color and the given
        color.
        """
        return deltaE_oklab(
            *self.to('oklab').coordinates,
            *Color(other).to('oklab').coordinates,
            version=version,
        )

    def closest(self, colors: Iterable[ColorSpec]) -> int:
        """
        Find the color with the smallest symmetric distance from this color and
        return its index.
        """
        index, _ = closest_oklab(
            cast(FloatCoordinateSpec, self.to('oklab').coordinates),
            (cast(FloatCoordinateSpec, Color(c).to('oklab').coordinates) for c in colors),
        )
        return index

    # ----------------------------------------------------------------------------------
    # Contrast Against Other Colors

    def contrast_against(self, background: ColorSpec) -> float:
        """
        Determine the perceptual contrast of text with this color against the
        given background.
        """
        fg = self.to('srgb')
        background = Color(background)
        bg = background.to('srgb')

        if fg.in_gamut() and bg.in_gamut():
            return luminance_to_contrast(
                srgb_to_luminance(*fg.coordinates),
                srgb_to_luminance(*bg.coordinates),
            )

        fg = self.to('p3')
        bg = background.to('p3')

        # P3 will have to do...
        return luminance_to_contrast(
            p3_to_luminance(*fg.coordinates),
            p3_to_luminance(*bg.coordinates),
        )


    def use_black_text(self) -> bool:
        """
        Determine whether to use black or white text against a background of
        this color for maximum perceptual contrast.
        """
        srgb = self.to('srgb')
        if srgb.in_gamut():
            return use_black_text(
                srgb_to_luminance(*srgb.coordinates)
            )

        return use_black_text(
            p3_to_luminance(*self.to('p3').coordinates)
        )


    def use_black_background(self) -> bool:
        """
        Determine whether to use a black or white background for text of this
        color for maximum perceptual contrast.
        """
        srgb = self.to('srgb')
        if srgb.in_gamut():
            return use_black_background(
                srgb_to_luminance(*srgb.coordinates)
            )

        return use_black_background(
            p3_to_luminance(*self.to('p3').coordinates)
        )

    # ----------------------------------------------------------------------------------
    # Serialization to Text

    def __format__(self, format_spec: str) -> str:
        fmt, precision = parse_format_spec(format_spec)
        return stringify(self.tag, self.coordinates, fmt, precision)

    def __str__(self) -> str:
        return stringify(self.tag, self.coordinates)
