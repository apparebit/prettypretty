from collections.abc import Iterator
from contextlib import contextmanager
import dataclasses
from typing import ContextManager, overload


@dataclasses.dataclass(frozen=True, slots=True)
class ColorSpec:
    """
    A color specification. It combines a ``tag`` identifying the color format or
    space with the color's ``coordinates``.

    This class is purposefully minimal. It avoids introducing dependencies on
    other prettypretty modules, which might get in the way of this module being
    a dependency of :mod:`prettypretty.color.conversion` when converting from
    OkLab to ANSI colors.

    If you prefer a higher-level, fully object-oriented API, check out
    :class:`prettypretty.color.color.Color`, which extends this class with a
    number of helpful methods. Because of that inheritance relationship,
    ``Color`` instances can be used as theme values, too.
    """

    tag: str
    coordinates: tuple[float, ...]


@dataclasses.dataclass(frozen=True, slots=True, kw_only=True)
class Theme:
    """
    A terminal color theme.

    Almost all terminals have robust support for configuring colors by switching
    between color themes. This class collects the colors belonging to such a
    theme. They comprise the text (foreground) and background colors as well as
    the 16 extended ANSI colors.
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


def _hex(s: str) -> ColorSpec:
    return ColorSpec(
        'rgb256',
        tuple(int(s[n:n+2], base=16) for n in range(0, 6, 2))
    )


# The default theme is the basic theme for macOS terminal
DEFAULT_THEME = MACOS_TERMINAL_BASIC = Theme(
    text=_hex("000000"),
    background=_hex("ffffff"),
    black=_hex("000000"),
    red=_hex("990000"),
    green=_hex("00a600"),
    yellow=_hex("999900"),
    blue=_hex("0000b2"),
    magenta=_hex("b200b2"),
    cyan=_hex("00a6b2"),
    white=_hex("bfbfbf"),
    bright_black=_hex("666666"),
    bright_red=_hex("e50000"),
    bright_green=_hex("00d900"),
    bright_yellow=_hex("e5e500"),
    bright_blue=_hex("0000ff"),
    bright_magenta=_hex("e500e5"),
    bright_cyan=_hex("00e5e5"),
    bright_white=_hex("e5e5e5"),
)

THE_DARK_THEME = Theme(
    text=_hex("cecece"),
    background=_hex("0e1415"),
    black=_hex("0e1415"),
    red=_hex("db4236"),
    green=_hex("159b1d"),
    yellow=_hex("a27a02"),
    blue=_hex("2e86d6"),
    magenta=_hex("b15ead"),
    cyan=_hex("0192a4"),
    white=_hex("aeaeae"),
    bright_black=_hex("484848"),
    bright_red=_hex("fea295"),
    bright_green=_hex("6eed6b"),
    bright_yellow=_hex("ffc200"),
    bright_blue=_hex("a8d1fc"),
    bright_magenta=_hex("ffabf9"),
    bright_cyan=_hex("2ce5fe"),
    bright_white=_hex("f2f2f2"),
)

THE_LIGHT_THEME = Theme(
    text=_hex("0e1415"),
    background=_hex("f2f2f2"),
    black=_hex("0e1415"),
    red=_hex("db4236"),
    green=_hex("159b1d"),
    yellow=_hex("a27a02"),
    blue=_hex("2e86d6"),
    magenta=_hex("b15ead"),
    cyan=_hex("0192a4"),
    white=_hex("aeaeae"),
    bright_black=_hex("484848"),
    bright_red=_hex("fea295"),
    bright_green=_hex("6eed6b"),
    bright_yellow=_hex("ffc200"),
    bright_blue=_hex("a8d1fc"),
    bright_magenta=_hex("ffabf9"),
    bright_cyan=_hex("2ce5fe"),
    bright_white=_hex("f2f2f2"),
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

    When invoked without an argument, this function returns the current theme.
    When invoked with a theme as argument, this function returns a context
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
