"""
A nicer unittest runtime. Compared to pytest, unittest seems the less terrible
option because it is much simpler and doesn't have all the unpredictable magic.
But it still is pretty terrible, too. Notably, its implementation does not
separate the logic for running tests and tabulating results from the code that
tracks progress and displays results to users. Furthermore, while subtests are
an incredibly useful feature, they aren't treated as first class. Generally
nesting tests within tests would have been a cleaner model. Because of this
mingling of concerns, this module must necessarily access some private
attributes and mirror some gnarly aspects of the original. All that code is
hidden in the `testunit` adapter.
"""

import dataclasses
import inspect
import json
import os
from types import TracebackType
from typing import Any, Callable, TextIO, TypeAlias, TYPE_CHECKING
import unittest


ExcInfo: TypeAlias = tuple[type[BaseException], BaseException, TracebackType]
OptExcInfo: TypeAlias = ExcInfo | tuple[None, None, None]


@dataclasses.dataclass(frozen=True, slots=True)
class testunit:
    """Adapter for making instances of `unittest.TestCase` usable."""

    testcase: unittest.TestCase

    @property
    def is_subtest(self) -> bool:
        return self.testcase.__class__.__name__ == "_SubTest"

    @property
    def base(self) -> unittest.TestCase:
        return getattr(self.testcase, "test_case", self.testcase)

    @property
    def module(self) -> str:
        return self.base.__class__.__module__

    @property
    def source_file(self) -> None | str:
        return inspect.getsourcefile(self.base.__class__)

    @property
    def method(self) -> str:
        return self.base._testMethodName

    @property
    def message(self) -> str:
        """Get the message for subtests and an empty string otherwise."""
        msg = getattr(self.testcase, "_message", "")
        # Sigh, `_message` may have `_subtest_msg_sentinel` as value, which is
        # an arbitrary object. Avoid accessing more private state like so:
        return msg if isinstance(msg, str) else ""

    @property
    def params(self) -> dict[str, object]:
        """Get the parameters for subtests and empty dictionary otherwise."""
        return getattr(self.testcase, "params", {})

    @property
    def invocation(self) -> str:
        """
        Format a stylized method invocation suitable as human-readable
        identifier for the specific test case, which may be a subtest.
        """
        try:
            message = json.dumps(self.message) if self.message else ""
        except:
            print("***", self.message)
            message = ""
        params = ", ".join(f"{k}={v}" for k, v in self.params.items())
        between = f"{message}, {params}" if message and params else message + params
        if self.is_subtest and not between:
            between = "<subtest>"
        return f"{self.method}({between})"

    def is_success(self, err: None | OptExcInfo) -> bool:
        return err is None or err[0] is None

    def is_failure(self, err: None | OptExcInfo) -> bool:
        return (
            err is not None
            and err[0] is not None
            and issubclass(err[0], self.testcase.failureException)
        )


TIGHT_WIDTH = 70


class StyledStream:
    def __init__(self, stream: TextIO) -> None:
        self.stream = stream
        self.isatty = stream.isatty()
        try:
            self.width = os.get_terminal_size().columns
        except:
            self.width = 80

    @property
    def tight_width(self) -> int:
        return min(self.width, TIGHT_WIDTH)

    def _hn(self, dash: str, length: int, text: str) -> str:
        length = self.tight_width - 4 - length - 1
        return f"\n{dash * 3} {text} {dash * length}"

    def h0(self, text: str) -> str:
        return self._hn("═", len(text), self.strong(text))

    def h1(self, text: str) -> str:
        return self._hn("━", len(text), self.strong(text))

    def h2(self, text: str) -> str:
        return self._hn("─", len(text), self.italic(text))

    def sgr(self, ps: str, text: str) -> str:
        return f"\x1b[{ps}m{text}\x1b[0m" if self.isatty else text

    def pad(self, text: str) -> str:
        return text.ljust(self.tight_width) if self.isatty else text

    def heading(self, text: str) -> str:
        return self.sgr("1;48;5;153", text)

    def strong(self, text: str) -> str:
        return self.sgr("1", text)

    def light(self, text: str) -> str:
        return self.sgr("38;5;243", text)

    def italic(self, text: str) -> str:
        return self.sgr("3", text)

    def err(self, text: str) -> str:
        return self.sgr("38;5;88", text)

    def failure(self, text: str) -> str:
        return self.sgr("1;38;5;255;48;5;88", text)

    def success(self, text: str) -> str:
        return self.sgr("1;48;5;119", text)


