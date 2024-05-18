import argparse
from collections.abc import Iterator
from contextlib import AbstractContextManager, contextmanager
import dataclasses
import enum
import json
import os
import select
import sys
import termios
import tty
from types import TracebackType
from typing import Any, cast, ClassVar, Never, Self, TextIO, TypeAlias


from .color.spec import ColorSpec
from .color.theme import Theme


TerminalMode: TypeAlias = list[Any]


class t:
    CC = 6
    LFLAG = 3


class e:
    BEL = '\a'
    CSI = '\x1b['
    DCS = '\x1bP'
    OSC = '\x1b]'
    Q = '?'
    SEMI = ';'
    ST = '\x1b\\'


def _fuse(*fragments: None | int | str) -> str:
    return ''.join('' if s is None else str(s) for s in fragments)


class BatchMode(enum.Enum):
    """
    A terminal's `batch mode
    <https://gist.github.com/christianparpart/d8a62cc1ab659194337d73e399004036>`_.

    Attributes:
        NOT_SUPPORTED: indicates that the terminal does not support batching
        ENABLED: indicates that the terminal is currently batching
        DISABLED: indicates that the terminal is not currently batching
        UNDEFINED: really is permanently enabled, which makes no sense
        PERMANENTLY_DISABLED: effectively is the same as the terminal not
            supporting batch mode

    Since three out of five status codes for batching more (``NOT_SUPPORTED``,
    ``PERMANENTLY_DISABLED``) or less (``UNDEFINED``) imply that the terminal
    doesn't support batching, this enumeration also defines the more precise
    computed properties ``is_supported``, ``is_enabled``, and ``is_disabled``.
    """
    NOT_SUPPORTED = 0
    ENABLED = 1
    DISABLED = 2
    UNDEFINED = 3
    PERMANENTLY_DISABLED = 4

    @property
    def is_supported(self) -> bool:
        """Determine whether the terminal supports batching."""
        return self is BatchMode.ENABLED or self is BatchMode.DISABLED

    @property
    def is_enabled(self) -> bool:
        """Determine whether the terminal is currently batching."""
        return self is BatchMode.ENABLED

    @property
    def is_disabled(self) -> bool:
        """Determine whether the terminal is currently not batching."""
        return self is not BatchMode.ENABLED


class TerminalContextManager(AbstractContextManager['TermIO']):
    """
    A context manager for terminal state.

    This class manages terminal updates. It ensures that updates are written in
    the right order, that the output is flushed only when necessary, and that
    the terminal is restored to its original state after use.

    To make this work, all updates must be registered with this class before
    entering a ``with`` block. Updates further consist of a pair of ANSI escape
    sequences, one to set up the new terminal state and one to restore the old
    terminal state.

    On entry, this class writes out the first ANSI escape sequence of every
    registered pair in registration order and then flushes the output. On exit,
    this class writes out the second sequence in reverse registration order and
    then flushes the output.

    Instances of this class are both reentrant and reusable. This class writes
    *nothing* to the terminal upon reentrant use, i.e., when ``__enter__()`` is
    invoked again before ``__exit__()``. But it writes the first sequences again
    upon reuse, when ``__enter__()`` is invoked again after ``__exit__()``.

    :class:`TermIO` has several methods with the same names and signatures as
    this class. They are the preferred way of creating terminal context manager
    instances because they are far more convenient. Assuming that ``term`` is an
    instance of ``TermIO``, this rather clumsy ``with`` statement

    .. code-block:: python

        with TerminalContextManager(term).alternate_screen().hidden_cursor():
            ...

    is equivalent to the far nicer

    .. code-block:: python

        with term.alternate_screen().hidden_cursor():
            ...

    """
    def __init__(self, terminal: 'TermIO') -> None:
        self._terminal = terminal
        self._updates: list[tuple[str, str]] = []
        self._block_depth = 0


    def register(self, do: str, undo: str) -> Self:
        """
        Register an update with this terminal context manager. Both ``do`` and
        ``undo`` should be ANSI escape sequences, with ``undo`` restoring the
        terminal to the state from before ``do``.
        """
        if self._block_depth > 0:
            raise ValueError(
                'unable to register updates after __enter__ has been called'
            )
        self._updates.append((do, undo))
        return self


    def window_title(self, title: str) -> Self:
        """Update the window title."""
        # Save window title on stack, then update window title
        return self.register(
            _fuse(e.CSI, "22;2t", e.OSC, "0;", title, e.ST),
            _fuse(e.CSI, "23;2t")
        )


    def alternate_screen(self) -> Self:
        """Switch to the terminal's alternate (unbuffered) screen."""
        return self.register(
            _fuse(e.CSI, "?1049h"),
            _fuse(e.CSI, "?1049l"),
        )


    def hidden_cursor(self) -> Self:
        """Make cursor invisible."""
        return self.register(
            _fuse(e.CSI, "?25l"),
            _fuse(e.CSI, "?25h"),
        )


    def bracketed_paste(self) -> Self:
        """
        Enable `bracketed pasting
        <https://gitlab.com/gnachman/iterm2/-/wikis/Paste-Bracketing>`_.
        """
        return self.register(
            _fuse(e.CSI, "?2004h"),
            _fuse(e.CSI, "?2004l"),
        )


    def __enter__(self) -> 'TermIO':
        if self._block_depth == 0:
            for do, _ in self._updates:
                self._terminal.write_control(do)
            self._terminal.flush()
        self._block_depth += 1
        return self._terminal


    def __exit__(
        self,
        exc_type: None | type[BaseException],
        exc_value: None | BaseException,
        traceback: None | TracebackType,
    ) -> None:
        if self._block_depth == 1:
            for _, undo in reversed(self._updates):
                self._terminal.write_control(undo)
            self._terminal.flush()
        self._block_depth -= 1


