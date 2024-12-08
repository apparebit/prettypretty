import argparse
from collections import Counter
import io
import math
import sys
from textwrap import dedent, indent
from typing import Any, Self

from prettypretty.color import Color, ColorSpace
from prettypretty.color.gamut import ( # pyright: ignore [reportMissingModuleSource]
    GamutTraversalStep
)
from prettypretty.color.spectrum import ( # pyright: ignore [reportMissingModuleSource]
    CIE_ILLUMINANT_D50, CIE_ILLUMINANT_D65, CIE_ILLUMINANT_E, CIE_OBSERVER_2DEG_1931,
    CIE_OBSERVER_10DEG_1964, IlluminatedObserver
)


if sys.stderr.isatty():
    BOLD = "\x1b[1m"
    RESET = "\x1b[m"
else:
    BOLD = "" # pyright: ignore [reportConstantRedefinition]
    RESET = "" # pyright: ignore [reportConstantRedefinition]


def create_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        formatter_class=argparse.RawDescriptionHelpFormatter,
        description=indent(dedent(f"""
        {BOLD} Visualization of Human Visual Gamut {RESET}

        Generate a point cloud or mesh for the outer volume of the human visual
        gamut in the XYZ color space and store as a PLY file in the
        `visual-gamut` directory.

        Each point's color is the sRGB color resulting from gamut mapping the
        original XYZ color. In other words, colors are far from accurate.

        The best possible resolution, a stride of 1nm, produces a clean shape
        with good coverage of the entire gamut's surface. However, it also
        generates too many vertices (and mesh triangles) for parts of the shape,
        suggesting the need for mesh simplification. Experiments to identify a
        strategy yielding good results are on-going.


        {BOLD}Mesh Generation{RESET}

        This script uses the standard algorithm for generating points: It shifts
        and rotates pulses of increasing width across the visible spectrum. Mesh
        generation is based on two observations:

         1. Points resulting from a pulse with the same width can be arranged on
            a line. Every line has the same number of points.
         2. Lines for subsequent pulse widths traverse largely similar paths in
            3D space shifted by some (varying) distance.

        Mesh generation leverages that structure as follows:

         3. By treating subsequent lines as ropes of a rope ladder, generating
            virtual quads for much of the gamut body becomes straight-forward.
            In practice, the quads are divided into two triangles.
         4. The first and last line leave holes roughly shaped like spoons, with
            a distinct handle and bowl. The point marking the transition is the
            cusp.

              - Close the handle by treating both sides as ropes for a rope
                ladder again. This requires that the cusp's offset from the
                first line point is not divisible by two (since offset are
                zero-based).
              - Mesh the bowl as a fan of triangles. This probably can also be
                conceptualized as another rope ladder, but a fan of triangles
                seems simpler.

         5. Add two capstone triangles, which don't quite fit into the fan
            generation.

        Filling the holes holes remaining after step 4 was easy. But identifying
        the holes as simple triangles was much harder. Towards that end, mesh
        generation keeps track of edge counts and reports any remaining
        boundaries. There are none, but boundary detection remains active.
        """), "    ")
    )
    parser.add_argument(
        "--ok",
        action="store_true",
        help="render gamut in the Oklrab color space instead of XYZ"
    )
    parser.add_argument(
        "-i", "--illuminant",
        default="D65",
        choices=["D50", "d50", "D65", "d65", "E", "e"],
        help="choose between the CIE's D50, D65, and E illuminants",
    )
    parser.add_argument(
        "-b", "--observer",
        default="2",
        choices=["2", "10"],
        help="choose between the CIE's 1931 2º or the 1964 10º standard observer",
    )
    parser.add_argument(
        "-s", "--stride",
        help="use stride s measured in integral nanometers, with 1 <= s <= 20"
    )
    parser.add_argument(
        "-m", "--mesh",
        action="store_true",
        help="generate a closed, manifold, triangular mesh covering the entire visual "
        "gamut",
    )
    parser.add_argument(
        "--darken",
        action="store_true",
        help="darken vertex colors by reducing their lightness in Oklrab to 90%%"
    )
    parser.add_argument(
        "--alpha",
        default="1.0",
        help="make vertex colors transparent, using given alpha between 0 and 1",
    )
    parser.add_argument(
        "-r", "--render",
        action="store_true",
        help="interactively render the gamut in 3D with projected silhouettes; "
        "requires the Vedo 3D library"
    )
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="run in verbose mode"
    )
    parser.add_argument(
        "--gamut",
        choices=["sRGB", "srgb", "P3", "p3", "Rec2020", "rec2020"],
        help="also plot boundary of gamut for given color space",
    )
    parser.add_argument(
        "--planar-gamut",
        action="store_true",
        help="plot gamut boundary on X/Z plane for XYZ and a/b plane for Oklrab, "
        "with Y=0 or Lr= 0, respectively"
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
        stride: int,
        space: ColorSpace,
        darken: bool = False,
        planar: bool = False,
        mesh: bool = False,
        alpha: float = 1.0,
        label: None | str = None,
    ) -> None:
        self._stride = stride
        self._space = space
        self._should_darken = darken
        self._should_project_to_plane = planar
        self._should_generate_mesh = mesh
        self._points: list[tuple[float, float, float]] = []
        self._colors: list[bytes] = []
        self._faces: list[tuple[int, int, int]] = []
        self._alpha: int = min(max(int(alpha * 255), 0), 255)
        self._line_count = 0
        self._line_length = 0
        self._label = label

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

    def add(self, *, color: Color, is_xyz: bool, highlight: None | Color = None) -> None:
        x = color[0]
        y = color[1]
        z = color[2]

        if self._should_project_to_plane:
            if is_xyz:
                # Zero out luminosity
                self._points.append((x, 0, z))
            else:
                # Zero out lightness
                self._points.append((0, y, z))
        else:
            self._points.append((x, y, z))

        if highlight is not None:
            self._colors.append(highlight.to_24bit())
            return

        if self._should_darken:
            lr, c, h = color.to(ColorSpace.Oklrch)
            color = Color.oklrch(0.9 * lr, c, h)

        self._colors.append(color.to_24bit())

    def generate_faces(self) -> None:
        # See description in tool help!

        if not self._should_generate_mesh:
            return

        line_count = self.line_count
        line_length = self.line_length
        edges = Counter[tuple[int, int]]()

        # Create rope ladder of quads from series for pulse widths n and n+1.
        # E.g., at 1nm resolution, for n=0, that includes the edge (0, 471).
        for line_index in range(line_count - 1):
            if trace_enabled:
                trace("-" * 141)

            for index in range(line_length - 1):
                s = line_index * line_length + index
                t = s + 1
                u = t + line_length
                v = u - 1

                self._faces.append((s, v, u))
                self._faces.append((s, u, t))

                msg = ""
                if trace_enabled:
                    msg = f"pulse {line_index:3} @ {index:3}"

                for edge in [(s, v), (v, u), (s, u), (s, u), (t, u), (s, t)]:
                    assert edge[0] <= edge[1]
                    edges[edge] += 1

                    if trace_enabled:
                        msg += f" | {edge[0]:7,} -> {edge[1]:7,}"

                trace(msg)

                # Connect end of rope ladder to start of next rope ladder.
                if index == line_length - 2 and line_index < line_count - 2:
                    s = t + 1
                    v = u + 1

                    self._faces.append((t, u, v))
                    self._faces.append((t, v, s))

                    msg = ""
                    if trace_enabled:
                        msg = f"cross {line_index:3} @ {index:3}"

                    for edge in [(s, v), (u, v), (s, u), (s, u), (t, u), (t, s)]:
                        assert edge[0] <= edge[1]
                        edges[edge] += 1

                        if trace_enabled:
                            msg += f" | {edge[0]:7,} -> {edge[1]:7,}"

                    trace(msg)

        # For first & last line, that leaves a hole roughly shaped like a bent
        # spoon. The spoon's handle transitions to the bowl at about 32% of the
        # line, i.e., at the cusp.
        line_cusp = self.line_cusp
        halfcusp = line_cusp // 2  # Rounds downward

        for base in (0, (line_count - 1) * line_length):
            # Fill spoon's handle with another rope ladder of quads.
            if trace_enabled:
                trace("-" * 141)

            for index in range(1, halfcusp + 1):
                # s,t are growing from base,u,v are shrinking from base + line_cusp
                s = base + index - 1
                t = s + 1
                u = base + line_cusp - index
                v = u + 1

                self._faces.append((s, v, u))
                self._faces.append((s, u, t))

                msg = ""
                if trace_enabled:
                    label = "#1" if base == 0 else "#2"
                    msg = f"patch {label}  @ {index:3}"

                for edge in [(s, v), (u, v), (s, u), (s, u), (t, u), (s, t)]:
                    assert edge[0] <= edge[1]
                    edges[edge] += 1

                    if trace_enabled:
                        msg += f" | {edge[0]:7,} -> {edge[1]:7,}"

                trace(msg)

            # Fill spoon's bowl with fan of triangles. E.g., at 1nm resolution,
            # that includes the edge (0, 470).
            for index in range(line_cusp, line_length - 1):
                s = base
                t = s + index
                u = t + 1

                self._faces.append((s, u, t))

                msg = ""
                if trace_enabled:
                    label = "#1" if base == 0 else "#2"
                    msg = f"patch {label}  @ {index:3}"

                for edge in [(s, u), (t, u), (s, t)]:
                    assert edge[0] <= edge[1]
                    edges[edge] += 1

                    if trace_enabled:
                        msg += f" | {edge[0]:7,} -> {edge[1]:7,}"

                trace(msg)

        r = 0
        s = line_length
        t = s - 1
        self._faces.append((r, s, t))

        u = (line_count - 1) * line_length - 1
        v = u + 1
        w = v + line_length - 1
        self._faces.append((u, w, v))

        msg = ""
        if trace_enabled:
            msg = "capstones      "

        for edge in [(r, s), (t, s), (r, t), (u, w), (v, w), (u, v)]:
            assert edge[0] <= edge[1]
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
        # Blender chokes on PLY files with more than one comment line. Sad!
        file.write(
            f"comment Visual gamut in {self._label}  "
            "<https://github.com/apparebit/prettypretty>\n"
        )

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
            f = 100.0 if self._space is ColorSpace.Xyz else 1.0
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

