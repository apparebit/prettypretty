import argparse
import io
import math
import sys
from typing import Callable, Self

from prettypretty.color import Color, ColorSpace
from prettypretty.color.gamut import ( # pyright: ignore [reportMissingModuleSource]
    GamutTraversalStep
)
from prettypretty.color.spectrum import ( # pyright: ignore [reportMissingModuleSource]
    CIE_ILLUMINANT_D65, CIE_OBSERVER_2DEG_1931, SpectrumTraversal
)


def create_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="generate a point cloud for the visual gamut in XYZ as well as "
        "Oklrab and store the data in 'cloud-xyz.ply' and 'cloud-ok.ply' files"
    )
    parser.add_argument(
        "--gamut", "-g",
        help="plot boundary of gamut for 'sRGB', 'P3', or 'Rec2020' color space "
        "(with case-insensitive name matching)",
    )
    parser.add_argument(
        "--planar",
        action="store_true",
        help="plot gamut boundary X/Z plane for XYZ and a/b plane for Oklrab, "
        "with Y=0 or Lr= 0, respectively"
    )
    parser.add_argument(
        "--quad",
        action="store_true",
        help="include quadrilaterals with vertex data; quads are formed by connecting "
        "the nth pulse of each line with the nth pulse of the next line, much like "
        "a rope ladder"
    )
    parser.add_argument(
        "--step", "-s",
        help="use given step size (in integral nanometers)"
    )

    return parser


log: Callable[[str], None] = lambda msg: print(msg, file=sys.stderr)


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
    def __init__(
        self,
        darken: bool = False,
        planar: bool = False,
        quad: bool = False,
    ) -> None:
        self.darken = darken
        self.planar = planar
        self.quad = quad
        self.points: list[tuple[float, float, float]] = []
        self.colors: list[tuple[int, int, int]] = []
        self.line_starts: list[int] = []

    def __len__(self) -> int:
        return len(self.points)

    @property
    def line_length(self) -> int:
        return self.line_starts[1]

    @property
    def line_count(self) -> int:
        return len(self.line_starts) - 1

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

    def write_header(
        self,
        file: io.TextIOWrapper,
        include: None | Self = None
    ) -> tuple[int, int]:
        file.write("ply\n")
        file.write("format ascii 1.0\n")

        vertex_count = len(self.points) + (0 if include is None else len(include))
        file.write(f"element vertex {vertex_count}\n")

        file.write("property float x\n")
        file.write("property float y\n")
        file.write("property float z\n")
        file.write("property uchar red\n")
        file.write("property uchar green\n")
        file.write("property uchar blue\n")

        face_count = 0
        if self.quad:
            face_count = (self.line_length - 1) * (self.line_count - 1)
            file.write(f"element face {face_count}\n")
            file.write("property list uchar int vertex_indices\n")

        file.write("end_header\n")
        return vertex_count, face_count

    def write_vertex_data(self, file: io.TextIOWrapper) -> None:
        for point, color in zip(self.points, self.colors):
            f = 100.0
            x, y, z = point[0] * f, point[1] * f, point[2] * f
            r, g, b = color

            file.write(f"{x:f} {y:f} {z:f} {r} {g} {b}\n")

    def write_face_data(self, file: io.TextIOWrapper) -> None:
        for line_count in range(1, self.line_count):
            for point_count in range(1, self.line_length):
                p = (line_count - 1) * self.line_length + point_count - 1
                q = p + 1
                r = q + self.line_length
                s = r - 1

                file.write(f"4 {p} {q} {r} {s}\n")


def render(
    space: ColorSpace,
    segment_size: int = 50,
    gamut: None | ColorSpace = None,
    planar: bool = False,
    quad: bool = False,
    sampler: None | Sampler = None,
    step_size: int = 2,
) -> None:
    traversal = SpectrumTraversal(CIE_ILLUMINANT_D65, CIE_OBSERVER_2DEG_1931)
    traversal.set_step_sizes(step_size)

    log(f"Traversing visual gamut in {space}")
    points = PointManager(quad=quad)
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

    gamut_points = None
    if gamut is not None:
        log(f"Traversing color space gamut of {space}")
        gamut_points = PointManager(planar=planar)
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

    log(f"Writing {filename}")
    with open(filename, mode="w", encoding="utf8") as file:
        vertex_count, face_count = points.write_header(file, gamut_points)
        log(f"Writing {filename}: {vertex_count:,} vertices")
        points.write_vertex_data(file)
        if gamut_points is not None:
            gamut_points.write_vertex_data(file)

        if quad:
            log(f"Writing {filename}: {face_count:,} faces")
            points.write_face_data(file)


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
        quad=options.quad,
        step_size=step_size
    )
    render(
        ColorSpace.Oklrab,
        gamut=gamut,
        planar=options.planar,
        quad=options.quad,
        sampler=sampler,
        step_size=step_size
    )

    log("\nShape of visible gamut (sampled on chroma/hue plane):\n")
    log(str(sampler))

