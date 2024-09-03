import argparse
from collections import Counter
import io
import math
import sys
from typing import Any, Self

from prettypretty.color import Color, ColorSpace
from prettypretty.color.gamut import ( # pyright: ignore [reportMissingModuleSource]
    GamutTraversalStep
)
from prettypretty.color.spectrum import ( # pyright: ignore [reportMissingModuleSource]
    CIE_ILLUMINANT_D65, CIE_ILLUMINANT_E, CIE_OBSERVER_2DEG_1931, Illuminant,
    SpectrumTraversal
)


def create_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="generate a point cloud or mesh for the visual gamut in XYZ "
        "as well as Oklrab and store the data in 'visual-gamut-xyz.ply' and "
        "'visual-gamut-ok.ply' files"
    )
    parser.add_argument(
        "--illuminant-e",
        action="store_true",
        help="use CIE standard illuminant E instead of D65"
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
        "-m", "--mesh",
        action="store_true",
        help="include face mesh with vertex data; the mesh is formed by connecting "
        "the nth starting wavelength of the mth pulse width with the nth starting "
        "wavelength of the (m-1)th pulse width, much like a rope ladder"
    )
    parser.add_argument(
        "--darken",
        action="store_true",
        help="darken vertex colors"
    )
    parser.add_argument(
        "--step", "-s",
        help="use given step size (in integral nanometers)"
    )
    parser.add_argument(
        "--alpha",
        default="1.0",
        help="make vertex colors transparent, using given alpha between 0 and 1",
    )
    parser.add_argument(
        "-r", "--render",
        action="store_true",
        help="render the resulting 3D mesh and its silhouettes in XYZ color space; "
        "requires the Vedo 3D library"
    )
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="run in verbose mode"
    )

    return parser


def log(msg: str = "") -> None:
    print(msg, file=sys.stderr)


