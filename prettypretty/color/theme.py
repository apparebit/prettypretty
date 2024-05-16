"""
Support for representing the sixteen extended ANSI colors and for mapping them
to meaningful color values.
"""

from collections.abc import Iterator
from contextlib import contextmanager
import dataclasses
from typing import cast, ContextManager, overload

from .spec import ColorSpec, IntCoordinateSpec


@dataclasses.dataclass(frozen=True, slots=True, kw_only=True)
class Theme:
    """
    A terminal color theme.

    This class collects color values for the default text (foreground) and
    background colors as well as the 16 extended ANSI colors. While many
    terminal themes affect additional user interface elements, such as cursor
    color, ANSI escape codes cannot modify the appearance of these additional UI
    elements and hence they can be safely ignored.

    Instances of this class are immutable.
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

    def ansi(self, color: int) -> ColorSpec:
        """Look up the RGB256 coordinates for the extended ANSI color."""
        assert 0 <= color <= 15
        return getattr(self, dataclasses.fields(self)[2 + color].name)


def hex(color: str) -> ColorSpec:
    """
    Convert a string with exactly six hexadecimal digits into an RGB256 color.
    """
    return ColorSpec(
        'rgb256',
        cast(
            IntCoordinateSpec,
            tuple(int(color[n : n+2], base=16) for n in range(0, 6, 2)),
        ),
    )

MACOS_TERMINAL = Theme(
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


VGA = Theme(
    text=hex("000000"),
    background=hex("ffffff"),
    black=hex("000000"),
    red=hex("aa0000"),
    green=hex("00aa00"),
    yellow=hex("aa5500"),
    blue=hex("0000aa"),
    magenta=hex("aa00aa"),
    cyan=hex("00aaaa"),
    white=hex("aaaaaa"),
    bright_black=hex("555555"),
    bright_red=hex("ff5555"),
    bright_green=hex("55ff55"),
    bright_yellow=hex("ffff55"),
    bright_blue=hex("5555ff"),
    bright_magenta=hex("ff55ff"),
    bright_cyan=hex("55ffff"),
    bright_white=hex("ffffff"),
)

XTERM = Theme(
    text=hex("000000"),
    background=hex("ffffff"),
    black=hex("000000"),
    red=hex("cd0000"),
    green=hex("00cd00"),
    yellow=hex("cdcd00"),
    blue=hex("0000ee"),
    magenta=hex("cd00cd"),
    cyan=hex("00cdcd"),
    white=hex("e5e5e5"),
    bright_black=hex("7f7f7f"),
    bright_red=hex("ff0000"),
    bright_green=hex("00ff00"),
    bright_yellow=hex("ffff00"),
    bright_blue=hex("5c5cff"),
    bright_magenta=hex("ff00ff"),
    bright_cyan=hex("00ffff"),
    bright_white=hex("ffffff"),
)


_current_theme = VGA

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

    The default theme uses the same colors as good ol' VGA text mode.
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
