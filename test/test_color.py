import math
import unittest

from prettypretty.color import (
    Color,
    ColorSpace,
    style, # pyright: ignore [reportMissingModuleSource]
)
from prettypretty.theme import current_translator


class ColorValues:

    def __init__(
        self,
        spec: str,
        parsed: tuple[int, int, int],
        srgb: tuple[float, float, float],
        linear_srgb: tuple[float, float, float],
        p3: tuple[float, float, float],
        linear_p3: tuple[float, float, float],
        xyz: tuple[float, float, float],
        oklab: tuple[float, float, float],
        oklch: tuple[float, float, float],
        ansi: int,
        black_text: bool,
        black_background: bool,
        closest_index: int,
        xterm: str,
        css: str,
    ) -> None:
        self.spec = spec
        self.parsed = style.TrueColor(*parsed)
        self.srgb = Color(ColorSpace.Srgb, srgb)
        self.linear_srgb = Color(ColorSpace.LinearSrgb, linear_srgb)
        self.p3 = Color(ColorSpace.DisplayP3, p3)
        self.linear_p3 = Color(ColorSpace.LinearDisplayP3, linear_p3)
        self.xyz = Color(ColorSpace.Xyz, xyz)
        self.oklab = Color(ColorSpace.Oklab, oklab)
        self.oklch = Color(ColorSpace.Oklch, oklch)
        self.ansi = style.AnsiColor.try_from_8bit(ansi)
        self.black_text = black_text
        self.black_background = black_background
        self.closest_index = closest_index
        self.xterm = xterm
        self.css = css


