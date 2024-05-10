#!.venv/bin/python

import os
import subprocess
import sys
import traceback
import unittest

from test.runtime import ResultAdapter, StyledStream


if __name__ == "__main__":
    successful = False
    stream = sys.stdout
    styled = StyledStream(stream)

    def println(s: str = "") -> None:
        if s:
            stream.write(s)
        stream.write("\n")
        stream.flush()

    try:
        println(styled.h1("§1 Setup"))
        println(styled.h2("Python"))
        println(f"{sys.executable}")
        println(styled.h2("Python Path"))
        for path in sys.path:
            println(f"{path}")
        println(styled.h2("Current Directory"))
        println(f"{os.getcwd()}")
        println(styled.h2("Current Module"))
        println(f"{__file__}")

        println(styled.h1("§2  Type Checking"))
        subprocess.run(["./node_modules/.bin/pyright"], check=True)

        println(styled.h1("§3  Unit Testing"))
        runner = unittest.main(
            module="test",
            exit=False,
            testRunner=unittest.TextTestRunner(
                stream=stream, resultclass=ResultAdapter
            ),
        )
        successful = runner.result.wasSuccessful()

    except subprocess.CalledProcessError:
        println(styled.failure("demicode failed to type check!"))
        exit(1)
    except Exception as x:
        trace = traceback.format_exception(x)
        println("".join(trace[:-1]))
        println(styled.err(trace[-1]))

    sys.exit(not successful)
