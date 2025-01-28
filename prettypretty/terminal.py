from collections.abc import Iterator, Sequence
from contextlib import AbstractContextManager, contextmanager, ExitStack
import enum
import os
import select
import sys
import termios
import textwrap
import tty
from typing import (
    Any,
    Callable,
    cast,
    ClassVar,
    ContextManager,
    Literal,
    Never,
    overload,
    Self,
    TextIO,
    TypeAlias,
)

from .ansi import Ansi, RawAnsi
from .color import Color, theme # pyright: ignore [reportMissingModuleSource]
from .color.style import (Fidelity, Layer, Style) # pyright: ignore [reportMissingModuleSource]
from .color.termco import ( # pyright: ignore [reportMissingModuleSource]
    AnsiColor, Colorant, EmbeddedRgb, GrayGradient, Rgb
)
from .theme import new_theme, current_translator
from .ident import identify_terminal, normalize_terminal_name


TerminalMode: TypeAlias = list[Any]


class TerminalModeComponent:
    CC = 6
    LFLAG = 3


_REFLECTED_METHODS = {
    'write_control', 'at', 'column', 'down', 'left', 'link', 'right', 'up'
}


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
        change_mode = not self._terminal.is_cbreak_mode(saved_mode)
        if change_mode:
            tty.setcbreak(fileno)
        try:
            yield self._terminal
        finally:
            if change_mode:
                termios.tcsetattr(fileno, termios.TCSAFLUSH, saved_mode)

    def cbreak_mode(self) -> Self:
        """
        Put the terminal into cbreak mode.

        If the terminal is not yet in cbreak mode, the context manager sets
        cbreak mode upon entry and restores the previous mode upon exit. If the
        terminal is in cbreak mode already, the context manager does not modify
        the terminal mode. Mode changes only take effect after all queued output
        has been written but queued input is discarded.
        """
        self._check_not_active()
        self._updates.append(lambda: self._cbreak_mode())
        return self

    def _request_theme(self) -> theme.Theme:
        with self._cbreak_mode():
            return self._terminal.request_theme()

    def terminal_theme(self, theme: None | theme.Theme = None) -> Self:
        """
        Use the terminal's color theme. Unless a theme is provided as argument,
        the context manager puts the terminal temporarily into cbreak mode and
        determines the current theme colors upon entry. It then makes that theme
        the current theme until exit.
        """
        self._check_not_active()
        if theme is None:
            factory = lambda: new_theme(self._request_theme())
        else:
            factory = lambda: new_theme(theme)
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
        control sequences. The latter automatically fuses fragments together,
        adding semicolons between empty and numeric parameters. It also checks
        that the terminal supports ANSI escapes. Finally, it provides a
        convenient hook for intercepting them.

        See :meth:`write`, :meth:`writeln`:, :meth:`write_control`, and
        :meth:`flush`; also :attr:`fidelity`, :meth:`check_output_tty`, and
        :meth:`check_tty`.

    **Reading terminal input and ingesting ANSI escapes**
        Python's standard library has extensive support for reading from
        streams, but only blocking calls including for line-oriented input are
        convenient to use. This class makes up for that by implementing support
        for character-oriented, non-blocking input as well as for ANSI escape
        sequences. The latter require three levels of parsing:

         1. Parse individual character to read an entire control sequence, no
            less, no more.
         2. Parse message to to separate the integral or textual payload from
            message header and tail.
         3. Parse text to extract terminal name, colors, etc.

        See :meth:`read` and :meth:`read_control`; also
        :meth:`make_raw_request`, :meth:`parse_textual_response`, and
        :meth:`parse_numeric_response`; also :meth:`request_terminal_identity`,
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

    **Simple updates of terminal state**
        Some terminal updates, notably for positioning the cursor and for
        erasing (parts of) the screen need not or can not be easily undone but
        still are eminently useful. You can also move the cursor and write links
        in :class:`.rich` text.

        See :meth:`up`, :meth:`down`, :meth:`left`, :meth:`right`,
        :meth:`set_position`, :meth:`set_column`, :meth:`erase_screen`,
        :meth:`erase_line`, and :meth:`link`.

    **Setting terminal styles**
        What's the point of integrating terminal colors with robust color
        management? Styling terminal output, of course! This class has methods
        to set bold, italic, or plain text and to set the fore/background
        colors. Those methods do not, however, adjust to the runtime context.
        For that, you want to use prettypretty's :class:`.Style` objects and
        :func:`rich` text.

        See :meth:`reset_style`, :meth:`rich_text`, :meth:`bold`,
        :meth:`italic`, :meth:`fg`, and :meth:`bg`.

    """
    def __init__(
        self,
        input: None | TextIO = None,
        output: None | TextIO = None,
        fidelity: None | Fidelity = None,
    ) -> None:
        self._identity: Literal[False] | None | tuple[str, str] = False

        self._input = cast(TextIO, input or sys.__stdin__)
        self._input_fileno = self._input.fileno()
        self._output = cast(TextIO, output or sys.__stdout__)
        self._all_tty = self._input.isatty() and self._output.isatty()

        if fidelity is None:
            fidelity = Fidelity.from_environment(self._output.isatty())
        self._fidelity = fidelity

        self._width, self._height = self.request_size() or (80, 24)

    # ----------------------------------------------------------------------------------

    @property
    def fidelity(self) -> Fidelity:
        """This terminal's color fidelity."""
        return self._fidelity

    def check_output_tty(self) -> Self:
        """Check that the output is a TTY."""
        if self._fidelity is Fidelity.Plain:
            raise ValueError('output does not accept ANSI escape sequences')
        return self

    def check_tty(self) -> Self:
        """Check that both input and output are TTYs."""
        if not self._all_tty:
            raise ValueError('input and output are not both TTYs')
        return self

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

    def write_paragraph(self, text: str) -> Self:
        """
        Write the paragraph to this terminal's output.

        This method strips all leading and trailing white space from each line
        of the text. It then treats each span of consecutive, non-empty lines as
        a paragraph and rewraps it to fit into the terminal width while still
        being convenient to read. Finally, it writes the resulting text to this
        terminal's output. This method does not flush the output.
        """
        lines = [line.strip() for line in text.splitlines()]
        start = stop = 0

        while True:
            # Skip empty lines
            while start < len(lines):
                if lines[start]:
                    break
                start += 1
            else:
                break

            # Determine span of non-empty lines
            stop = start + 1
            while stop < len(lines):
                if not lines[stop]:
                    break
                stop += 1

            # And rewrap the text in that span
            for line in textwrap.wrap(
                ' '.join(lines[start : stop]),
                width=min(self._width, 76),
                expand_tabs=False,
            ):
                self.writeln(line)

            # Repeat
            self.writeln()
            start = stop

        return self

    def write_control(self, *fragments: None | int | str) -> Self:
        """
        Write a control sequence to this terminal.

        This method :meth:`fuses <.Ansi.fuse>` the fragments of the inline
        control sequence (i.e., ANSI escape sequence) into a string and writes
        that string to this terminal's output. This method does not flush the
        terminal's output.

        This terminal's fidelity must not be :data:`Fidelity.PLAIN`, which is
        the case if the output is not a TTY. That restriction applies to all
        methods that write control sequences, since they always delegate to
        this method.
        """
        self.check_output_tty()
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
                raise TimeoutError("timed out waiting to read from terminal input")
        return os.read(self._input_fileno, length)

    ESCAPE_TIMEOUT: ClassVar[float] = 0.5

    def read_control(self) -> bytes:
        """
        Read a complete ANSI escape sequence from this terminal.

        This method implements a reasonable but not entirely complete state
        machine for parsing ANSI escape sequences and keeps calling ``read()``
        for more bytes as necessary. It uses ``ESCAPE_TIMEOUT`` as timeout.

        The terminal must have TTYs for input and output. It also must be in
        cbreak mode.
        """
        self.check_tty().check_cbreak_mode()
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

        The terminal must have TTYs for input and output. It also must be in
        cbreak mode.
        """
        try:
            return (
                self
                .check_tty()
                .check_cbreak_mode()
                .write_control(*query)
                .flush()
                .read_control()
            )
        except TimeoutError:
            return None

    def parse_textual_response(
        self,
        response: None | bytes,
        prefix: str,
        suffix: str,
    ) -> None | str:
        """
        Parse the terminal's textual response to an ANSI escape query.

        This method converts the response to a string, checks that is starts
        with the prefix and ends with the suffix, and then returns the text
        between prefix and suffix. If the suffix is ST, this method also allows
        BEL, as both are used interchangeably for terminating DSC/OSC.

        If the response is ``None`` or malformed, this method returns ``None``.
        """
        if response is None:
            return None

        r = response.decode('utf8')
        if not r.startswith(prefix):
            return None
        if not r.endswith(suffix):
            if suffix != Ansi.ST or not r.endswith(Ansi.BEL):
                return None
            suffix = Ansi.BEL

        end = -len(suffix) if suffix else len(r)
        return r[len(prefix): end]

    def parse_numeric_response(
        self,
        response: None | bytes,
        prefix: bytes,
        suffix: bytes,
    ) -> list[int]:
        """
        Parse the terminal's numeric response to an ANSI escape query.

        This method checks that the given response starts with the prefix and
        ends with the suffix, splits the bytes between prefix and suffix by
        semicolons, and parses the resulting byte fragments as integers. Empty
        byte fragments are parsed as -1. If the suffix ends with ST, this method
        also allows BEL, as both are used interchangeably for terminating
        DSC/OSC.

        If the response is ``None`` or malformed, this method returns an empty
        list.
        """
        if response is None or not response.startswith(prefix):
            return []

        if not response.endswith(suffix):
            # BEL may stand in for ST
            if not suffix.endswith(RawAnsi.ST):
                return []

            suffix = suffix[:-2] + RawAnsi.BEL
            if not response.endswith(suffix):
                return []

        # Allow for empty suffixes
        end = -len(suffix) if suffix else len(response)
        payload = response[len(prefix): end]

        # Allow for ``:`` separator in addition to ``;``
        params: list[int] = []
        for ps in payload.split(b';'):
            for p in ps.split(b':'):
                params.append(int(p) if p else -1)
        return params

    # ----------------------------------------------------------------------------------

    def _do_request_terminal_version(self) -> None | tuple[str, str]:
        response = self.parse_textual_response(
            self.make_raw_request(Ansi.CSI, '>q'),
            prefix=f'{Ansi.DCS}>|',
            suffix=Ansi.ST,
        )
        if response is None:
            return None

        if response.endswith(')'):
            name, _, version = response[:-1].partition('(')
        else:
            name, _, version = response.partition(' ')

        return name, version

    def request_terminal_identity(self) -> None | tuple[str, str]:
        """
        Request the terminal name and version.

        Since support for the CSI >q escape sequence for querying a terminal for
        its name and version is far from universal, this method employs the
        following strategies:

        1. Use CSI >q escape sequence to query terminal.
        2. Inspect the ``TERMINAL_PROGRAM`` and ``TERMINAL_PROGRAM_VERSION``
           environment variables.
        3. On macOS only, get the bundle identifier from the
           ``__CFBundleIdentifier`` environment variable and then use the
           ``mdfind`` and ``mdls`` command line tools to extract the bundle's
           version.

        If any of these methods is successful, this method normalizes the
        terminal name based on a list of known aliases. That includes bundle
        identifiers for Linux and macOS. It also caches the result and returns
        it for future invocations.

        The terminal must have TTYs for input and output. It also must be in
        cbreak mode.
        """
        if self._identity is False:
            identity = self._do_request_terminal_version()
            if identity:
                self._identity = normalize_terminal_name(identity[0]), identity[1]
            else:
                self._identity = identify_terminal()

        return self._identity

    def request_cursor_position(self) -> None | tuple[int, int]:
        """
        Request the cursor position in (x, y) order from this terminal.

        The terminal must have TTYs for input and output. It also must be in
        cbreak mode.
        """
        parameters = self.parse_numeric_response(
            self.make_raw_request(Ansi.CSI, '6n'),
            prefix=RawAnsi.CSI, suffix=b'R'
        )
        return None if len(parameters) != 2 else (parameters[0], parameters[1])

    def request_batch_mode(self) -> BatchMode:
        """
        Determine the terminal's current batch mode.

        The terminal must have TTYs for input and output. It also must be in
        cbreak mode.
        """
        parameters = self.parse_numeric_response(
            self.make_raw_request(Ansi.CSI, "?2026$p"),
            prefix=b"\x1b[?2026;",
            suffix=b"$y",
        )
        return (
            BatchMode(parameters[0])
            if len(parameters) == 1
            else BatchMode.NOT_SUPPORTED
        )

    def request_active_style(self) -> list[int]:
        """
        Request the terminal's current style settings.

        The returned list contains the corresponding SGR parameters. Terminals
        differ significantly in their support for this query. Since just this
        query would help determine color support levels, that is rather ironic.
        For instance, macOS Terminal.app does not handle the query, whereas
        Visual Studio Code's builtin terminal and iTerm 2 both respond with
        well-formed styles, which are completely wrong in case of Visual Studio
        Code.

        The terminal must have TTYs for input and output. It also must be in
        cbreak mode.
        """
        return self.parse_numeric_response(
            self.make_raw_request(Ansi.DCS, '$qm', Ansi.ST),
            prefix=RawAnsi.fuse(RawAnsi.DCS, b'1$r'),
            suffix=RawAnsi.fuse(b'm', RawAnsi.ST),
        )

    def request_color_support(self) -> None | Fidelity:
        """
        Request the terminal's color support.

        This method uses style queries to test for 24-bit and 8-bit color.

        The terminal must have TTYs for input and output. It also must be in
        cbreak mode. This method resets the current style.
        """
        response = (
            self
            .reset_style()
            .write_control(Ansi.CSI, '31m')
            .write_control(Ansi.CSI, '38;2;6;65;234m')
            .flush()
            .request_active_style()
        )

        self.reset_style().flush()

        if not response:
            return None
        if response == [0, 38, 2, 1, 6, 65, 234]:
            return Fidelity.TwentyFourBit

        response = (
            self
            .write_control(Ansi.CSI, '31m')
            .write_control(Ansi.CSI, '38;5;66m')
            .flush()
            .request_active_style()
        )

        self.reset_style().flush()
        if not response:
            return None
        elif response == [0, 38, 5, 66]:
            return Fidelity.EightBit
        else:
            return Fidelity.Ansi

    # ----------------------------------------------------------------------------------

    def _parse_color(
        self,
        index: int,
        name: str,
        response: None | bytes,
    ) -> Color:
        if response is None:
            raise ValueError(f"no response to request for {name}'s color")

        if index <= 1:
            prefix = f'{Ansi.OSC}{10 + index};'
        else:
            prefix = f'{Ansi.OSC}4;{index - 2};'

        r = self.parse_textual_response(response, prefix=prefix, suffix=Ansi.ST)
        if r is None or not r.startswith('rgb:'):
            raise ValueError(f"malformed response for {name}'s color")

        return Color.parse(r)

    def request_ansi_color(self, color: int) -> Color:
        """
        Determine the color for the given extended ANSI color. This method
        queries the terminal, parses the result, which by convention uses four
        hexadecimal digits per component, and normalizes it to sRGB.

        The terminal must have TTYs for input and output. It also must be in
        cbreak mode.
        """
        assert 0 <= color <= 15

        return self._parse_color(
            color + 2,
            theme.ThemeEntry.try_from_index(color + 2).name(),
            self.make_raw_request(Ansi.OSC, 4, color, ';?', Ansi.ST)
        )

    def request_dynamic_color(self, code: int) -> Color:
        """
        Determine the color for the user interface element identified by
        ``code``:

            * 10 is the foreground or text color
            * 11 is the background color

        This method queries the terminal, parses the result, which by convention
        uses four hexadecimal digits per component, and normalizes it to sRGB.

        The terminal must have TTYs for input and output. It also must be in
        cbreak mode.
        """
        assert 10 <= code <= 11

        return self._parse_color(
            code - 10,
            theme.ThemeEntry.try_from_index(code - 10).name(),
            self.make_raw_request(Ansi.OSC, code, ';?', Ansi.ST),
        )

    def _request_theme_v1(self) -> theme.Theme:
        # (1) Completely process each color.
        colors: list[Color] = []

        for code in range(10, 12):
            colors.append(self.request_dynamic_color(code))
        for code in range(16):
            colors.append(self.request_ansi_color(code))

        return theme.Theme(colors)

    def _request_theme_v2(self) -> theme.Theme:
        # (1) Write all requests. (2) Read + parse all responses.
        colors: list[Color] = []

        self.check_tty().check_cbreak_mode()

        for code in range(10, 12):
            self.write_control(Ansi.OSC, code, ';?', Ansi.ST)
        for color in range(16):
            self.write_control(Ansi.OSC, 4, color, ';?', Ansi.ST)
        self.flush()

        for index in range(18):
            response = self.read_control()
            colors.append(self._parse_color(
                index,
                theme.ThemeEntry.try_from_index(index).name(),
                response,
            ))

        return theme.Theme(colors)

    def _request_theme_v3(self) -> theme.Theme:
        # (1) Write all requests. (2) Read all responses. (3) Parse all responses.
        colors: list[Color] = []

        self.check_tty().check_cbreak_mode()

        for code in range(10, 12):
            self.write_control(Ansi.OSC, code, ';?', Ansi.ST)
        for color in range(16):
            self.write_control(Ansi.OSC, 4, color, ';?', Ansi.ST)
        self.flush()

        responses: list[None | bytes] = []
        for _ in range(18):
            responses.append(self.read_control())

        for index, response in enumerate(responses):
            colors.append(self._parse_color(
                index,
                theme.ThemeEntry.try_from_index(index).name(),
                response,
            ))

        return theme.Theme(colors)

    request_theme = _request_theme_v3
    """
    Request all theme colors from the terminal.

    Currently, there are three different implementations of this method:

     1. The first version completely processes one color at a time. It writes
        the query, then reads the response, and then parses the response.
     2. The second version operates in two phases: It first writes all 18
        queries and then reads and parses all 18 responses.
     3. The third version operates in three phases: It first writes all 18
        queries, then reads all 18 responses, and finally parses all 18
        responses.

    In my measurements, the second and third version take only half the time of
    the first version and, usually, the third version is a bit faster still. But
    I have seen a couple of spurious failures for terminal queries and hence
    expect the need for some retry logic, which would complicate things. So for
    now, I am not ready to commit to either of those three versions. If you feel
    like experimenting, you can run the microbenchmarks by running this module:

    .. code-block:: console

        python -m prettypretty.terminal

    You can also switch between versions by updating the assignment above this
    documentation comment in the source code. In either case, please report back
    about your experiences by `filing an issue
    <https://github.com/apparebit/prettypretty/issues/new/choose>`_.

    The terminal must have TTYs for input and output. It also must be in cbreak
    mode.
    """

    # ----------------------------------------------------------------------------------
    # Terminal Context

    def terminal_theme(self, theme: None | theme.Theme = None) -> TerminalContextManager:
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
        if self._fidelity is Fidelity.Plain:
            self.write(text)
            return self

        code = f"8;id={id};" if id else "8;;"
        return (
            self
            .write_control(Ansi.OSC, code, href, Ansi.ST)
            .write(text)
            .write_control(Ansi.OSC, "8;;", Ansi.ST)
        )

    # ----------------------------------------------------------------------------------

    def is_dark_theme(self) -> bool:
        """Determine whether the current theme is a dark theme."""
        return current_translator().is_dark_theme()

    def reset_style(self) -> Self:
        """Reset all styles."""
        return self.write_control(Ansi.CSI, 'm')

    @overload
    def render(self, __fragments: Sequence[Style|str]) -> Self:
        ...
    @overload
    def render(self, *__fragments: Style | str) -> Self:
        ...
    def render(self, *fragments: Sequence[Style|str] | Style | str) -> Self:
        """Write rich text to terminal output"""
        translator = current_translator()

        if len(fragments) == 1 and isinstance(fragments[0], Sequence):
            ff = fragments[0]
        else:
            ff = fragments

        for fragment in ff:
            if isinstance(fragment, str):
                self.write(fragment)
            else:
                assert isinstance(fragment, Style)
                self.write_control(str(fragment.cap(self._fidelity, translator)))

        return self

    def bold(self) -> Self:
        """Set bold style."""
        return self.write_control(Ansi.CSI, '1m')

    def italic(self) -> Self:
        """Set italic style."""
        return self.write_control(Ansi.CSI, '2m')

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
    def fg(
        self,
        color: int | AnsiColor | EmbeddedRgb | GrayGradient | Rgb | Color | Colorant,
        /
    ) -> Self:
        ...
    def fg(
        self,
        c1: int | AnsiColor | EmbeddedRgb | GrayGradient | Rgb | Color | Colorant,
        c2: None | int = None,
        c3: None | int = None,
    ) -> Self:
        """Set the foreground color."""
        if isinstance(c1, int):
            if c2 is None:
                assert c3 is None
                c1 = Colorant.of(c1)
            else:
                assert c3 is not None
                c1 = Colorant.of(Rgb(c1, c2, c3))
        elif isinstance(c1, Color):
            c1 = Colorant.of(c1)

        translator = current_translator()
        color = translator.cap(c1, self._fidelity)
        if color is not None:
            self.write_control(color.display(Layer.Foreground))
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
    def bg(
        self,
        color: int | AnsiColor | EmbeddedRgb | GrayGradient | Rgb | Color | Colorant,
        /
    ) -> Self:
        ...
    def bg(
        self,
        c1: int | AnsiColor | EmbeddedRgb | GrayGradient | Rgb | Color | Colorant,
        c2: None | int = None,
        c3: None | int = None,
    ) -> Self:
        """Set the background color."""
        if isinstance(c1, int):
            if c2 is None:
                assert c3 is None
                c1 = Colorant.of(c1)
            else:
                assert c3 is not None
                c1 = Colorant.of(Rgb(c1, c2, c3))
        elif isinstance(c1, Color):
            c1 = Colorant.of(c1)

        translator = current_translator()
        color = translator.cap(c1, self._fidelity)
        if color is not None:
            self.write_control(color.display(Layer.Background))
        return self


if __name__ == '__main__':
    import timeit

    with Terminal().cbreak_mode() as term:
        timer1: Any = timeit.Timer(lambda: term._request_theme_v1())  # type: ignore
        time_taken = timer1.timeit(10)
        print(f'1 stage : {time_taken / 10:.6f}')

        timer2: Any = timeit.Timer(lambda: term._request_theme_v2())  # type: ignore
        time_taken = timer2.timeit(10)
        print(f'2 stages: {time_taken / 10:.6f}')

        timer3: Any = timeit.Timer(lambda: term._request_theme_v3())  # type: ignore
        time_taken = timer3.timeit(10)
        print(f'3 stages: {time_taken / 10:.6f}')
