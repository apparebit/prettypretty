"""
A script to visualize 8-bit terminal colors as well as prettypretty's support
for down-sampling colors and maximizing contrast.
"""
import argparse
import os
from typing import Literal

from .ansi import Layer
from .color.conversion import get_converter
from .color.apca import use_black_text, use_black_background
from .color.lores import (
    rgb6_to_eight_bit,
    rgb6_to_srgb,
    eight_bit_to_srgb,
    oklab_to_ansi,
    oklab_to_eight_bit,
)
from .color.theme import MACOS_TERMINAL, VGA, XTERM, builtin_theme_name, current_theme
from .style import Style
from .termio import TermIO


class FramedBoxes:
    """
    Emit boxes in a frame.

    This class helps with incrementally emitting a grid of boxes with a
    surrounding frame (or border). Each box has the same width and a height of
    one line. Boxes are formatted left-to-right.
    """
    def __init__(self, term: TermIO, box_count: int = 1, min_width: int = 5) -> None:
        self._term = term
        self._box_count = box_count
        self._box_width = (term.width - 2) // box_count
        if self._box_width < min_width:
            raise ValueError(
                f'unable to fit {box_count} boxes into {term.width} columns'
            )
        self._width = self._box_count * self._box_width + 2
        self._indent = (term.width - self.outer_width) // 2
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
        self._term.writeln(f'{" " * self._indent}┏━{title}{"━" * filling}━┓')

    def left(self) -> None:
        """Start formatting a line of content."""
        if self._line_content_width != 0:
            raise ValueError('Line started before line ended')
        self._term.write(f'{" " * self._indent}┃')

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

        self._term.set_style(Style().fg(*foreground).bg(*background)).write(box)
        self._line_content_width += self._box_width

    def right(self) -> None:
        """Complete formatting a line of content."""
        self._term.reset_style()
        self._term.writeln('┃')

        if self._line_content_width != self.inner_width:
            raise ValueError(
                f'content spans {self._line_content_width}, '
                f'not {self.inner_width} columns'
            )

        self._line_content_width = 0

    def bottom(self) -> None:
        """Format the bottom of the frame."""
        self._term.writeln(f'{" " * self._indent}┗{"━" * self.inner_width}┛')


def write_color_cube(
    term: TermIO,
    *,
    layer: Layer = Layer.BACKGROUND,
    ansi_only: bool = False,
    label: bool = True,
) -> None:
    """
    Format a framed grid with 216 cells, where each cell displays a distinct
    color from the 6x6x6 cube of 8-bit terminal colors.

    Args:
        term: is the terminal for write the framed grid to
        layer: determines whether to color text or background, with background
            the default
        ansi_only: determines whether to down-sample the 8-bit color to an
            extended ANSI color
    """
    frame = FramedBoxes(term, 6)
    frame.top(
        layer.name.capitalize()
        + ': '
        + ('Downsampled ' if ansi_only else '')
        + '6•6•6 RGB Cube'
    )

    srgb_to_oklab = get_converter('srgb', 'oklab')

    for r in range(6):
        for b in range(6):
            frame.left()

            for g in range(6):
                srgb = rgb6_to_srgb(r, g, b)

                if ansi_only:
                    eight_bit = oklab_to_ansi(*srgb_to_oklab(*srgb))
                    srgb = eight_bit_to_srgb(*eight_bit)
                else:
                    eight_bit = rgb6_to_eight_bit(r, g, b)

                # Pick black or white for other color based on contrast
                if layer is Layer.BACKGROUND:
                    foreground = (232 if use_black_text(*srgb) else 255),
                    background = eight_bit
                else:
                    foreground = eight_bit
                    background = (232 if use_black_background(*srgb) else 255),

                frame.box(f'{r}•{g}•{b}' if label else ' ', foreground, background)

            frame.right()

    frame.bottom()
    term.writeln()


def write_hires_slice(
    term: TermIO,
    *,
    hold: Literal['r', 'g', 'b'] = 'g',
    level: int = 0,
    eight_bit_only: bool = False,
) -> None:
    frame = FramedBoxes(term, 32, min_width=1)
    label = '/'.join(
        f'{l.upper()}={level}' if l == hold else l.upper() for l in ('r', 'g', 'b')
    )
    frame.top(
        ('Downsampled ' if eight_bit_only else '')
        + 'Hi-Res Color Slice for '
        + label
    )

    rgb256_to_oklab = None
    if eight_bit_only:
        rgb256_to_oklab = get_converter('rgb256', 'oklab')

    def emit_box(r: int, g: int, b: int) -> None:
        color = r, g, b
        if eight_bit_only:
            assert rgb256_to_oklab is not None
            color = oklab_to_eight_bit(*rgb256_to_oklab(*color))
        frame.box(' ', (0,), color)

    for x in range(0, 256, 8):
        frame.left()
        for y in range(0, 256, 8):
            if hold == 'r':
                r = level
                g = x
                b = y
            elif hold == 'g':
                r = x
                g = level
                b = y
            elif hold == 'b':
                r = x
                g = y
                b = level
            else:
                raise AssertionError(f'invalid hold "{hold}"')

            emit_box(r, g, b)

        frame.right()

    frame.bottom()
    term.writeln()


def create_parser() -> argparse.ArgumentParser:
    """Create a command line argument parser."""
    parser = argparse.ArgumentParser(
        description="""
            Display color grids that visualize the range of terminal colors,
            while also exercising prettypretty's support for maximizing
            contrast and down-sampling colors. If not color theme is requested
            with a command line argument, try to use terminal's current theme.
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

    parser.add_argument(
        '--truecolor',
        action=argparse.BooleanOptionalAction,
        default=None,
        help='ignore advertised capabilities and force/suppress 24-bit color slices',
    )

    parser.add_argument(
        '--no-label',
        action='store_const',
        const=False,
        default=True,
        dest='label',
        help='do not display color labels',
    )

    return parser


if __name__ == '__main__':
    options = create_parser().parse_args()

    term = TermIO()
    with term.cbreak_mode().terminal_theme(options.theme).scoped_style():
        term.writeln()

        write_color_cube(term, label=options.label)
        write_color_cube(term, ansi_only=True, label=options.label)
        write_color_cube(term, layer=Layer.TEXT)

        # The truecolor option only fires if it was actually used.
        if options.truecolor in (None, True) and os.getenv('COLORTERM') == 'truecolor':
            for hold in ('r', 'g', 'b'):
                for level in (0, 128, 255):
                    for downsample in (False, True):
                        write_hires_slice(
                            term,
                            hold=hold,
                            level=level,
                            eight_bit_only=downsample,
                        )

        theme_name = builtin_theme_name(current_theme())
        term.set_style(Style().italic).writeln(
            f'Used ', theme_name or 'current terminal theme', '!\n'
        ).flush()
