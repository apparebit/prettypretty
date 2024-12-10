"""
Making sense of ANSI colors.
"""
import sys

try:
    import matplotlib.pyplot as plt
    from matplotlib.ticker import FixedFormatter, FixedLocator, FuncFormatter
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

from .color import (
    close_enough,
    Color,
    ColorSpace,
    spectrum, # pyright: ignore [reportMissingModuleSource]
    theme, # pyright: ignore [reportMissingModuleSource]
)
from .color.gamut import GamutTraversalStep # pyright: ignore [reportMissingModuleSource]
from .progress import add_fidelity, ProgressBar
from .terminal import Terminal
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
        """,
        epilog="""
            Plot accepts a subset of CSS color syntax, including hash
            hexadecimal notation (#123abc) as well as functional notation. For
            the latter, it recognizes "color(<space> <c1> <c2> <c3>)",
            "oklab(<l> <a> <b>)", and "oklch(<l> <c> <h>)". Valid color space
            names are sRGB, linear-sRGB, Display-P3, --linear-Display-P3,
            Rec2020, --linear-Rec2020, --Oklrab, --Oklrch, XYZ, and XYZ-D65 (all
            case-insensitive).
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
        const=theme.VGA_COLORS,
        dest="theme",
        help="use VGA colors instead of querying terminal"
    )
    parser.add_argument(
        "-c", "--color",
        action="append",
        dest="colors",
        help="also plot color specified in CSS syntax"
    )

    inputs = parser.add_mutually_exclusive_group()
    inputs.add_argument(
        "-i", "--input",
        help="read newline-separated colors in CSS syntax from named file",
    )
    inputs.add_argument(
        "--oktriples",
        help=(
            "read newline-separated Oklab colors "
            "as comma-separated floating point numbers from named file"
        ),
    )

    parser.add_argument(
        "--no-light",
        action="store_true",
        help="don't include bar chart for lightness",
    )
    parser.add_argument(
        "--no-term",
        action="store_true",
        help=(
            "don't render terminal colors, "
            "only those specified with --color, --input, --oktriples"
        ),
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
        "-o", "--output",
        help="write color plot to the named file"
    )
    return parser


