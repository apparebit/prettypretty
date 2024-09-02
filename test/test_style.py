import unittest

from prettypretty.color.style.format import ( # pyright: ignore [reportMissingModuleSource]
    Attribute, Format
)

class TestStyle(unittest.TestCase):

    def test_format(self) -> None:
        format = Format()
        attributes = [*format.attributes()]
        self.assertListEqual(attributes, [])

        format = format.bold()
        attributes = [*format.attributes()]
        self.assertListEqual(attributes, [Attribute.Bold])

        format = format.underlined()
        attributes = [*format.attributes()]
        self.assertListEqual(attributes, [Attribute.Bold, Attribute.Underlined])

        format = ~format
        attributes = [*format.attributes()]
        self.assertListEqual(attributes, [Attribute.NotBoldOrThin, Attribute.NotUnderlined])

        format = format.thin()
        attributes = [*format.attributes()]
        self.assertListEqual(attributes, [Attribute.Thin, Attribute.NotUnderlined])
