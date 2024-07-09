import argparse
from collections.abc import Iterator
import random
import time

from prettypretty.color import Color
from prettypretty.style import rich, RichText, Style
from prettypretty.terminal import Terminal
from prettypretty.theme import current_sampler


def create_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description='simulate a progress bar'
    )
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


BLOCKS = ' ▎▌▊█'  # <empty> <partial>+ <full>
STEPS = len(BLOCKS) - 1
WIDTH = 100 // STEPS + (1 if 100 % STEPS != 0 else 0)
assert WIDTH * STEPS >= 100  # Without the adjustment, this wouldn't hold

LIGHT_MODE_BAR = rich().fg(Color.p3(0.0, 1.0, 0.0)).style()
DARK_MODE_BAR = rich().fg(3, 151, 49).style()


def format_bar(percent: float, style: Style) -> RichText:
    """Generate progress bar for given percentage."""
    percent = min(percent, 100)  # Clamp max at 100.0

    # Need integer multiple (full) and index (partial), hence must round
    full, partial = divmod(round(percent), STEPS)
    bar = BLOCKS[-1] * full
    if partial > 0:
        # Only add partial character if it is non-empty
        bar += BLOCKS[partial]
    bar = bar.ljust(WIDTH, BLOCKS[0])

    # Displayed percentage remains nicely floating point
    return RichText.of('  ┫', style, bar, ~style, '┣', f' {percent:5.1f}%')


def progress_reports() -> Iterator[float]:
    percent = 0.0
    while True:
        yield percent
        if percent >= 100.0:
            return
        percent += random.gauss(1, 1/3)


def main() -> None:
    options = create_parser().parse_args()

    with (
        Terminal(fidelity=options.fidelity)
        .terminal_theme()
        .hidden_cursor()
        .scoped_style()
    ) as term:
        style = DARK_MODE_BAR if current_sampler().is_dark_theme() else LIGHT_MODE_BAR
        style = style.prepare(term.fidelity)

        fg = style.foreground
        term.writeln(f'Using {"no color" if fg is None else str(fg)}\n').flush()

        for percent in progress_reports():
            bar = format_bar(percent, style)
            term.column(0).rich_text(bar).flush()
            time.sleep(random.uniform(1/60, 1/10))

        term.writeln('\n').flush()


if __name__ == '__main__':
    main()
