"""
Support for representing the sixteen extended ANSI colors and for mapping them
to meaningful color values.
"""

from collections.abc import Iterator
from contextlib import contextmanager

from .color import Color, Sampler, OkVersion

MACOS_TERMINAL = (
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
)


VGA = (
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
)


XTERM = (
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
)


_current_sampler: list[Sampler] = [Sampler(OkVersion.Revised, VGA)]

@contextmanager
def new_theme(theme_colors: list[Color]) -> Iterator[Sampler]:
    """
    Create a new context manager to make the theme colors the current theme
    colors. This function expects exactly 18 colors.
    """
    _current_sampler.append(Sampler(OkVersion.Revised, theme_colors))
    try:
        yield _current_sampler[-1]
    finally:
        _current_sampler.pop()

def current_sampler() -> Sampler:
    """
    Access the current sampler.

    The sampler is automatically updated when using different theme colors.
    """
    return _current_sampler[-1]
