from typing import TypeAlias

from .color import Color
from .color.style import ( # pyright: ignore [reportMissingModuleSource]
    AnsiColor, Colorant, EmbeddedRgb, GrayGradient, TrueColor
)

IntoColorant: TypeAlias = (
    # An 8-bit index
    int |
    # Terminal colors
    AnsiColor | EmbeddedRgb | GrayGradient | TrueColor |
    # High-resolution colors
    Color |
    # Colorant itself
    Colorant
)

