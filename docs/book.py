# This script is automatically generated from markdown sources.
# Please do *not* edit.

# pyright: reportMissingModuleSource=false

import os
import sys
sys.path.insert(0, os.path.abspath("."))


def test0() -> None:
    print('Testing file "docs/src/colors/2020s.md", line 141, chapter "2020s High-Resolution Colors"',
        file=sys.stderr)
    
    from prettypretty.color import Color, ColorSpace
    oklch = Color.oklch(0.716, 0.349, 335.0)
    p3 = oklch.to(ColorSpace.DisplayP3)
    assert p3.in_gamut()
    
    not_srgb = oklch.to(ColorSpace.Srgb)
    assert not not_srgb.in_gamut()
    
    srgb = not_srgb.to_gamut()
    assert srgb == Color.srgb(1.0, 0.15942348587138203, 0.9222706101768445)


def test1() -> None:
    print('Testing file "docs/src/colors/1970s.md", line 384, chapter "1970s Terminal Colors"',
        file=sys.stderr)
    
    from prettypretty.color import Color
    from prettypretty.color.style import AnsiColor, Colorant, EmbeddedRgb, GrayGradient
    from prettypretty.color.style import TrueColor
    red = AnsiColor.BrightRed
    assert red.to_8bit() == 9
    # What's the color value of ANSI red? We don't know!
    
    purple = EmbeddedRgb(3, 1, 4)
    index = 16 + 3 * 36 + 1 * 6 + 4 * 1
    assert index == 134
    assert purple.to_8bit() == index
    true_purple = TrueColor(*purple.to_24bit())
    assert true_purple == TrueColor(175, 95, 215)
    assert purple.to_color() == Color.from_24bit(175, 95, 215)
    
    gray = GrayGradient(18)
    index = 232 + 18
    assert index == 250
    assert gray.level() == 18
    assert gray.to_8bit() == index
    true_gray = TrueColor(*gray.to_24bit())
    assert true_gray == TrueColor(188, 188, 188)
    assert gray.to_color() == Color.from_24bit(188, 188, 188)
    
    green = Colorant.of(71)
    assert isinstance(green, Colorant.Embedded)
    also_green = green[0] # The only valid index is 0!
    assert also_green[0] == 1
    assert also_green[1] == 3
    assert also_green[2] == 1
    true_green = TrueColor(*green.try_to_24bit())
    assert true_green == TrueColor(95, 175, 95)
    assert also_green.to_color() == Color.from_24bit(95, 175, 95)


def test2() -> None:
    print('Testing file "docs/src/colors/integration.md", line 215, chapter "Accommodating All Colors"',
        file=sys.stderr)
    
    from prettypretty.color import Color, OkVersion
    from prettypretty.color.style import AnsiColor, Colorant, EmbeddedRgb
    from prettypretty.color.style import Fidelity, TrueColor
    from prettypretty.color.trans import Translator, VGA_COLORS
    red = VGA_COLORS[AnsiColor.BrightRed.to_8bit() + 2]
    assert red == Color.srgb(1.0, 0.333333333333333, 0.333333333333333)
    
    translator = Translator(OkVersion.Revised, VGA_COLORS)
    also_red = translator.resolve(AnsiColor.BrightRed)
    assert red == also_red
    
    black = translator.to_ansi(Color.srgb(0.15, 0.15, 0.15))
    assert black == AnsiColor.Black
    
    maroon = translator.cap(TrueColor(148, 23, 81), Fidelity.EightBit)
    assert maroon == Colorant.Embedded(EmbeddedRgb(2, 0, 1))


if __name__ == "__main__":
    test0()
    test1()
    test2()
    print("happy, happy, joy, joy!")
