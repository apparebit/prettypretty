"""
Making sense of ANSI colors.
"""
import sys
from typing import Literal

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
import pathlib
from typing import Any, cast

from .terminal import Terminal
from .color import close_enough, Color, ColorSpace, ThemeEntry
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
        help="read newline-separated colors from the named file",
    )
    parser.add_argument(
        "-g", "--gamut",
        action="append",
        dest="gamuts",
        help="also plot boundary of colorspace gamut; value must be sRGB, P3, or Rec2020",
    )
    parser.add_argument(
        "--spectrum",
        action="store_true",
        help="also plot boundary of visible spectrum",
    )
    parser.add_argument(
        "-o", "--output",
        help="write color plot to the named file"
    )
    return parser


class ColorPlotter:
    def __init__(
        self,
        collection_name: None | str = None,
        color_kind: None | str = None,
        gamut_step: None | int = None,
        gamut_range: None | int = None,
        volume: int = 1,
    ) -> None:
        self._collection_name = collection_name
        self._color_kind = color_kind

        # Scattered colors
        self._hues: list[float] = []
        self._chromas: list[float] = []
        self._marks: list[str] = []
        self._mark_colors: list[str] = []

        # Averaged grays as one
        self._grays: list[float] = []
        self._gray_mark = None

        # Lightness bars
        self._lightness: list[float] = []
        self._bar_color: list[str] = []

        # Counts
        self._colors: set[Color] = set()
        self._duplicate_count = 0
        self._base_count = 0
        self._extra_count = 0

        # Gamut boundaries
        self._gamut_step = gamut_step or 1
        self._gamut_range = gamut_range or 20
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

        # Convert to Oklrch
        oklrch = color.to(ColorSpace.Oklrch)
        lr, c, h = oklrch
        c = round(c, 14)  # Chop off one digit of precision.

        # Update status
        light = f'{lr:.5}'
        if len(light) > 7:
            light = f'{lr:.5f}'
        chroma = f'{c:.5}'
        if len(chroma) > 7:
            chroma = f'{c:.5f}'
        hue = f'{h:.1f}'
        self.status(f"{name:14}  {hex_color}  {light:<7}  {chroma:<7}  {hue:>5}")

        # Handle grays
        if c < 1e-9 or math.isnan(h):
            self._grays.append(lr)
            if self._gray_mark is None:
                self._gray_mark = marker
            elif marker != self._gray_mark:
                raise ValueError(
                    f"inconsistent markers for gray: {marker} vs {self._gray_mark}"
                )
            return

        # Skip duplicates
        if oklrch in self._colors:
            self._duplicate_count += 1
            return

        # Record hue, chroma, color, marker
        h_radian = h * math.pi / 180
        self._hues.append(h_radian)
        self._chromas.append(c)
        self._marks.append(marker)
        self._mark_colors.append(hex_color)

        self._lightness.append(lr)
        self._bar_color.append(Color(ColorSpace.Oklrch, [lr, c, h]).to_hex_format())

        self._colors.add(oklrch)
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

    def format_ytick_label_05(self, y: float, _: int) -> str:
        if y % 0.1 < 1e-9 or math.isclose(y, self.effective_max_chroma()):
            return ""
        else:
            return f"{y:.2}"

    def format_ytick_label_10(self, y: float, _: int) -> str:
        if y > 0.01 and y % 0.1 < 1e-9 and y < self.effective_max_chroma() - 0.01:
            return f"{y:.2}"
        else:
            return ""

    def to_point_and_color(self, color: Color) -> tuple[float, float, str]:
        _, c, h = color.to(ColorSpace.Oklrch).coordinates()
        hex_format = Color(ColorSpace.Oklrch, [0.75, c / 3, h]).to_hex_format()

        r, g, b = color.coordinates()
        self.detail(
            f"{color.space()} gamut: "
            f"{r:.2f}, {g:.2f}, {b:.2f} --> "
            f"{c:.5f}, {h:5.1f}º ({hex_format})"
        )

        h = h * math.pi / 180
        return c, h, hex_format

    def generate_boundary_points(
        self, space: ColorSpace, template: list[int], index: int, sign: Literal[1, -1]
    ) -> tuple[list[tuple[float, float]], list[str]]:
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
        colors: list[str] = []

        # Iterate over the range.
        for step in steps:
            rgb = [t / self._gamut_range for t in template]
            rgb[index] = step / self._gamut_range

            # Compute the coordinates.
            c, h, hex_format = self.to_point_and_color(Color(space, rgb))
            points.append((c, h))
            colors.append(hex_format)

        return points, colors

    def trace_gamut(
        self, space: ColorSpace
    ) -> tuple[list[tuple[float, float]], list[str]]:
        """
        Trace the boundary of the gamut for the given color space.

        This method traces the boundary by producing coordinates, matplotlib
        path instructions, and gamut-mapped colors for a series of points from
        the red primary to the green primary to the blue primary and back again.
        """
        if (
            self._largest_gamut is None
            or space is ColorSpace.Rec2020
            or space is ColorSpace.DisplayP3 and self._largest_gamut is ColorSpace.Srgb
        ):
            self._largest_gamut = space

        all_points: list[tuple[float, float]]= []
        all_colors: list[str] = []

        def trace(template: list[None | int], index: int, sign: Literal[1, -1]) -> None:
            """Trace primary to secondary or secondary to primary boundary segment."""
            # Scale the template by the number of steps.
            template = [(0 if t is None else t * self._gamut_range) for t in template]

            # Generate the boundary points and add to the complete lists.
            points, colors = self.generate_boundary_points(
                space, cast(list[int], template), index, sign
            )
            all_points.extend(points)
            all_colors.extend(colors)

        hr = "-" * (len(f"{space}") + 54)

        self.detail(hr)
        c, h, hex_format = self.to_point_and_color(Color(space, [1.0, 0.0, 0.0]))
        all_points.append((c, h))
        all_colors.append(hex_format)

        trace([1, None, 0], 1, 1) # red to yellow
        self.detail(hr)
        trace([None, 1, 0], 0, -1) # yellow to green
        self.detail(hr)
        trace([0, 1, None], 2, 1) # green to cyan
        self.detail(hr)
        trace([0, None, 1], 1, -1) # cyan to blue
        self.detail(hr)
        trace([None, 0, 1], 0, 1) # blue to magenta
        self.detail(hr)
        trace([1, 0, None], 2, -1) # magenta to red
        self.detail(hr)

        assert close_enough(all_points[0][0], all_points[-1][0])
        assert close_enough(all_points[0][1], all_points[-1][1])
        return all_points, all_colors

    def add_gamut(self, axes: Any, space: ColorSpace) -> None:
        points, colors = self.trace_gamut(space)

        previous_chroma: None | float = None
        previous_hue: None | float = None
        for (chroma, hue), color in zip(points, colors):
            if previous_chroma is not None:
                assert previous_hue is not None
                axes.plot(  # type: ignore
                    [previous_hue, hue],
                    [previous_chroma, chroma],
                    c = color,
                    lw = 1.5,
                )

            previous_chroma = chroma
            previous_hue = hue

    def add_gamut_label(
        self,
        axes: Any,
        space: ColorSpace,
        label: str,
        green: float,
        blue: float,
        dtext: float,
        pad: int,
    ) -> None:
        _, c, h = Color(space, [0.0, green , blue]).to(ColorSpace.Oklch)
        h = h * math.pi / 180
        axes.annotate(  # type: ignore
            label, xy=(h, c), xytext=(h, c + dtext), textcoords="data",
            bbox=dict(
                pad=pad,
                facecolor="none",
                edgecolor="none"
            ),
            arrowprops=dict(
                arrowstyle="-",
                connectionstyle="arc3",
                shrinkA=0,
                shrinkB=0,
            ),
        )

    def trace_spectrum_boundary(self) -> list[tuple[float, float]]:
        points: list[tuple[float, float]] = []

        # FIXME

        return points

    def create_figure(
        self,
        collection_name: None | str = None,
        color_kind: None | str = None,
        gamuts: None | list[ColorSpace] = None,
        spectrum: bool = False,
    ) -> Any:
        fig: Any = plt.figure(layout="constrained", figsize=(5, 6.5))  # type: ignore
        axes: Any = fig.add_subplot(6, 10, (1, 50), polar=True)
        light_axes: Any = fig.add_subplot(6, 10, (52, 59))

        # Add gamut boundaries if so requested.
        gamuts = gamuts or []
        for space in gamuts:
            self.add_gamut(axes, space)

        # Since markers are shared for all marks in a series, we use a new
        # series for every single color.
        for hue, chroma, color, marker in zip(
            self._hues, self._chromas, self._mark_colors, self._marks
        ):
            size = 80 if marker == "o" else 60
            axes.scatter(
                [hue],
                [chroma],
                c=[color],
                s=[size],
                marker=marker,  # type: ignore
                edgecolors='#000',
                zorder=5,
            )

        if self._grays:
            gray = Color.oklab(sum(self._grays) / len(self._grays), 0.0, 0.0).to_hex_format()

            axes.scatter(
                [0],
                [0],
                c=[gray],
                s=[80],
                marker=self._gray_mark,
                edgecolors='#000',
            )

        axes.set_aspect(1)
        axes.set_rmin(0)
        axes.set_rmax(self.effective_max_chroma())

        # Don't show tick labels at angle
        axes.set_rlabel_position(0)

        # If max chroma is 0.3 or 0.4: matplotlib puts grid circles every 0.05.
        #     To reduce clutter and maximize label utility, place label every
        #     0.10, but start at 0.05. Max chroma at 0.5, matplotlib puts grid
        # If max chroma is 0.5: matplotlib puts grid circles every 0.1. Only
        #     suppress labels for origin and max chroma.
        axes.yaxis.set_major_formatter(FuncFormatter(
                self.format_ytick_label_10 if self.effective_max_chroma() == 0.5
                else self.format_ytick_label_05
        ))

        # Center tick labels on tick
        plt.setp(axes.yaxis.get_majorticklabels(), ha="center")  # type: ignore

        # Make grid appear below points
        axes.set_axisbelow(True)

        # Add label for gamut boundaries
        if ColorSpace.Srgb in gamuts:
            self.add_gamut_label(axes, ColorSpace.Srgb, "sRGB", 1.0, 0.85, 0.12, 0)
        if ColorSpace.DisplayP3 in gamuts:
            self.add_gamut_label(axes, ColorSpace.DisplayP3, "P3", 1.0, 0.95, 0.07, 0)
        if ColorSpace.Rec2020 in gamuts:
            self.add_gamut_label(axes, ColorSpace.Rec2020, "Rec. 2020", 0.95, 1.0, 0.18, 1)

        axes.set_title("Hue & Chroma", style="italic", size=13, x=0.11, y=1.01)

        light_axes.set_yticks([0, 0.5, 1], minor=False)
        light_axes.set_yticks([0.25, 0.75], minor=True)
        light_axes.yaxis.grid(True, which="major")
        light_axes.yaxis.grid(True, which="minor")

        light_axes.bar(
            [x for x in range(len(self._lightness))],
            self._lightness,
            color=self._bar_color,
            zorder=5,
        )
        light_axes.set_ylim(0, 1)
        light_axes.get_xaxis().set_visible(False)

        collection_name = collection_name or self._collection_name
        color_kind = color_kind or self._color_kind or "Colors"
        title = f"{collection_name}: " if collection_name else ""
        title += f"{self.format_counts()} {color_kind} in Oklab"
        fig.suptitle(title, ha="left", x=0.044, weight="bold", size=13)
        fig.text(0.045, 0.085, "Lr", fontdict=dict(style="italic", size=13))

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
            for index in [0, 1, 3, 2, 6, 4, 5, 7]:
                color = sampler.resolve(index)
                plotter.add(ThemeEntry.try_from_index(index + 2).name(), color)
                color = sampler.resolve(index + 8)
                plotter.add(ThemeEntry.try_from_index(index + 8 + 2).name(), color)

    for color in [Color.parse("#" + c) for c in cast(list[str], options.colors) or []]:
        plotter.add("<extra>", color, marker="d")

    plotter.stop_adding()

    # ----------------------------------------------------------------------------------
    # Labels and file names

    if options.theme:
        collection_name = "VGA"
    elif options.input:
        collection_name = None
    elif terminal_id:
        collection_name = terminal_id[0]
    else:
        collection_name = "Unknown Terminal"

    if options.input:
        color_kind = "Colors"
    else:
        color_kind = "ANSI Colors"

    if options.output is not None:
        file_name = options.output
    elif options.input is not None:
        file_name = pathlib.Path(options.input).with_suffix(".svg")
    else:
        assert collection_name is not None
        file_name = f'{collection_name.replace(" ", "-").lower()}-colors.svg'

    # ----------------------------------------------------------------------------------
    # Create and save plot

    fig = plotter.create_figure(
        collection_name=collection_name,
        color_kind=color_kind,
        gamuts=gamuts,
        spectrum=options.spectrum,
    )
    plotter.status(f"Saving plot to `{file_name}`")
    fig.savefig(file_name, bbox_inches="tight")  # type: ignore


if __name__ == "__main__":
    main()
