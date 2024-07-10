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
from typing import cast, Literal, overload, Self, TypeAlias, TypeVar

from .ansi import Ansi
from .color import Color, DefaultColor, Fidelity, Layer, TerminalColor
from .theme import current_sampler


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


def invert_color(color: None | TerminalColor, layer: Layer) -> None | TerminalColor:
    """Invert the given color."""
    if color is None or color.is_default():
        return None
    elif layer is Layer.Foreground:
        return TerminalColor.Default(DefaultColor.Foreground)
    else:
        return TerminalColor.Default(DefaultColor.Background)


class Instruction:
    """
    The superclass of all terminal instructions available to rich text.
    """
    __slots__ = ()

    def delegate(self) -> tuple[str, tuple[object, ...]]:
        raise NotImplementedError()

    @property
    def has_text(self) -> bool:
        return False


@dataclasses.dataclass(frozen=True, slots=True, kw_only=True)
class Style(Instruction):
    """
    A terminal style.

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
    :attr:`.Fidelity.NOCOLOR` indicates that the style does not contain any
    colors and :attr:`.Fidelity.PLAIN` indicates an empty style.

    .. note::

        Styles overload Python's inversion operator. The result of that
        operation is another style that restores the terminal to its default
        appearance. Styles also overload Python's subtraction operator, which
        returns the style that incrementally transitions from the second to the
        first style. Finally, the string representation of styles is the
        corresponding SGR ANSI escape sequence.

    Instances of this class are immutable.
    """
    weight: None | Weight = None
    slant: None | Slant = None
    underline: None | Underline = None
    overline: None | Overline = None
    strikeline: None | Strikeline = None
    coloring: None | Coloring = None
    visibility: None | Visibility = None
    foreground: None | TerminalColor = None
    background: None | TerminalColor = None
    fidelity: Fidelity = dataclasses.field(init=False)

    def __post_init__(self) -> None:
        if all(
            getattr(self, f.name) is None
            for f in dataclasses.fields(self)
            if f.name != 'fidelity'  # Careful, the attribute has not been set!
        ):
            # No styles
            fidelity = Fidelity.Plain
        elif self.foreground is None and self.background is None:
            # No colors
            fidelity = Fidelity.NoColor
        else:
            # Some color
            fg = self.foreground
            bg = self.background

            # Fill in the fidelity
            fg_fid = Fidelity.Plain if fg is None else Fidelity.from_color(fg)
            bg_fid = Fidelity.Plain if bg is None else Fidelity.from_color(bg)

            fidelity = max(fg_fid, bg_fid)

        object.__setattr__(self, 'fidelity', fidelity)

    @property
    def plain(self) -> bool:
        """The flag for an empty style specification."""
        return self.fidelity is Fidelity.Plain

    def __invert__(self) -> Self:
        if self.plain:
            # Nothing to invert
            return self

        return type(self)(
            weight = invert_attr(self.weight),
            slant = invert_attr(self.slant),
            underline = invert_attr(self.underline),
            overline = invert_attr(self.overline),
            strikeline = invert_attr(self.strikeline),
            coloring = invert_attr(self.coloring),
            visibility = invert_attr(self.visibility),
            foreground = invert_color(self.foreground, Layer.Foreground),
            background = invert_color(self.background, Layer.Background),
        )

    def __sub__(self, other: object) -> Self:
        if not isinstance(other, type(self)):
            return NotImplemented

        inverted_other = ~other
        if inverted_other.plain:
            return self

        return type(self)(
            weight = self.weight or inverted_other.weight,
            slant = self.slant or inverted_other.slant,
            underline = self.underline or inverted_other.underline,
            overline = self.overline or inverted_other.overline,
            strikeline = self.strikeline or inverted_other.strikeline,
            coloring = self.coloring or inverted_other.coloring,
            visibility = self.visibility or inverted_other.visibility,
            foreground = self.foreground or inverted_other.foreground,
            background = self.background or inverted_other.background,
        )

    def __or__(self, other: object) -> Self:
        if not isinstance(other, type(self)):
            return NotImplemented

        return type(self)(
            weight = self.weight or other.weight,
            slant = self.slant or other.slant,
            underline = self.underline or other.underline,
            overline = self.overline or other.overline,
            strikeline = self.strikeline or other.strikeline,
            coloring = self.coloring or other.coloring,
            visibility = self.visibility or other.visibility,
            foreground = self.foreground or other.foreground,
            background = self.background or other.background,
        )

    def prepare(self, fidelity: Fidelity) -> Self:
        """
        Adjust this style specification for rendering with the given fidelity.
        """
        if self.fidelity <= fidelity:
            return self

        if fidelity is Fidelity.Plain:
            # Erase all styles
            return type(self)()

        sampler = current_sampler()

        if self.foreground is None:
            fg = None
        else:
            fg = sampler.cap(self.foreground, fidelity)

        if self.background is None:
            bg = None
        else:
            bg = sampler.cap(self.background, fidelity)

        return dataclasses.replace(self, foreground=fg, background=bg)

    def sgr_parameters(self) -> list[int]:
        """Convert this style to the equivalent SGR parameters."""
        parameters: list[int] = []

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
            parameters.extend(self.foreground.sgr_parameters(Layer.Foreground))
        if self.background is not None:
            parameters.extend(self.background.sgr_parameters(Layer.Background))

        return parameters

    def sgr(self) -> str:
        """Convert this style specification into an SGR ANSI escape sequence."""
        return f'{Ansi.CSI}{";".join(str(p) for p in self.sgr_parameters())}m'

    def __str__(self) -> str:
        return self.sgr()

    def delegate(self) -> tuple[str, tuple[object, ...]]:
        return 'write_control', (self.sgr(),)