# --------------------------------------------------------- Generate 3D Points and Mesh

def generate(
    *,
    space: ColorSpace,
    filename: str,
    label: str,
    illuminated_observer: IlluminatedObserver,
    stride: int = 2,
    darken: bool = False,
    gamut: None | ColorSpace = None,
    segment_size: int = 200,
    planar_gamut: bool = False,
    mesh: bool = False,
    alpha: float = 1.0,
    sampler: None | Sampler = None,
) -> None:
    log(f"Traversing visual gamut in {BOLD}{label}{RESET}:")
    points = PointManager(
        stride=stride, space=space, darken=darken, mesh=mesh, alpha=alpha, label=label
    )
    is_xyz = space.is_xyz()

    # Lines resulting from square wave pulses
    for step in illuminated_observer.visual_gamut(stride):
        if isinstance(step, GamutTraversalStep.MoveTo):
            points.mark_newline()

        color = step.color()
        if sampler:
            sampler.sample(color)
        if is_xyz:
            points.add(color=color, is_xyz=True)
        else:
            points.add(color=color.to(ColorSpace.Oklrab), is_xyz=False)

    points.generate_faces()

    log(f"    {points.point_count:,} individual points")
    log(f"    {points.line_count:,} lines, each {points.line_length:,} points long")
    log(f"    {points.face_count:,} faces")

    mn = illuminated_observer.minimum()
    mx = illuminated_observer.maximum()
    log(
        f"    min/max in XYZ: {mn[0]:.5f} {mn[1]:.5f} {mn[2]:.5f}   "
        f"{mx[0]:.5f} {mx[1]:.5f} {mx[2]:.5f}"
    )

    gamut_points = None
    if gamut is not None:
        log(f"Traversing {gamut} gamut with segment size {segment_size}")
        gamut_points = PointManager(
            stride=stride, space=space, darken=darken, planar=planar_gamut
        )

        highlight = None

        it = gamut.gamut(segment_size)
        assert it is not None
        for step in it:
            color = step.color()
            if is_xyz:
                gamut_points.add(
                    color=color.to(ColorSpace.Xyz),
                    is_xyz=True,
                    highlight=highlight,
                )
            else:
                gamut_points.add(
                    color=color.to(ColorSpace.Oklrab),
                    is_xyz=False,
                    highlight=highlight,
                )

        log(f"    {gamut_points.point_count:,} points")

    log(f"Writing {BOLD}{filename}{RESET}:")
    with open(filename, mode="w", encoding="utf8") as file:
        points.write_all(file, gamut_points)