class TestColor(unittest.TestCase):

    BLACK = ColorValues(
        spec = '#000000',
        parsed = (0, 0, 0),
        srgb = (0.0, 0.0, 0.0),
        linear_srgb = (0.0, 0.0, 0.0),
        p3 = (0.0, 0.0, 0.0),
        linear_p3 = (0.0, 0.0, 0.0),
        xyz = (0.0, 0.0, 0.0),
        oklab = (0.0, 0.0, 0.0),
        oklch = (0.0, 0.0, math.nan),
        ansi = 0,
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
        ansi = 11,
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
        ansi = 12,
        black_text = False,
        black_background = False,
        closest_index = 2,
        xterm = 'rgb:31/78/ea',
        css = 'rgb(49 120 234)',
    )

    WHITE = ColorValues(
        spec = '#ffffff',
        parsed = (255, 255, 255),
        srgb = (1.0, 1.0, 1.0),
        linear_srgb = (1.0, 1.0, 1.0),
        p3 = (0.9999999999999999, 0.9999999999999997, 0.9999999999999999),
        linear_p3 = (1, 0.9999999999999998, 1),
        xyz = (0.9504559270516717, 1.0, 1.0890577507598784),
        oklab = (1.0000000000000002, -4.996003610813204e-16, 1.734723475976807e-17),
        oklch = (1.0000000000000002, 4.999014376318559e-16, math.nan),
        ansi = 15,
        black_text = True,
        black_background = True,
        closest_index = 3,
        xterm = 'rgb:ff/ff/ff',
        css = 'rgb(255 255 255)',
    )

    def test_same_color(self) -> None:
        green = Color(ColorSpace.Srgb, (0.0, 1.0, 0.0))
        self.assertEqual(green.space(), ColorSpace.Srgb)
        self.assertListEqual(green.coordinates(), [0.0, 1.0, 0.0])

        also_green = style.TerminalColor.from_8bit(46)
        self.assertIsInstance(also_green, style.TerminalColor.Rgb6)
        self.assertIsInstance(also_green, style.TerminalColor)
        assert isinstance(also_green, style.TerminalColor.Rgb6)
        self.assertIsInstance(also_green.color, style.EmbeddedRgb)
        self.assertEqual(also_green.color, style.EmbeddedRgb(0, 5, 0))
        self.assertListEqual(also_green.color.coordinates(), [0, 5, 0])

        green_too = Color.from_24bit(0, 255, 0)
        self.assertEqual(green_too.space(), ColorSpace.Srgb)
        self.assertEqual(green_too.coordinates(), [0.0, 1.0, 0.0])

        translator = current_translator()
        green3 = translator.resolve(also_green)
        self.assertEqual(green_too, green3)


    def test_conversions(self) -> None:
        for color_name in ('BLACK', 'YELLOW', 'BLUE', 'WHITE'):
            values = getattr(self, color_name)
            spec = values.spec

            color_name = color_name.lower()
            with self.subTest('hex-string to sRGB', color=color_name):
                srgb = Color.parse(spec)
                self.assertEqual(srgb, values.srgb)

            with self.subTest('sRGB back to hex-string', color=color_name):
                self.assertEqual(srgb.to_hex_format(), values.spec)

            with self.subTest('sRGB to linear sRGB', color=color_name):
                linear_srgb = srgb.to(ColorSpace.LinearSrgb)
                self.assertEqual(linear_srgb, values.linear_srgb)

            with self.subTest('linear sRGB back to sRGB', color=color_name):
                self.assertEqual(linear_srgb.to(ColorSpace.Srgb), srgb)

            with self.subTest('linear sRGB to XYZ', color=color_name):
                xyz = linear_srgb.to(ColorSpace.Xyz)
                self.assertEqual(xyz, values.xyz)

            with self.subTest('XYZ back to linear sRGB', color=color_name):
                self.assertEqual(xyz.to(ColorSpace.LinearSrgb), linear_srgb)

            with self.subTest('XYZ to linear P3', color=color_name):
                linear_p3 = xyz.to(ColorSpace.LinearDisplayP3)
                self.assertEqual(linear_p3, values.linear_p3)

            with self.subTest('linear P3 back to XYZ', color=color_name):
                self.assertEqual(linear_p3.to(ColorSpace.Xyz), xyz)

            with self.subTest('linear P3 to P3', color=color_name):
                p3 = linear_p3.to(ColorSpace.DisplayP3)
                self.assertEqual(p3, values.p3)

            with self.subTest('P3 back to linear P3', color=color_name):
                self.assertEqual(p3.to(ColorSpace.LinearDisplayP3), linear_p3)

            with self.subTest('XYZ to Oklab', color=color_name):
                oklab = xyz.to(ColorSpace.Oklab)
                self.assertEqual(oklab, values.oklab)

            with self.subTest('Oklab back to XYZ', color=color_name):
                self.assertEqual(oklab.to(ColorSpace.Xyz), xyz)

            with self.subTest('Oklab to Oklch', color=color_name):
                oklch = oklab.to(ColorSpace.Oklch)
                self.assertEqual(oklch, values.oklch)

            with self.subTest('Oklch back to Oklab', color=color_name):
                self.assertEqual(oklch.to(ColorSpace.Oklab), oklab)


    def test_gamut_mapping(self) -> None:
        # A very green green
        green = Color.p3(0.0, 1.0, 0.0).to(ColorSpace.Srgb)
        self.assertEqual(
            green,
            Color(
                ColorSpace.Srgb,
                (-0.5116049825853448, 1.0182656579378029, -0.3106746212905826),
            ),
        )

        green_mapped = green.to_gamut()
        self.assertEqual(
            green_mapped,
            Color(
                ColorSpace.Srgb,
                (0.0, 0.9857637107710327, 0.15974244397343721)
            )
        )

        # A strong yellow
        yellow = Color.p3(1.0, 1.0, 0.0).to(ColorSpace.LinearSrgb)
        self.assertEqual(
            yellow,
            Color(
                ColorSpace.LinearSrgb,
                (1.0, 1.0000000000000002, -0.09827360014096621),
            ),
        )

        yellow_mapped = yellow.to_gamut()
        self.assertEqual(
            yellow_mapped,
            Color(
                ColorSpace.LinearSrgb,
                (0.9914525477996113, 0.9977581974546286, 0.0),
            ),
        )


    def test_x_parse_color(self) -> None:
        for text, expected in {
            'rgb:0/8/80': (0.0, 0.5333333333333333, 0.5019607843137255),
            'rgb:800/8000/0': (0.5001221001221001, 0.5000076295109483, 0.0),
            # 'rgbi:0/0.5/1': (0.0, 0.5, 1.0),
            # 'rgbi:1e0/1e-1/1e-2': (1.0, 0.1, 0.01),
        }.items():
            color1 = Color.parse(text)
            color2 = Color(ColorSpace.Srgb, expected)
            self.assertEqual(color1, color2)


    # def test_parameters(self) -> None:
    #     from prettypretty.style import rich

    #     for style, expected_params, inverse_params in (
    #         (rich().fg(-1).style(), (39,), ()),
    #         (rich().fg(1).style(), (31,), (39,)),
    #         (rich().bg(9).style(), (101,), (49,)),
    #         (rich().bg(240).style(), (48, 5, 240), (49,)),
    #         (rich().fg(ColorSpec('eight_bit', (12,))).style(), (38, 5, 12), (39,)),
    #     ):
    #         actual_params = style.sgr_parameters()
    #         self.assertSequenceEqual(actual_params, expected_params)
    #         actual_inverse = (~style).sgr_parameters()
    #         self.assertSequenceEqual(actual_inverse, inverse_params)
