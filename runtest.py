#!.venv/bin/python

# The shebang may point towards a venv, but GitHub CI executes python -m runtest

import os
import subprocess
import sys
import traceback
import unittest

from test.runtime import ResultAdapter, StyledStream


if __name__ == "__main__":
    stream = sys.stdout
    styled = StyledStream(stream)

    def println(s: str = "") -> None:
        if s:
            stream.write(s)
        stream.write("\n")
        stream.flush()

    println(styled.h1("1. Setup"))
    println(styled.h2("PYTHONIOENCODING"))
    println(os.environ.get("PYTHONIOENCODING", "n/a"))
    if os.name == "nt":
        println(styled.h2("PYTHONLEGACYWINDOWSFSENCODING"))
        println(os.environ.get("PYTHONLEGACYWINDOWSFSENCODING", "n/a"))
        println(styled.h2("PYTHONLEGACYWINDOWSSTDIO"))
        println(os.environ.get("PYTHONLEGACYWINDOWSSTDIO", "n/a"))
    println(styled.h2("PYTHONUTF8"))
    println(os.environ.get("PYTHONUTF8", "n/a"))
    println(styled.h2("Standard Out/Err Encoding"))
    println(sys.stdout.encoding)
    println(sys.stderr.encoding)
    println(styled.h2("Python"))
    println(f"{sys.executable}")
    println(styled.h2("Python Prefix"))
    println(f"{sys.prefix}")
    println(f"{sys.base_prefix}")
    println(styled.h2("Python Path"))
    for path in sys.path:
        println(f"{path}")
    println(styled.h2("Current Directory"))
    println(f"{os.getcwd()}")
    println(styled.h2("Current Module"))
    println(f"{__file__}")

    println(styled.h1("2. Type Checking"))
    try:
        subprocess.run(["npm", "run", "pyright"], check=True)
    except subprocess.CalledProcessError:
        println(styled.failure("prettypretty failed to type check!"))
        exit(1)

    println(styled.h1("3. Unit Testing"))
    try:
        runner = unittest.main(
            module="test",
            exit=False,
            testRunner=unittest.TextTestRunner(
                stream=stream, resultclass=ResultAdapter
            ),
        )
        sys.exit(not runner.result.wasSuccessful())
    except Exception as x:
        trace = traceback.format_exception(x)
        println("".join(trace[:-1]))
        println(styled.err(trace[-1]))
        sys.exit(1)
