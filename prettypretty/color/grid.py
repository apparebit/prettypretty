"""
A script to visualize 8-bit terminal colors as well as prettypretty's support
for down-sampling colors and maximizing contrast.
"""
import argparse
import os

from .conversion import rgb256_to_srgb, get_converter
from .apca import use_black_text, use_black_background
from .lores import (
    rgb6_to_eight_bit,
    rgb6_to_rgb256,
    eight_bit_to_rgb256,
    oklab_to_ansi,
    oklab_to_eight_bit,
)
from .theme import MACOS_TERMINAL, VGA, XTERM, current_theme
from .style import eight_bit_to_sgr_params, Layer, sgr

class FramedBoxes:
    """
    Boxes in a frame.

    This class helps with incrementally formatting a grid of boxes with a
    surrounding frame (or border). Each box has the same width and a height of
    one line. Boxes are formatted left-to-right.
    """
    width: int
    box_count: int
    box_width: int
    fragments: list[str]

    def __init__(self, width: int, box_count: int = 1, min_width: int = 5) -> None:
        self._box_count = box_count
        self._box_width = (width - 2) // box_count
        if self._box_width < min_width:
            raise ValueError(f'unable to fit {box_count} boxes into {width} columns')
        self._width = self._box_count * self._box_width + 2
        self._fragments: list[str] = []
        self._line_content_width = 0

    @property
    def outer_width(self) -> int:
        """The total width of the frame."""
        return self._width

    @property
    def inner_width(self) -> int:
        """
        The width of the content inside the frame, which is ``outer_width - 2``.
        """
        return self._width - 2

    def top(self, title: str = '') -> None:
        """Format the top of the frame."""
        title = f' {t} ' if (t := title.strip()) != '' else t
        filling = self.outer_width - 4 - len(title)
        if filling < 0:
            raise ValueError(f'"{title}" is too long for {self.outer_width}-wide frame')
        if title:
            title = f'\x1b[1m{title}\x1b[m'
        self._fragments.append(f'┏━{title}{"━" * filling}━┓\n')

    def left(self) -> None:
        """Start formatting a line of content."""
        self._fragments.append('┃')
        if self._line_content_width != 0:
            raise ValueError('Line started before line ended')

    def box(
        self,
        text: str,
        foreground: tuple[int] | tuple[int, int, int],
        background: tuple[int] | tuple[int, int, int]
    ) -> None:
        """Format one box of the content."""
        box = text.center(self._box_width)
        if len(box) != self._box_width:
            raise ValueError(f'"{text}" does not fit into {self._box_width}-wide box')

        if len(background) == 1:
            bg_params = eight_bit_to_sgr_params(*background, Layer.BACKGROUND)
        else:
            bg_params = 48, 2, *background

        if len(foreground) == 1:
            fg_params = eight_bit_to_sgr_params(*foreground, Layer.TEXT)
        else:
            fg_params = 38, 2, *foreground

        self._fragments.append(f'{sgr(*bg_params, *fg_params)}{box}\x1b[m')
        self._line_content_width += self._box_width

    def right(self) -> None:
        """Complete formatting a line of content."""
        if self._line_content_width != self.inner_width:
            raise ValueError(
                f'content spans {self._line_content_width}, '
                f'not {self.inner_width} columns'
            )

        self._fragments.append('┃\n')
        self._line_content_width = 0

    def bottom(self) -> None:
        """Format the bottom of the frame."""
        self._fragments.append(f'┗{"━" * self.inner_width}┛')

    def __str__(self) -> str:
        return ''.join(self._fragments)


def format_color_cube(
    width: int,
    *,
    layer: Layer = Layer.BACKGROUND,
    ansi_only: bool = False,
) -> str:
    """
    Format a framed grid with 216 cells, where each cell displays a distinct
    color from the 6x6x6 cube of 8-bit terminal colors.

    Args:
        width: is the number of columns available to the framed grid
        layer: determines whether to color text or background, with background
            the default
        ansi_only: determines whether to down-sample the 8-bit color to an
            extended ANSI color

    Returns:
        The fully formated and framed grid
    """
    frame = FramedBoxes(width, 6)
    frame.top(
        layer.name.capitalize()
        + ': '
        + ('Downsampled ' if ansi_only else '')
        + '6•6•6 RGB Cube'
    )

    rgb256_to_oklab = get_converter('rgb256', 'oklab')

    for r in range(6):
        for b in range(6):
            frame.left()

            for g in range(6):
                rgb256 = rgb6_to_rgb256(r, g, b)

                if ansi_only:
                    eight_bit = oklab_to_ansi(*rgb256_to_oklab(*rgb256))
                    srgb = rgb256_to_srgb(*eight_bit_to_rgb256(*eight_bit))
                else:
                    eight_bit = rgb6_to_eight_bit(r, g, b)
                    srgb = rgb256_to_srgb(*rgb256)

                # Pick black or white for other color based on contrast
                if layer is Layer.BACKGROUND:
                    foreground = (232 if use_black_text(*srgb) else 255),
                    background = eight_bit
                else:
                    foreground = eight_bit
                    background = (232 if use_black_background(*srgb) else 255),

                frame.box(f'{r}•{g}•{b}', foreground, background)

            frame.right()

    frame.bottom()
    return str(frame)


def format_hires_slice(
    width: int,
    *,
    eight_bit_only: bool = False,
) -> str:
    frame = FramedBoxes(width, 32, min_width=1)
    frame.top(
        'High-Resolution Color Slice'
    )

    rgb256_to_oklab = get_converter('rgb256', 'oklab')

    for r in range(0, 256, 8):
        frame.left()
        g = 0

        for b in range(0, 256, 8):
            color = r, g, b
            if eight_bit_only:
                color = oklab_to_eight_bit(*rgb256_to_oklab(*color))
            frame.box(' ', (0,), color)

        frame.right()

    frame.bottom()
    return str(frame)


def create_parser() -> argparse.ArgumentParser:
    """Create a command line argument parser."""
    parser = argparse.ArgumentParser(
        description="""
            Display color grids that visualize the range of terminal colors,
            while also exercising prettypretty's support for maximizing
            contrast and down-sampling colors.
        """,
    )

    group = parser.add_mutually_exclusive_group()
    group.add_argument(
        '--macos-terminal',
        action='store_const',
        const=MACOS_TERMINAL,
        dest='theme',
        help="use the same colors as the Basic theme for macOS Terminal"
    )
    group.add_argument(
        '--xterm',
        action='store_const',
        const=XTERM,
        dest='theme',
        help='use the same colors as xterm'
    )
    group.add_argument(
        '--vga',
        action='store_const',
        const=VGA,
        dest='theme',
        help='use the same colors as VGA in text mode'
    )

    return parser


if __name__ == '__main__':
    options = create_parser().parse_args()
    width, _ = os.get_terminal_size()

    with current_theme(options.theme or VGA):
        print(f'\n{format_color_cube(width)}')
        print(f'\n{format_color_cube(width, ansi_only=True)}')
        print(f'\n{format_color_cube(width, layer=Layer.TEXT)}')

        if os.getenv('COLORTERM') == 'truecolor':
            print(f'\n{format_hires_slice(width)}')
            print(f'\n{format_hires_slice(width, eight_bit_only=True)}')