trace_enabled = False
def trace(msg: str = "") -> None:
    if trace_enabled:
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
        *,
        step_size: int,
        space: ColorSpace,
        darken: bool = False,
        planar: bool = False,
        mesh: bool = False,
        alpha: float = 1.0,
    ) -> None:
        self._step_size = step_size
        self._space = space
        self._should_darken = darken
        self._should_project_to_plane = planar
        self._should_generate_mesh = mesh
        self._points: list[tuple[float, float, float]] = []
        self._colors: list[tuple[int, int, int]] = []
        self._faces: list[tuple[int, int, int]] = []
        self._alpha: int = min(max(int(alpha * 255), 0), 255)
        self._line_count = 0
        self._line_length = 0

    @property
    def line_count(self) -> int:
        return self._line_count

    @property
    def line_length(self) -> int:
        return self._line_length

    @property
    def line_cusp(self) -> int:
        cusp = self.line_length * 32 // 100
        if cusp % 2 == 0:
            cusp += 1
        return cusp

    def __len__(self) -> int:
        return len(self._points)

    @property
    def point_count(self) -> int:
        return len(self._points)

    @property
    def face_count(self) -> int:
        return len(self._faces)

    def mark_newline(self) -> None:
        self._line_count += 1
        if self._line_count == 2:
            self._line_length = self.point_count

    def add(self, color: Color, highlight: None | Color = None) -> None:
        x, y, z = color

        if self._should_project_to_plane:
            if color.space().is_ok():
                # Zero out lightness
                self._points.append((0, y, z))
            else:
                # Zero out luminosity
                self._points.append((x, 0, z))
        else:
            self._points.append((x, y, z))

        if highlight is not None:
            r, g, b = highlight.to_24bit()
            self._colors.append((r, g, b))
            return

        if self._should_darken:
            lr, c, h = color.to(ColorSpace.Oklrch)
            color = Color.oklrch(self.darken(lr), c, h)

        r, g, b = color.to_24bit()
        self._colors.append((r, g, b))

    def darken(self, l: float) -> float:
        return 0.9 * l

    def generate_faces(self) -> None:
        edges = Counter[tuple[int, int]]()

        def e(x: int, y: int) -> tuple[int, int]:
            return min(x, y), max(x, y)

        # Create rope ladder of quads from series for pulse widths n and n+1.
        # E.g., at 1nm resolution, for n=0, that includes the edge (0, 471).
        for line_index in range(self.line_count - 1):
            if trace_enabled:
                trace("-" * 141)

            for index in range(self.line_length - 1):
                s = line_index * self.line_length + index
                t = s + 1
                u = t + self.line_length
                v = u - 1

                self._faces.append((s, v, u))
                self._faces.append((s, u, t))

                msg = ""
                if trace_enabled:
                    msg = f"pulse {line_index:3} @ {index:3}"

                for edge in [e(s, v), e(v, u), e(u, s), e(s, u), e(u, t), e(t, s)]:
                    edges[edge] += 1

                    if trace_enabled:
                        msg += f" | {edge[0]:7,} -> {edge[1]:7,}"

                trace(msg)

                # Connect end of rope ladder to start of next rope ladder.
                if index == self.line_length - 2 and line_index < self.line_count - 2:
                    s = t + 1
                    v = u + 1

                    self._faces.append((t, u, v))
                    self._faces.append((t, v, s))

                    msg = ""
                    if trace_enabled:
                        msg = f"cross {line_index:3} @ {index:3}"

                    for edge in [e(s, v), e(v, u), e(u, s), e(s, u), e(u, t), e(t, s)]:
                        edges[edge] += 1

                        if trace_enabled:
                            msg += f" | {edge[0]:7,} -> {edge[1]:7,}"

                    trace(msg)

        # For first & last series, that leaves a hole shaped like a bent spoon.
        # The handle transitions to bowl at ~32% of series, i.e., the cusp.
        cusp = self.line_cusp
        halfcusp = cusp // 2  # Rounds downward

        for base in (0, (self.line_count - 1) * self.line_length):
            # Fill spoon's handle with another rope ladder of quads.
            if trace_enabled:
                trace("-" * 141)

            for index in range(1, halfcusp + 1):
                s = base + index - 1
                t = s + 1
                u = base + cusp - index
                v = u + 1

                self._faces.append((s, v, u))
                self._faces.append((s, u, t))

                msg = ""
                if trace_enabled:
                    label = "#1" if base == 0 else "#2"
                    msg = f"patch {label}  @ {index:3}"

                for edge in [e(s, v), e(v, u), e(u, s), e(s, u), e(u, t), e(t, s)]:
                    edges[edge] += 1

                    if trace_enabled:
                        msg += f" | {edge[0]:7,} -> {edge[1]:7,}"

                trace(msg)

            # Fill spoon's bowl with fan of triangles. E.g., at 1nm resolution,
            # that includes the edge (0, 470).
            for index in range(cusp, self.line_length - 1):
                s = base
                t = s + index
                u = t + 1

                self._faces.append((s, u, t))

                msg = ""
                if trace_enabled:
                    label = "#1" if base == 0 else "#2"
                    msg = f"patch {label}  @ {index:3}"

                for edge in [e(s, u), e(u, t), e(t, s)]:
                    edges[edge] += 1

                    if trace_enabled:
                        msg += f" | {edge[0]:7,} -> {edge[1]:7,}"

                trace(msg)

        r = 0
        s = self.line_length
        t = s - 1
        self._faces.append((r, s, t))

        u = (self.line_count - 1) * self.line_length - 1
        v = u + 1
        w = v + self.line_length - 1
        self._faces.append((u, w, v))

        msg = ""
        if trace_enabled:
            msg = f"capstones      "

        for edge in [e(r, s), e(s, t), e(t, r), e(u, w), e(w, v), e(v, u)]:
            edges[edge] += 1

            if trace_enabled:
                msg += f" | {edge[0]:7,} -> {edge[1]:7,}"

        trace(msg)

        # Check mesh boundary
        boundary = [e for e, count in edges.items() if count == 1]
        if len(boundary) == 0:
            return

        print(
            "\x1b[1;48;5;220mWARNING: Mesh has boundary with "
            f"{len(boundary)} unique edges!\x1b[m"
        )

        if not trace_enabled:
            return

        boundary.sort()
        msg = ""
        for index, edge in enumerate(boundary):
            if index % 6 == 0:
                if index > 0:
                    trace(msg)
                msg = "unique edges   "

            msg += f" | {edge[0]:7,} -> {edge[1]:7,}"

        trace(msg)

    def write_header(
        self,
        file: io.TextIOWrapper,
        *,
        include: None | Self = None,
    ) -> tuple[int, int]:
        file.write("ply\n")
        file.write("format ascii 1.0\n")
        c = f"Visual gamut in {self._space}, step size {self._step_size}nm, "
        c += "w/mesh" if self._should_generate_mesh else "w/o mesh"
        file.write(f"comment {c} <https://github.com/apparebit/prettypretty>\n")

        vertex_count = self.point_count + (0 if include is None else include.point_count)
        file.write(f"element vertex {vertex_count}\n")

        file.write("property float x\n")
        file.write("property float y\n")
        file.write("property float z\n")
        file.write("property uchar red\n")
        file.write("property uchar green\n")
        file.write("property uchar blue\n")
        if self._alpha != 255:
            file.write("property uchar alpha\n")

        face_count = 0
        if self._should_generate_mesh:
            face_count = self.face_count
            file.write(f"element face {face_count}\n")
            file.write("property list uchar int vertex_indices\n")

        file.write("end_header\n")
        return vertex_count, face_count

    def write_vertex_data(self, file: io.TextIOWrapper) -> None:
        for point, color in zip(self._points, self._colors):
            f = 100.0
            x, y, z = point[0] * f, point[1] * f, point[2] * f
            r, g, b = color
            line = f"{x:f} {y:f} {z:f} {r} {g} {b}"
            if self._alpha < 255:
                line += f" {self._alpha}"
            file.write(f"{line}\n")

    def write_face_data(self, file: io.TextIOWrapper) -> None:
        for i, j, k in self._faces:
            file.write(f"3 {i} {j} {k}\n")

    def write_all(
        self,
        file: io.TextIOWrapper,
        include: None | Self = None,
    ) -> None:
        # Write header
        vertex_count, face_count = self.write_header(file, include=include)

        # Write vertices
        log(f"    {vertex_count:,} vertices")
        self.write_vertex_data(file)
        if include is not None:
            include.write_vertex_data(file)

        # Write faces
        if self._should_generate_mesh:
            log(f"    {face_count:,} faces")
            self.write_face_data(file)