def ingest_oktriples(file: str) -> list[Color]:
    """
    Ingest the file with floating point triplets. Unless the line is empty or
    starts with a hash, it must contain three comma-separated floating point
    values.
    """
    colors: list[Color] = []

    with open(file, mode="r", encoding="utf8") as fd:
        for line in fd:
            line = line.strip()
            if line == "" or line.startswith("#"):
                continue

            color = Color(
                ColorSpace.Oklab,
                [float(num.strip()) for num in line.split(",")]
            )
            colors.append(color)

    return colors


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
        progress: None | ProgressBar = None,
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
        self._gray_mark = "o"

        # Lightness bars
        self._lightness: list[float] = []
        self._bar_color: list[str] = []
        self._bar_label: list[None | str] = []

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

        # Spectrum trace
        self._illuminated_observer: None | spectrum.IlluminatedObserver = None
        self._white_point: None | Color = None

        self._volume = volume
        self._progress = progress if 1 <= volume else None
        print(self._progress)

    def status(self, msg: str) -> None:
        if self._volume >= 1:
            print(msg)

    def detail(self, msg: str) -> None:
        if self._volume >= 2:
            print(msg)

    # ----------------------------------------------------------------------------------
    # Individual Color Markers

    def start_adding(self) -> None:
        self.status("                Color    L        Chroma   Hue    Dup")
        self.status("-----------------------------------------------------")

    def add(
        self, name: str, color: Color, marker: str = "o", label: None | str = None
    ) -> None:
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
        self._bar_label.append(label)

        # Handle grays
        if oklrch.is_achromatic_threshold(ColorPlotter.ACHROMATIC_THRESHOLD):
            self._grays.append(lr)
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
            f"\nTotal: {self._base_count}+{self._extra_count} chromatic colors, "
            f"{len(self._grays)} achromatic colors, "
            f"and {self._duplicate_count} duplicates"
        )

    @property
    def total_count(self) -> int:
        return self._base_count + self._extra_count + len(self._grays)

    def format_counts(self) -> str:
        counts = f"{self._base_count + self._extra_count}"
        grays = len(self._grays)
        if grays > 0:
            counts += f"+{grays}"
        return counts

    # ----------------------------------------------------------------------------------
    # Gamut Tracing

    def trace_spectrum(
        self,
        axes: Any,
        locus_only: bool = False
    ) -> None:
        if self._illuminated_observer is None:
            self._illuminated_observer = spectrum.IlluminatedObserver(
                spectrum.CIE_ILLUMINANT_D65,
                spectrum.CIE_OBSERVER_2DEG_1931,
            )
            self._white_point = self._illuminated_observer.white_point()

        points: tuple[list[float], list[float], list[float], list[str]] = [], [], [], []
        lines2d: list[list[tuple[float, float]]] = []
        all_colors: list[list[str]] = []

        iterator = self._illuminated_observer.visual_gamut(3)
        total_steps = len(iterator)

        self.status(f"Sampling human visual gamut at {total_steps:,} points")
        if self._progress is not None:
            self.status("")

        for index, step in enumerate(iterator):
            if self._progress is not None:
                self._progress.render(index / total_steps * 100)

            if isinstance(step, GamutTraversalStep.MoveTo):
                lines2d.append([])
                all_colors.append([])
                if locus_only and len(lines2d) > 1:
                    break
            c = step.color()
            points[0].append(c[0])
            points[1].append(c[1])
            points[2].append(c[2])
            points[3].append(c.to_hex_format())
            lines2d[-1].append(self.to_2d(c))
            all_colors[-1].append(c.to_hex_format())

        if self._progress is not None:
            self._progress.done()

        if not locus_only:
            f, a = plt.subplots(subplot_kw={"projection": "3d"})  # type: ignore
            a.scatter(points[0], points[1], points[2], c=points[3])  # type: ignore
            f.show()
            #f.savefig("spectrum-3d.svg") # type: ignore
            #plt.close(f) # type: ignore

        total_lines = len(lines2d)

        self.status(f"Connecting gamut samples into {total_lines} lines")
        if self._progress is not None:
            self.status("")

        for index, (line, colors) in enumerate(zip(lines2d, all_colors)):
            if self._progress is not None:
                self._progress.render(index / total_lines * 100)

            self.add_line(
                axes,
                line,
                color=colors, #'#000' if locus_only else '#ccc',
                width=2 if locus_only else 0.8,
            )
        if self._progress is not None:
            self._progress.done()

    def trace_gamut(self, space: ColorSpace, axes: Any) -> None:
        """Trace the boundary of the gamut for the given color space."""
        self.status(f"Tracing {space} gamut")

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

            if isinstance(step, GamutTraversalStep.CloseWith):
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

    # ----------------------------------------------------------------------------------
    # Figure Creation

    def create_figure(
        self,
        collection_name: None | str = None,
        color_kind: None | str = None,
        gamuts: None | list[ColorSpace] = None,
        with_spectrum: bool = False,
        with_lightness: bool = True,
    ) -> Any:
        self.status("Creating figure")

        if self._chromaticity:
            fig: Any = plt.figure(figsize=(5, 5.5)) # type: ignore
            axes: Any = fig.add_subplot()
            light_axes: Any = None
        elif with_lightness:
            fig = plt.figure(layout="constrained", figsize=(5, 6.5))  # type: ignore
            axes = fig.add_subplot(6, 10, (1, 50), polar=True)
            light_axes = fig.add_subplot(6, 10, (51, 60))
        else:
            fig = plt.figure(layout="constrained", figsize=(5, 5.5)) # type: ignore
            axes = fig.add_subplot(polar=True)
            light_axes = None

        # Add spectrum and gamut boundaries if so requested.
        # if with_spectrum or self._chromaticity:
        #     self.premultiply(spectrum.CIE_OBSERVER_2DEG_1931, spectrum.CIE_ILLUMINANT_D65)
        if with_spectrum:
            self._with_spectrum = with_spectrum
            self.trace_spectrum(axes)
        if self._chromaticity:
            self.trace_spectrum(axes, locus_only=True)

            # Add white point
            assert self._white_point is not None
            x, y = self.to_2d(self._white_point)
            self._xs.append(x)
            self._ys.append(y)
            self._marks.append("*")
            self._mark_colors.append(self._white_point.to_hex_format())

        gamuts = gamuts or []
        for space in gamuts:
            self.trace_gamut(space, axes)

        # Since markers are shared for all marks in a series, we use a new
        # series for every single color.
        total_markers = len(self._xs)

        self.status(f"Setting {total_markers:,} chromatic markers")
        if self._progress is not None and 100 <= total_markers:
            self.status("")

        for index, (x, y, color, marker) in enumerate(zip(
            self._xs, self._ys, self._mark_colors, self._marks
        )):
            if self._progress is not None and 100 <= total_markers:
                self._progress.render(index / total_markers * 100)

            if marker == "o":
                size = 80
            elif marker == "*":
                size = 120
            else:
                size = 60
            axes.scatter(
                [x],
                [y],
                c=[color],
                s=[size],
                marker=marker,  # type: ignore
                edgecolors='#000',
                zorder=5,
            )
        if self._progress is not None and 100 <= total_markers:
            self._progress.done()

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

        lry_offset = 0
        if not self._chromaticity:
            # Add label for gamut boundaries
            if ColorSpace.Srgb in gamuts:
                self.add_gamut_label(axes, ColorSpace.Srgb, "sRGB", 1.0, 0.85, 0.12, 0)
            if ColorSpace.DisplayP3 in gamuts:
                self.add_gamut_label(axes, ColorSpace.DisplayP3, "P3", 1.0, 0.95, 0.07, 0)
            if ColorSpace.Rec2020 in gamuts:
                self.add_gamut_label(axes, ColorSpace.Rec2020, "Rec. 2020", 0.95, 1.0, 0.18, 1)

        if not self._chromaticity and with_lightness:
            light_axes.set_yticks([0, 0.5, 1], minor=False)
            light_axes.set_yticks([0.25, 0.75], minor=True)
            light_axes.yaxis.grid(True, which="major")
            light_axes.yaxis.grid(True, which="minor")

            light_axes.bar(
                [x for x in range(len(self._lightness))],
                self._lightness,
                color=self._bar_color,
                zorder=5,
                edgecolor="#000",
                linewidth=0.5,
            )

            light_axes.set_ylim(0, 1)
            light_axes.margins(x=0.02, tight=True)

            if all(label is not None for label in self._bar_label):
                lry_offset = 0.015
                light_axes.xaxis.set_major_locator(
                    FixedLocator([*range(len(self._bar_label))])
                )
                light_axes.xaxis.set_major_formatter(
                    FixedFormatter(self._bar_label) # pyright: ignore [reportArgumentType]
                )
            else:
                light_axes.get_xaxis().set_visible(False)

        collection_name = collection_name or self._collection_name
        color_kind = color_kind or self._color_kind

        title = f"{collection_name}: " if collection_name else ""
        title += f"{self.format_counts()} "
        if color_kind:
            title += f"{color_kind} "
        title += "Color"
        if self.total_count != 1 or self._base_count != 1:
            title += "s"

        title += "" if self._chromaticity else " in Oklab"
        if self._chromaticity:
            fig.suptitle("CIE 1931 xy Chromaticity", weight="bold", size=13)
            axes.set_title(title, style="italic", size=13)
        else:
            fig.suptitle(title, ha="left", x=0.044, weight="bold", size=13)
            axes.set_title("Hue & Chroma", style="italic", size=13, x=0.11, y=1.01)
            if with_lightness:
                fig.text(
                    0.09, 0.185 + lry_offset,
                    "Lightness (Lr)",
                    fontdict=dict(style="italic", size=13)
                )

        return fig

    def effective_max_chroma(self) -> float:
        if self._with_spectrum:
            return 0.5
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

