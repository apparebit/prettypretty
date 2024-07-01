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
import math
from pathlib import Path
from typing import Any, cast

from .terminal import Terminal
from .color import Color, ColorSpace, ThemeEntry
from .theme import current_theme, VGA


def create_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="""
            Plot colors on the chroma/hue plane of Oklab while ignoring their
            lightness. If the -i/--input option is specified, this script reads
            newline-separated colors from the named file. If the --vga option is
            specified, it uses the VGA colors. Otherwise, it uses the terminal's
            current ANSI theme colors. This script correctly plots colors
            irrespective of their color space and the current display gamut.
            However, since its visualization library (matplotlib) is limited to
            sRGB, each mark's color is gamut-mapped to sRGB.
        """
    )
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
        "-i", "--input",
        help="read newline-separated colors from the named file"
    )
    parser.add_argument(
        "-o", "--output",
        help="write color plot to the named file"
    )
    return parser


class ColorPlotter:
    def __init__(
        self,
        silent: bool = False,
        figure_label: None | str = None,
        color_label: None | str = None,
    ) -> None:
        self._silent = silent
        self._figure_label = figure_label
        self._color_label = color_label

        self._hues: list[float] = []
        self._chromas: list[float] = []
        self._colors: list[str] = []
        self._markers: list[str] = []

        self._grays: list[float] = []
        self._gray_marker = None

        self._duplicate_count = 0
        self._base_count = 0
        self._extra_count = 0

    def status(self, msg: str) -> None:
        if not self._silent:
            print(msg)

    def start_adding(self) -> None:
        self.status("                Color    L        Chroma   Hue")
        self.status("------------------------------------------------")

    def add(self, name: str, color: Color, marker: str = "o") -> None:
        # Matplotlib is sRGB only
        hex_color = color.to_hex_format()

        # Convert to Oklch
        oklch = color.to(ColorSpace.Oklch)
        l, c, h = oklch[0], oklch[1], oklch[2]
        c = round(c, 14)  # Chop off one digit of precision.

        # Update status
        light = f'{l:.5}'
        if len(light) > 7:
            light = f'{l:.5f}'
        chroma = f'{c:.5}'
        if len(chroma) > 7:
            chroma = f'{c:.5f}'
        hue = f'{h:.1f}'
        self.status(f"{name:14}  {hex_color}  {light:<7}  {chroma:<7}  {hue:>5}")

        # Handle grays
        if c < 1e-9 or math.isnan(h):
            self._grays.append(l)
            if self._gray_marker is None:
                self._gray_marker = marker
            elif marker != self._gray_marker:
                raise ValueError(
                    f"inconsistent markers for gray: {marker} vs {self._gray_marker}"
                )
            return

        # Skip duplicates
        if hex_color in self._colors:
            self._duplicate_count += 1
            return

        # Record hue, chroma, color, marker
        h = h * math.pi / 180
        self._hues.append(h)
        self._chromas.append(c)
        self._colors.append(hex_color)
        self._markers.append(marker)
        if marker == "o":
            self._base_count += 1
        else:
            self._extra_count += 1

    def stop_adding(self) -> None:
        self.status(
            f"\nAltogether {self.format_counts()} colors, "
            f" {len(self._grays)} grays, and {self._duplicate_count} duplicates"
        )

    def format_counts(self) -> str:
        gray_count = len(self._grays)
        if gray_count == 0 and self._extra_count == 0:
            return f"{self._base_count}"

        counts = f"{self._base_count}+{gray_count}"
        if self._extra_count > 0:
            counts += f"+{self._extra_count}"
        return counts

    def effective_max_chroma(self) -> float:
        if all(c < 0.3 for c in self._chromas):
            return 0.3
        else:
            return 0.4

    def format_ytick_label(self, y: float, _: int) -> str:
        if y % 0.1 < 1e-9 or math.isclose(y, self.effective_max_chroma()):
            return ""
        else:
            return f"{y:.2}"

    def create_figure(
        self,
        figure_label: None | str = None,
        color_label: None | str = None,
    ) -> Any:
        fig, axes = plt.subplots(  # type: ignore
            figsize=(5, 5),
            subplot_kw={'projection': 'polar'},
        )

        # Since marker can only be set for all marks in a series, we use a new
        # series for every single color.
        for hue, chroma, color, marker in zip(
            self._hues, self._chromas, self._colors, self._markers
        ):
            size = 80 if marker == "o" else 60
            axes.scatter(
                [hue],
                [chroma],
                c=[color],
                s=[size],
                marker=marker,
                edgecolors='#000',
            )

        if self._grays:
            gray = Color.oklab(sum(self._grays) / len(self._grays), 0.0, 0.0).to_hex_format()

            axes.scatter(
                [0],
                [0],
                c=[gray],
                s=[80],
                marker=self._gray_marker,
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

        figure_label = figure_label or self._figure_label
        color_label = color_label or self._color_label or "Colors"
        title = f"{figure_label}: " if figure_label else ""
        title += f"Hue & Chroma for {self.format_counts()} {color_label} in Oklab"
        axes.set_title(title, pad=15, weight="bold")  # type: ignore

        return fig


def main() -> None:
    options = create_parser().parse_args()
    plotter = ColorPlotter(silent=options.silent)
    terminal_id = None

    # ----------------------------------------------------------------------------------
    # Prepare Colors for Plotting

    plotter.start_adding()

    if options.input is not None:
        with open(options.input, mode="r", encoding="utf8") as file:
            for color in [Color.parse(l) for l in file.readlines() if l.strip()]:
                plotter.add("", color)
    else:
        with Terminal().cbreak_mode().terminal_theme(options.theme) as term:
            if not options.theme:
                terminal_id = term.request_terminal_identity()

            theme = current_theme()
            for index in range(2, 18):
                color = theme[index]
                plotter.add(ThemeEntry.from_index(index).name(), color)

    for color in [Color.parse("#" + c) for c in cast(list[str], options.colors) or []]:
        plotter.add("<extra>", color, marker="d")

    plotter.stop_adding()

    # ----------------------------------------------------------------------------------
    # Labels and file names

    if options.theme:
        label = "VGA"
    elif options.input:
        label = None
    elif terminal_id:
        label = terminal_id[0]
    else:
        label = "Unknown Terminal"

    if options.input:
        color_label = "Colors"
    else:
        color_label = "ANSI Colors"

    if options.output is not None:
        file_name = options.output
    elif options.input is not None:
        file_name = Path(options.input).with_suffix(".svg")
    else:
        assert label is not None
        file_name = f'{label.replace(" ", "-").lower()}-colors.svg'

    # ----------------------------------------------------------------------------------
    # Create and save plot

    fig = plotter.create_figure(figure_label=label, color_label=color_label)
    plotter.status(f"Saving plot to `{file_name}`")
    fig.savefig(file_name, bbox_inches="tight")  # type: ignore


if __name__ == "__main__":
    main()
