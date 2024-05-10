from dataclasses import dataclass
import math
import unittest

from prettypretty.color.object import Color, parse_hex, parse_x_rgb, parse_x_rgbi
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
    linear_srgb_to_oklab,
    #oklab_to_linear_srgb,
)


@dataclass(frozen=True)
class ColorValues:
    spec: str
    parsed: tuple[int, int, int]
    sRGB: tuple[float, float, float]
    linear_sRGB: tuple[float, float, float]
    P3: tuple[float, float, float]
    linear_P3: tuple[float, float, float]
    XYZ: tuple[float, float, float]
    OkLab: tuple[float, float, float]
    OkLCh: tuple[float, float, float]
    ANSI: tuple[int]


class TestColor(unittest.TestCase):

    BLACK = ColorValues(
        spec = '#000',
        parsed = (0, 0, 0),
        sRGB = (0.0, 0.0, 0.0),
        linear_sRGB = (0.0, 0.0, 0.0),
        P3 = (0.0, 0.0, 0.0),
        linear_P3 = (0.0, 0.0, 0.0),
        XYZ = (0.0, 0.0, 0.0),
        OkLab = (0.0, 0.0, 0.0),
        OkLCh = (0.0, 0.0, math.nan),
        ANSI= (0,),
    )

    YELLOW = ColorValues(
        spec = '#ffca00',
        parsed = (255, 202, 0),
        sRGB = (1.0, 0.792156862745098, 0.0),
        linear_sRGB = (1.0, 0.5906188409193369, 0.0),
        P3 = (0.967346220711791, 0.8002244967941964, 0.27134084647161244),
        linear_P3 = (0.9273192749713864, 0.6042079205196976, 0.059841923211596565),
        XYZ = (0.6235868473237722, 0.635031101987136, 0.08972950140152941),
        OkLab = (0.8613332073307732, 0.0017175723640959761, 0.1760013937170005),
        OkLCh = (0.8613332073307732, 0.17600977428868128, 89.440876452466),
        ANSI = (11,),
    )

    BLUE = ColorValues(
        spec = '#3178ea',
        parsed = (49, 120, 234),
        sRGB = (0.19215686274509805, 0.47058823529411764, 0.9176470588235294),
        linear_sRGB = (0.030713443732993635, 0.18782077230067787, 0.8227857543962835),
        P3 = (0.2685153556355094, 0.464457615084287, 0.8876966971452301),
        linear_P3 = (0.058605969547446096, 0.18260572039525874, 0.763285235993837),
        XYZ = (0.22832473003420622, 0.20025321836938537, 0.80506528557483),
        OkLab = (0.5909012953108558, -0.03348086515869708, -0.1836287492414714),
        OkLCh = (0.5909012953108558, 0.18665606306724153, 259.66681920272583),
        ANSI = (6,),
    )

    WHITE = ColorValues(
        spec = '#fff',
        parsed = (255, 255, 255),
        sRGB = (1.0, 1.0, 1.0),
        linear_sRGB = (1.0, 1.0, 1.0),
        P3 = (0.9999999999999999, 0.9999999999999997, 0.9999999999999999),
        linear_P3 = (1, 0.9999999999999998, 1),
        XYZ = (0.9504559270516717, 1.0, 1.0890577507598784),
        OkLab = (1.0000000000000002, -4.996003610813204e-16, 1.734723475976807e-17),
        OkLCh = (1.0000000000000002, 4.999014376318559e-16, math.nan),
        ANSI = (15,),
    )

    def assertCloseEnough(
        self,
        color1: tuple[float, float, float],
        color2: tuple[float, float, float],
        places: int = 14,
    ) -> None:
        for c1, c2 in zip(color1, color2):
            self.assertAlmostEqual(c1, c2, places=places)

    def test_one_step_conversions(self) -> None:
        for color_name in ('BLACK', 'YELLOW', 'BLUE', 'WHITE'):
            values = getattr(self, color_name)
            spec = values.spec

            color_name = color_name.lower()
            with self.subTest('hex-string to RGB256', color=color_name):
                _, rgb256 = parse_hex(spec)
                self.assertEqual(rgb256, values.parsed)

            with self.subTest('RGB256 to sRGB', color=color_name):
                srgb = rgb256_to_srgb(*rgb256)
                self.assertEqual(srgb, values.sRGB)

            with self.subTest('sRGB back to RGB256', color=color_name):
                self.assertEqual(srgb_to_rgb256(*srgb), rgb256)

            with self.subTest('sRGB to linear sRGB', color=color_name):
                linear_srgb = srgb_to_linear_srgb(*srgb)
                self.assertEqual(linear_srgb, values.linear_sRGB)

            with self.subTest('linear sRGB back to sRGB', color=color_name):
                self.assertCloseEnough(linear_srgb_to_srgb(*linear_srgb), srgb)

            with self.subTest('linear sRGB to XYZ', color=color_name):
                xyz = linear_srgb_to_xyz(*linear_srgb)
                self.assertEqual(xyz, values.XYZ)

            with self.subTest('XYZ back to linear sRGB', color=color_name):
                self.assertCloseEnough(xyz_to_linear_srgb(*xyz), linear_srgb)

            with self.subTest('XYZ to linear P3', color=color_name):
                linear_p3 = xyz_to_linear_p3(*xyz)
                self.assertEqual(linear_p3, values.linear_P3)

            with self.subTest('linear P3 back to XYZ', color=color_name):
                self.assertCloseEnough(linear_p3_to_xyz(*linear_p3), xyz)

            with self.subTest('linear P3 to P3', color=color_name):
                p3 = linear_p3_to_p3(*linear_p3)
                self.assertEqual(p3, values.P3)

            with self.subTest('P3 back to linear P3', color=color_name):
                self.assertCloseEnough(p3_to_linear_p3(*p3), linear_p3)

            with self.subTest('XYZ to OkLab', color=color_name):
                oklab = xyz_to_oklab(*xyz)
                self.assertEqual(oklab, values.OkLab)

            with self.subTest('OkLab back to XYZ', color=color_name):
                self.assertCloseEnough(oklab_to_xyz(*oklab), xyz)

            with self.subTest('OkLab to OkLCh', color=color_name):
                oklch = oklab_to_oklch(*oklab)
                expected = values.OkLCh
                self.assertEqual(oklch[0], expected[0])
                self.assertEqual(oklch[1], expected[1])
                self.assertTrue(
                    (math.isnan(oklch[2]) and math.isnan(expected[2]))
                    or oklch[2] == expected[2]
                )

            with self.subTest('OkLCh to OkLab', color=color_name):
                self.assertCloseEnough(oklch_to_oklab(*oklch), oklab)

            with self.subTest('linear sRGB to OkLab via Ottosson', color=color_name):
                ottosson1 = linear_srgb_to_oklab(*linear_srgb)
                self.assertCloseEnough(ottosson1, values.OkLab, places=7)

            # with self.subTest('OkLab to linear sRGB via Ottosson', color=color_name):
            #     ottosson2 = oklab_to_linear_srgb(*oklab)
            #     self.assertCloseEnough(ottosson2, getattr(values, 'linear_sRGB'), places=7)

    def test_multistep_conversions(self) -> None:
        for color_name in ('BLACK', 'YELLOW', 'BLUE', 'WHITE'):
            values = getattr(self, color_name)
            spec = getattr(values, 'spec')

            color_name = color_name.lower()
            with self.subTest('parse color', color=color_name):
                color = Color.of(spec)
                self.assertEqual(color.tag, 'rgb256')
                self.assertEqual(color.coordinates, values.parsed)

            with self.subTest('convert to OkLab', color=color_name):
                oklab = color.to('oklab')
                self.assertEqual(oklab.tag, 'oklab')
                self.assertEqual(oklab.coordinates, values.OkLab)

            with self.subTest('convert to ANSI', color=color_name):
                ansi = color.to('ansi')
                self.assertEqual(ansi.tag, 'ansi')
                self.assertEqual(ansi.coordinates, values.ANSI)

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
