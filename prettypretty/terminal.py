from collections.abc import Iterator, Sequence
from contextlib import AbstractContextManager, contextmanager, ExitStack
import dataclasses
import enum
import os
import select
import sys
import termios
import tty
from typing import (
    Any,
    Callable,
    cast,
    ClassVar,
    ContextManager,
    Never,
    overload,
    Self,
    TextIO,
    TypeAlias,
)

from .ansi import Ansi, Layer
from .color.spec import ColorSpec
from .color.theme import current_theme, Theme
from .fidelity import environment_fidelity, Fidelity, FidelityTag
from .style import RichText, RichTextElement, StyleSpec


TerminalMode: TypeAlias = list[Any]


class TerminalModeComponent:
    CC = 6
    LFLAG = 3


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


class TerminalContextManager(AbstractContextManager['Terminal']):
    """
    A context manager for terminal state.

    This class manages operations that update and restore terminal state. It
    ensures that all updates are applied on entry and restored on exit, that
    updates are applied in registration order and restored in the opposite
    order, and that terminal output is flushed after applying and also after
    reverting all updates.

    An update may be a pair of escape sequences or a function that instantiates
    a context manager. In case of the former, the first sequence is written to
    the output upon entry and the second sequence upon exit. In case of the
    latter, the function and its result's ``__enter__()`` method are invoked on
    entry and the result's ``__exit__()`` method is invoked on exit. Currently,
    there is no public interface for registering the latter.

    This class is reentrant and reusable, though an instance does nothing on
    nested invocations.

    :class:`Terminal` has several methods with the same names and signatures as
    this class. They are the preferred way of creating terminal context manager
    instances because they are far more convenient. For example, to get started
    using prettypretty, you might write:

    .. code-block:: python

        terminal = Terminal()
        with TerminalContextManager(terminal).terminal_theme().scoped_style():
            ...

    is equivalent to this far nicer alternative

    .. code-block:: python

        with Terminal().alternate_screen().hidden_cursor() as terminal:
            ...

    """
    def __init__(self, terminal: 'Terminal') -> None:
        self._terminal = terminal
        self._updates: list[Callable[[], ContextManager[object]] | tuple[str, str]] = []
        self._block_depth = 0
        self._exit_stack: None | ExitStack = None

    def _check_not_active(self) -> None:
        if self._block_depth > 0:
            raise ValueError(
                'unable to update context manager after __enter__() has been called'
            )

    def register(self, do: str, undo: str) -> Self:
        """
        Register an update with this terminal context manager. Both ``do`` and
        ``undo`` should be ANSI escape sequences, with ``undo`` restoring the
        terminal to the state from before ``do``.
        """
        self._check_not_active()
        self._updates.append((do, undo))
        return self

    @contextmanager
    def _cbreak_mode(self) -> 'Iterator[Terminal]':
        fileno = self._terminal._input_fileno  # type: ignore[reportPrivateUsage]
        saved_mode = termios.tcgetattr(fileno)
        if not self._terminal.is_cbreak_mode(saved_mode):
            tty.setcbreak(fileno)
        try:
            yield self._terminal
        finally:
            termios.tcsetattr(fileno, termios.TCSAFLUSH, saved_mode)

    def cbreak_mode(self) -> Self:
        """
        Put the terminal into cbreak mode.

        If the terminal is not yet in cbreak mode, the context manager sets
        cbreak mode upon entry and restores the previous mode upon exit. If the
        terminal is in cbreak mode already, the context manager does not modify
        the terminal mode, but it still restores the previous mode upon exit.
        Mode changes only take effect after all queued output has been written
        but queued input is discarded.
        """
        self._check_not_active()
        self._updates.append(lambda: self._cbreak_mode())
        return self

    def _request_theme(self) -> Theme:
        with self._cbreak_mode():
            return self._terminal.request_theme()

    def terminal_theme(self, theme: None | Theme = None) -> Self:
        """
        Use the terminal's color theme. Unless a theme is provided as argument,
        the context manager puts the terminal temporarily into cbreak mode and
        determines the current theme colors upon entry. It then makes that theme
        the current theme until exit.
        """
        self._check_not_active()
        if theme is None:
            factory = lambda: current_theme(self._request_theme())
        else:
            factory = lambda: current_theme(theme)
        self._updates.append(factory)
        return self

    def window_title(self, title: str) -> Self:
        """Update the window title."""
        # Save window title on stack, then update window title
        return self.register(
            Ansi.fuse(Ansi.CSI, "22;2t", Ansi.OSC, "0;", title, Ansi.ST),
            Ansi.fuse(Ansi.CSI, "23;2t"),
        )

    def alternate_screen(self) -> Self:
        """Switch to the terminal's alternate (unbuffered) screen."""
        return self.register(
            Ansi.fuse(Ansi.CSI, "?1049h"),
            Ansi.fuse(Ansi.CSI, "?1049l"),
        )

    def hidden_cursor(self) -> Self:
        """Make cursor invisible."""
        return self.register(
            Ansi.fuse(Ansi.CSI, "?25l"),
            Ansi.fuse(Ansi.CSI, "?25h"),
        )

    def batched_output(self) -> Self:
        """
        Batch terminal output.

        While batching, a terminal temporarily delays updating the screen by
        buffering output. It avoids visual artifacts when rapidly updating the
        screen.
        """
        return self.register(
            Ansi.fuse(Ansi.CSI, "?2026h"),
            Ansi.fuse(Ansi.CSI, "?2026l"),
        )

    def bracketed_paste(self) -> Self:
        """
        Enable `bracketed pasting
        <https://gitlab.com/gnachman/iterm2/-/wikis/Paste-Bracketing>`_.
        """
        return self.register(
            Ansi.fuse(Ansi.CSI, "?2004h"),
            Ansi.fuse(Ansi.CSI, "?2004l"),
        )

    def scoped_style(self) -> Self:
        """Scope style changes by resetting the style on exit."""
        return self.register('', Ansi.fuse(Ansi.CSI, 'm'))

    def _create_control_writer(self, control: str) -> Callable[[], None]:
        def control_writer() -> None:
            self._terminal.write_control(control)
        return control_writer

    def __enter__(self) -> 'Terminal':
        if self._block_depth == 0:
            with ExitStack() as stack:
                # Protect against partial terminal updates with an exit stack.
                for update in self._updates:
                    if isinstance(update, tuple):
                        self._terminal.write_control(update[0])
                        stack.callback(self._create_control_writer(update[1]))
                    else:
                        stack.enter_context(update())

                # All updates have been processed. Flush terminal output.
                self._terminal.flush()

                # All went well. Delay clean up until __exit__().
                self._exit_stack = stack.pop_all()

        self._block_depth += 1
        return self._terminal

    def __exit__(self, *args: Any) -> bool:
        # Exception suppression isn't needed right now but easy enough to support
        received_exception = args[0] is not None
        suppressed_exception = False

        if self._block_depth == 1:
            assert self._exit_stack is not None
            suppressed_exception = bool(self._exit_stack.__exit__(*args))
            self._terminal.flush()
            self._exit_stack = None

        self._block_depth -= 1
        return received_exception and suppressed_exception