def generate(
    space: ColorSpace,
    step_size: int = 2,
    darken: bool = False,
    gamut: None | ColorSpace = None,
    segment_size: int = 50,
    planar_gamut: bool = False,
    mesh: bool = False,
    alpha: float = 1.0,
    sampler: None | Sampler = None,
    illuminant: Illuminant = CIE_ILLUMINANT_D65,
) -> None:
    traversal = SpectrumTraversal(illuminant, CIE_OBSERVER_2DEG_1931)
    traversal.set_step_sizes(step_size)

    log(f"Traversing visual gamut in {space} with step size {step_size}:")
    points = PointManager(
        step_size=step_size, space=space, darken=darken, mesh=mesh, alpha=alpha
    )

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

    points.generate_faces()

    log(f"    {points.point_count:,} individual points")
    log(f"    {points.line_count:,} lines, each {points.line_length:,} points long")
    log(f"    {points.face_count:,} faces")

    gamut_points = None
    if gamut is not None:
        log(f"Traversing color space gamut of {space} with step size {step_size}")
        gamut_points = PointManager(
            step_size=step_size, space=space, darken=darken, planar=planar_gamut
        )
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
        filename = "visual-gamut-ok.ply"
    else:
        filename = "visual-gamut-xyz.ply"

    log(f"Writing {filename}:")
    with open(filename, mode="w", encoding="utf8") as file:
        points.write_all(file, gamut_points)


def render() -> None:
    try:
        from vedo import ( # pyright: ignore [reportMissingImports, reportMissingTypeStubs]
            Mesh, show # pyright: ignore [reportUnknownVariableType]
        )
    except ImportError:
        log("prettypretty.viz3d requires vedo for rendering the visual gamut.")
        log("Please install the package, e.g., by executing `pip install vedo`,")
        log("and then run `python -m prettypretty.viz3d -mr` again.")
        sys.exit(1)

    mesh: Any = Mesh("visual-gamut-xyz.ply")
    sx = mesh.clone().project_on_plane('x').c('r').x(-3) # sx is 2d
    sy = mesh.clone().project_on_plane('y').c('g').y(-3)
    sz = mesh.clone().project_on_plane('z').c('b').z(-3)

    show(mesh,  # pyright: ignore [reportUnknownMemberType]
        sx, sx.silhouette('2d'), # 2d objects dont need a direction
        sy, sy.silhouette('2d'),
        sz, sz.silhouette('2d'),
        "The Visual Gamut in XYZ",
        axes=7,
        viewup='z',
        bg="#555555",
    ).close() # pyright: ignore [reportOptionalMemberAccess]


if __name__ == "__main__":
    parser = create_parser()
    options = parser.parse_args()

    if options.verbose:
        trace_enabled = True

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
            log(f"gamut {name} is not srgb, p3, or rec2020 (case-insensitive).")
            sys.exit(1)

    if options.render and not options.mesh:
        log("The --render option requires the --mesh option.")
        sys.exit(1)

    step_size = 2
    if options.step:
        step_size = int(options.step)
    if step_size <= 0 or 10 < step_size:
        log(f"step size {step_size} is not between 1 and 10.")

    sampler = Sampler()
    illuminant = CIE_ILLUMINANT_E if options.illuminant_e else CIE_ILLUMINANT_D65

    generate(
        ColorSpace.Xyz,
        step_size=step_size,
        gamut=gamut,
        planar_gamut=options.planar_gamut,
        mesh=options.mesh,
        darken=options.darken,
        alpha=float(options.alpha),
        illuminant=illuminant,
    )

    log()

    generate(
        ColorSpace.Oklrab,
        step_size=step_size,
        gamut=gamut,
        planar_gamut=options.planar_gamut,
        mesh=options.mesh,
        sampler=sampler,
        darken=options.darken,
        alpha=float(options.alpha),
        illuminant=illuminant,
    )

    log("\nShape of visible gamut (sampled on chroma/hue plane):\n")
    log(str(sampler))

    if options.mesh and options.render:
        render()
