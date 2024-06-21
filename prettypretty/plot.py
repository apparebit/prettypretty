"""
Making sense of ANSI colors.
"""
import sys

try:
    import matplotlib.pyplot as plt
    from matplotlib.ticker import FuncFormatter
except ImportError:
    print("prettypretty.plot requires matplotlib. Please install the package,")
    print("e.g., by executing `pip install matplotlib`, and then")
    print("run `python -m prettypretty.plot` again.")
    sys.exit(1)

import argparse
from collections.abc import Iterator
from dataclasses import dataclass, field
import math
from typing import Any, cast

from .terminal import Terminal
from .color.object import Color
from .color.spec import ColorSpec, FloatCoordinateSpec
from .color.theme import current_theme, VGA


def create_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--silent",
        action="store_true",
        help="run silently, without printing status updates"
    )
    parser.add_argument(
        "--vga",
        action="store_const",
        const=VGA,
        dest="theme",
        help="use VGA colors instead of querying terminal"
    )
    parser.add_argument(
        "-c", "--color",
        action="append",
        dest="colors",
        help="also plot color written as six hexadecimal digits"
    )
    parser.add_argument(
        "-o", "--output",
        help="write color plot to named file instead of `<terminal>-colors.svg`"
    )
    return parser


@dataclass
class ColorPlotter:
    state: str = "init"
    silent: bool = False
    kind: str = "ANSI Colors"

    hues: list[float] = field(default_factory=list)
    chromas: list[float] = field(default_factory=list)
    max_chroma: float = 0
    colors: list[str] = field(default_factory=list)
    markers: list[str] = field(default_factory=list)

    duplicates = 0  # Duplicates are not plotted, only counted
    grays = 0   # Grays are not plotted, only counted
    base_count = 0  # Colors with default marker "o"
    extra_count = None

    def status(self, msg: str) -> None:
        if not self.silent:
            print(msg)

    def start_adding(self) -> None:
        if self.state != "init":
            raise ValueError(
                "color plotter is not in initial state anymore; create new instance first"
            )
        self.state = "adding"

        self.status("                Color    L        Chroma   Hue")
        self.status("------------------------------------------------")

    def add(self, name: str, color: ColorSpec, marker: str = "o") -> None:
        if self.state != "adding":
            raise ValueError(
                "color plotter is not in adding state; call start_adding() first"
            )

        color = Color(color)

        # Matplotlib is sRGB only
        srgb = color.to("srgb")
        if not srgb.in_gamut():
            srgb = srgb.to_gamut()
        hex_label = f'{srgb.to("rgb256"):h}'

        # Convert to Oklch
        oklch = color.to("oklch")
        l, c, h = cast(FloatCoordinateSpec, oklch.coordinates)
        l = round(l, 5)
        c = round(c, 5)
        h = round(h, 1)

        # Update status
        light = f'{l:.5}'
        chroma = f'{c:.5}'
        hue = f'{h}'
        self.status(f"{name:14}  {hex_label}  {light:<7}  {chroma:<7}  {hue:>5}")

        # Skip grays and duplicates
        if c == 0 or math.isnan(h):
            self.grays += 1
            return

        if hex_label in self.colors:
            self.duplicates += 1
            return

        # Record hue, chroma, color, marker
        h = h * math.pi / 180
        self.hues.append(h)
        self.chromas.append(c)
        self.max_chroma = max(c, self.max_chroma)
        self.colors.append(hex_label)
        self.markers.append(marker)
        if marker == "o":
            self.base_count += 1

    def stop_adding(self) -> tuple[int, int]:
        if self.state != "adding":
            raise ValueError(
                "color plotter is not in adding state; call start_adding(),"
                " then add() several times, only then stop_adding()"
            )
        self.state = "plotting"

        self.extra_count = len(self.colors) - self.base_count
        self.status(
            f"\nAltogether {self.base_count}+{self.extra_count} colors "
            f"without {self.grays} grays and {self.duplicates} duplicates"
        )

        return self.base_count, self.extra_count

    def format_counts(self) -> str:
        if self.state != "plotting":
            raise ValueError(
                "color plotter is not in plotting state; call stop_adding() first"
            )
        return (
            f"{self.base_count}" if self.extra_count == 0
            else f"{self.base_count}+{self.extra_count}"
        )

    def effective_max_chroma(self) -> float:
        if self.state != "plotting":
            raise ValueError(
                "color plotter is not in plotting state; call stop_adding() first"
            )
        if self.max_chroma < 0.3:
            return 0.3
        else:
            return 0.4

    def __iter__(self) -> Iterator[tuple[float, float, str, str]]:
        if self.state != "plotting":
            raise ValueError(
                "color plotter is not in plotting state; call stop_adding() first"
            )

        for hue, chroma, color, marker in zip(
            self.hues, self.chromas, self.colors, self.markers
        ):
            yield hue, chroma, color, marker

    def format_ytick_label(self, y: float, _: int) -> str:
        if y % 0.1 > 0.01 or abs(y - self.effective_max_chroma()) < 0.01:
            return ""
        else:
            return f"{y:.2}"

    def create_figure(self, terminal_name: str) -> Any:
        fig, axes = plt.subplots(figsize=(5, 5), subplot_kw={'projection': 'polar'})  #type: ignore

        # Since marker can only be set for all marks in a series, we use a new
        # series for every single color.
        for hue, chroma, color, marker in self:
            size = 80 if marker == "o" else 60
            axes.scatter(
                [hue],
                [chroma],
                c=[color],
                s=[size],
                marker=marker,
                edgecolors='#000',
            )

        axes.set_rmax(self.effective_max_chroma())  # type: ignore

        # Don't show tick labels at angle
        axes.set_rlabel_position(0)  # type: ignore

        # By default matplotlib puts a label every 0.05 units, 0.10 suffices
        axes.yaxis.set_major_formatter(FuncFormatter(self.format_ytick_label))

        # Center tick labels on tick
        plt.setp(axes.yaxis.get_majorticklabels(), ha="center")  # type: ignore

        # Make grid appear below points
        axes.set_axisbelow(True)

        axes.set_title(  # type: ignore
            f"{terminal_name}: Hue & Chroma for {self.format_counts()} ANSI Colors in Oklab",
            pad=15,
            weight="bold",
        )

        return fig




