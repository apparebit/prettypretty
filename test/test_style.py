import unittest

from prettypretty.color.style import ( # pyright: ignore [reportMissingModuleSource]
    Attribute, Format, FormatUpdate
)

class TestStyle(unittest.TestCase):

    def test_format(self) -> None:
        format = FormatUpdate.of(Format())
        attributes = [*format.disable().attributes()]
        self.assertListEqual(attributes, [])
        attributes = [*format.enable().attributes()]
        self.assertListEqual(attributes, [])

        format = FormatUpdate.of(Attribute.Bold)
        attributes = [*format.disable().attributes()]
        self.assertListEqual(attributes, [])
        attributes = [*format.enable().attributes()]
        self.assertListEqual(attributes, [Attribute.Bold])

        format = format + Attribute.Underlined
        attributes = [*format.disable().attributes()]
        self.assertListEqual(attributes, [])
        attributes = [*format.enable().attributes()]
        self.assertListEqual(attributes, [Attribute.Bold, Attribute.Underlined])

        format = -format
        attributes = [*format.disable().attributes()]
        self.assertListEqual(attributes, [Attribute.Bold, Attribute.Underlined])
        attributes = [*format.enable().attributes()]
        self.assertListEqual(attributes, [])

        format = format + Attribute.Thin
        attributes = [*format.disable().attributes()]
        self.assertListEqual(attributes, [Attribute.Underlined])
        attributes = [*format.enable().attributes()]
        self.assertListEqual(attributes, [Attribute.Thin])
