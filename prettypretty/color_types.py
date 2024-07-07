from typing import TypeAlias

from .color import (
    AnsiColor,
    DefaultColor,
    EmbeddedRgb,
    GrayGradient,
    TerminalColor,
    TrueColor,
)

IntoTerminalColor: TypeAlias = (
    # The constituent color types
    DefaultColor | AnsiColor | EmbeddedRgb | GrayGradient | TrueColor
    # An 8-bit index
    | int
    # Terminal color itself
    | TerminalColor
)

