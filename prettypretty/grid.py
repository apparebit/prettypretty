"""
A script to visualize 8-bit terminal colors as well as prettypretty's support
for down-sampling colors and maximizing contrast.
"""
import argparse
from typing import cast, Literal

from .ansi import Layer
from .color.conversion import get_converter
from .color.contrast import srgb_to_luminance, use_black_text, use_black_background
from .color.lores import (
    rgb6_to_eight_bit,
    rgb6_to_srgb,
    eight_bit_to_srgb,
    oklab_to_ansi,
    oklab_to_eight_bit,
    naive_eight_bit_to_ansi,
)
from .color.theme import MACOS_TERMINAL, VGA, XTERM, builtin_theme_name, current_theme
from .terminal import Terminal


class FramedBoxes:
    """
    Emit boxes in a frame.

    This class helps with incrementally emitting a grid of boxes with a
    surrounding frame (or border). Each box has the same width and a height of
    one line. Boxes are formatted left-to-right.
    """
    def __init__(
        self,
        term: Terminal,
        box_count: int = 1,
        min_width: int = 5,
        max_width: int = 1_000,
    ) -> None:
        self._term = term
        self._box_count = box_count
        self._box_width = min(term.width - 2, max_width) // box_count
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

        self._term.fg(*foreground).bg(*background).bold().write(box)
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
    term: Terminal,
    *,
    layer: Layer = Layer.BACKGROUND,
    strategy: Literal['8bit', 'pretty', 'naive'] = '8bit',
    show_label: bool = True,
) -> None:
    """
    Format a framed grid with 216 cells, where each cell displays a distinct
    color from the 6x6x6 cube of 8-bit terminal colors.

    Args:
        term: is the terminal for write the framed grid to
        layer: determines whether to color text or background, with background
            the default
        strategy: determines whether to display the original 8-bit colors, to
            downsample using prettypretty, or to use the naive RGB conversion
        label: determines whether boxes are labelled with their color
            components
    """
    frame = FramedBoxes(term, 6)

    prefix = (
        'Original ' if strategy == '8bit'
        else 'Pretty Compression of ' if strategy == 'pretty'
        else 'Naive Compression of '
    )

    frame.top(
        layer.name.capitalize()
        + ': '
        + prefix
        + '6•6•6 RGB Cube'
    )

    srgb_to_oklab = get_converter('srgb', 'oklab')

    for r in range(6):
        for b in range(6):
            frame.left()

            for g in range(6):
                srgb = rgb6_to_srgb(r, g, b)

                if strategy == '8bit':
                    eight_bit = rgb6_to_eight_bit(r, g, b)
                elif strategy == 'pretty':
                    eight_bit = oklab_to_ansi(*srgb_to_oklab(*srgb))
                    srgb = eight_bit_to_srgb(*eight_bit)
                elif strategy == 'naive':
                    eight_bit = naive_eight_bit_to_ansi(*rgb6_to_eight_bit(r, g, b))
                    srgb = eight_bit_to_srgb(*eight_bit)
                else:
                    raise ValueError(f'invalid strategy "{strategy}"')

                # Pick black or white for other color based on contrast
                if layer is Layer.BACKGROUND:
                    foreground = (
                        16 if use_black_text(srgb_to_luminance(*srgb))
                        else 231
                    ),
                    background = eight_bit
                else:
                    foreground = eight_bit
                    background = (
                        16 if use_black_background(srgb_to_luminance(*srgb))
                        else 231
                    ),

                frame.box(
                    f'{r}•{g}•{b}' if show_label else ' ', foreground, background
                )

            frame.right()

    frame.bottom()
    term.writeln()


def write_hires_slice(
    term: Terminal,
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


def write_theme_test(term: Terminal, show_label: bool = True):
    frame = FramedBoxes(term, 2, max_width=40)
    frame.top('Actual vs Claimed Color')

    theme = current_theme()
    ansi_to_srgb = get_converter('ansi', 'srgb')

    for index in range(16):
        srgb = ansi_to_srgb(index)
        fg = (16 if use_black_text(srgb_to_luminance(*srgb)) else 231)

        bg_spec = theme.ansi(index)
        bg = get_converter(bg_spec.tag, 'rgb256')(*bg_spec.coordinates)

        label = ', '.join(f'{c:3d}' for c in bg)

        frame.left()
        frame.box(f'{index}' if show_label else ' ', (fg,), (index,))
        frame.box(label if show_label else ' ', (fg,), cast(tuple[int, int, int], bg))
        frame.right()

    frame.bottom()
    term.writeln()

def create_parser() -> argparse.ArgumentParser:
    """Create a command line argument parser."""
    parser = argparse.ArgumentParser(
        description="""
            Display color grids that visualize the range of terminal colors,
            while also exercising prettypretty's support for maximizing contrast
            and down-sampling colors. By default, this script uses the
            terminal's current color theme and makes a best guess as to the
            terminal's level of color support. That guess is likely less than
            accurate when running ssh.
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
        help='ignore advertised capabilities and force/suppress 24-bit mode',
    )
    parser.add_argument(
        '--slices',
        action='store_true',
        help='show slices through 24-bit RGB cube (in truecolor mode only)'
    )
    parser.add_argument(
        '--no-label',
        action='store_false',
        dest='label',
        help='do not display color labels',
    )

    return parser


if __name__ == '__main__':
    options = create_parser().parse_args()

    fidelity = None
    if options.truecolor is not None:
        fidelity = 'rgb256' if options.truecolor else 'eight_bit'

    with (
        Terminal(fidelity=fidelity)
        .terminal_theme(options.theme)
        .scoped_style()
        as term
    ):
        term.writeln()

        write_color_cube(term, show_label=options.label)
        write_color_cube(term, strategy='pretty', show_label=options.label)
        write_color_cube(term, strategy='naive', show_label=options.label)
        write_color_cube(term, layer=Layer.TEXT)

        if term.fidelity.name == 'RGB256':
            if options.slices:
                for hold in ('r', 'g', 'b'):
                    for level in (0, 128, 255):
                        for downsample in (False, True):
                            write_hires_slice(
                                term,
                                hold=hold,
                                level=level,
                                eight_bit_only=downsample,
                            )

            write_theme_test(term, show_label=options.label)

            term.write_paragraph("""
                The frame showing actual vs claimed colors has two columns. The
                background colors of the first are the 16 extended ANSI colors.
                The background colors of the second are the corresponding 24-bit
                RGB colors advertised by the terminal.

                Both columns should have the exact same visual background color.
            """)

        theme_name = builtin_theme_name(current_theme()) or 'current terminal theme'
        color_mode = 'truecolor' if term.fidelity.name == 'RGB256' else '8-bit color'

        term.writeln(
            'The above charts use the ', theme_name, ' in ', color_mode, ' mode!\n'
        )

        color_support = None
        with term.cbreak_mode():
            color_support = term.request_color_support()
        if color_support is None:
            support_level = 'does not respond to style queries'
        else:
            support_level = f'reports support for {color_support.name.lower()} colors'

        term.writeln(f'The terminal {support_level}!').flush()
