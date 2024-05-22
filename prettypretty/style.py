import dataclasses
import enum
from typing import cast, overload, Self, TypeAlias

from .ansi import Ansi, Layer
from .color.spec import ColorSpec


TerminalColor: TypeAlias = tuple[int] | tuple[int, int, int]


class TextAttribute(enum.Enum):
    """
    The superclass of all enumerations representing style choices. Each
    enumeration represents a choice of values, which is binary in the common
    case and ternary for :class:`Weight`. It also encodes the default state by
    aliasing the ``DEFAULT`` attribute. And it encodes SGR parameters, which are
    the values of the enumeration constants.

    :meth:`__invert__` uses the above information to automatically determine
    which text attributes must be set to restore the default appearance of
    terminal output again.
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
    """
    The font weight.

    Attributes:
        REGULAR, DEFAULT: mark the default weight
        BOLD: is a heavier weight
        LIGHT: is a lighter weight
    """
    REGULAR = 22
    DEFAULT = 22
    BOLD = 1
    LIGHT = 2


class Slant(TextAttribute):
    """
    The slant.

    Attributes:
        UPRIGHT, DEFAULT: mark the default, a lack of slant
        ITALIC: is text that is very much slanted
    """
    UPRIGHT = 23
    DEFAULT = 23
    ITALIC = 3


class Underline(TextAttribute):
    """
    Underlined or not.

    Attributes:
        NOT_UNDERLINED, DEFAULT: mark the default, which has no inferior lines
        UNDERLINED: marks text that should be underlined after all
    """
    NOT_UNDERLINED = 24
    DEFAULT = 24
    UNDERLINED = 4


class Coloring(TextAttribute):
    """
    Reversed or not.

    Attributes:
        NOT_REVERSED, DEFAULT: mark the default, which is inconspicuous
        REVERSED: reverses foreground and background colors
    """
    NOT_REVERSED = 27
    DEFAULT = 27
    REVERSED = 7


class Visibility(TextAttribute):
    """
    The visibility of text.

    Attributes:
        NOT_HIDDEN, DEFAULT: mark the default, which is inconspicuous
        HIDDEN: makes text invisible
    """
    NOT_HIDDEN = 28
    DEFAULT = 28
    HIDDEN = 8


DEFAULT_COLOR: tuple[int] = -1,
"""The default color."""


@dataclasses.dataclass(frozen=True, slots=True)
class Style:
    """
    A terminal style.

    This class combines one attribute for each enumeration of text attributes
    with two more attributes for foreground and background color. It only
    supports color formats that are supported by several terminals, i.e., 8-bit
    terminal colors and RGB256. At the same time, it supports one more color for
    foreground and background *each*, the **default color** represented by a
    unary tuple with -1 as value.

    Attributes:
        weight:
        slant:
        underline:
        coloring:
        visibility:
        foreground:
        background:

    This class supports the definition of terminal styles by themselves, maybe
    in a single module for the entire application. Such an application-wide
    style registry greatly simplifies the reuse of styles. They all are defined
    in the same module after all. Centralization also helps with tuning styles
    so that they harmonize with each other and the design guidelines. In case
    where no such guidelines exist, a central module may just help define them.

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


    def _handle_color(
        self, r: int | ColorSpec, g: None | int = None, b: None | int = None
    ) -> TerminalColor:
        if g is None:
            assert b is None
            if isinstance(r, int):
                return r,
            if r.tag in ('ansi', 'eight_bit', 'rgb256'):
                return cast(TerminalColor, r.coordinates)
            raise ValueError(f'{r.tag} is not a suitable color format or space')

        assert isinstance(r, int)
        assert b is not None
        return r, g, b


    @overload
    def fg(self, color: ColorSpec, /) -> Self:
        ...
    @overload
    def fg(self, color: int, /) -> Self:
        ...
    @overload
    def fg(self, r: int, g: int, b: int, /) -> Self:
        ...
    def fg(
        self, r: int | ColorSpec, g: None | int = None, b: None | int = None
    ) -> Self:
        """
        Set the foreground color to the 8-bit terminal or RGB256 color. Terminal
        support for RGB256 or truecolor is spotty.
        """
        return dataclasses.replace(self, foreground=self._handle_color(r, g, b))


    @overload
    def bg(self, color: ColorSpec, /) -> Self:
        ...
    @overload
    def bg(self, color: int, /) -> Self:
        ...
    @overload
    def bg(self, r: int, g: int, b: int, /) -> Self:
        ...
    def bg(
        self, r: int | ColorSpec, g: None | int = None, b: None | int = None
    ) -> Self:
        """
        Set the background color to the 8-bit terminal or RGB256 color. Terminal
        support for RGB256 or truecolor is spotty.
        """
        return dataclasses.replace(self, background=self._handle_color(r, g, b))


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
            parameters.extend(Ansi.color_parameters(Layer.TEXT, *self.foreground))
        if self.background is not None:
            parameters.extend(Ansi.color_parameters(Layer.BACKGROUND, *self.background))

        return parameters
