from dataclasses import dataclass
import math
import sys
from typing import cast
import unittest

from prettypretty.color.conversion import (
    rgb256_to_srgb,
    srgb_to_rgb256,
    srgb_to_linear_srgb,
    linear_srgb_to_srgb,
    linear_srgb_to_xyz,
    xyz_to_linear_srgb,
    xyz_to_linear_p3,
    linear_p3_to_xyz,
    linear_p3_to_p3,
    p3_to_linear_p3,
    xyz_to_oklab,
    oklab_to_xyz,
    oklab_to_oklch,
    oklch_to_oklab,
    get_converter,
)
from prettypretty.color.equality import normalize, PRECISION
from prettypretty.color.gamut import map_into_gamut
from prettypretty.color.object import Color
from prettypretty.color.serde import parse_hex, parse_x_rgb, parse_x_rgbi
from prettypretty.color.spec import ColorSpec
from prettypretty.color.theme import VGA


@dataclass(frozen=True)
class ColorValues:
    spec: str
    parsed: tuple[int, int, int]
    srgb: tuple[float, float, float]
    linear_srgb: tuple[float, float, float]
    p3: tuple[float, float, float]
    linear_p3: tuple[float, float, float]
    xyz: tuple[float, float, float]
    oklab: tuple[float, float, float]
    oklch: tuple[float, float, float]
    ansi: tuple[int]
    black_text: bool
    black_background: bool
    closest_index: int
    xterm: str
    css: str


