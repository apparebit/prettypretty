"""
Support for representing the sixteen extended ANSI colors and for mapping them
to meaningful color values.
"""

from collections.abc import Iterator
from contextlib import contextmanager

from .color import OkVersion, Translator
from .color.theme import Theme, VGA_COLORS # pyright: ignore [reportMissingModuleSource]


_current_translator: list[Translator] = [Translator(OkVersion.Revised, VGA_COLORS)]

@contextmanager
def new_theme(theme_colors: Theme) -> Iterator[Translator]:
    """
    Create a new context manager to make the theme colors the current theme
    colors. This function expects exactly 18 colors.
    """
    _current_translator.append(Translator(OkVersion.Revised, theme_colors))
    try:
        yield _current_translator[-1]
    finally:
        _current_translator.pop()

def current_translator() -> Translator:
    """
    Access the current translator.

    The translator is automatically updated when using different theme colors.
    """
    return _current_translator[-1]
