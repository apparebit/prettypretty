import argparse
from collections.abc import Iterator
from contextlib import contextmanager
import dataclasses
import json
import os
import select
import sys
import termios
import tty
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


class TermIO:
    """
    Terminal input/output.
    """
    def __init__(
        self,
        input: None | TextIO = None,
        output: None | TextIO = None
    ) -> None:
        self._input = input or sys.__stdin__
        self._input_fileno: int = self._input.fileno()
        self._output = output or sys.__stdout__

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
    def cbread_mode(self) -> Iterator[Self]:
        """
        Put the terminal into cbreak mode for the lifetime of the context.

        If the terminal is not yet in cbreak mode, the context manager sets
        cbreak mode upon entry and restores the previous mode upon exit. If the
        terminal is in cbreak mode already, the context manager does not modify
        the terminal mode, but it still restores the previous mode upon exit.
        The mode change is immediate, irrespective of queued input or output.
        """
        saved_mode = termios.tcgetattr(self._input_fileno)
        if not self.is_cbreak_mode(saved_mode):
            tty.setcbreak(self._input_fileno)
        try:
            yield self
        finally:
            termios.tcsetattr(self._input_fileno, termios.TCSANOW, saved_mode)

    # ----------------------------------------------------------------------------------

    def write(self, s: str) -> Self:
        """Write the string to the output."""
        self._output.write(s)
        return self


    def writeln(self, s: str) -> Self:
        """Write the string followed by a line terminator to the output."""
        self.write(s)
        self.write('\n')
        return self


    def write_ansi(self, *query: int | str) -> Self:
        """
        Write the query comprising an ANSI escape sequence to the output. Like
        the other methods whose names start with ``write``, this method does
        *not* flush the output.
        """
        self.write(''.join(str(q) for q in query))
        return self


    def flush(self) -> Self:
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

    def make_raw_request(self, *query: int | str) -> None | bytes:
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
                .write_ansi(*query)
                .flush()
                .read_escape()
            )
        except TimeoutError:
            return None


    def make_textual_request(
        self,
        *query: int | str,
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
        *query: int | str,
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

    # TODO: Bracketing mode, alternative screen, erase screen, erase line,
    # show/hide cursor, goto


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
    with termio.cbread_mode():
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
