"""Low-level support for assembling ANSI escape sequences"""
import enum


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


class RawAnsi(bytes, enum.Enum):
    """
    An enumeration of raw ANSi escape sequence components.

    This is a simpler, ``bytes``-valued version of :class:`Ansi`; see its
    documentation for more details.

    Attributes:
        BEL:
        CSI:
        DCS:
        ST:
    """
    BEL = b'\a'
    CSI = b'\x1b['
    DCS = b'\x1bP'
    ST = b'\x1b\\'

    @staticmethod
    def fuse(*fragments: bytes) -> bytes:
        """Fuse the bytes fragments together."""
        return b''.join(fragments)