class TermIO:
    """
    Terminal input/output.

    This class uses a number of naming conventions:

      * Basic methods for emitting text or control sequences start with
        ``write``. Always use ``write()`` and ``writeln`` for content, but
        ``write_control()`` for control sequences.
      * Basic methods for ingesting text or control sequences start with
        ``read``. Reading control sequences is far more complicated than writing
        them because reading them requires correctly parsing them.
      * Basic methods for emitting a control sequence as request and then
        consuming a control sequence as response are named
        ``make_some_request()``, with some replaced by ``raw``, ``textual``, or
        ``numeric``. You can safely ignore them, unless you want to implement a
        query not currently supported by this class.
      * Methods that query the terminal about a specific property start with
        ``request``. That includes :meth:`request_size`, even though that method
        does not use control sequences.
      * Methods validate some condition and throw an exception if the condition
        does not hold start with ``check``.
    """
    def __init__(
        self,
        input: None | TextIO = None,
        output: None | TextIO = None
    ) -> None:
        self._input = input or sys.__stdin__
        self._input_fileno: int = self._input.fileno()
        self._output = output or sys.__stdout__

        self._width, self._height = self.request_size() or (80, 24)

    # ----------------------------------------------------------------------------------

    @property
    def width(self) -> int:
        return self._width

    @property
    def height(self) -> int:
        return self._height


    def request_size(self) -> None | tuple[int, int]:
        """
        Determine the terminal's size in fixed-width columns and rows. If the
        underlying platform hook fails for both input and output, typically
        because both input and output have been redirected, this method returns
        ``None``.
        """
        try:
            return os.get_terminal_size(self._input_fileno)
        except OSError:
            pass
        try:
            return os.get_terminal_size(self._output.fileno())
        except OSError:
            return None


    def update_size(self) -> Self:
        """Update the width and height cached by this class."""
        self._width, self._height = self.request_size() or (80, 24)
        return self


    def check_size_unmodified(self) -> Self:
        """
        Check that the terminal size has *not* changed since the last update.
        """
        w, h = self.request_size() or (80, 24)
        if self._width != w or self._height != h:
            raise AssertionError(
                f'terminal size changed from {self._width}×{self._height} to {w}×{h}'
            )
        return self

    # ----------------------------------------------------------------------------------

    def is_cbreak_mode(self, mode: None | TerminalMode = None) -> bool:
        """"
        Determine whether *cbreak mode* is enabled. This method inspects the
        current terminal mode to see whether characters are not echoed (``ECHO``
        is not set), line editing is disabled (``ICANON`` is not set), and reads
        return upon the first available character (``VMIN`` is 1, ``VTIME`` is
        0).

        Since raw mode makes the same changes and then some, this method detects
        raw mode as cbreak mode. That's just fine for its intended purpose,
        which is checking whether the terminal is prepared for handling ANSI
        escape sequences that require ANSI escape sequences as responses.
        """
        if mode is None:
            mode = termios.tcgetattr(self._input_fileno)

        # tty.setcbreak disables ECHO and ICANON, sets VMIN to 1, and VTIME to
        # 0. tty.setraw does the same and then some. For our purposes, it
        # suffices to check for cbreak's minimal settings, since we want the
        # terminal to respond right away and not stuck in line-buffer mode.
        return (
            not (mode[t.LFLAG] & termios.ECHO)
            and not (mode[t.LFLAG] & termios.ICANON)
            and mode[t.CC][termios.VMIN] == 1
            and mode[t.CC][termios.VTIME] == 0
        )


    def check_cbreak_mode(self) -> Self:
        """
        Check that cbreak mode is enabled. THis method signals an exception if
        cbreak mode is not enabled.
        """
        if not self.is_cbreak_mode():
            raise ValueError('terminal is expected to be in cbreak mode but is not')
        return self


    @contextmanager
    def cbreak_mode(self) -> Iterator[Self]:
        """
        Put the terminal into cbreak mode for the lifetime of the context.

        If the terminal is not yet in cbreak mode, the context manager sets
        cbreak mode upon entry and restores the previous mode upon exit. If the
        terminal is in cbreak mode already, the context manager does not modify
        the terminal mode, but it still restores the previous mode upon exit.
        Mode changes only take effect after all queued output has been written
        but queued input is discarded.
        """
        saved_mode = termios.tcgetattr(self._input_fileno)
        if not self.is_cbreak_mode(saved_mode):
            tty.setcbreak(self._input_fileno)
        try:
            yield self
        finally:
            termios.tcsetattr(self._input_fileno, termios.TCSAFLUSH, saved_mode)

    # ----------------------------------------------------------------------------------

    def write(self, s: str) -> Self:
        """
        Write the string to this terminal's output. This method does not flush
        the output.
        """
        self._output.write(s)
        return self


    def writeln(self, text: None | str = None) -> Self:
        """
        Write optional text and a line terminator to this terminal's output.
        This method does not flush the output.
        """
        if text is not None:
            self.write(text)
        self.write('\n')
        return self


    def write_control(self, *fragments: None | int | str) -> Self:
        """
        Write a control sequence to this terminal.

        This method combines the fragments of the control sequence, that is,
        ANSI escape sequence, into a string and writes the string to this
        terminal's output. This method does not flush the output.
        """
        self.write(''.join('' if q is None else str(q) for q in fragments))
        return self


    def flush(self) -> Self:
        """Flush this terminal's output."""
        self._output.flush()
        return self

    # ----------------------------------------------------------------------------------

    def read(self, *, length: int = 3, timeout: float = 0) -> bytes:
        """
        Read raw bytes from this terminal.

        This method reads up to ``length`` bytes from this terminal. If the
        ``timeout`` is 0, this method does *not* wait for input and immediately
        returns, possibly with an empty byte string. If the ``timeout`` is
        greater than 0, this method does wait for input, up to as many seconds,
        using ``select()``.

        This terminal must be in cbreak mode.
        """
        self.check_cbreak_mode()
        if timeout > 0:
            ready, _, _ = select.select([self._input_fileno], [], [], timeout)
            if not ready:
                raise TimeoutError()
        return os.read(self._input_fileno, length)

    ESCAPE_TIMEOUT: ClassVar[float] = 0.5

    def read_escape(self) -> bytes:
        """
        Read a complete ANSI escape sequence from this terminal.

        This method implements a reasonable but not entirely complete state
        machine for parsing ANSI escape sequences and keeps calling ``read()``
        for more bytes as necessary. It uses ``ESCAPE_TIMEOUT`` as timeout.

        The terminal must be in cbreak mode.
        """
        self.check_cbreak_mode()
        buffer = bytearray()

        def next_byte() -> int:
            b = self.read(length=1, timeout=self.ESCAPE_TIMEOUT)[0]
            buffer.append(b)
            return b

        def bad_byte(b: int) -> Never:
            raise ValueError(f"unexpected key code 0x{b:02X}")

        # TODO: Support ESC, CAN, SUB for cancellation

        b = next_byte()
        if b != 0x1B:
            bad_byte(b)

        # CSI Control Sequence
        # --------------------

        b = next_byte()
        if b == 0x5B:  # [
            b = next_byte()
            while 0x30 <= b <= 0x3F:
                b = next_byte()
            while 0x20 <= b <= 0x2F:
                b = next_byte()
            if 0x40 <= b <= 0x7E:
                return bytes(buffer)
            bad_byte(b)

        # DCS/SOS/OSC/PM/APC Control Sequence (Ending in ST)
        # --------------------------------------------------

        if b in (0x50, 0x58, 0x5D, 0x5E, 0x5F):  # P,X,],^,_
            b = next_byte()
            while b not in (0x07, 0x1B):
                b = next_byte()
            if b == 0x07:
                return bytes(buffer)
            b = next_byte()
            if b == 0x5C:  # \\
                return bytes(buffer)
            bad_byte(b)

        # Escape Sequence
        # ---------------

        while 0x20 <= b <= 0x2F:
            b = next_byte()
        if 0x30 <= b <= 0x7E:
            return bytes(buffer)
        bad_byte(b)

    # ----------------------------------------------------------------------------------

    def make_raw_request(self, *query: None | int | str) -> None | bytes:
        """
        Make a request to this terminal.

        This method writes a query as ANSI escape sequence to this terminal and
        then reads the corresponding response. This terminal must be in cbreak
        mode.
        """
        try:
            return (
                self
                .check_cbreak_mode()
                .write_control(*query)
                .flush()
                .read_escape()
            )
        except TimeoutError:
            return None


    def make_textual_request(
        self,
        *query: None | int | str,
        prefix: str,
        suffix: str,
    ) -> None | str:
        """
        Process a request with textual response for this terminal.

        This method write the request to this terminal, reads in the response,
        converts it to UTF8, checks that the resulting string starts with
        ``prefix`` and ends with ``suffix``, respectively, and returns the text
        in between. This method correctly accounts for the fact that ``ST`` and
        ``BEL`` may be used interchangeably at the end of OSC sequences. In such
        cases, please pass ``ST`` as  the suffix.

        .. warning::
            This method requires that ``prefix`` and ``suffix`` each are at
            least one character long.
        """
        byte_response = self.make_raw_request(*query)
        if byte_response is None:
            return None

        response = byte_response.decode('utf8')
        if not response.startswith(prefix):
            return None
        if not response.endswith(suffix):
            # BEL is just as valid as ST
            if suffix != e.ST or not response.endswith(e.BEL):
                return None
            suffix = e.BEL

        return response[len(prefix): -len(suffix)]


    def make_numeric_request(
        self,
        *query: None | int | str,
        prefix: bytes,
        suffix: bytes,
    ) -> list[int]:
        """
        Process a request with a numeric response for this terminal.

        This method write the request to this terminal, reads in the response,
        converts it to UTF8, checks that the resulting string starts with
        ``prefix`` and ends with ``suffix``, respectively, and returns the text
        in between. This method correctly accounts for the fact that ``ST`` and
        ``BEL`` may be used interchangeably at the end of OSC sequences. In such
        cases, please pass ``ST`` as  the suffix.

        .. warning::
            This method requires that ``prefix`` and ``suffix`` each are at
            least one character long.
        """
        byte_response = self.make_raw_request(*query)
        if byte_response is None:
            return []

        if not byte_response.startswith(prefix) or not byte_response.endswith(suffix):
            return []

        return [int(p) for p in byte_response[len(prefix): -len(suffix)].split(b';')]

    # ----------------------------------------------------------------------------------

    def request_terminal_version(self) -> None | str:
        """Request the terminal name and version."""
        terminal = self.make_textual_request(
            e.CSI, '>q', prefix=f'{e.DCS}>|', suffix=e.ST
        )
        return terminal


    def request_cursor_position(self) -> None | tuple[int, int]:
        """Request the cursor position in (x, y) order from this terminal."""
        numbers = self.make_numeric_request(
            e.CSI, '6n', prefix=b'\x1b[', suffix=b'R'
        )
        return None if len(numbers) != 2 else (numbers[0], numbers[1])


    def request_batch_mode(self) -> BatchMode:
        """Determine the terminal's current batch mode."""
        response = self.make_numeric_request(
            e.CSI, "?2026$p", prefix=b"\x1b[?2026;", suffix=b"$y"
        )
        return (
            BatchMode(response[0]) if len(response) == 1 else BatchMode.NOT_SUPPORTED
        )

    # ----------------------------------------------------------------------------------

    def _process_color(self, response: None | str) -> None | tuple[int, int, int]:
        if response is None or not response.startswith('rgb:'):
            return None
        return cast(
            tuple[int, int, int],
            tuple(int(v, base=16) for v in response[4:].split('/'))
        )


    def request_ansi_color(self, color: int) -> None | tuple[int, int, int]:
        """
        Determine the color for the given extended ANSI color. This method
        parses the color but does not normalize it. That matters because
        terminals respond to OSC-4 queries with colors that comprise four
        hexadecimal digits per component and hence have higher resolution than
        RGB256. The returned color coordinates could be called RGB65536, since
        they are integers ranging from 0 to 65,535, inclusive.
        """
        assert 0 <= color <= 15

        return self._process_color(self.make_textual_request(
            e.OSC, 4, e.SEMI, color, e.SEMI, e.Q, e.ST,
            prefix=f'{e.OSC}4;{color};',
            suffix=e.ST,
        ))


    def request_dynamic_color(self, code: int) -> None | tuple[int, int, int]:
        """
        Determine the color for the user interface element identified by
        ``code``:

            * 10 is the foreground or text color
            * 11 is the background color

        This method parses the color but does not normalize it. That matters
        because terminals respond to OSC-10 and OSC-11 queries with colors that
        comprise four hexadecimal digits per component and hence have higher
        resolution than RGB256. The returned color coordinates could be called
        RGB65536, since they are integers ranging from 0 to 65,535, inclusive.
        """
        assert 10 <= code <= 11

        return self._process_color(self.make_textual_request(
            e.OSC, code, e.SEMI, e.Q, e.ST,
            prefix=f'{e.OSC}{code};',
            suffix=e.ST,
        ))

    def extract_theme(self) -> None | Theme:
        """Extract the current color theme from the terminal."""
        colors: list[tuple[int, int, int]] = []

        for code in range(10, 12):
            color = self.request_dynamic_color(code)
            if color is None:
                return None
            colors.append(color)

        for code in range(16):
            color = self.request_ansi_color(code)
            if color is None:
                return None
            colors.append(color)

        return Theme(**{
            f.name: ColorSpec('srgb', (c[0] / 0xffff, c[1] / 0xffff, c[2] / 0xffff))
            for f, c in zip(dataclasses.fields(Theme), colors)
        })

    # ----------------------------------------------------------------------------------
    # Terminal Context

    def window_title(self, title: str) -> TerminalContextManager:
        """Use a different window title."""
        return TerminalContextManager(self).window_title(title)


    def alternate_screen(self) -> TerminalContextManager:
        """Switch to the terminal's alternate (unbuffered) screen."""
        return TerminalContextManager(self).alternate_screen()


    def hidden_cursor(self) -> TerminalContextManager:
        """Make cursor invisible."""
        return TerminalContextManager(self).hidden_cursor()


    def bracketed_paste(self) -> TerminalContextManager:
        """
        Enable `bracketed pasting
        <https://gitlab.com/gnachman/iterm2/-/wikis/Paste-Bracketing>`_.
        """
        return TerminalContextManager(self).bracketed_paste()

    # ----------------------------------------------------------------------------------

    def home(self) -> Self:
        """Move the cursor to the top left corner."""
        return self.write_control(e.CSI, ';H')

    def at(self, row: None | int = None, column: None | int = None) -> Self:
        """Move the cursor to the given row and column."""
        return self.write_control(e.CSI, row, e.SEMI, column, "H")

    def erase_screen(self) -> Self:
        """Erase the entire screen."""
        return self.write_control(e.CSI, '2J')

    def erase_line(self) -> Self:
        """Erase the entire current line."""
        return self.write_control(e.CSI, '2K')

    def reset_style(self) -> Self:
        """Reset all styles."""
        return self.write_control(e.CSI, 'm')

    def link(self, text: str, href: str, id: None | str = None) -> Self:
        """Mark a hyperlink."""
        # https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda
        code = f"8;id={id};" if id else "8;;"
        return (
            self
            .write_control(e.OSC, code, href, e.ST)
            .write(text)
            .write_control(e.OSC, "8;;", e.ST)
        )


