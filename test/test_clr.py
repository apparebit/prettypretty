from dataclasses import dataclass
import math
import unittest

import pitty.clr as clr


@dataclass(frozen=True)
class ColorValues:
    spec: str
    parsed: clr.RGB
    sRGB: clr.TrueColor
    linear_sRGB: clr.TrueColor
    XYZ: clr.TrueColor
    OkLab: clr.TrueColor
    OkLCh: clr.TrueColor


class TestColor(unittest.TestCase):

    BLACK = ColorValues(
        spec = '#000',
        parsed = (0, 0, 0),
        sRGB = (0.0, 0.0, 0.0),
        linear_sRGB = (0.0, 0.0, 0.0),
        XYZ = (0.0, 0.0, 0.0),
        OkLab = (0.0, 0.0, 0.0),
        OkLCh = (0.0, 0.0, math.nan),
    )

    YELLOW = ColorValues(
        spec = '#ffca00',
        parsed = (255, 202, 0),
        sRGB = (1.0, 0.792156862745098, 0.0),
        linear_sRGB = (1.0, 0.5906188409193369, 0.0),
        XYZ = (0.6235868473237722, 0.635031101987136, 0.08972950140152941),
        OkLab = (0.8613332073307732, 0.0017175723640959761, 0.1760013937170005),
        OkLCh = (0.8613332073307732, 0.17600977428868128, 89.440876452466),
    )

    BLUE = ColorValues(
        spec = '#3178ea',
        parsed = (49, 120, 234),
        sRGB = (0.19215686274509805, 0.47058823529411764, 0.9176470588235294),
        linear_sRGB = (0.030713443732993635, 0.18782077230067787, 0.8227857543962835),
        XYZ = (0.22832473003420622, 0.20025321836938537, 0.80506528557483),
        OkLab = (0.5909012953108558, -0.03348086515869708, -0.1836287492414714),
        OkLCh = (0.5909012953108558, 0.18665606306724153, 259.66681920272583),
    )

    WHITE = ColorValues(
        spec = '#fff',
        parsed = (255, 255, 255),
        sRGB = (1.0, 1.0, 1.0),
        linear_sRGB = (1.0, 1.0, 1.0),
        XYZ = (0.9504559270516717, 1.0, 1.0890577507598784),
        OkLab = (1.0000000000000002, -4.996003610813204e-16, 1.734723475976807e-17),
        OkLCh = (1.0000000000000002, 4.999014376318559e-16, math.nan),
    )

    def assertCloseEnough(self, color1: clr.TrueColor, color2: clr.TrueColor) -> None:
        for c1, c2 in zip(color1, color2):
            self.assertAlmostEqual(c1, c2, places=14)

    def test_conversions(self) -> None:
        for color_name in ('BLACK', 'YELLOW', 'BLUE', 'WHITE'):
            values = getattr(self, color_name)
            spec = getattr(values, 'spec')

            color_name = color_name.lower()
            with self.subTest('to RGB', color=color_name):
                color1 = clr.hex_string_to_rgb256(spec)
                self.assertEqual(color1, getattr(values, 'parsed'))

            with self.subTest('to sRGB', color=color_name):
                color2 = clr.rgb256_to_srgb(*color1)
                self.assertEqual(color2, getattr(values, 'sRGB'))

            with self.subTest('from sRGB', color=color_name):
                self.assertEqual(clr.srgb_to_rgb256(*color2), color1)

            with self.subTest('to linear sRGB', color=color_name):
                color3 = clr.srgb_to_linear_srgb(*color2)
                self.assertEqual(color3, getattr(values, 'linear_sRGB'))

            with self.subTest('from linear sRGB', color=color_name):
                self.assertCloseEnough(clr.linear_srgb_to_srgb(*color3), color2)

            with self.subTest('to XYZ', color=color_name):
                color4 = clr.linear_srgb_to_xyz(*color3)
                self.assertEqual(color4, getattr(values, 'XYZ'))

            with self.subTest('from XYZ', color=color_name):
                self.assertCloseEnough(clr.xyz_to_linear_srgb(*color4), color3)

            with self.subTest('to OkLab', color=color_name):
                color5 = clr.xyz_to_oklab(*color4)
                self.assertEqual(color5, getattr(values, 'OkLab'))

            with self.subTest('from OkLab', color=color_name):
                self.assertCloseEnough(clr.oklab_to_xyz(*color5), color4)

            with self.subTest('to OkLCh', color=color_name):
                color6 = clr.oklab_to_oklch(*color5)
                expected = getattr(values, 'OkLCh')
                self.assertEqual(color6[0], expected[0])
                self.assertEqual(color6[1], expected[1])
                self.assertTrue(
                    (math.isnan(color6[2]) and math.isnan(expected[2]))
                    or color6[2] == expected[2]
                )

            with self.subTest('from OkLCh', color=color_name):
                self.assertCloseEnough(clr.oklch_to_oklab(*color6), color5)

    def test_more_conversions(self) -> None:
        for high, low in {
            (0x20, 0x76, 0xB8): (0, 2, 3),
            (0x30, 0x87, 0xEF): (1, 2, 5),
            (0xAF, 0xC3, 0xC4): (3, 3, 4),
        }.items():
            self.assertEqual(clr.rgb256_to_rgb6(*high), low)


    def test_x_parse_color(self) -> None:
        for text, expected in {
            'rgb:0/8/80': (0, 0.5, 0.5),
            'rgb:800/8000/0': (0.5, 0.5, 0.0),
            'rgbi:0/0.5/1': (0.0, 0.5, 1.0),
            'rgbi:1e0/1e-1/1e-2': (1.0, 0.1, 0.01),
        }.items():
            actual = clr.x_parse_color_to_srgb(text)
            self.assertEqual(actual, expected)
