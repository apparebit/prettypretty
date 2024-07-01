"""
Support for representing the sixteen extended ANSI colors and for mapping them
to meaningful color values.
"""

from collections.abc import Iterator
from contextlib import contextmanager
from typing import ContextManager, overload

from .color import Color, Sampler, OkVersion, Theme

MACOS_TERMINAL = Theme([
    Color.parse("#000000"),
    Color.parse("#ffffff"),
    Color.parse("#000000"),
    Color.parse("#990000"),
    Color.parse("#00a600"),
    Color.parse("#999900"),
    Color.parse("#0000b2"),
    Color.parse("#b200b2"),
    Color.parse("#00a6b2"),
    Color.parse("#bfbfbf"),
    Color.parse("#666666"),
    Color.parse("#e50000"),
    Color.parse("#00d900"),
    Color.parse("#e5e500"),
    Color.parse("#0000ff"),
    Color.parse("#e500e5"),
    Color.parse("#00e5e5"),
    Color.parse("#e5e5e5"),
])


VGA = Theme([
    Color.parse("#000000"),
    Color.parse("#ffffff"),
    Color.parse("#000000"),
    Color.parse("#aa0000"),
    Color.parse("#00aa00"),
    Color.parse("#aa5500"),
    Color.parse("#0000aa"),
    Color.parse("#aa00aa"),
    Color.parse("#00aaaa"),
    Color.parse("#aaaaaa"),
    Color.parse("#555555"),
    Color.parse("#ff5555"),
    Color.parse("#55ff55"),
    Color.parse("#ffff55"),
    Color.parse("#5555ff"),
    Color.parse("#ff55ff"),
    Color.parse("#55ffff"),
    Color.parse("#ffffff"),
])


XTERM = Theme([
    Color.parse("#000000"),
    Color.parse("#ffffff"),
    Color.parse("#000000"),
    Color.parse("#cd0000"),
    Color.parse("#00cd00"),
    Color.parse("#cdcd00"),
    Color.parse("#0000ee"),
    Color.parse("#cd00cd"),
    Color.parse("#00cdcd"),
    Color.parse("#e5e5e5"),
    Color.parse("#7f7f7f"),
    Color.parse("#ff0000"),
    Color.parse("#00ff00"),
    Color.parse("#ffff00"),
    Color.parse("#5c5cff"),
    Color.parse("#ff00ff"),
    Color.parse("#00ffff"),
    Color.parse("#ffffff"),
])


def builtin_theme_name(theme: Theme) -> None | str:
    """
    Determine the name of the given theme. If the theme is one of the built-in
    themes, this function returns a descriptive name. Otherwise, it returns
    ``None``.
    """
    if theme is MACOS_TERMINAL:
        return 'macOS Terminal.app default theme'
    elif theme is VGA:
        return 'VGA text theme'
    elif theme is XTERM:
        return 'xterm default theme'
    else:
        return None


_current_theme_and_sampler: list[tuple[Theme, Sampler]] = [
    (VGA, Sampler(VGA, OkVersion.Revised))
]

@contextmanager
def _manage_theme_and_sampler(theme: Theme) -> Iterator[Theme]:
    _current_theme_and_sampler.append((theme, Sampler(theme, OkVersion.Revised)))
    try:
        yield theme
    finally:
        _current_theme_and_sampler.pop()


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
    return (
        _current_theme_and_sampler[-1][0]
        if theme is None
        else _manage_theme_and_sampler(theme)
    )


def current_sampler() -> Sampler:
    """
    Access the current sampler.

    The sampler is automatically updated whenever the current theme changes. It
    uses the revised version of Oklab, i.e., Oklrab, for measuring color
    differences.
    """
    return _current_theme_and_sampler[-1][1]
