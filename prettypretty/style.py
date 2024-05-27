"""
High-level support for terminal styles.

A terminal style covers the full repertoire of text attributes controllable via
ANSI escape codes. In addition to a fluent interface for assembling styles in
the first place, terminal styles support an algebra of inversion and difference
to easily compute the minimal changes necessary for restoring the default
appearance or transitioning to another appearance.

Reifying terminal styles this way not only simplifies updating the terminal, but
it also encourages the definition of terminal styles in a dedicated module. Such
an application-wide style registry greatly simplifies the reuse of styles. They
all are defined in the same module after all. Centralization also helps with
tuning styles so that they harmonize with each other and the design guidelines.
In case where no such guidelines exist, a central module may just help define
them.
"""
from collections.abc import Sequence
import dataclasses
import enum
from typing import cast, overload, Self, TypeAlias, TypeVar

from .ansi import Ansi, DEFAULT_COLOR, is_default, Layer
from .color.spec import ColorSpec
from .fidelity import Fidelity, FidelityTag


class TextAttribute(enum.Enum):
    """
    The superclass of all enumerations representing what should be orthogonal
    stylistic choices. Each enumeration represents a choice of values, which is
    binary in the common case and ternary for :class:`Weight`. It also encodes
    the default state by aliasing the ``DEFAULT`` attribute. Finally, it encodes
    SGR parameters, which are the values of the enumeration constants.

    :meth:`__invert__` uses the above information to automatically determine
    which text attributes must be set to restore the default appearance of
    terminal output again.
    """
    @property
    def is_default(self) -> bool:
        """Determine whether the text attribute is the default value."""
        return self.value == type(self)['DEFAULT'].value

    def __invert__(self) -> None | Self:
        """
        Invert the text attribute. This method returns the text attribute that
        restores the default appearance again. If this text attribute already
        sets the default state, this method returns ``None``.
        """
        if self.is_default:
            return None
        return type(self)['DEFAULT']


class Weight(TextAttribute):
    """
    The font weight.

    Attributes:
        REGULAR: is the regular, medium weight
        DEFAULT: marks the regular weight as the default
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
        UPRIGHT: is text without slant
        DEFAULT: marks upright text as the default
        ITALIC: is text that is very much slanted
    """
    UPRIGHT = 23
    DEFAULT = 23
    ITALIC = 3


class Underline(TextAttribute):
    """
    Underlined or not.

    Attributes:
        NOT_UNDERLINED: has no inferior line
        DEFAULT: marks the lack of lines as the default
        UNDERLINED: has inferior line
    """
    NOT_UNDERLINED = 24
    DEFAULT = 24
    UNDERLINED = 4


class Overline(TextAttribute):
    """
    Overlined or not.

    Attributes:
        NOT_OVERLINED: has no superior line
        DEFAULT: marks the lack of lines as the default
        OVERLINED: has superior line
    """
    NOT_OVERLINED = 55
    DEFAULT = 55
    OVERLINED = 53


class Strikeline(TextAttribute):
    """
    Stricken or not.

    Attributes:
        NOT_STRICKEN: has no line through
        DEFAULT: marks the lack of lines as the default
        STRICKEN: has a line through
    """
    NOT_STRICKEN = 29
    DEFAULT = 29
    STRICKEN = 9


class Coloring(TextAttribute):
    """
    Reversed or not.

    Attributes:
        NOT_REVERSED: is the inconspicuous, regular order
        DEFAULT: marks the regular order as the default
        REVERSED: reverses foreground and background colors
    """
    NOT_REVERSED = 27
    DEFAULT = 27
    REVERSED = 7


class Visibility(TextAttribute):
    """
    The visibility of text.

    Attributes:
        NOT_HIDDEN: makes text visible
        DEFAULT: marks visible text as the default
        HIDDEN: makes text invisible
    """
    NOT_HIDDEN = 28
    DEFAULT = 28
    HIDDEN = 8


TA = TypeVar('TA', bound=TextAttribute)


def invert_attr(attr: None | TA) -> None | TA:
    """Invert the given text attribute."""
    return None if attr is None else ~attr


def invert_color(color: None | ColorSpec) -> None | ColorSpec:
    """Invert the given color."""
    return None if color is None or is_default(color) else DEFAULT_COLOR


