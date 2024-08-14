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
from collections.abc import Sequence
import math
import pathlib
from typing import Any, cast

from .terminal import Terminal
from .color import (
    close_enough,
    Color,
    ColorSpace,
    gamut, # pyright: ignore [reportMissingModuleSource]
    spectrum, # pyright: ignore [reportMissingModuleSource]
    trans, # pyright: ignore [reportMissingModuleSource]
)
from .theme import current_translator


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
        const=trans.VGA_COLORS,
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
        "--strong-gamut",
        action="store_true",
        help="use darker, more intense colors for gamut boundaries",
    )
    parser.add_argument(
        "--spectrum",
        action="store_true",
        help="also plot boundary of visible spectrum (experimental)",
    )
    parser.add_argument(
        "--chromaticity",
        action="store_true",
        help="create xy chromaticity diagram"
    )
    parser.add_argument(
        "--illuminant",
        action="store_true",
        help="scale color matching function by D65 illuminant",
    )
    parser.add_argument(
        "-o", "--output",
        help="write color plot to the named file"
    )
    return parser


class ColorPlotter:
    ACHROMATIC_THRESHOLD = 0.05

    def __init__(
        self,
        collection_name: None | str = None,
        color_kind: None | str = None,
        segment_size: None | int = None,
        strong_gamut: bool = False,
        chromaticity: bool = False,
        volume: int = 1,
    ) -> None:
        self._collection_name = collection_name
        self._color_kind = color_kind

        # Scattered colors
        self._xs: list[float] = []
        self._ys: list[float] = []
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
        self._segment_size = segment_size or 20
        self._largest_gamut: None | ColorSpace = None
        self._strong_gamut = strong_gamut
        self._with_spectrum = False
        self._chromaticity = chromaticity

        self._volume = volume

    def status(self, msg: str) -> None:
        if self._volume >= 1:
            print(msg)

    def detail(self, msg: str) -> None:
        if self._volume >= 2:
            print(msg)

    def start_adding(self) -> None:
        self.status("                Color    L        Chroma   Hue    Dup")
        self.status("-----------------------------------------------------")

    def add(self, name: str, color: Color, marker: str = "o") -> None:
        # Matplotlib is sRGB only
        hex_color = color.to_hex_format()

        # Convert to Oklrch
        oklrch = color.to(ColorSpace.Oklrch)
        lr, c, h = oklrch
        c = round(c, 14)  # Chop off one digit of precision.

        # Detect duplicates
        dup = " ✘ " if oklrch in self._colors else " - "

        # Update status
        light = f'{lr:.5}'
        if len(light) > 7:
            light = f'{lr:.5f}'
        chroma = f'{c:.5}'
        if len(chroma) > 7:
            chroma = f'{c:.5f}'
        hue = f'{h:.1f}'
        self.status(f"{name:14}  {hex_color}  {light:<7}  {chroma:<7}  {hue:>5}  {dup}")

        # Skip duplicates
        if oklrch in self._colors:
            self._duplicate_count += 1
            return

        # Display lightness for *all* non-duplicate colors
        self._lightness.append(lr)
        self._bar_color.append(hex_color)

        # Handle grays
        if oklrch.is_achromatic_threshold(ColorPlotter.ACHROMATIC_THRESHOLD):
            self._grays.append(lr)
            if self._gray_mark is None:
                self._gray_mark = marker
            elif marker != self._gray_mark:
                raise ValueError(
                    f"inconsistent markers for achromatic color: {marker} vs {self._gray_mark}"
                )
            return

        # Add hue/chroma/mark and update counts accordingly
        x, y = self.to_2d(color)
        self._xs.append(x)
        self._ys.append(y)
        self._marks.append(marker)
        self._mark_colors.append(hex_color)
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

    def trace_spectrum(
        self,
        axes: Any,
        with_pulses: bool = True,
        with_illuminant: bool = False,
    ) -> None:
        self._with_spectrum = True

        sample_count = len(spectrum.CIE_OBSERVER_2DEG_1931)
        pulse_increment = 11
        line_count = 44 if with_pulses else 1
        max_width = sample_count if with_pulses else 1

        markwaves = {380, 460, 480, 490, 500, 520, 540, 560, 580, 600, 700}
        marks: dict[int, tuple[float, float]] = {}

        all_points: list[list[float]] = [[], [], []]
        all_series: list[list[tuple[float, float]]] = [list() for _ in range(line_count)]
        for index in range(sample_count):
            total_xyz = [0.0, 0.0, 0.0]
            series_index = 0

            wave = spectrum.CIE_OBSERVER_2DEG_1931.start() + index

            for width in range(max_width):
                xyz = spectrum.CIE_OBSERVER_2DEG_1931[index + width]

                if with_illuminant:
                    d65 = spectrum.CIE_ILLUMINANT_D65[index + width]
                    for c in range(3):
                        total_xyz[c] += d65 * xyz[c]
                else:
                    for c in range(3):
                        total_xyz[c] += xyz[c]

                emit_point = width % pulse_increment == 0 or width == sample_count - 1
                emit_mark = self._chromaticity and not with_pulses and wave in markwaves
                if emit_point or emit_mark:
                    luminance = spectrum.CIE_OBSERVER_2DEG_1931.weight()
                    coordinates = [
                        total_xyz[0] / luminance,
                        total_xyz[1] / luminance,
                        total_xyz[2] / luminance,
                    ]
                    two_dee = self.to_2d(Color(ColorSpace.Xyz, coordinates))

                    if emit_point:
                        all_points[0].append(coordinates[0])
                        all_points[1].append(coordinates[1])
                        all_points[2].append(coordinates[2])

                        all_series[series_index].append(two_dee)
                        series_index += 1

                    if emit_mark:
                        marks[wave] = two_dee

        if with_pulses:
            f3d: Any
            a3d: Any
            f3d, a3d = plt.subplots(subplot_kw={"projection": "3d"})  # type: ignore
            a3d.scatter(*all_points)
            f3d.savefig("spectrum-3d.svg")
            plt.close(f3d) # type: ignore
            #plt.show(f3d) # type: ignore

        color = "#bbb" if with_pulses else "#000"
        width = 1.0 if with_pulses else 2.0
        for series in all_series:
            self.add_line(axes, series, color = color, width = width)

        if marks:
            xs = [x for x, _ in marks.values()]
            ys = [y for _, y in marks.values()]
            axes.scatter(xs, ys, c="#f00")

    def trace_gamut(self, space: ColorSpace, axes: Any) -> None:
        """Trace the boundary of the gamut for the given color space."""
        if (
            self._largest_gamut is None
            or space is ColorSpace.Rec2020
            or space is ColorSpace.DisplayP3 and self._largest_gamut is ColorSpace.Srgb
        ):
            self._largest_gamut = space

        all_points: list[tuple[float, float]]= []
        all_colors: list[str] = []

        iter = space.gamut(self._segment_size)
        assert iter is not None
        for step in iter:
            color = step.color()

            _, c, h = color.to(ColorSpace.Oklrch)
            if self._strong_gamut:
                hex_format = Color(ColorSpace.Oklrch, [0.6, c, h]).to_hex_format()
            else:
                hex_format = Color(ColorSpace.Oklrch, [0.75, c / 3, h]).to_hex_format()

            pt = self.to_2d(color)

            self.detail(
                f"{color.space()} gamut: {color[0]:.2f}, {color[1]:.2f}, {color[2]:.2f} --> "
                f"{self.format_2d(*pt)} ({hex_format})"
            )

            all_points.append(pt)
            all_colors.append(hex_format)

            if isinstance(step, gamut.GamutTraversalStep.CloseWith):
                break

        assert close_enough(all_points[0][0], all_points[-1][0])
        assert close_enough(all_points[0][1], all_points[-1][1])

        self.add_line(axes, all_points, all_colors)

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
        x, y = self.to_2d(Color(space, [0.0, green, blue]))
        axes.annotate(  # type: ignore
            label, xy=(x, y), xytext=(x, y + dtext), textcoords="data",
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

    def add_line(
        self,
        axes: Any,
        points: Sequence[tuple[float, float]],
        color: str | Sequence[str],
        width: float = 1.5,
    ) -> None:
        should_index_color = not isinstance(color, str)

        x0: None | float = None
        y0: None | float = None
        for index, (x1, y1) in enumerate(points):
            if x0 is not None:
                assert y0 is not None
                c = color[index] if should_index_color else color

                axes.plot(
                    [x0, x1],
                    [y0, y1],
                    c = c,
                    lw = width,
                )

            x0 = x1
            y0 = y1

    def to_2d(self, color: Color) -> tuple[float, float]:
        return color.xy_chromaticity() if self._chromaticity else color.hue_chroma()

    def format_2d(self, x: float, y: float) -> str:
        if not self._chromaticity:
            return f'h={x:.5f}  c={y:.5f}'
        else:
            return f'x={x:.5f}  y={y:.5f}'

    def create_figure(
        self,
        collection_name: None | str = None,
        color_kind: None | str = None,
        gamuts: None | list[ColorSpace] = None,
        spectrum: bool = False,
        with_illuminant: bool = False,
    ) -> Any:
        if self._chromaticity:
            fig: Any = plt.figure(figsize=(5, 5.5)) # type: ignore
            axes: Any = fig.add_subplot()
            light_axes = None
        else:
            fig: Any = plt.figure(layout="constrained", figsize=(5, 6.5))  # type: ignore
            axes: Any = fig.add_subplot(6, 10, (1, 50), polar=True)
            light_axes: Any = fig.add_subplot(6, 10, (52, 59))

        # Add spectrum and gamut boundaries if so requested.
        if spectrum:
            self.trace_spectrum(axes, with_illuminant=with_illuminant)
        if self._chromaticity:
            self.trace_spectrum(axes, with_pulses=False)

        gamuts = gamuts or []
        for space in gamuts:
            self.trace_gamut(space, axes)

        # Since markers are shared for all marks in a series, we use a new
        # series for every single color.
        for x, y, color, marker in zip(
            self._xs, self._ys, self._mark_colors, self._marks
        ):
            size = 80 if marker == "o" else 60
            axes.scatter(
                [x],
                [y],
                c=[color],
                s=[size],
                marker=marker,  # type: ignore
                edgecolors='#000',
                zorder=5,
            )

        if self._grays and not self._chromaticity:
            gray = Color.oklab(sum(self._grays) / len(self._grays), 0.0, 0.0).to_hex_format()

            axes.scatter(
                [0],
                [0],
                c=[gray],
                s=[80],
                marker=self._gray_mark,
                edgecolors='#000',
            )

        if self._chromaticity:
            axes.set_xlim([0, 0.8])
            axes.set_ylim([0, 0.9])
            axes.grid()

        else:
            axes.set_aspect(1)
            axes.set_rmin(0)
            axes.set_rmax(self.effective_max_chroma())

            # Don't show tick labels at angle
            axes.set_rlabel_position(0)

            # When max chroma is 0.3 or 0.4, matplotlib generates grid circles every
            # 0.05 units and, for larger max chroma, every 0.1 units. Always
            # generate labels every 0.1 units, but shift them by 0.05 in the former
            # case. Never generate labels for origin or max chroma.
            axes.yaxis.set_major_formatter(FuncFormatter(
                    self.format_ytick_label_10 if self.effective_max_chroma() >= 0.5
                    else self.format_ytick_label_05
            ))

            # Center tick labels on tick
            plt.setp(axes.yaxis.get_majorticklabels(), ha="center")  # type: ignore

        # Make grid appear below points
        axes.set_axisbelow(True)

        if not self._chromaticity:
            # Add label for gamut boundaries
            if ColorSpace.Srgb in gamuts:
                self.add_gamut_label(axes, ColorSpace.Srgb, "sRGB", 1.0, 0.85, 0.12, 0)
            if ColorSpace.DisplayP3 in gamuts:
                self.add_gamut_label(axes, ColorSpace.DisplayP3, "P3", 1.0, 0.95, 0.07, 0)
            if ColorSpace.Rec2020 in gamuts:
                self.add_gamut_label(axes, ColorSpace.Rec2020, "Rec. 2020", 0.95, 1.0, 0.18, 1)

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
        title += f"{self.format_counts()} {color_kind}"
        title += "" if self._chromaticity else " in Oklab"
        if self._chromaticity:
            fig.suptitle("CIE 1931 xy Chromaticity", weight="bold", size=13)
            axes.set_title(title, style="italic", size=13)
        else:
            fig.suptitle(title, ha="left", x=0.044, weight="bold", size=13)
            axes.set_title("Hue & Chroma", style="italic", size=13, x=0.11, y=1.01)
            fig.text(0.045, 0.085, "Lr", fontdict=dict(style="italic", size=13))

        return fig

    def effective_max_chroma(self) -> float:
        if self._with_spectrum:
            return 0.6
        elif self._largest_gamut in (ColorSpace.Srgb, ColorSpace.DisplayP3):
            return 0.4
        elif self._largest_gamut is ColorSpace.Rec2020:
            return 0.5
        elif all(c < 0.3 for c in self._ys):
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


def main() -> None:
    options = create_parser().parse_args()
    plotter = ColorPlotter(
        volume=1-options.quiet+options.verbose,
        strong_gamut=options.strong_gamut,
        chromaticity=options.chromaticity,
    )
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

            translator = current_translator()
            for index in [0, 1, 3, 2, 6, 4, 5, 7]:
                color = translator.resolve(index)
                plotter.add(trans.ThemeEntry.try_from_index(index + 2).name(), color)
                color = translator.resolve(index + 8)
                plotter.add(trans.ThemeEntry.try_from_index(index + 8 + 2).name(), color)

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
        with_illuminant=options.illuminant,
    )
    plotter.status(f"Saving plot to `{file_name}`")
    fig.savefig(file_name, bbox_inches="tight")  # type: ignore


if __name__ == "__main__":
    main()
