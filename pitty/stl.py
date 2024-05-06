import enum


class Layer(enum.Enum):
    """The two layers for terminal presentation, text and background."""
    TEXT = 0
    BACKGROUND = 10


def eight_bit_to_sgr_params(color: int, layer: Layer) -> tuple[int, ...]:
    """Convert the 8-bit terminal color to the corresponding SGR parameters."""
    if 0 <= color <= 7:
        return 30 + layer.value + color,
    if 8 <= color <= 15:
        return 90 + layer.value + color - 8,
    return 38 + layer.value, 5, color


def sgr(*parameters: int) -> str:
    """Create an SGR escape sequence for the given parameters."""
    return f'\x1b[{";".join(str(p) for p in parameters)}m'
