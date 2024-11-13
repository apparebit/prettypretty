from typing import Self

from . import Color, OkVersion
from .style import (
    AnsiColor, Colorant, EightBitColor, EmbeddedRgb, Fidelity, GrayGradient, Layer, TrueColor
)
from .theme import Theme


class Translator:
    """A class for translating between terminal and high-resolution colors."""
    def __new__(cls, version: OkVersion, theme: Theme) -> Self: ...
    def __repr__(self) -> str: ...

    # Interrogate the color theme
    def is_dark_theme(self) -> bool: ...

    # Translate terminal to high-resolution colors
    def resolve(
        self,
        color: (
            int | AnsiColor | EmbeddedRgb | GrayGradient | EightBitColor | TrueColor
            | Color | Colorant
        ),
    ) -> Color: ...
    def resolve_all(
        self,
        color: (
            int | AnsiColor | EmbeddedRgb | GrayGradient | EightBitColor | TrueColor
            | Color | Colorant
        ),
        layer: Layer,
    ) -> Color: ...

    # Translate high-resolution to ANSI colors
    def to_ansi(self, color: Color) -> Color: ...
    def supports_hue_lightness(self) -> bool: ...
    def to_ansi_hue_lightness(self, color: Color) -> None | AnsiColor: ...
    def to_closest_ansi(self, color: Color) -> AnsiColor: ...
    def to_ansi_rgb(self, color: Color) -> AnsiColor: ...

    # Translate high-resolution to 8-bit colors
    def to_closest_8bit(self, color: Color) -> EightBitColor: ...
    def to_closest_8bit_with_ansi(self, color: Color) -> EightBitColor: ...

    # Cap terminal colors
    def cap_hires(self, color: Color, fidelity: Fidelity) -> None | Colorant: ...
    def cap_colorant(self, color: Colorant, fidelity: Fidelity) -> None | Colorant: ...
    def cap(
        self,
        color: (
            int | AnsiColor | EmbeddedRgb | GrayGradient | EightBitColor | TrueColor
            | Color | Colorant
        ),
        fidelity: Fidelity,
    ) -> None | Colorant: ...
