def is_default(color: int) -> bool:
    """Determine whether the given color is the default color, i.e., -1."""
    return color == -1


def is_ansi(color: int) -> bool:
    """
    Determine whether the given color is an (extended) ANSI standard color,
    i.e., 0–15.
    """
    return 0 <= color <= 15


def is_cube(color: int) -> bool:
    """Determine whether the given color is from the 6x6x6 RGB cube."""
    return 16 <= color <= 231


def is_grey(color: int) -> bool:
    """Determine whether the given color is a grey from the 24-step gradient."""
    return 232 <= color <= 255
