"""Low-level support for assembling ANSI escape sequences"""
import enum
from typing import overload


class Layer(enum.Enum):
    """
    The display layer.

    Attributes:
        TEXT: is in the foreground
        BACKGROUND: is behind the text
    """
    TEXT = 0
    BACKGROUND = 10


class Ansi(enum.StrEnum):
    """
    An enumeration of ANSI escape sequence components.

    Attributes:
        ESC: is the escape character by itself
        CSI, DCS, OSC: start ANSI escape sequences; they are defined with the
            two-character C0 sequences and not the one-character C1 sequences,
            since the latter conflict with UTF-8
        BEL, ST: are interchangeable and terminate OSC sequences; the latter,
            again, is defined using the two-character C0 sequence.

    All enumeration constants are strings and hence can be directly used when
    assembling ANSI escape sequences. For example:

    .. code-block:: python

        print(f"{Ansi.CSI}1m" "Parrot!" f"{Ansi.CSI}m")

    The example prints a bold excited parrot to the terminal. It leverages
    Python's support for splitting string literals into more than one successive
    literal for clarity and the fact that missing ANSI parameters default to 0
    in the second ANSI escape sequence.

    Since ANSI escape sequences usually are more complex than the above, this
    class also defines :meth:`color_parameters` for converting colors to ANSI
    escape sequence parameters and :meth:`fuse` for fusing all these parts into
    a complete ANSI escape sequence. Using the methods, we can improve on the
    above example as follows:

    .. code-block:: python

        print(
            Ansi.fuse(Ansi.CSI, *Ansi.color_parameters(40), 1, 'm'),
            'Parrot!',
            Ansi.fuse(Ansi.CSI, 'm'),
        )

    The example prints a *green*, bold, excited parrot thanks to the 8-bit color
    40. Fuse not only joins its arguments, but it also inserts semicolons
    between parameters. In other words, the above example prints the same as the
    following statement:

    .. code-block:: python

        print("\\x1b[38;5;40;1m" "Parrot!" "\\x1b[m")

    The example is, again, leveraging Python's support for multiple consecutive
    string literals for improved clarity. Ironically, the Sphinx documentation
    processor mangles an escaped escape character ``\\x1b``, which is now
    written with an escaped backslash in the docstring source. Apparently, there
    is value in hiding escape characters behind enumeration constants...
    """
    BEL = '\a'
    CSI = '\x1b['
    DCS = '\x1bP'
    ESC = '\x1b'
    OSC = '\x1b]'
    ST = '\x1b\\'

    @overload
    @staticmethod
    def color_parameters(layer: Layer, color: int, /) -> tuple[int, ...]:
        ...
    @overload
    @staticmethod
    def color_parameters(layer: Layer, r: int, g: int, b: int, /) -> tuple[int, ...]:
        ...
    @staticmethod
    def color_parameters(
        layer: Layer, r: int, g: None | int = None, b: None | int = None,
    ) -> tuple[int, ...]:
        """
        Convert the 8-bit color or RGB256 coordinates to parameters for an SGR
        ANSI escape sequence. The layer argument determines whether the
        resulting parameters update the foreground or background color. To
        maximize compatibility, this method uses 30–37, 40–47, 90–97, and
        100–107 as parameters for setting the 16 extended ANSI color and the
        triple 38, 5, ``color`` only for the remaining 240 8-bit colors.
        """
        if g is not None:
            assert b is not None
            return 38 + layer.value, 2, r, g, b

        assert b is None
        if 0 <= r <= 7:
            return 30 + r + layer.value,
        elif 8 <= r <= 15:
            return 90 + r + layer.value - 8,
        elif 16 <= r <= 255:
            return 38 + layer.value, 5, r
        else:
            raise ValueError(f'"{r}" is not a valid 8-bit color')


    @staticmethod
    def fuse(*fragments: None | int | str) -> str:
        """
        Fuse the ANSI escape sequence fragments into a single string. This
        method treats ``None`` as a default parameter and replaces it with an
        empty string. It also inserts semicolons between parameters, i.e., when
        two successive arguments are either ``None`` or an integer.
        """
        processed: list[str] = []
        previous_was_parameter = False

        for fragment in fragments:
            current_is_parameter = fragment is None or isinstance(fragment, int)
            if previous_was_parameter and current_is_parameter:
                processed.append(';')
            processed.append('' if fragment is None else str(fragment))
            previous_was_parameter = current_is_parameter

        return ''.join(processed)
