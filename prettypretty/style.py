import dataclasses
import enum
from typing import overload, Self, TypeAlias

from .ansi import Ansi, Layer


TerminalColor: TypeAlias = tuple[int] | tuple[int, int, int]



class TextAttribute(enum.Enum):
    """
    The superclass of all enumerations representing text attributes. It
    leverages convention to implement methods once instead of five times.
    """

    def is_default(self) -> bool:
        """Determine whether the text attribute is the default value."""
        return self.value == type(self)['DEFAULT'].value

    def __invert__(self) -> None | Self:
        """
        Invert the text attribute. This method returns the text attribute that
        restores the default appearance again. If this text attribute already
        sets the default state, this method returns ``None``.
        """
        if self.is_default():
            return None
        return type(self)['DEFAULT']


class Weight(TextAttribute):
    REGULAR = 22
    DEFAULT = 22
    BOLD = 1
    LIGHT = 2


class Slant(TextAttribute):
    UPRIGHT = 23
    DEFAULT = 23
    ITALIC = 3


class Underline(TextAttribute):
    NOT_UNDERLINED = 24
    DEFAULT = 24
    UNDERLINED = 4


class Coloring(TextAttribute):
    NOT_REVERSED = 27
    DEFAULT = 27
    REVERSED = 7


class Visibility(TextAttribute):
    NOT_HIDDEN = 28
    DEFAULT = 28
    HIDDEN = 8


DEFAULT_COLOR: tuple[int] = -1,


@dataclasses.dataclass(frozen=True, slots=True)
class Style:
    """
    A terminal style.

    Attributes:
        weight: is the font weight comprising *regular* (the default), *light*,
            and *bold*
        slant: is the font slant comprising *upright* (the default) and *italic*
        underlined: flags the inferior adornment and is either *not underlined*
            (the default) or *underlined*
        coloring: flags color order, which is either *not reversed* (the default)
            or *reversed*
        visibility: flags whether text is rendered after all and comprises *visible*
            (the default) and *invisible*
        foreground: is the foreground color
        background: is the background color

    The foreground and background colors each represent one more color in
    addition to the 8-bit terminal and RGB256 colors. If the color is a
    one-tuple with -1 as value, it denotes the default foreground or background
    color, which is distinct from 16 extended ANSI colors.

    This class supports the definition of terminal styles by themselves, maybe
    in a single module for the entire application. Such an application-wide
    style registry module greatly simplifies the reuse of styles. They all are
    defined in the same module after all. Centralization also helps with tuning
    styles so they harmonize with each other and are consistent with the design
    principles for the application's user interface. In cases where no such
    design system exists, a central module may help define one.

    Regarding color attributes and the methods for building styles, the
    attributes have the longer names because most code shouldn't need to access
    them. But many styles update at least one color.

    Instances of this class are immutable.
    """
    weight: None | Weight = None
    slant: None | Slant = None
    underline: None | Underline = None
    coloring: None | Coloring = None
    visibility: None | Visibility = None
    foreground: None | TerminalColor = None
    background: None | TerminalColor = None


    def __invert__(self) -> Self:
        weight = None if self.weight is None else ~self.weight
        slant = None if self.slant is None else ~self.slant
        underline = None if self.underline is None else ~self.underline
        coloring = None if self.coloring is None else ~self.coloring
        visibility = None if self.visibility is None else ~self.visibility

        foreground = (
            None
            if self.foreground is None or self.foreground == DEFAULT_COLOR
            else DEFAULT_COLOR
        )
        background = (
            None
            if self.background is None or self.background == DEFAULT_COLOR
            else DEFAULT_COLOR
        )

        return type(self)(
            weight,
            slant,
            underline,
            coloring,
            visibility,
            foreground,
            background,
        )


    @property
    def regular(self) -> Self:
        """Render text with regular weight."""
        return dataclasses.replace(self, weight=Weight.REGULAR)

    @property
    def light(self) -> Self:
        """Render text with light weight."""
        return dataclasses.replace(self, weight=Weight.LIGHT)

    @property
    def bold(self) -> Self:
        """Render text with bold weight."""
        return dataclasses.replace(self, weight=Weight.BOLD)

    @property
    def upright(self) -> Self:
        """Render text in upright."""
        return dataclasses.replace(self, slant=Slant.UPRIGHT)

    @property
    def italic(self) -> Self:
        """Render text in italic."""
        return dataclasses.replace(self, slant=Slant.ITALIC)

    @property
    def not_underlined(self) -> Self:
        """Render text underlined."""
        return dataclasses.replace(self, underlined=Underline.NOT_UNDERLINED)

    @property
    def underlined(self) -> Self:
        """Render text underlined."""
        return dataclasses.replace(self, underlined=Underline.UNDERLINED)

    @property
    def not_reversed(self) -> Self:
        """Render text with background and foreground colors reversed."""
        return dataclasses.replace(self, coloring=Coloring.NOT_REVERSED)

    @property
    def reversed(self) -> Self:
        """Render text with background and foreground colors reversed."""
        return dataclasses.replace(self, coloring=Coloring.REVERSED)

    @property
    def not_hidden(self) -> Self:
        """Do not render text."""
        return dataclasses.replace(self, visibility=Visibility.NOT_HIDDEN)

    @property
    def hidden(self) -> Self:
        """Do not render text."""
        return dataclasses.replace(self, visibility=Visibility.HIDDEN)

    @overload
    def fg(self, color: int, /) -> Self:
        ...
    @overload
    def fg(self, r: int, g: int, b: int, /) -> Self:
        ...
    def fg(self, r: int, g: None | int = None, b: None | int = None) -> Self:
        """
        Set the foreground color to the 8-bit terminal or RGB256 color. Terminal
        support for RGB256 or truecolor is spotty.
        """
        if g is None:
            assert b is None
            return dataclasses.replace(self, foreground=(r,))

        assert b is not None
        return dataclasses.replace(self, foreground=(r, g, b))


    @overload
    def bg(self, color: int, /) -> Self:
        ...
    @overload
    def bg(self, r: int, g: int, b: int, /) -> Self:
        ...
    def bg(self, r: int, g: None | int = None, b: None | int = None) -> Self:
        """
        Set the background color to the 8-bit terminal or RGB256 color. Terminal
        support for RGB256 or truecolor is spotty.
        """
        if g is None:
            assert b is None
            return dataclasses.replace(self, background=(r,))

        assert b is not None
        return dataclasses.replace(self, background=(r, g, b))


    def sgr_parameters(self) -> list[int]:
        """Convert this style to the equivalent SGR parameters."""
        parameters: list[int] = []

        if self.weight is not None:
            parameters.append(self.weight.value)
        if self.slant is not None:
            parameters.append(self.slant.value)
        if self.underline is not None:
            parameters.append(self.underline.value)
        if self.coloring is not None:
            parameters.append(self.coloring.value)
        if self.visibility is not None:
            parameters.append(self.visibility.value)
        if self.foreground is not None:
            if self.foreground == DEFAULT_COLOR:
                parameters.append(38)
            else:
                parameters.extend(Ansi.color_parameters(Layer.TEXT, *self.foreground))
        if self.background is not None:
            if self.background == DEFAULT_COLOR:
                parameters.append(48)
            else:
                parameters.extend(
                    Ansi.color_parameters(Layer.BACKGROUND, *self.background)
                )

        return parameters
