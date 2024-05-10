"""
Visualize the 6x6x6 RGB cube for 8-bit terminal color, while also exercising
contrast computation and, optionally, down-sampling. You can run this module as
a script::

    $ python -m prettypretty.grid

"""
import os
from typing import cast

from .color.conversion import convert
from .color.apca import use_black_text, use_black_background
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

    def box(self, text: str, foreground: int, background: int) -> None:
        """Format one box of the content."""
        box = text.center(self._box_width)
        if len(box) != self._box_width:
            raise ValueError(f'"{text}" does not fit into {self._box_width}-wide box')

        code = sgr(
            *eight_bit_to_sgr_params(background, Layer.BACKGROUND),
            *eight_bit_to_sgr_params(foreground, Layer.TEXT),
        )
        self._fragments.append(f'{code}{box}\x1b[m')
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

    for r in range(6):
        for b in range(6):
            frame.left()

            for g in range(6):
                # Produce 8-bit color from RGB components
                srgb = convert((r, g, b), 'rgb6', 'srgb')

                if ansi_only:
                    color = cast(int, convert((r, g, b), 'rgb6', 'ansi')[0])
                else:
                    color = cast(int, convert((r, g, b), 'rgb6', 'eight_bit_cube')[0])

                # Pick black or white for other color based on contrast
                if layer is Layer.BACKGROUND:
                    foreground = 232 if use_black_text(*srgb) else 255
                    background = color
                else:
                    foreground = color
                    background = 232 if use_black_background(*srgb) else 255

                frame.box(f'{r}•{g}•{b}', foreground, background)

            frame.right()

    frame.bottom()
    return str(frame)


if __name__ == '__main__':
    width, _ = os.get_terminal_size()

    print(f'\n{format_color_cube(width)}')
    print(f'\n{format_color_cube(width, ansi_only=True)}')
    print(f'\n{format_color_cube(width, layer=Layer.TEXT)}\n')