def main() -> None:
    options = create_parser().parse_args()
    extra_colors: list[str] = ["#" + c for c in cast(list[str], options.colors) or []]

    def status(msg: str) -> None:
        if not options.silent:
            print(msg)

    plotter = ColorPlotter()
    plotter.silent = options.silent

    terminal_id = None

    # ----------------------------------------------------------------------------------
    # Prepare Colors for Plotting

    plotter.start_adding()
    with Terminal().cbreak_mode().terminal_theme(options.theme) as term:
        if options.theme:
            terminal_id = "VGA", ""
        else:
            terminal_id = term.request_terminal_identity()

        for name, color in current_theme().colors():
            if name in ("text", "background"):
                continue
            plotter.add(name, color)

    for color in extra_colors:
        plotter.add("<extra>", Color(color), marker="d")
    plotter.stop_adding()

    # ----------------------------------------------------------------------------------
    # Terminal and file names

    if terminal_id is None:
        terminal_id = "Unknown Terminal", ""
    terminal_name, _ = terminal_id
    file_name = options.output or f'{terminal_name.replace(" ", "-").lower()}-colors.svg'

    # ----------------------------------------------------------------------------------
    # Create and save plot

    fig = plotter.create_figure(terminal_name)
    status(f"Saving as `{file_name}`")
    fig.savefig(file_name, bbox_inches="tight")  # type: ignore


if __name__ == "__main__":
    main()
