from typing import TypeAlias

from .color.term import ( # pyright: ignore [reportMissingModuleSource]
    AnsiColor, DefaultColor, EmbeddedRgb, GrayGradient, TrueColor, TerminalColor
)

IntoTerminalColor: TypeAlias = (
    # The constituent color types
    DefaultColor | AnsiColor | EmbeddedRgb | GrayGradient | TrueColor
    # An 8-bit index
    | int
    # Terminal color itself
    | TerminalColor
)