@dataclasses.dataclass(frozen=True, slots=True, kw_only=True)
class StyleSpec:
    """
    Specification of a terminal style.

    Attributes:
        weight: for font weight
        slant: for font slant
        underline: for inferior lines
        overline: for superior lines
        strikeline: for lines through
        coloring: for color order
        visibility: for visibility
        foreground: for foreground color
        background: for background color
        fidelity: is the minimum color fidelity and computed automatically,
            with ``None`` indicating unbounded fidelity

    This class captures the state of all text attributes controllable through
    ANSI escape sequences. It distinguishes between:

      * setting an attribute, e.g., when :attr:`underline` is
        :data:`Underline.UNDERLINED`;
      * unsetting an attribute, e.g., when :attr:`underline` is
        :data:`Underline.NOT_UNDERLINED`;
      * ignoring an attribute, e.g., when :attr:`underline` is ``None``.

    Since colors may just have to be downsampled or discarded for rendering even
    if styles only allow color formats directly supported by ANSI escape
    sequences, this class accepts all color formats and spaces while having
    robust facilities for adjusting formats as needed.

    The :attr:`fidelity` attribute is automatically computed during
    initialization and identifies the minimum fidelity level needed for
    rendering this style. A null fidelity indicates that this style contains
    arbitrary colors and hence has unbounded fidelity. Meanwhile
    :attr:`.Fidelity.NOCOLOR` indicates that the style specification does not
    contain any colors.

    .. note::

        Creating style specifications by invoking the constructor is *not*
        idiomatic. It simply is too verbose to be ergonomic. Instead,
        application code should use the fluent properties and methods defined by
        this class to instantiate the desired style specification off the
        module-level :data:`Style` object:

        .. code-block:: python

            WARNING = Style.bold.fg(0).bg(220)

        The example defines the warning style as bold black text on a
        yellow-orange background.

    .. note::

        Style specifications overload Python's inversion operator. The result of
        that operation is another style specification that restores the default
        terminal state after the original style specification.

        Also, the string representation of a style specification is the
        corresponding SGR ANSI escape sequence.

        The combination of the two features makes setting and unsetting styles
        rather convenient. For example, to set the above warning style, print
        " Warning! " (with a leading and trailing space) in that style, and then
        restore the default style, we just write:

        .. code-block:: python

            print(WARNING, 'Warning!', ~WARNING)

    .. note::

        Style specifications also overload Python's subtraction operator. The
        result of that operation is another style specification that takes the
        terminal state from the second specification to the first. Assuming that
        ``STYLE1`` and ``STYLE2`` are style specifications, the following two
        print statements are equivalent:

        .. code-block:: python

            print(STYLE2)
            print(STYLE1, STYLE2 - STYLE1)

    Instances of this class are immutable.
    """
    weight: None | Weight = None
    slant: None | Slant = None
    underline: None | Underline = None
    overline: None | Overline = None
    strikeline: None | Strikeline = None
    coloring: None | Coloring = None
    visibility: None | Visibility = None
    foreground: None | ColorSpec = None
    background: None | ColorSpec = None
    fidelity: None | Fidelity = dataclasses.field(init=False)

    def __post_init__(self) -> None:
        fg = self.foreground
        bg = self.background

        # Fill in the fidelity
        fg_fid = Fidelity.from_color(fg)
        bg_fid = Fidelity.from_color(bg)

        fidelity = None if fg_fid is None or bg_fid is None else max(fg_fid, bg_fid)
        object.__setattr__(self, 'fidelity', fidelity)

    def __invert__(self) -> Self:
        return type(self)(
            weight = invert_attr(self.weight),
            slant = invert_attr(self.slant),
            underline = invert_attr(self.underline),
            overline = invert_attr(self.overline),
            strikeline = invert_attr(self.strikeline),
            coloring = invert_attr(self.coloring),
            visibility = invert_attr(self.visibility),
            foreground = invert_color(self.foreground),
            background = invert_color(self.background),
        )

    def __sub__(self, other: object) -> Self:
        if not isinstance(other, StyleSpec):
            return NotImplemented

        not_other = ~other
        return type(self)(
            weight = self.weight or not_other.weight,
            slant = self.slant or not_other.slant,
            underline = self.underline or not_other.underline,
            overline = self.overline or not_other.overline,
            strikeline = self.strikeline or not_other.strikeline,
            coloring = self.coloring or not_other.coloring,
            visibility = self.visibility or not_other.visibility,
            foreground = self.foreground or not_other.foreground,
            background = self.background or not_other.background,
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
        """Render text not underlined."""
        return dataclasses.replace(self, underline=Underline.NOT_UNDERLINED)

    @property
    def underlined(self) -> Self:
        """Render text underlined."""
        return dataclasses.replace(self, underline=Underline.UNDERLINED)

    @property
    def not_overlined(self) -> Self:
        """Render text not overlined."""
        return dataclasses.replace(self, overline=Overline.NOT_OVERLINED)

    @property
    def overlined(self) -> Self:
        """Render text overlined."""
        return dataclasses.replace(self, overline=Overline.OVERLINED)

    @property
    def not_stricken(self) -> Self:
        """Render text not stricken."""
        return dataclasses.replace(self, strikeline=Strikeline.NOT_STRICKEN)

    @property
    def stricken(self) -> Self:
        """Render text stricken."""
        return dataclasses.replace(self, strikeline=Strikeline.STRICKEN)

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
    def fg(self, color: ColorSpec, /) -> Self:
        ...
    @overload
    def fg(self, tag: str, c: int, /) -> Self:
        ...
    @overload
    def fg(self, tag: str, c1: float, c2: float, c3: float, /) -> Self:
        ...
    def fg(
        self,
        color: int | str | ColorSpec,
        c1: None | float = None,
        c2: None | float = None,
        c3: None | float = None,
    ) -> Self:
        """Set the foreground color."""
        return dataclasses.replace(self, foreground=ColorSpec.of(color, c1, c2, c3))

    @overload
    def bg(self, color: int, /) -> Self:
        ...
    @overload
    def bg(self, color: ColorSpec, /) -> Self:
        ...
    @overload
    def bg(self, tag: str, c: int, /) -> Self:
        ...
    @overload
    def bg(self, tag: str, c1: float, c2: float, c3: float, /) -> Self:
        ...
    def bg(
        self,
        color: int | str | ColorSpec,
        c1: None | float = None,
        c2: None | float = None,
        c3: None | float = None,
    ) -> Self:
        """Set the background color."""
        return dataclasses.replace(self, background=ColorSpec.of(color, c1, c2, c3))

    def prepare(self, fidelity: Fidelity | FidelityTag) -> Self:
        """
        Adjust this style specification for rendering with the given fidelity.
        """
        fidelity = Fidelity.from_tag(fidelity)
        if self.fidelity is not None and self.fidelity <= fidelity:
            return self

        fg = fidelity.prepare_to_render(self.foreground)
        bg = fidelity.prepare_to_render(self.background)
        return dataclasses.replace(self, foreground=fg, background=bg)

    def sgr_parameters(self) -> list[int]:
        """Convert this style to the equivalent SGR parameters."""
        if self.fidelity is None:
            raise ValueError('style has unbounded color fidelity')

        parameters: list[int] = []

        def handle_color(layer: Layer, color: ColorSpec) -> None:
            parameters.extend(
                Ansi.color_parameters(
                    layer,
                    *cast(tuple[int, ...], color.coordinates),
                    use_ansi=color.tag=='ansi',
                )
            )

        if self.weight is not None:
            parameters.append(self.weight.value)
        if self.slant is not None:
            parameters.append(self.slant.value)
        if self.underline is not None:
            parameters.append(self.underline.value)
        if self.overline is not None:
            parameters.append(self.overline.value)
        if self.strikeline is not None:
            parameters.append(self.strikeline.value)
        if self.coloring is not None:
            parameters.append(self.coloring.value)
        if self.visibility is not None:
            parameters.append(self.visibility.value)
        if self.foreground is not None:
            handle_color(Layer.TEXT, self.foreground)
        if self.background is not None:
            handle_color(Layer.BACKGROUND, self.background)

        return parameters

    def sgr(self) -> str:
        """Convert this style specification into an SGR ANSI escape sequence."""
        return f'{Ansi.CSI}{";".join(str(p) for p in self.sgr_parameters())}m'

    def __str__(self) -> str:
        return self.sgr()


Style = StyleSpec()
"""
An empty style for starting fluent style configurations. It is used in the
example below to define the style for error messages as bold white text on a
deep red background.

.. code-block:: python

    ERROR_STYLE = Style.bold.fg(15).bg(88)

"""


RichTextElement: TypeAlias = str | StyleSpec
"""The type of all rich text elements."""


@dataclasses.dataclass(frozen=True, slots=True)
class RichText(Sequence[RichTextElement]):
    """
    The terminal version of rich text mixes text, which is to be rendered
    literally, and style specifications, which are to be rendered as SGR ANSI
    escape codes. The terminal version of rich text also tracks the current
    fidelity level and can be easily prepared for a different fidelity.
    """
    fragments: tuple[RichTextElement, ...]
    fidelity: None | Fidelity = dataclasses.field(init=False)

    def __post_init__(self) -> None:
        fidelity = Fidelity.NOCOLOR
        for fragment in self.fragments:
            if not isinstance(fragment, StyleSpec):
                continue

            f = fragment.fidelity
            if f is None:
                fidelity = None
                break
            fidelity = max(fidelity, f)

        object.__setattr__(self, 'fidelity', fidelity)

    @classmethod
    def of(cls, *fragments: RichTextElement) -> Self:
        """Create a rich text object from the the given fragments."""
        return cls(fragments)

    def __getitem__(self, index: int) -> RichTextElement:  # type: ignore
        return self.fragments[index]

    def __len__(self) -> int:
        return len(self.fragments)

    def prepare(self, fidelity: Fidelity) -> Self:
        """Prepare this rich text for rendering at the given fidelity."""
        if self.fidelity is not None and self.fidelity <= fidelity:
            return self
        return type(self)(tuple(
            f.prepare(fidelity) if isinstance(f, StyleSpec) else f
            for f in self.fragments
        ))