def create_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group()
    group.add_argument(
        '--raw',
        action='store_const',
        const='raw',
        dest='format',
        help='show raw responses only',
    )
    group.add_argument(
        '--theme',
        action='store_const',
        const='theme',
        dest='format',
        help='show parsed theme'
    )
    return parser


if __name__ == '__main__':
    options = create_parser().parse_args()

    termio = TermIO()
    with termio.cbreak_mode():
        if options.format in (None, 'theme'):
            theme = termio.extract_theme()
            if theme is not None:
                for name, color in theme.colors():
                    cs = cast(tuple[float, float, float], color.coordinates)
                    print(f'{name:<15} {cs[0]:.5f}, {cs[1]:.5f}, {cs[2]:.5f}')
        else:
            for index, field in enumerate(dataclasses.fields(Theme)):
                if 0 <= index <= 1:
                    response = termio.make_raw_request(
                        e.OSC, 10 + index, e.SEMI, e.Q, e.ST
                    )
                else:
                    response = termio.make_raw_request(
                        e.OSC, 4, e.SEMI, index - 2, e.SEMI, e.Q, e.ST,
                    )
                if response is None:
                    print(f'{field.name:<15} error means halt!')
                    break
                print(f'{field.name:<15} {json.dumps(response.decode("utf8"))}')
