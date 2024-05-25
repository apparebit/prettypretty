import argparse
from collections.abc import Iterator
import random
import time

from prettypretty.style import RichText, Style
from prettypretty.terminal import Terminal


def create_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description='simulate a progress bar'
    )
    group = parser.add_mutually_exclusive_group()
    group.add_argument(
        '--ansi-only',
        action='store_const',
        const='ansi',
        default='rgb256',
        dest='fidelity',
        help='use ANSI colors only',
    )
    group.add_argument(
        '--eight-bit-only',
        action='store_const',
        const='eight_bit',
        dest='fidelity',
        help='use 8-bit colors only',
    )
    return parser


BLOCKS = ' ▎▌▊█'
STEPS = len(BLOCKS) - 1
WIDTH = 100 // STEPS + (1 if 100 % STEPS != 0 else 0)
assert WIDTH * STEPS >= 100


STYLE = Style.fg('p3', 0, 1, 0)


def format_bar(percent: float) -> RichText:
    """Generate progress bar for given percentage."""
    percent = min(percent, 100)  # Clamp max at 100.0

    # To format bar itself, we turn percent into integer indices.
    full, partial = divmod(round(percent), STEPS)
    bar = BLOCKS[-1] * full
    if partial > 0:
        # Only add character if it visually grows bar
        bar += BLOCKS[partial]
    bar = bar.ljust(WIDTH, BLOCKS[0])

    # But the displayed percentage is nicely floating point.
    return RichText.of('┫', STYLE, bar, ~STYLE, '┣', f' {percent:5.1f}%')


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
        Terminal()
        .terminal_theme()
        .fidelity(options.fidelity)
        .hidden_cursor()
        .scoped_style()
    ) as term:
        term.writeln('\n')

        for percent in progress_reports():
            term.column(0).rich_text(format_bar(percent)).flush()
            time.sleep(random.uniform(1/60, 1/10))

        term.writeln('\n').flush()


if __name__ == '__main__':
    main()
