import os

import termkit.clr as clr
import termkit.stl as stl


class Frame:

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
        return self._width

    @property
    def inner_width(self) -> int:
        return self._width - 2

    def top(self, title: str = '') -> None:
        title = f' {t} ' if (t := title.strip()) != '' else t
        filling = self.outer_width - 4 - len(title)
        if filling < 0:
            raise ValueError(f'"{title}" is too long for {self.outer_width}-wide frame')
        if title:
            title = f'\x1b[1m{title}\x1b[m'
        self._fragments.append(f'┏━{title}{"━" * filling}━┓\n')

    def left(self) -> None:
        self._fragments.append('┃')
        if self._line_content_width != 0:
            raise ValueError('Line started before line ended')

    def box(self, text: str, foreground: int, background: int) -> None:
        box = text.center(self._box_width)
        if len(box) != self._box_width:
            raise ValueError(f'"{text}" does not fit into {self._box_width}-wide box')

        code = stl.sgr(
            *stl.eight_bit_to_sgr_params(background, stl.Layer.BACKGROUND),
            *stl.eight_bit_to_sgr_params(foreground, stl.Layer.TEXT),
        )
        self._fragments.append(f'{code}{box}\x1b[m')
        self._line_content_width += self._box_width

    def right(self) -> None:
        if self._line_content_width != self.inner_width:
            raise ValueError(
                f'content spans {self._line_content_width}, '
                f'not {self.inner_width} columns'
            )

        self._fragments.append('┃\n')
        self._line_content_width = 0

    def bottom(self) -> None:
        self._fragments.append(f'┗{"━" * self.inner_width}┛')

    def __str__(self) -> str:
        return ''.join(self._fragments)


def format_grid(
    width: int,
    *,
    vary: stl.Layer = stl.Layer.BACKGROUND,
    ansi_only: bool = False,
) -> str:
    frame = Frame(width, 6)
    frame.top(
        vary.name.capitalize()
        + ': '
        + ('Downsampled ' if ansi_only else '')
        + '6•6•6 RGB Cube'
    )

    for r in range(6):
        for b in range(6):
            frame.left()

            for g in range(6):
                # Produce 8-bit color from RGB components
                if ansi_only:
                    color = clr.rgb6_to_ansi(r, g, b)
                else:
                    color = clr.rgb6_to_eight_bit(r, g, b)

                # Pick black or white for other color based on contrast
                if vary is stl.Layer.BACKGROUND:
                    foreground = 0 if clr.apca_use_black_text(color) else 15
                    background = color
                else:
                    foreground = color
                    background = 232 if clr.apca_use_black_background(color) else 15

                frame.box(f'{r}•{g}•{b}', foreground, background)

            frame.right()

    frame.bottom()
    return str(frame)


if __name__ == '__main__':
    width, _ = os.get_terminal_size()

    print(f'\n{format_grid(width)}')
    print(f'\n{format_grid(width, ansi_only=True)}')
    print(f'\n{format_grid(width, vary=stl.Layer.TEXT)}\n')
