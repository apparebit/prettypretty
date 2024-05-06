__all__ = (
    # (De)Compose 8-bit colors into RGB6 components
    'eight_bit_cube_to_rgb6',
    'rgb6_to_eight_bit',
    'rgb6_to_ansi',
    # Compute contrast, pick black or white for maximum contrast
    'apca_contrast',
    'apca_use_black_text',
    'apca_use_black_background',

    'sgr',
)

from .clr import (
    eight_bit_cube_to_rgb6,
    rgb6_to_eight_bit,
    rgb6_to_ansi,

    apca_contrast,
    apca_use_black_text,
    apca_use_black_background,
)

from .stl import (
    sgr,
)
