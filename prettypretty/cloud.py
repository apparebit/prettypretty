import argparse
import io
import math
from typing import Self

from prettypretty.color import Color, ColorSpace
from prettypretty.color.gamut import ( # pyright: ignore [reportMissingModuleSource]
    GamutTraversalStep
)
from prettypretty.color.spectrum import ( # pyright: ignore [reportMissingModuleSource]
    CIE_ILLUMINANT_D65, CIE_OBSERVER_2DEG_1931, SpectrumTraversal
)


def create_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--gamut", "-g",
        help="plot boundary of colorspace gamut; value must be sRGB, P3, or Rec2020",
    )
    parser.add_argument(
        "--planar",
        action="store_true",
        help="plot gamut boundary in 2D",
    )
    parser.add_argument(
        "--step", "-s",
        help="use given step size (in integral nanometers)"
    )

    return parser


class Sampler:
    def __init__(self) -> None:
        impossible = Color(ColorSpace.Oklrch, [1.0, math.inf, 0.0])
        black = Color(ColorSpace.Oklrch, [0.0, 0.0, 0.0])
        self.minima = [impossible] * 8
        self.maxima = [black] * 8
        self.achromatic = 0
        self.chromatic = 0

    def sample(self, color: Color) -> None:
        color = color.to(ColorSpace.Oklrch)
        if color.is_achromatic():
            self.achromatic += 1
            return

        self.chromatic += 1
        _, chroma, hue = color
        index = int(hue // 45)
        if chroma < self.minima[index][1]:
            self.minima[index] = color
        elif chroma > self.maxima[index][1]:
            self.maxima[index] = color

    def __str__(self) -> str:
        lines: list[str] = []

        lines.append("Hue bin   Lr      Min C   h      Lr      Max C   h")
        lines.append("------------------------------------------------------")

        for index in range(8):
            mini = self.minima[index]
            maxi = self.maxima[index]

            lines.append(
                f"{index * 45:3}-{(index+1) * 45:3}º  "
                f"{mini[0]:.5f} {mini[1]:.5f} {mini[2]:5.1f}  "
                f"{maxi[0]:.5f} {maxi[1]:.5f} {maxi[2]:5.1f}"
            )

        lines.append("")
        lines.append(
            f"(Based on {self.chromatic:,} samples, "
            f"ignoring {self.achromatic:,} achromatic colors.)")
        return "\n".join(lines)


class PointManager:
    def __init__(self, darken: bool = False, planar: bool = False) -> None:
        self.darken = darken
        self.planar = planar
        self.points: list[tuple[float, float, float]] = []
        self.colors: list[tuple[int, int, int]] = []
        self.line_starts: list[int] = []

    def __len__(self) -> int:
        return len(self.points)

    def add(self, color: Color) -> None:
        x, y, z = color

        if self.planar:
            if color.space().is_ok():
                # Zero out lightness
                self.points.append((0, y, z))
            else:
                # Zero out luminosity
                self.points.append((x, 0, z))
        else:
            self.points.append((x, y, z))

        if self.darken:
            lr, c, h = color.to(ColorSpace.Oklrch)
            color = Color.oklrch(lr * 2 / 3, c * 2 / 3, h)

        r, g, b = color.to_24bit()
        self.colors.append((r, g, b))

    def mark_newline(self) -> None:
        self.line_starts.append(len(self.points))

    def write_header(self, file: io.TextIOWrapper, include: None | Self = None) -> None:
        extra = 0 if include is None else len(include)

        file.write(f"element vertex {len(self.points) + extra}\n")
        file.write("property float x\n")
        file.write("property float y\n")
        file.write("property float z\n")
        file.write("property uchar red\n")
        file.write("property uchar green\n")
        file.write("property uchar blue\n")

        # file.write(f"element face {665}\n")
        # file.write("property list uchar int vertex_indices\n")

    def write_data(self, file: io.TextIOWrapper) -> None:
        for point, color in zip(self.points, self.colors):
            f = 100.0
            x, y, z = point[0] * f, point[1] * f, point[2] * f
            r, g, b = color

            file.write(f"{x:f} {y:f} {z:f} {r} {g} {b}\n")


def render(
    space: ColorSpace,
    segment_size: int = 50,
    gamut: None | ColorSpace = None,
    planar: bool = False,
    sampler: None | Sampler = None,
    step_size: int = 2,
) -> None:
    traversal = SpectrumTraversal(CIE_ILLUMINANT_D65, CIE_OBSERVER_2DEG_1931)
    traversal.set_step_sizes(step_size)

    points = PointManager()
    for step in traversal:
        if isinstance(step, GamutTraversalStep.MoveTo):
            points.mark_newline()

        color = step.color()
        if sampler:
            sampler.sample(color)
        if space.is_ok():
            points.add(color.to(ColorSpace.Oklrab))
        else:
            points.add(color.to(ColorSpace.Xyz))

    gamut_points = PointManager(planar=planar)
    if gamut is not None:
        it = gamut.gamut(segment_size)
        assert it is not None
        for step in it:
            if isinstance(step, GamutTraversalStep.MoveTo):
                gamut_points.mark_newline()

            color = step.color()
            if space.is_ok():
                gamut_points.add(color.to(ColorSpace.Oklrab))
            else:
                gamut_points.add(color.to(ColorSpace.Xyz))

    if space.is_ok():
        filename = "cloud-ok.ply"
    else:
        filename = "cloud-xyz.ply"

    with open(filename, mode="w", encoding="utf8") as file:
        file.write("ply\n")
        file.write("format ascii 1.0\n")
        if gamut is None:
            points.write_header(file)
        else:
            points.write_header(file, gamut_points)
        file.write("end_header\n")

        points.write_data(file)
        if gamut is not None:
            gamut_points.write_data(file)


if __name__ == "__main__":
    parser = create_parser()
    options = parser.parse_args()
    if options.gamut is None:
        gamut = None
    else:
        name = options.gamut.lower()
        if name == "srgb":
            gamut = ColorSpace.Srgb
        elif name == "p3":
            gamut = ColorSpace.DisplayP3
        elif name == "rec2020":
            gamut = ColorSpace.Rec2020
        else:
            raise ValueError(f"{name} is not a valid gamut name")

    step_size = 2
    if options.step:
        step_size = int(options.step)
    if step_size <= 0:
        raise ValueError(f"{step_size} is not a valid step size")

    sampler = Sampler()

    render(
        ColorSpace.Xyz,
        gamut=gamut,
        planar=options.planar,
        step_size=step_size
    )
    render(
        ColorSpace.Oklrab,
        gamut=gamut,
        planar=options.planar,
        sampler=sampler,
        step_size=step_size
    )

    print("Shape of visible gamut (sampled on chroma/hue plane):\n")
    print(sampler)

