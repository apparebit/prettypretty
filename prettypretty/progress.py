import argparse
from collections.abc import Iterator
import random
import time

from prettypretty.color import Color
from prettypretty.color.style import Style # pyright: ignore [reportMissingModuleSource]
from prettypretty.color.termco import (Colorant, Rgb) # pyright: ignore [reportMissingModuleSource]
from prettypretty.terminal import Terminal
from prettypretty.theme import current_translator


def add_fidelity(parser: argparse.ArgumentParser) -> argparse.ArgumentParser:
    """Add command line arguments to control use of colors."""
    group = parser.add_mutually_exclusive_group()
    group.add_argument(
        '--nocolor', action='store_const', const='nocolor', dest='fidelity',
        help='do not use any colors'
    )
    group.add_argument(
        '--ansi', action='store_const', const='ansi', dest='fidelity',
        help='use at most ANSI colors only',
    )
    group.add_argument(
        '--eight-bit', action='store_const', const='eight_bit', dest='fidelity',
        help='use at most 8-bit colors',
    )
    return parser


class ProgressBar:
    """A reusable progress bar."""
    BLOCKS = ' ▎▌▊█'  # <empty> <partial>+ <full>
    STEPS = len(BLOCKS) - 1
    WIDTH = 100 // STEPS + (1 if 100 % STEPS != 0 else 0)
    assert WIDTH * STEPS >= 100  # Without the adjustment, this wouldn't hold

    LIGHT_MODE_BAR = Style().with_foreground(Color.p3(0.0, 1.0, 0.0))
    DARK_MODE_BAR = Style().with_foreground(Rgb(3, 151, 49))

    def __init__(self, term: Terminal) -> None:
        """Select light/dark mode style and adjust color."""
        style = self.DARK_MODE_BAR if term.is_dark_theme() else self.LIGHT_MODE_BAR
        self._style = style.cap(term.fidelity, current_translator())
        self._term = term
        self._percent = 0

    @property
    def color(self) -> None | Colorant:
        """Access the effective progress bar color."""
        return self._style.foreground()

    def _format(self, percent: float) -> list[Style| str]:
        """Generate progress bar for given percentage."""
        percent = min(percent, 100)  # Clamp max at 100.0

        # Need integer multiple (full) and index (partial), hence must round
        full, partial = divmod(round(percent), self.STEPS)
        bar = self.BLOCKS[-1] * full
        if partial > 0:
            # Only add partial character if it is non-empty
            bar += self.BLOCKS[partial]
        bar = bar.ljust(self.WIDTH, self.BLOCKS[0])

        # Displayed percentage remains nicely floating point
        return ['  ┫', self._style, bar, -self._style, '┣', f' {percent:5.1f}%']

    def render(self, percent: float) -> None:
        """Render the progress bar for the given percentage."""
        # Ensure empty/full bar renders, throttle rendering in between
        if (self._percent == 0) or self._percent + 1 < percent or (100 <= percent):
            self._percent = percent
            self._term.column(0).render(self._format(percent)).flush()

    def done(self) -> None:
        """Complete progress bar, write newline, and reset internal state."""
        self.render(100)
        self._term.writeln('\n').flush()
        self._percent = 0


def progress_reports() -> Iterator[float]:
    percent = 0.0
    while True:
        yield percent
        if percent >= 100.0:
            return
        percent += random.gauss(1, 1/3)


def main() -> None:
    options = add_fidelity(
        argparse.ArgumentParser("simulate progress bar")
    ).parse_args()

    with (
        Terminal(fidelity=options.fidelity)
        .terminal_theme()
        .hidden_cursor()
        .scoped_style()
    ) as term:
        progress = ProgressBar(term)
        term.writeln(f'Using {progress.color} as color!\n').flush()

        for percent in progress_reports():
            progress.render(percent)
            time.sleep(random.uniform(1/60, 1/10))
        progress.done()


if __name__ == '__main__':
    main()