# ------------------------------------------------------------------ Interactive Viewer

def render(*, filename: str, label: str, is_ok: bool) -> None:
    try:
        from vedo import ( # pyright: ignore [reportMissingImports, reportMissingTypeStubs]
            Mesh, show # pyright: ignore [reportUnknownVariableType]
        )
    except ImportError:
        log("prettypretty.viz3d requires vedo for rendering the visual gamut.")
        log("Please install the package, e.g., by executing `pip install vedo`,")
        log("and then run `python -m prettypretty.viz3d -mr` again.")
        sys.exit(1)

    mesh: Any = Mesh(filename)
    sx = mesh.clone().project_on_plane('x').c('r').x(0 if is_ok else 0)
    sy = mesh.clone().project_on_plane('y').c('g').y(-0.4 if is_ok else 0)
    sz = mesh.clone().project_on_plane('z').c('b').z(-0.4 if is_ok else 0)

    show(mesh,  # pyright: ignore [reportUnknownMemberType]
        sx, sx.silhouette('2d'), # 2d objects dont need a direction
        sy, sy.silhouette('2d'),
        sz, sz.silhouette('2d'),
        f"The Visual Gamut in {label}",
        axes=7,
        viewup='z',
        bg="#555555",
    ).close() # pyright: ignore [reportOptionalMemberAccess]

