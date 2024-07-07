"""
Making sense of ANSI colors.
"""
import sys
from typing import Literal

try:
    import matplotlib.pyplot as plt
    from matplotlib.ticker import FuncFormatter
    #from matplotlib.path import Path as PlotPath
    #import matplotlib.patches as patches
except ImportError:
    print("prettypretty.plot requires matplotlib. Please install the package,")
    print("e.g., by executing `pip install matplotlib`, and then")
    print("run `python -m prettypretty.plot` again.")
    sys.exit(1)

import argparse
import math
import pathlib
from typing import Any, cast

from .terminal import Terminal
from .color import Color, ColorSpace, ThemeEntry
from .theme import current_sampler, VGA


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
        "-q", "--quiet",
        action="count",
        default=0,
        help="run silently, without printing status updates"
    )
    parser.add_argument(
        "-v", "--verbose",
        action="count",
        default=0,
        help="run in verbose mode, which prints extra information to the console"
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
        "-g", "--gamut",
        action="append",
        dest="gamuts",
        help="also plot boundary of colorspace gamut; value must be sRGB, P3, or Rec2020"
    )
    parser.add_argument(
        "-o", "--output",
        help="write color plot to the named file"
    )
    return parser


class ColorPlotter:
    def __init__(
        self,
        figure_label: None | str = None,
        color_label: None | str = None,
        gamut_step: None | int = None,
        gamut_range: None | int = None,
        volume: int = 1,
    ) -> None:
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

        self._gamut_step = gamut_step or 1
        self._gamut_range = gamut_range or 100
        self._largest_gamut: None | ColorSpace = None

        self._volume = volume

    def status(self, msg: str) -> None:
        if self._volume >= 1:
            print(msg)

    def detail(self, msg: str) -> None:
        if self._volume >= 2:
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
        if self._largest_gamut in (ColorSpace.Srgb, ColorSpace.DisplayP3):
            return 0.4
        elif self._largest_gamut is ColorSpace.Rec2020:
            return 0.5
        elif all(c < 0.3 for c in self._chromas):
            return 0.3
        else:
            return 0.4

    def format_ytick_label(self, y: float, _: int) -> str:
        if y % 0.1 < 1e-9 or math.isclose(y, self.effective_max_chroma()):
            return ""
        else:
            return f"{y:.2}"

    def to_boundary_color(self, color: Color) -> tuple[float, float, str]:
        lr, c, h = color.to(ColorSpace.Oklrch).coordinates()
        hex_format = Color(ColorSpace.Oklrch, [lr / 1.5, c, h]).to_hex_format()

        r, g, b = color.coordinates()
        self.detail(f"{r:.2f}, {g:.2f}, {b:.2f} --> {c:.5f}, {h:5.1f} ({hex_format})")

        h = h * math.pi / 180
        return c, h, hex_format

    def generate_boundary_points(
        self, space: ColorSpace, template: list[int], index: int, sign: Literal[1, -1]
    ) -> tuple[list[tuple[float, float]], list[int], list[str]]:
        """
        Generate points on the boundary of the given RGB color space.

        This method varies the values of the component with the given index,
        while using the given template's values for the other components. If the
        sign is negative, this method varies the values from (1 - step_size)
        down to 0. Otherwise, it varies the values from (step_size) to 1.
        """
        assert space.is_rgb()

        # Pick the range of steps.
        if sign < 0:
            steps = range(self._gamut_range - self._gamut_step, -1, -self._gamut_step)
        else:
            steps = range(self._gamut_step, self._gamut_range + 1, self._gamut_step)

        points: list[tuple[float, float]] = []
        instructions: list[int] = []
        colors: list[str] = []

        # Iterate over the range.
        for step in steps:
            rgb = [t / self._gamut_range for t in template]
            rgb[index] = step / self._gamut_range

            # Compute the coordinates.
            c, h, hex_format = self.to_boundary_color(Color(space, rgb))
            points.append((c, h))
            instructions.append(2)
            colors.append(hex_format)

        return points, instructions, colors

    def trace_gamut(
        self, space: ColorSpace
    ) -> tuple[list[tuple[float, float]], list[int], list[str]]:
        """
        Trace the boundary of the gamut for the given color space.

        This method traces the boundary by producing coordinates, matplotlib
        path instructions, and gamut-mapped colors for a series of points from
        the red primary to the green primary to the blue primary and back again.
        """
        self.detail(f"\nTracing {space} gamut:")

        if self._largest_gamut is None:
            self._largest_gamut = space

        c, h, hex_format = self.to_boundary_color(Color(space, [1.0, 0.0, 0.0]))

        all_points: list[tuple[float, float]]= [(c, h)]
        all_instructions: list[int] = [1]
        all_colors: list[str] = [hex_format]

        def trace(template: list[None | int], index: int, sign: Literal[1, -1]) -> None:
            """Trace primary to secondary or secondary to primary boundary segment."""
            # Scale the template by the number of steps.
            template = [(0 if t is None else t * self._gamut_range) for t in template]

            # Generate the boundary points and add to the complete lists.
            points, instructions, colors = self.generate_boundary_points(
                space, cast(list[int], template), index, sign
            )
            all_points.extend(points)
            all_instructions.extend(instructions)
            all_colors.extend(colors)

        trace([1, None, 0], 1, 1) # red to yellow
        self.detail("---------------------------------------------")
        trace([None, 1, 0], 0, -1) # yellow to green
        self.detail("---------------------------------------------")
        trace([0, 1, None], 2, 1) # green to cyan
        self.detail("---------------------------------------------")
        trace([0, None, 1], 1, -1) # cyan to blue
        self.detail("---------------------------------------------")
        trace([None, 0, 1], 0, 1) # blue to magenta
        self.detail("---------------------------------------------")
        trace([1, 0, None], 2, -1) # magenta to red

        all_points[-1] = all_points[0]
        all_instructions[-1] = 79

        return all_points, all_instructions, all_colors

    def create_figure(
        self,
        figure_label: None | str = None,
        color_label: None | str = None,
        gamuts: None | list[ColorSpace] = None,
    ) -> Any:
        fig, axes = plt.subplots(  # type: ignore
            figsize=(5, 5),
            subplot_kw={'projection': 'polar'},
        )

        # Add gamut boundaries if so requested.
        for space in gamuts or []:
            points, _, colors = self.trace_gamut(space)
            for (chroma, hue), color in zip(points, colors):
                axes.scatter(  # type: ignore
                    [hue],
                    [chroma],
                    c=[color],
                    s=[3],
                    marker='o',  # type: ignore
                    #zorder=0,
                ) #type:ignore

            # patch = patches.PathPatch(path, facecolor=None, lw=1)
            # axes.add_patch(patch)

        # Since marker can only be set for all marks in a series, we use a new
        # series for every single color.
        for hue, chroma, color, marker in zip(
            self._hues, self._chromas, self._colors, self._markers
        ):
            size = 80 if marker == "o" else 60
            axes.scatter(  # type: ignore
                [hue],
                [chroma],
                c=[color],
                s=[size],
                marker=marker,  # type: ignore
                edgecolors='#000',
            )

        if self._grays:
            gray = Color.oklab(sum(self._grays) / len(self._grays), 0.0, 0.0).to_hex_format()

            axes.scatter(  # type: ignore
                [0],
                [0],
                c=[gray],
                s=[80],
                marker=self._gray_marker,  # type: ignore
                edgecolors='#000',
            )

        axes.set_rmin(0)  # type: ignore
        axes.set_rmax(self.effective_max_chroma())  # type: ignore

        # Don't show tick labels at angle
        axes.set_rlabel_position(0)  # type: ignore

        # By default matplotlib puts a label every 0.05 units, 0.10 suffices
        axes.yaxis.set_major_formatter(  # type: ignore
            FuncFormatter(self.format_ytick_label)
        )

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
    plotter = ColorPlotter(volume=1-options.quiet+options.verbose)
    terminal_id = None

    gamuts: list[ColorSpace] = []
    for gamut in cast(list[str], options.gamuts or []):
        gamut = gamut.casefold()
        if gamut == 'srgb':
            gamuts.append(ColorSpace.Srgb)
        elif gamut == 'p3':
            gamuts.append(ColorSpace.DisplayP3)
        elif gamut == 'rec2020':
            gamuts.append(ColorSpace.Rec2020)
        else:
            raise ValueError(f"invalid gamut name {gamut}")

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

            sampler = current_sampler()
            for index in range(16):
                color = sampler.resolve(index)
                plotter.add(ThemeEntry.try_from_index(index + 2).name(), color)

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
        file_name = pathlib.Path(options.input).with_suffix(".svg")
    else:
        assert label is not None
        file_name = f'{label.replace(" ", "-").lower()}-colors.svg'

    # ----------------------------------------------------------------------------------
    # Create and save plot

    fig = plotter.create_figure(
        figure_label=label,
        color_label=color_label,
        gamuts=gamuts,
    )
    plotter.status(f"Saving plot to `{file_name}`")
    fig.savefig(file_name, bbox_inches="tight")  # type: ignore


if __name__ == "__main__":
    main()