@dataclasses.dataclass(frozen=True, slots=True)
class Link(Instruction):
    """
    A hyperlink.

    Attributes:
        text:
        href:
        id:
    """
    text: str
    href: str
    id: None | str = None

    def delegate(self) -> tuple[str, tuple[object, ...]]:
        return 'link', (self.text, self.href, self.id)

    @property
    def has_text(self) -> bool:
        return bool(self.text)


@dataclasses.dataclass(frozen=True, slots=True)
class PlaceCursor(Instruction):
    """
    Cursor placement by row and column.

    Attributes:
        row:
        column:
    """
    row: None | int = None
    column: None | int = None

    def delegate(self) -> tuple[str, tuple[object, ...]]:
        return 'at', (self.row, self.column)


@dataclasses.dataclass(frozen=True, slots=True)
class MoveCursor(Instruction):
    """
    Cursor movement along one dimension.

    Attributes:
        move:
        offset:
    """
    move: Literal['up', 'down', 'left', 'right', 'column']
    offset: None | int = None

    def delegate(self) -> tuple[str, tuple[object, ...]]:
        return self.move, (self.offset,)


RichTextElement: TypeAlias = str | Instruction
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
        fidelity = Fidelity.Plain
        for fragment in self.fragments:
            if isinstance(fragment, str):
                continue
            if isinstance(fragment, Style):
                f = fragment.fidelity
            else:
                f = Fidelity.NoColor

            fidelity = max(f, fidelity)

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

        fragments: list[RichTextElement] = []
        for fragment in self.fragments:
            if isinstance(fragment, str) or fragment.has_text:
                # Keep all text
                pass
            elif fidelity is Fidelity.Plain:
                # Drop instructions on plain fidelity
                continue
            elif isinstance(fragment, Style):
                # Adjust style
                fragment = fragment.prepare(fidelity)
                if fragment.plain:
                    continue

            fragments.append(fragment)

        return type(self)(tuple(fragments))