class TestColor(unittest.TestCase):

    BLACK = ColorValues(
        spec = '#000',
        parsed = (0, 0, 0),
        srgb = (0.0, 0.0, 0.0),
        linear_srgb = (0.0, 0.0, 0.0),
        p3 = (0.0, 0.0, 0.0),
        linear_p3 = (0.0, 0.0, 0.0),
        xyz = (0.0, 0.0, 0.0),
        oklab = (0.0, 0.0, 0.0),
        oklch = (0.0, 0.0, math.nan),
        ansi = (0,),
        black_text = False,
        black_background = False,
        closest_index = 0,
        xterm = 'rgb:00/00/00',
        css = 'rgb(0 0 0)',
    )

    YELLOW = ColorValues(
        spec = '#ffca00',
        parsed = (255, 202, 0),
        srgb = (1.0, 0.792156862745098, 0.0),
        linear_srgb = (1.0, 0.5906188409193369, 0.0),
        p3 = (0.967346220711791, 0.8002244967941964, 0.27134084647161244),
        linear_p3 = (0.9273192749713864, 0.6042079205196976, 0.059841923211596565),
        xyz = (0.6235868473237722, 0.635031101987136, 0.08972950140152941),
        oklab = (0.8613332073307732, 0.0017175723640959761, 0.1760013937170005),
        oklch = (0.8613332073307732, 0.17600977428868128, 89.440876452466),
        ansi = (11,),
        black_text = True,
        black_background = True,
        closest_index = 1,
        xterm = 'rgb:ff/ca/00',
        css = 'rgb(255 202 0)',
    )

    BLUE = ColorValues(
        spec = '#3178ea',
        parsed = (49, 120, 234),
        srgb = (0.19215686274509805, 0.47058823529411764, 0.9176470588235294),
        linear_srgb = (0.030713443732993635, 0.18782077230067787, 0.8227857543962835),
        p3 = (0.2685153556355094, 0.464457615084287, 0.8876966971452301),
        linear_p3 = (0.058605969547446096, 0.18260572039525874, 0.763285235993837),
        xyz = (0.22832473003420622, 0.20025321836938537, 0.80506528557483),
        oklab = (0.5909012953108558, -0.03348086515869708, -0.1836287492414714),
        oklch = (0.5909012953108558, 0.18665606306724153, 259.66681920272583),
        ansi = (12,),
        black_text = False,
        black_background = False,
        closest_index = 2,
        xterm = 'rgb:31/78/ea',
        css = 'rgb(49 120 234)',
    )

    WHITE = ColorValues(
        spec = '#fff',
        parsed = (255, 255, 255),
        srgb = (1.0, 1.0, 1.0),
        linear_srgb = (1.0, 1.0, 1.0),
        p3 = (0.9999999999999999, 0.9999999999999997, 0.9999999999999999),
        linear_p3 = (1, 0.9999999999999998, 1),
        xyz = (0.9504559270516717, 1.0, 1.0890577507598784),
        oklab = (1.0000000000000002, -4.996003610813204e-16, 1.734723475976807e-17),
        oklch = (1.0000000000000002, 4.999014376318559e-16, math.nan),
        ansi = (15,),
        black_text = True,
        black_background = True,
        closest_index = 3,
        xterm = 'rgb:ff/ff/ff',
        css = 'rgb(255 255 255)',
    )

    def assertSameCoordinates(
        self,
        coordinates1: tuple[float, float, float],
        coordinates2: tuple[float, float, float],
        *,
        angular_index: int = -1,
    ) -> None:
        # A double cannot encode more than 17 decimal digits...
        s1 = ', '.join(f'{c:.17f}' for c in coordinates1)
        s2 = ', '.join(f'{c:.17f}' for c in coordinates2)

        # Show coordinates above each other to help compare close values
        msg = (
            f'coordinates differ within {PRECISION} '
            f'digits:\n    {s1}\n    {s2}'
        )

        n1 = normalize(coordinates1, angular_index=angular_index)
        n2 = normalize(coordinates2, angular_index=angular_index)
        self.assertTrue(n1 == n2, msg)

    def test_same_coordinates(self) -> None:
        # One less than the precision is rounded up or down, with 0.5
        # represented by a floating point number slightly smaller and hence
        # still rounding down.
        f00 = 0.0
        f01 = float(f'1e-{PRECISION + 1}')
        f02 = float(f'2e-{PRECISION + 1}')
        f05 = float(f'5e-{PRECISION + 1}')
        f07 = float(f'7e-{PRECISION + 1}')
        f08 = float(f'8e-{PRECISION + 1}')
        f09 = float(f'9e-{PRECISION + 1}')
        f10 = float(f'1e-{PRECISION}')
        f20 = float(f'2e-{PRECISION}')

        self.assertSameCoordinates(
            (f01, f02, f05),
            (f00, f00, f00),
        )

        self.assertSameCoordinates(
            (f07, f08, f09),
            (f10, f10, f10),
        )

        with self.assertRaises(AssertionError):
            self.assertSameCoordinates(
                (f10, f10, f10),
                (f20, f20, f20),
            )


    def test_conversions(self) -> None:
        for color_name in ('BLACK', 'YELLOW', 'BLUE', 'WHITE'):
            values = getattr(self, color_name)
            spec = values.spec

            color_name = color_name.lower()
            with self.subTest('hex-string to RGB256', color=color_name):
                _, rgb256 = parse_hex(spec)
                self.assertTupleEqual(rgb256, values.parsed)

            with self.subTest('RGB256 to sRGB', color=color_name):
                srgb = rgb256_to_srgb(*rgb256)
                self.assertSameCoordinates(srgb, values.srgb)

            with self.subTest('sRGB back to RGB256', color=color_name):
                self.assertTupleEqual(srgb_to_rgb256(*srgb), rgb256)

            with self.subTest('sRGB to linear sRGB', color=color_name):
                linear_srgb = srgb_to_linear_srgb(*srgb)
                self.assertSameCoordinates(linear_srgb, values.linear_srgb)

            with self.subTest('linear sRGB back to sRGB', color=color_name):
                self.assertSameCoordinates(linear_srgb_to_srgb(*linear_srgb), srgb)

            with self.subTest('linear sRGB to XYZ', color=color_name):
                xyz = linear_srgb_to_xyz(*linear_srgb)
                self.assertSameCoordinates(xyz, values.xyz)

            with self.subTest('XYZ back to linear sRGB', color=color_name):
                self.assertSameCoordinates(xyz_to_linear_srgb(*xyz), linear_srgb)

            with self.subTest('XYZ to linear P3', color=color_name):
                linear_p3 = xyz_to_linear_p3(*xyz)
                self.assertSameCoordinates(linear_p3, values.linear_p3)

            with self.subTest('linear P3 back to XYZ', color=color_name):
                self.assertSameCoordinates(linear_p3_to_xyz(*linear_p3), xyz)

            with self.subTest('linear P3 to P3', color=color_name):
                p3 = linear_p3_to_p3(*linear_p3)
                self.assertSameCoordinates(p3, values.p3)

            with self.subTest('P3 back to linear P3', color=color_name):
                self.assertSameCoordinates(p3_to_linear_p3(*p3), linear_p3)

            with self.subTest('XYZ to Oklab', color=color_name):
                oklab = xyz_to_oklab(*xyz)
                self.assertSameCoordinates(oklab, values.oklab)

            with self.subTest('Oklab back to XYZ', color=color_name):
                self.assertSameCoordinates(oklab_to_xyz(*oklab), xyz)

            with self.subTest('Oklab to Oklch', color=color_name):
                oklch = oklab_to_oklch(*oklab)
                self.assertSameCoordinates(oklch, values.oklch, angular_index=2)

            with self.subTest('Oklch back to Oklab', color=color_name):
                self.assertSameCoordinates(oklch_to_oklab(*oklch), oklab)


    def test_lores(self) -> None:
        self.assertFalse('prettypretty.color.lores' in sys.modules)
        cache = sys.modules['prettypretty.color.conversion']._converter_cache
        self.assertEqual(len(cache['ansi']), 0)

        for color_name in ('BLACK', 'YELLOW', 'BLUE', 'WHITE'):
            values = getattr(self, color_name)

            color_name = color_name.lower()
            with self.subTest('lores conversion', color=color_name):
                color = Color(values.spec)
                self.assertEqual(color.tag, 'rgb256')
                self.assertTupleEqual(color.coordinates, values.parsed)

                ansi = color.to('ansi')
                self.assertEqual(ansi.tag, 'ansi')
                self.assertTupleEqual(ansi.coordinates, values.ansi)

                rgb256 = ansi.to('rgb256')
                self.assertTupleEqual(
                    rgb256.coordinates,
                    VGA.ansi(cast(int, ansi.coordinates[0])).coordinates
                )

        self.assertTrue('prettypretty.color.lores' in sys.modules)
        self.assertEqual(len(cache['ansi']), 3)


    def test_gamut_mapping(self) -> None:
        # A very green green
        p3 = 0, 1, 0
        srgb = get_converter('p3', 'srgb')(*p3)
        self.assertSameCoordinates(
            srgb, (-0.5116049825853448, 1.0182656579378029, -0.3106746212905826)
        )

        srgb_mapped = map_into_gamut('srgb', srgb)
        self.assertSameCoordinates(
            srgb_mapped, (0, 0.9857637107710327, 0.15974244397343721)
        )

        # A strong yellow
        p3 = 1, 1, 0
        linear_srgb = get_converter('p3', 'linear_srgb')(*p3)
        self.assertSameCoordinates(
            linear_srgb, (1, 1.0000000000000002, -0.09827360014096621)
        )

        linear_srgb_mapped = map_into_gamut('linear_srgb', linear_srgb)
        self.assertSameCoordinates(
            linear_srgb_mapped, (0.9914525477996113, 0.9977581974546286, 0)
        )


    def test_color_object(self) -> None:
        for color_name in ('BLACK', 'YELLOW', 'BLUE', 'WHITE'):
            values = getattr(self, color_name)
            spec = values.spec

            color_name = color_name.lower()
            with self.subTest('parse color', color=color_name):
                color = Color(spec)
                self.assertEqual(color.tag, 'rgb256')
                self.assertTupleEqual(color.coordinates, values.parsed)
                self.assertEqual(color, color)
                self.assertEqual(color, ColorSpec('rgb256', values.parsed))

            with self.subTest('convert to OkLab', color=color_name):
                oklab = color.to('oklab')
                self.assertEqual(oklab.tag, 'oklab')
                self.assertSameCoordinates(
                    cast(tuple[float, float, float], oklab.coordinates),
                    values.oklab
                )
                self.assertEqual(oklab, oklab)
                self.assertNotEqual(oklab, color)
                self.assertEqual(oklab, ColorSpec('oklab', values.oklab))

            with self.subTest('check distance', color=color_name):
                self.assertEqual(
                    oklab.closest([
                        Color('#444'),
                        Color('#bb0'),
                        Color('#00b'),
                        Color('#bbb'),
                    ]),
                    values.closest_index
                )

            with self.subTest('check contrast', color=color_name):
                self.assertEqual(oklab.use_black_text(), values.black_text)
                self.assertEqual(oklab.use_black_background(), values.black_background)

            with self.subTest('check string', color=color_name):
                self.assertEqual(f'{color:x}', values.xterm)
                self.assertEqual(f'{color:c}', values.css)


    def test_x_parse_color(self) -> None:
        for text, expected in {
            'rgb:0/8/80': (0, 0x88, 0x80),
            'rgb:800/8000/0': (0.5, 0.5, 0.0),
            'rgbi:0/0.5/1': (0.0, 0.5, 1.0),
            'rgbi:1e0/1e-1/1e-2': (1.0, 0.1, 0.01),
        }.items():
            fn = parse_x_rgb if text.startswith('rgb:') else parse_x_rgbi
            _, actual = fn(text)
            self.assertEqual(actual, expected)


    def test_parameters(self) -> None:
        from prettypretty.style import rich

        for style, expected_params, inverse_params in (
            (rich().fg(-1).style(), (39,), ()),
            (rich().fg(1).style(), (31,), (39,)),
            (rich().bg(9).style(), (101,), (49,)),
            (rich().bg(240).style(), (48, 5, 240), (49,)),
            (rich().fg(ColorSpec('eight_bit', (12,))).style(), (38, 5, 12), (39,)),
        ):
            actual_params = style.sgr_parameters()
            self.assertSequenceEqual(actual_params, expected_params)
            actual_inverse = (~style).sgr_parameters()
            self.assertSequenceEqual(actual_inverse, inverse_params)