# ======================================================================================

def main(options: Any, term: Terminal) -> None:
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

    plotter = ColorPlotter(
        volume=1-options.quiet+options.verbose,
        strong_gamut=options.strong_gamut,
        chromaticity=options.chromaticity,
        progress=ProgressBar(term),
    )
    terminal_id = None

    plotter.start_adding()

    if not options.no_term:
        if not options.theme:
            terminal_id = term.request_terminal_identity()

        translator = current_translator()
        for base_index in [0, 1, 3, 2, 6, 4, 5, 7]:
            for index in [base_index, base_index + 8]:
                color = translator.resolve(index)
                name = theme.ThemeEntry.try_from_index(index + 2).name()
                label = theme.ThemeEntry.try_from_index(index + 2).abbr()
                plotter.add(name, color, label=label)

    cname = "" if options.no_term else "<extra>"
    marker = "o" if options.no_term else "d"

    if options.input is not None:
        with open(options.input, mode="r", encoding="utf8") as file:
            for color in [Color.parse(line) for line in file.readlines() if line.strip()]:
                plotter.add(cname, color, marker=marker)
    elif options.oktriples is not None:
        for color in ingest_oktriples(options.oktriples):
            plotter.add(cname, color, marker=".")

    for color in [Color.parse(c) for c in cast(list[str], options.colors) or []]:
        plotter.add(cname, color, marker=marker)

    plotter.stop_adding()

    # ----------------------------------------------------------------------------------
    # Labels and file names

    if options.no_term:
        collection_name = None
    elif options.theme:
        collection_name = "VGA"
    elif terminal_id:
        collection_name = terminal_id[0]
    else:
        collection_name = "Unknown Terminal"

    if options.no_term:
        color_kind = None
    else:
        color_kind = "ANSI"

    if options.output is not None:
        file_name = options.output
    elif options.input is not None:
        file_name = pathlib.Path(options.input).with_suffix(".svg")
    elif options.oktriples is not None:
        file_name = pathlib.Path(options.oktriples).with_suffix(".svg")
    else:
        assert collection_name is not None
        file_name = f'{collection_name.replace(" ", "-").lower()}-colors.svg'

    # ----------------------------------------------------------------------------------
    # Create and save plot

    fig = plotter.create_figure(
        collection_name=collection_name,
        color_kind=color_kind,
        gamuts=gamuts,
        with_spectrum=options.spectrum,
        with_lightness=not options.no_light
    )
    plotter.status(f"Saving plot to `{file_name}`")
    fig.savefig(file_name, bbox_inches="tight")  # type: ignore
    plotter.status("Happy, happy, joy, joy!")


if __name__ == "__main__":
    options = add_fidelity(create_parser()).parse_args()
    with (
        Terminal(options.fidelity)
        .cbreak_mode()
        .terminal_theme(options.theme)
        .hidden_cursor()
        .scoped_style()
    ) as term:
        main(options, term)
