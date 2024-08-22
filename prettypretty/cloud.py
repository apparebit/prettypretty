import argparse
import io
import math
import sys
from typing import Self

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
        "--planar-gamut",
        action="store_true",
        help="plot gamut boundary X/Z plane for XYZ and a/b plane for Oklrab, "
        "with Y=0 or Lr= 0, respectively"
    )
    parser.add_argument(
        "--mesh",
        action="store_true",
        help="include face mesh with vertex data; the mesh is formed by connecting "
        "the nth starting wavelength of the mth pulse width with the nth starting "
        "wavelength of the (m-1)th pulse width, much like a rope ladder"
    )
    parser.add_argument(
        "--step", "-s",
        help="use given step size (in integral nanometers)"
    )

    return parser


def log(msg: str = "") -> None:
    print(msg, file=sys.stderr)


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

        chromatic = f"{self.chromatic:9,}"
        chromatic_label = "chromatic".rjust(len(chromatic))
        achromatic = f"{self.achromatic:10,}"
        achromatic_label = "achromatic".rjust(len(achromatic))
        total = f"{self.chromatic + self.achromatic:5,}"
        total_label = "total".rjust(len(total))

        lines.append(f"{total} = {chromatic} + {achromatic} samples")
        lines.append(f"{total_label} = {chromatic_label} + {achromatic_label}")

        return "\n".join(lines)


class PointManager:
    def __init__(
        self,
        darken: bool = False,
        planar: bool = False,
        mesh: bool = False,
    ) -> None:
        self.darken = darken
        self.planar = planar
        self.mesh = mesh
        self.points: list[tuple[float, float, float]] = []
        self.colors: list[tuple[int, int, int]] = []
        self.line_count = 0

    def __len__(self) -> int:
        return len(self.points)

    def mark_newline(self) -> None:
        self.line_count += 1
        if self.line_count == 2:
            self.line_length = len(self.points)

    def add(self, color: Color, highlight: None | Color = None) -> None:
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

        if highlight is not None:
            r, g, b = highlight.to_24bit()
            self.colors.append((r, g, b))
            return

        if self.darken:
            lr, c, h = color.to(ColorSpace.Oklrch)
            color = Color.oklrch(lr * 2 / 3, c * 2 / 3, h)

        r, g, b = color.to_24bit()
        self.colors.append((r, g, b))

    def write_header(
        self,
        file: io.TextIOWrapper,
        *,
        include: None | Self = None,
        extra_faces: int = 0,
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
        if self.mesh:
            face_count = (self.line_length - 1) * (self.line_count - 1) + extra_faces
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
        for line_index in range(self.line_count - 1):
            for index in range(0, self.line_length - 1):
                s = line_index * self.line_length + index
                t = s + 1
                u = t + self.line_length
                v = u - 1

                file.write(f"4 {s} {t} {u} {v}\n")

    def write_face_patch(self, file: io.TextIOWrapper, base: int) -> int:
        face_count = 0

        cusp = self.line_length * 32 // 100
        if cusp % 2 == 0:
            cusp += 1
        halfcusp = cusp // 2  # Rounds downward

        for index in range(1, halfcusp + 1):
            s = base + index - 1
            t = s + 1
            u = base + cusp - index
            v = u + 1

            face_count += 1
            file.write(f"4 {s} {t} {u} {v}\n")

        for index in range(cusp, self.line_length - 1):
            s = base
            t = s + index
            u = t + 1

            face_count += 1
            file.write(f"3 {s} {t} {u}\n")

        return face_count

    def write_all(
        self,
        file: io.TextIOWrapper,
        include: None | Self = None,
    ) -> None:
        buffer: None | io.StringIO = None
        extra_faces = 0

        # TODO: Pre-compute face count to avoid buffering
        if self.mesh:
            buffer = io.StringIO()
            extra_faces = self.write_face_patch(buffer, 0)
            base = (self.line_count - 1) * self.line_length
            extra_faces += self.write_face_patch(buffer, base)

        # Write header
        vertex_count, face_count = self.write_header(
            file,
            include=include,
            extra_faces=extra_faces
        )
        log(f"    {vertex_count:,} vertices")

        # Write vertices
        self.write_vertex_data(file)
        if include is not None:
            include.write_vertex_data(file)

        # Write faces
        if self.mesh:
            assert buffer is not None
            log(f"    {face_count:,} faces")
            self.write_face_data(file)
            file.write(buffer.getvalue())


def render(
    space: ColorSpace,
    segment_size: int = 50,
    gamut: None | ColorSpace = None,
    planar_gamut: bool = False,
    mesh: bool = False,
    sampler: None | Sampler = None,
    step_size: int = 2,
) -> None:
    traversal = SpectrumTraversal(CIE_ILLUMINANT_D65, CIE_OBSERVER_2DEG_1931)
    traversal.set_step_sizes(step_size)

    log(f"Traversing visual gamut in {space} with step size {step_size}:")
    points = PointManager(mesh=mesh)

    # Lines resulting from square wave pulses
    for step in traversal:
        if isinstance(step, GamutTraversalStep.MoveTo):
            points.mark_newline()

        color = step.color()
        if sampler:
            sampler.sample(color)
        if space.is_ok():
            points.add(color.to(ColorSpace.Oklrab))
        else:
            points.add(color)

    log(f"    {len(points):,} individual points")
    log(f"    {points.line_count:,} lines, each {points.line_length:,} points long")

    gamut_points = None
    if gamut is not None:
        log(f"Traversing color space gamut of {space} with step size {step_size}")
        gamut_points = PointManager(planar=planar_gamut)
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

    log(f"Writing {filename}:")
    with open(filename, mode="w", encoding="utf8") as file:
        points.write_all(file, gamut_points)


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
        planar_gamut=options.planar_gamut,
        mesh=options.mesh,
        step_size=step_size
    )

    log()

    render(
        ColorSpace.Oklrab,
        gamut=gamut,
        planar_gamut=options.planar_gamut,
        mesh=options.mesh,
        sampler=sampler,
        step_size=step_size
    )

    log("\nShape of visible gamut (sampled on chroma/hue plane):\n")
    log(str(sampler))