# =====================================================================================

if __name__ == "__main__":
    parser = create_parser()
    options = parser.parse_args()

    if options.verbose:
        trace_enabled = True

    # ---------------------------------------------------------------- Colorspace Gamut
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
            log(f"gamut {name} is not srgb, p3, or rec2020 (case-insensitive)!")
            sys.exit(1)

    if options.planar_gamut and gamut is None:
        log("The --planar-gamut option requires the --gamut option")
        sys.exit(1)

    # -------------------------------------------------------------------------- Stride
    stride = 2
    if options.stride:
        try:
            stride = int(options.stride)
        except ValueError:
            log(f"stride {stride} is not an integer")
            sys.exit(1)
    if not (1 <= stride <= 20):
        log(f"stride {stride} is not between 1 and 20 (inclusive)!")
        sys.exit(1)

    # ---------------------------------------------------------------------- Illuminant
    illuminant_name = options.illuminant.upper()
    if illuminant_name == "D50":
        illuminant = CIE_ILLUMINANT_D50
    elif illuminant_name == "D65":
        illuminant = CIE_ILLUMINANT_D65
    elif illuminant_name == "E":
        illuminant = CIE_ILLUMINANT_E
    else:
        raise ValueError(f'invalid value for illuminant "{illuminant_name}"')

    # ------------------------------------------------------------------------ Observer
    if options.observer == "2":
        observer = CIE_OBSERVER_2DEG_1931
    elif options.observer == "10":
        observer = CIE_OBSERVER_10DEG_1964
    else:
        raise ValueError(f'invalid value for observer "{options.observer}"')

    # ---------------------------------------------------------------- Filename & Label
    file_suffix = f"-{stride}nm-{illuminant_name.lower()}-{options.observer}deg.ply"
    label_suffix = f": {stride}nm @ {illuminant_name} * {options.observer}º"

    filename = "visual-gamut/" + ("ok" if options.ok else "xyz") + file_suffix
    label = ("Oklrab" if options.ok else "XYZ") + label_suffix

    # --------------------------------------------------------------------- Render File
    illuminated_observer = IlluminatedObserver(illuminant, observer)
    sampler = Sampler() if options.ok else None
    generate(
        space=ColorSpace.Oklrab if options.ok else ColorSpace.Xyz,
        illuminated_observer=illuminated_observer,
        stride=stride,
        gamut=gamut,
        planar_gamut=options.planar_gamut,
        mesh=options.mesh,
        darken=options.darken,
        alpha=float(options.alpha),
        sampler=sampler,
        filename=filename,
        label=label,
    )

    if options.ok:
        log("\nShape of visible gamut (sampled on chroma/hue plane):\n")
        log(str(sampler))

    # -------------------------------------------------------------- Interactive Viewer
    if options.render:
        render(filename=filename, label=label, is_ok=options.ok)