class rich:
    """
    A rich builder.

    This class helps create styles and rich text through fluent property
    accesses and method invocations. In addition to text attributes and colors,
    it also tracks cursor movements and hyperlinks.

    A rich builder can have isolated or incremental styles. Isolated styles are
    the default for builders created by :meth:`rich`, whereas builders created
    by :meth:`rich.incremental` have incremental styles. The trade-off is that,
    with isolated styles, you don't need to worry about undoing styles because
    this class does that for you. However, you do need to worry about all styles
    standing on their own and cannot rely on the previous style contributing
    attributes. It's just the opposite for incremental styles. You can rely on
    the attributes of the previous style and incrementally add to them. But you
    also need to undo your own styles.

    For isolated and incremental styles alike, if you want to undo the current
    style without setting a new style, you can just call :meth:`undo_style` and
    let prettypretty figure out the needed antistyle. For isolated styles, that
    always is the empty style. For incremental styles, that style may just
    depend on all preceding styles.
    """
    def __init__(self, incremental: bool = False) -> None:
        self._elements: list[None | RichTextElement] = []
        self._last_style: None | int = None
        self._incremental = incremental

    @classmethod
    def incremental(cls) -> Self:
        """Create a new builder with incremental styles."""
        return cls(incremental=True)

    @overload
    def style(self) -> Style:
        ...
    @overload
    def style(self, style: Style) -> Self:
        ...
    def style(self, style: None | Style = None) -> Self | Style:
        """
        Get or add a style.

        If invoked without arguments, this method returns the last updated
        style. It does not consider any preceding styles, even if this rich
        builder is in incremental mode.

        If invoked with a style argument, this method adds the style to the rich
        text sequence being built. Whether the style is treated as isolated or
        incremental depends on this rich builder's mode.
        """
        if style is None:
            if self._last_style is None:
                raise ValueError('rich text without style')
            return cast(Style, self._elements[self._last_style])

        self._elements.append(style)
        self._last_style = len(self._elements) - 1
        return self

    def undo(self) -> Self:
        """Undo the currently effective style."""
        self._elements.append(None)
        return self

    def emit(self, close_style: bool = True) -> RichText:
        """
        Emit the built rich text sequence.

        This method computes effective styles to fill locations that undo the
        current style. As long as the argument allows, it also adds a closing
        style to restore the terminal's default appearance if there isn't one.
        """
        elements: list[RichTextElement] = []
        previous_style = None

        for element in self._elements:
            if element is not None and not isinstance(element, Style):
                elements.append(element)
                continue

            if previous_style is None:
                # Current style is the only style
                if element is not None:
                    elements.append(element)
                    previous_style = element
            elif element is None:
                # Undo previous style, in elements and in previous_style
                elements.append(~previous_style)
                previous_style = None
            elif self._incremental:
                # Incremental mode applies style as is, composes previous_style
                elements.append(element)
                previous_style = element | previous_style
            else:
                # Isolated mode applies style difference, sets previous_style
                elements.append(element - previous_style)
                previous_style = element

        if close_style and previous_style is not None and not previous_style.plain:
            elements.append(~previous_style)

        self._elements = []
        return RichText(tuple(elements))

    def _prepare_style(self) -> Style:
        if not self._elements or not isinstance(self._elements[-1], Style):
            self._elements.append(Style())
            self._last_style = len(self._elements) - 1

        style = self._elements[-1]
        assert isinstance(style, Style)
        return style

    def _handle_text_attribute(self, attribute: TextAttribute) -> Self:
        self._elements[-1] = dataclasses.replace(
            self._prepare_style(),
            **{type(attribute).__name__.lower(): attribute}
        )
        return self

    @property
    def regular(self) -> Self:
        """Update style with regular weight."""
        return self._handle_text_attribute(Weight.REGULAR)

    @property
    def light(self) -> Self:
        """Update style with light weight."""
        return self._handle_text_attribute(Weight.LIGHT)

    @property
    def bold(self) -> Self:
        """Update style with bold weight."""
        return self._handle_text_attribute(Weight.BOLD)

    @property
    def upright(self) -> Self:
        """Update style with upright."""
        return self._handle_text_attribute(Slant.UPRIGHT)

    @property
    def italic(self) -> Self:
        """Update style with italic."""
        return self._handle_text_attribute(Slant.ITALIC)

    @property
    def not_underlined(self) -> Self:
        """Update style with not underlined."""
        return self._handle_text_attribute(Underline.NOT_UNDERLINED)

    @property
    def underlined(self) -> Self:
        """Update style with underlined."""
        return self._handle_text_attribute(Underline.UNDERLINED)

    @property
    def not_overlined(self) -> Self:
        """Update style with not overlined."""
        return self._handle_text_attribute(Overline.NOT_OVERLINED)

    @property
    def overlined(self) -> Self:
        """Update style with overlined."""
        return self._handle_text_attribute(Overline.OVERLINED)

    @property
    def not_stricken(self) -> Self:
        """Update style with not stricken."""
        return self._handle_text_attribute(Strikeline.NOT_STRICKEN)

    @property
    def stricken(self) -> Self:
        """Update style with stricken."""
        return self._handle_text_attribute(Strikeline.STRICKEN)

    @property
    def not_reversed(self) -> Self:
        """Update style with background and foreground colors reversed."""
        return self._handle_text_attribute(Coloring.NOT_REVERSED)

    @property
    def reversed(self) -> Self:
        """Update style with background and foreground colors reversed."""
        return self._handle_text_attribute(Coloring.REVERSED)

    @property
    def not_hidden(self) -> Self:
        """Update style with not hidden."""
        return self._handle_text_attribute(Visibility.NOT_HIDDEN)

    @property
    def hidden(self) -> Self:
        """Update style with hidden."""
        return self._handle_text_attribute(Visibility.HIDDEN)

    @overload
    def fg(self, color: int, /) -> Self:
        ...
    @overload
    def fg(self, c1: int, c2: int, c3: int, /) -> Self:
        ...
    @overload
    def fg(self, color: Color, /) -> Self:
        ...
    @overload
    def fg(self, color: TerminalColor, /) -> Self:
        ...
    def fg(
        self,
        c1: int | Color | TerminalColor,
        c2: None | int = None,
        c3: None | int = None,
    ) -> Self:
        """Update style with foreground color."""
        if isinstance(c1, int):
            if c2 is not None:
                assert c3 is not None
                c1 = TerminalColor.from_24bit(c1, c2, c3)
            else:
                assert c3 is None
                c1 = TerminalColor.from_8bit(c1)
        elif isinstance(c1, Color):
            c1 = TerminalColor.from_color(c1)

        self._elements[-1] = dataclasses.replace(self._prepare_style(), foreground=c1)
        return self

    @overload
    def bg(self, color: int, /) -> Self:
        ...
    @overload
    def bg(self, c1: int, c2: int, c3: int, /) -> Self:
        ...
    @overload
    def bg(self, color: Color, /) -> Self:
        ...
    @overload
    def bg(self, color: TerminalColor, /) -> Self:
        ...
    def bg(
        self,
        c1: int | Color | TerminalColor,
        c2: None | int = None,
        c3: None | int = None,
    ) -> Self:
        """Update style with background color."""
        if isinstance(c1, int):
            if c2 is not None:
                assert c3 is not None
                c1 = TerminalColor.from_24bit(c1, c2, c3)
            else:
                assert c3 is None
                c1 = TerminalColor.from_8bit(c1)
        elif isinstance(c1, Color):
            c1 = TerminalColor.from_color(c1)

        self._elements[-1] = dataclasses.replace(self._prepare_style(), background=c1)
        return self

    def link(self, text: str, href: str, id: None | str = None) -> Self:
        """Add hyperlink."""
        self._elements.append(Link(text, href, id))
        return self

    def up(self, offset: None | int = None) -> Self:
        """Move cursor up."""
        self._elements.append(MoveCursor('up', offset))
        return self

    def down(self, offset: None | int = None) -> Self:
        """"Move cursor down."""
        self._elements.append(MoveCursor('down', offset))
        return self

    def left(self, offset: None | int = None) -> Self:
        """Move cursor left."""
        self._elements.append(MoveCursor('left', offset))
        return self

    def right(self, offset: None | int = None) -> Self:
        """Move cursor right."""
        self._elements.append(MoveCursor('right', offset))
        return self

    def column(self, offset: None | int = None) -> Self:
        """Move cursor to the given column."""
        self._elements.append(MoveCursor('column', offset))
        return self

    def at(self, row: None | int = None, column: None | int = None) -> Self:
        """Move cursor to the given position."""
        self._elements.append(PlaceCursor(row, column))
        return self

    def text(self, text: str = '') -> Self:
        """Add the given text."""
        self._elements.append(text)
        return self