BrokenTest: TypeAlias = tuple[testunit, str]
ProgressTracker: TypeAlias = Callable[[testunit, None | OptExcInfo], None]
ResultPrinter: TypeAlias = Callable[[int, list[BrokenTest], list[BrokenTest]], None]


def track_progress(stream: TextIO) -> ProgressTracker:
    columns = 0

    def track_progress(test: testunit, err: None | OptExcInfo) -> None:
        nonlocal columns

        if test.is_success(err):
            stream.write("⋅" if test.is_subtest else "•")
        elif test.is_failure(err):
            stream.write("f" if test.is_subtest else "F")
        else:
            stream.write("e" if test.is_subtest else "E")

        columns += 1
        if columns >= TIGHT_WIDTH:
            stream.write("\n")
            columns = 0

        stream.flush()

    return track_progress


def print_summary(stream: TextIO) -> ResultPrinter:
    styled = StyledStream(stream)

    def print1(label: str, test: testunit, trace: str) -> None:
        lines = trace.splitlines()
        stream.write(styled.strong(f"{label}: {test.source_file}: {test.invocation}"))
        stream.write("\n")
        stream.write(styled.light("\n".join(f"    {l}" for l in lines[:-1])))
        stream.write(f"\n    {styled.err(lines[-1])}\n\n")

    def print_summary(
        tests: int,
        failures: list[BrokenTest],
        errors: list[BrokenTest],
    ) -> None:
        stream.write("\n")

        broken = len(failures) + len(errors)
        if broken:
            stream.write("\n")
            for test, trace in failures:
                print1("FAIL", test, trace)
            for test, trace in errors:
                print1("ERROR", test, trace)

            stream.write(styled.h0(f"{broken}/{tests} Tests Failed"))
        else:
            stream.write(styled.h0(f"All {tests} Tests Passed!"))
        stream.write("\n\n")
        stream.flush()

    return print_summary


class ResultAdapter(unittest.TestResult if TYPE_CHECKING else object):
    def __init__(
        self,
        stream: TextIO,
        descriptions: bool,
        verbosity: int,
        *,
        result: None | unittest.TestResult = None,
        tracker: None | ProgressTracker = None,
        printer: None | ResultPrinter = None,
    ) -> None:
        self._stream = stream
        self._test_count = 0
        self._subtest_count = 0
        self._result = (
            unittest.TestResult(stream, descriptions, verbosity)
            if result is None
            else result
        )
        self._tracker = track_progress(stream) if tracker is None else tracker
        self._printer = print_summary(stream) if printer is None else printer

    def __getattr__(self, name: str) -> Any:
        if name.startswith("_") or not hasattr(self._result, name):
            raise AttributeError(name)
        return getattr(self._result, name)

    def startTest(self, test: unittest.case.TestCase) -> None:
        self._result.startTest(test)
        self._subtest_count = 0

    def stopTest(self, test: unittest.case.TestCase) -> None:
        self._result.stopTest(test)
        self._test_count += 1 if self._subtest_count == 0 else self._subtest_count

    def addSubTest(
        self,
        test: unittest.TestCase,
        subtest: unittest.TestCase,
        err: None | OptExcInfo,
    ) -> None:
        self._result.addSubTest(test, subtest, err)
        self._subtest_count += 1
        self._tracker(testunit(subtest), err)

    def addError(self, test: unittest.case.TestCase, err: OptExcInfo) -> None:
        self._result.addError(test, err)
        self._tracker(testunit(test), err)

    def addFailure(self, test: unittest.case.TestCase, err: OptExcInfo) -> None:
        self._result.addFailure(test, err)
        self._tracker(testunit(test), err)

    def addSuccess(self, test: unittest.case.TestCase) -> None:
        self._result.addSuccess(test)
        if self._subtest_count == 0:
            self._tracker(testunit(test), None)

    def printErrors(self) -> None:
        self._printer(
            self._test_count,
            [(testunit(test), trace) for test, trace in self._result.failures],
            [(testunit(test), trace) for test, trace in self._result.errors],
        )