class Terminal:
    """
    Terminal input/output.

    This class manages terminal input and output. It encapsulates the nitty
    gritty of managing the terminal with ANSI escape codes behind methods with
    meaningful, human-readable names instead of cryptic mnemonics (what does ED
    do again and how does it differ from DECSED?). Furthermore, many operations
    that would leave the terminal in an unusable state upon

    This class supports the following features:

    **Terminal window size**
        ``Terminal`` caches the width and height of the terminal. It only
        updates the cached values if an application explicitly polls for
        changes. That way, the application is hopefully prepared to accommodate
        a terminal size change as well.

        See :attr:`width`, :attr:`height`, :meth:`request_size`,
        :meth:`update_size`, and :meth:`check_same_size`.

    **Cbreak mode**
        In this mode, the terminal does not support line editing; instead, it
        immediately forwards bytes. As such cbreak mode facilitates
        request/response interactions between application and terminal. Getting
        authoritative information from the terminal sure beats other, more
        indirect ways, such as environment variables, that help surmise
        specific conditions.

        See :meth:`is_cbreak_mode`, :meth:`check_cbreak_mode`, and
        :meth:`cbreak_mode`.

    **Writing to terminal output**
        This class exposes different methods for writing text and for writing
        control sequences. It helps add support for fragment fusion for the
        latter. More importantly, it supports applications that need to
        intercept the latter.

        See :meth:`write`, :meth:`writeln`:, :meth:`write_control`, and
        :meth:`flush`.

    **Reading terminal input and ingesting ANSI escapes**
        Python's standard library has extensive support for reading from
        streams. But when it comes to interactive use, only line-oriented input
        is reasonably well supported by standard library APIs. This class makes
        up for that by focusing on character-oriented input and in particular
        ANSI escape sequences. The latter require three levels of parsing:

         1. Character level to read an entire control sequence, no less, no more
         2. Message level to separate integral and textual parameters
         3. Semantic level to identify terminal properties

        See :meth:`read` and :meth:`read_control`; also
        :meth:`make_raw_request`, :meth:`make_textual_request`, and
        :meth:`make_numeric_request`; also :meth:`request_terminal_version`,
        :meth:`request_cursor_position`, :meth:`request_batch_mode`,
        :meth:`request_ansi_color`, :meth:`request_dynamic_color`, and
        :meth:`request_theme`.

    **Scoped changes of terminal state**
        To more easily update, restore, and flush terminal states, ``Terminal``
        delegates to a terminal context manager. It makes it possible to
        fluently queue up all restorable updates in a single ``with`` statement
        without worrying about many of the implementation details.

        See :meth:`cbreak_mode`, :meth:`terminal_theme`, :meth:`window_title`,
        :meth:`alternate_screen`, :meth:`hidden_cursor`, :meth:`batched_output`,
        :meth:`bracketed_paste`, and :meth:`scoped_style`.

    **Setting terminal state**
        Some terminal updates, notably for positioning the cursor and for
        erasing (parts of) the screen need not or can not be easily undone
        but still are eminently useful.

        See :meth:`up`, :meth:`down`, :meth:`left`, :meth:`right`,
        :meth:`set_position`, :meth:`set_column`, :meth:`erase_screen`,
        :meth:`erase_line`, and :meth:`link`.

    **Setting terminal styles**
        What's the point of integrating terminal colors with robust color
        management? Styling terminal output, of course! Alas, this class only
        has perfunctory support for styling the terminal on the quick. You
        probably want to use :data:`.Style` for defining styles instead and then
        apply them to this terminal.

        See :meth:`reset_style`, :meth:`rich_text`, :meth:`bold`,
        :meth:`italic`, :meth:`fg`, and :meth:`bg`.

    """
    def __init__(
        self,
        input: None | TextIO = None,
        output: None | TextIO = None,
        fidelity: None | Fidelity | FidelityTag = None,
    ) -> None:
        self._input = input or sys.__stdin__
        self._input_fileno: int = self._input.fileno()
        self._output = output or sys.__stdout__
        self._interactive = self._input.isatty() and self._output.isatty()
        if fidelity is None:
            fidelity = environment_fidelity(self._output.isatty())
        self._fidelity = Fidelity.from_tag(fidelity)
        self._width, self._height = self.request_size() or (80, 24)

    # ----------------------------------------------------------------------------------

    @property
    def fidelity(self) -> Fidelity:
        """This terminal's color fidelity."""
        return self._fidelity

    # ----------------------------------------------------------------------------------
    # Terminal Size

    @property
    def width(self) -> int:
        """The cached terminal width."""
        return self._width

    @property
    def height(self) -> int:
        """The cached terminal height."""
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

    def check_same_size(self) -> Self:
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
            not (mode[TerminalModeComponent.LFLAG] & termios.ECHO)
            and not (mode[TerminalModeComponent.LFLAG] & termios.ICANON)
            and mode[TerminalModeComponent.CC][termios.VMIN] == 1
            and mode[TerminalModeComponent.CC][termios.VTIME] == 0
        )

    def check_cbreak_mode(self) -> Self:
        """
        Check that cbreak mode is enabled. THis method signals an exception if
        cbreak mode is not enabled.
        """
        if not self.is_cbreak_mode():
            raise ValueError('terminal is expected to be in cbreak mode but is not')
        return self

    def cbreak_mode(self) -> TerminalContextManager:
        """
        Put the terminal into cbreak mode.

        If the terminal is not yet in cbreak mode, the context manager sets
        cbreak mode upon entry and restores the previous mode upon exit. If the
        terminal is in cbreak mode already, the context manager does not modify
        the terminal mode, but it still restores the previous mode upon exit.
        Mode changes only take effect after all queued output has been written
        but queued input is discarded.
        """
        return TerminalContextManager(self).cbreak_mode()

    # ----------------------------------------------------------------------------------

    def write(self, *fragments: str) -> Self:
        """
        Write the string fragments to this terminal's output. This method does
        not flush the output.
        """
        self._output.write(''.join(fragments))
        return self

    def writeln(self, *fragments: str) -> Self:
        """
        Write the string fragments followed by a line terminator to this
        terminal's output. This method does not flush the output.
        """
        self.write(*fragments, '\n')
        return self

    def write_control(self, *fragments: None | int | str) -> Self:
        """
        Write a control sequence to this terminal.

        This method :meth:`fuses <.Ansi.fuse>` the fragments of the inline
        control sequence (i.e., ANSI escape sequence) into a string and writes
        that string to this terminal's output. This method does not flush the
        terminal's output.

        This method's implementation does *not* delegate to the :meth:`write`
        method but directly writes to the terminal's output. While a separate
        method for writing control sequences helps prepare those sequences
        (here, by fusing the arguments), the primary motivation for exposing a
        separate method is to facilitate applications that need to separate
        content from styling etc.
        """
        self._output.write(Ansi.fuse(*fragments))
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

    def read_control(self) -> bytes:
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
        Make a request to this terminal. This method writes an ANSI escape
        sequence to this terminal as a query and then reads an ANSI escape
        sequence as the response.

        The terminal must be in cbreak mode.
        """
        try:
            return (
                self
                .check_cbreak_mode()
                .write_control(*query)
                .flush()
                .read_control()
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

        The terminal must be in cbreak mode.

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
            if suffix != Ansi.ST or not response.endswith(Ansi.BEL):
                return None
            suffix = Ansi.BEL

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

        The terminal must be in cbreak mode.

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
        """
        Request the terminal name and version. The terminal must be in cbreak
        mode.
        """
        terminal = self.make_textual_request(
            Ansi.CSI, '>q', prefix=f'{Ansi.DCS}>|', suffix=Ansi.ST
        )
        return terminal

    def request_cursor_position(self) -> None | tuple[int, int]:
        """
        Request the cursor position in (x, y) order from this terminal. The
        terminal must be in cbreak mode.
        """
        numbers = self.make_numeric_request(
            Ansi.CSI, '6n', prefix=b'\x1b[', suffix=b'R'
        )
        return None if len(numbers) != 2 else (numbers[0], numbers[1])

    def request_batch_mode(self) -> BatchMode:
        """
        Determine the terminal's current batch mode. The terminal must be in
        cbreak mode.
        """
        response = self.make_numeric_request(
            Ansi.CSI, "?2026$p", prefix=b"\x1b[?2026;", suffix=b"$y"
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

        The terminal must be in cbreak mode.
        """
        assert 0 <= color <= 15

        return self._process_color(self.make_textual_request(
            Ansi.OSC, 4, color, ';?', Ansi.ST,
            prefix=f'{Ansi.OSC}4;{color};',
            suffix=Ansi.ST,
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

        The terminal must be in cbreak mode.
        """
        assert 10 <= code <= 11

        return self._process_color(self.make_textual_request(
            Ansi.OSC, code, ';?', Ansi.ST,
            prefix=f'{Ansi.OSC}{code};',
            suffix=Ansi.ST,
        ))

    def request_theme(self) -> Theme:
        """
        Extract the entirety of the current color theme from the terminal. The
        terminal must be in cbreak mode.
        """
        colors: list[tuple[int, int, int]] = []

        for code in range(10, 12):
            color = self.request_dynamic_color(code)
            if color is None:
                raise ValueError(
                    f'Unable to determine value for default color {code - 9}'
                )
            colors.append(color)

        for code in range(16):
            color = self.request_ansi_color(code)
            if color is None:
                raise ValueError(f'Unable to determine value for ANSI color {code}')
            colors.append(color)

        return Theme(**{
            f.name: ColorSpec('srgb', (c[0] / 0xffff, c[1] / 0xffff, c[2] / 0xffff))
            for f, c in zip(dataclasses.fields(Theme), colors)
        })

    # ----------------------------------------------------------------------------------
    # Terminal Context

    def terminal_theme(self, theme: None | Theme = None) -> TerminalContextManager:
        """
        Use a different color theme. Unless a theme argument is provided, the
        implementation queries the terminal for its current theme, while
        temporarily putting the terminal in cbreak mode.
        """
        return TerminalContextManager(self).terminal_theme(theme)

    def window_title(self, title: str) -> TerminalContextManager:
        """Use a different window title."""
        return TerminalContextManager(self).window_title(title)

    def alternate_screen(self) -> TerminalContextManager:
        """Switch to the terminal's alternate (unbuffered) screen."""
        return TerminalContextManager(self).alternate_screen()

    def hidden_cursor(self) -> TerminalContextManager:
        """Make cursor invisible."""
        return TerminalContextManager(self).hidden_cursor()

    def batched_output(self) -> TerminalContextManager:
        """Batch terminal output."""
        return TerminalContextManager(self).batched_output()

    def bracketed_paste(self) -> TerminalContextManager:
        """
        Enable `bracketed pasting
        <https://gitlab.com/gnachman/iterm2/-/wikis/Paste-Bracketing>`_.
        """
        return TerminalContextManager(self).bracketed_paste()

    def scoped_style(self) -> TerminalContextManager:
        """
        Scope style changes by resetting the style on exit. The ``style()``
        context helps protect against unwanted style leakage upon unexpected
        exceptions or signals.
        """
        return TerminalContextManager(self).scoped_style()

    # ----------------------------------------------------------------------------------

    def up(self, rows: None | int = None) -> Self:
        """Move cursor up."""
        return self.write_control(Ansi.CSI, rows, 'A')

    def down(self, rows: None | int = None) -> Self:
        """Move cursor down."""
        return self.write_control(Ansi.CSI, rows, 'B')

    def left(self, columns: None | int = None) -> Self:
        """Move cursor left."""
        return self.write_control(Ansi.CSI, columns, 'C')

    def right(self, columns: None | int = None) -> Self:
        """Move cursor right."""
        return self.write_control(Ansi.CSI, columns, 'D')

    def at(self, row: None | int = None, column: None | int = None) -> Self:
        """Move the cursor to the given row and column."""
        return self.write_control(Ansi.CSI, row, column, 'H')

    def column(self, column: None | int = None) -> Self:
        """Move the cursor to the given column"""
        return self.write_control(Ansi.CSI, column, 'G')

    def erase_screen(self) -> Self:
        """Erase the entire screen."""
        return self.write_control(Ansi.CSI, '2J')

    def erase_line(self) -> Self:
        """Erase the entire current line."""
        return self.write_control(Ansi.CSI, '2K')

    def link(self, text: str, href: str, id: None | str = None) -> Self:
        """
        `Mark a hyperlink
        <https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda>`_.

        Underlined text should only be used for hyperlinks, in terminal
        emulators just as much as in documents and on web pages. That's just why
        this class does *not* have a separate method for styling text as
        underlined. If that's too stringent for your use case, please do `open
        an issue <https://github.com/apparebit/prettypretty/issues/new>`_.
        """
        code = f"8;id={id};" if id else "8;;"
        return (
            self
            .write_control(Ansi.OSC, code, href, Ansi.ST)
            .write(text)
            .write_control(Ansi.OSC, "8;;", Ansi.ST)
        )

    # ----------------------------------------------------------------------------------

    def reset_style(self) -> Self:
        """Reset all styles."""
        return self.write_control(Ansi.CSI, 'm')

    @overload
    def rich_text(self, fragments: Sequence[RichTextElement]) -> Self:
        ...
    @overload
    def rich_text(self, *fragments: RichTextElement) -> Self:
        ...
    def rich_text(
        self, *fragments: RichTextElement | Sequence[RichTextElement]
    ) -> Self:
        """Write rich text to terminal output"""
        # Coerce to actual rich text to simplify fidelity adjustment
        if (
            len(fragments) == 1
            and not isinstance(fragments[0], str)
            and isinstance(fragments[0], Sequence)
        ):
            # Some sequence that may be RichText already
            rich_text = fragments[0]
            if not isinstance(rich_text, RichText):
                rich_text = RichText(tuple(rich_text))
        else:
            # A tuple of strings and style specifications
            rich_text = RichText(cast(tuple[str | StyleSpec, ...], fragments))

        # Adjust rich text to terminal fidelity and then output contents
        for fragment in rich_text.prepare(self._fidelity):
            if isinstance(fragment, str):
                self.write(fragment)
            else:
                self.write_control(str(fragment))
        return self

    def bold(self) -> Self:
        """Set bold style."""
        return self.write_control(Ansi.CSI, '1m')

    def italic(self) -> Self:
        """Set italic style."""
        return self.write_control(Ansi.CSI, '2m')

    @overload
    def fg(self, color: ColorSpec, /) -> Self:
        ...
    @overload
    def fg(self, color: int, /) -> Self:
        ...
    @overload
    def fg(self, tag: str, c: int, /) -> Self:
        ...
    @overload
    def fg(self, tag: str, c1: float, c2: float, c3: float, /) -> Self:
        ...
    def fg(
        self,
        tag: int | str | ColorSpec,
        c1: None | float = None,
        c2: None | float = None,
        c3: None | float = None,
    ) -> Self:
        """Set the foreground color."""
        color = self._fidelity.prepare_to_render(ColorSpec.of(tag, c1, c2, c3))
        if color is not None:
            self.write_control(Ansi.CSI, *Ansi.color_parameters(
                Layer.TEXT, *cast(tuple[int, ...], color.coordinates)
            ), 'm')
        return self

    @overload
    def bg(self, color: ColorSpec, /) -> Self:
        ...
    @overload
    def bg(self, color: int, /) -> Self:
        ...
    @overload
    def bg(self, tag: str, c: int, /) -> Self:
        ...
    @overload
    def bg(self, tag: str, c1: float, c2: float, c3: float, /) -> Self:
        ...
    def bg(
        self,
        tag: int | str | ColorSpec,
        c1: None | float = None,
        c2: None | float = None,
        c3: None | float = None,
    ) -> Self:
        """Set the background color."""
        color = self._fidelity.prepare_to_render(ColorSpec.of(tag, c1, c2, c3))
        if color is not None:
            self.write_control(Ansi.CSI, *Ansi.color_parameters(
                Layer.BACKGROUND, *cast(tuple[int, ...], color.coordinates)
            ), 'm')
        return self
