from collections.abc import Iterator
from contextlib import contextmanager
import dataclasses
from typing import cast, ContextManager, overload


type IntCoordinateSpec = tuple[int, int, int]
type FloatCoordinateSpec = tuple[float, float, float]
type CoordinateSpec = tuple[int] | IntCoordinateSpec | FloatCoordinateSpec


@dataclasses.dataclass(frozen=True, slots=True)
class ColorSpec:
    """
    An immutable color specification.

    Attributes:
        tag: identifies the color format or space
        coordinates: are the color's components

    This class is purposefully minimal, so that using this module does *not*
    pull in modules other than a few standard library modules. That matters
    because :mod:`prettypretty.color.conversion` requires access to the current
    theme when converting from 8-bit terminal to 24-bit RGB and from Oklab to
    ANSI colors.

    If you are looking for a higher-level, fully object-oriented API, check out
    :class:`prettypretty.color.object.Color`. It has methods for all color
    functionality implemented by this package. It also extends this class, which
    means you can use instances of ``Color`` in themes.
    """
    tag: str
    coordinates: CoordinateSpec


@dataclasses.dataclass(frozen=True, slots=True, kw_only=True)
class Theme:
    """
    A terminal color theme.

    Manipulating ANSI colors like RGB colors is complicated by the facts that
    there is no canonical mapping between them and that most terminal emulators
    have robust support for color themes, which modify ANSI colors. This class
    captures such a terminal theme, well, the bits relevant for this package.
    """

    text: ColorSpec
    background: ColorSpec
    black: ColorSpec
    red: ColorSpec
    green: ColorSpec
    yellow: ColorSpec
    blue: ColorSpec
    magenta: ColorSpec
    cyan: ColorSpec
    white: ColorSpec
    bright_black: ColorSpec
    bright_red: ColorSpec
    bright_green: ColorSpec
    bright_yellow: ColorSpec
    bright_blue: ColorSpec
    bright_magenta: ColorSpec
    bright_cyan: ColorSpec
    bright_white: ColorSpec

    def colors(self) -> Iterator[tuple[str, ColorSpec]]:
        """Get an iterator over the name, color pairs of this theme."""
        for field in dataclasses.fields(self):
            yield field.name, getattr(self, field.name)


def hex(s: str) -> ColorSpec:
    return ColorSpec(
        'rgb256',
        cast(IntCoordinateSpec, tuple(int(s[n:n+2], base=16) for n in range(0, 6, 2)))
    )


# The default theme is the basic theme for macOS terminal
DEFAULT_THEME = Theme(
    text=hex("000000"),
    background=hex("ffffff"),
    black=hex("000000"),
    red=hex("990000"),
    green=hex("00a600"),
    yellow=hex("999900"),
    blue=hex("0000b2"),
    magenta=hex("b200b2"),
    cyan=hex("00a6b2"),
    white=hex("bfbfbf"),
    bright_black=hex("666666"),
    bright_red=hex("e50000"),
    bright_green=hex("00d900"),
    bright_yellow=hex("e5e500"),
    bright_blue=hex("0000ff"),
    bright_magenta=hex("e500e5"),
    bright_cyan=hex("00e5e5"),
    bright_white=hex("e5e5e5"),
)


_current_theme = DEFAULT_THEME

@overload
def current_theme() -> Theme:
    ...
@overload
def current_theme(theme: Theme) -> ContextManager[Theme]:
    ...
def current_theme(theme: None | Theme = None) -> Theme | ContextManager[Theme]:
    """
    Manage the current theme.

    This function does the work of two:

     1. When invoked without arguments, this function simply returns the current
        theme.
     2. When invoked with a theme as argument, this function returns a context
        manager that switches to the provided theme on entry and restores the
        current theme again on exit.
    """
    if theme is None:
        return _current_theme

    @contextmanager
    def another_theme() -> Iterator[Theme]:
        global _current_theme
        saved_theme = _current_theme
        _current_theme = theme
        try:
            yield theme
        finally:
            _current_theme = saved_theme

    return another_theme()
