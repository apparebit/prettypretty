#!.venv/bin/python

import time
start_time = time.perf_counter_ns()

import json
import os
from pathlib import Path
import re
import sys
import textwrap
from typing import Any


DOCS_DIR = Path(__file__).parent
ROOT_DIR = DOCS_DIR.parent
SRC_DIR = DOCS_DIR / "src"

PYTHON_BLOCK = re.compile(r"(?:^|\n)```py(?:thon)?\n([^`]*)```")
PYTHON_HIDDEN_LINE = re.compile("(?:^|\n)~")


def collect(items: list[Any]) -> list[str]:
    blocks: list[str] = []

    def traverse(item: Any) -> None:
        if item is None:
            return

        chapter = item.get("Chapter")
        if chapter is None:
            return

        content = chapter.get("content")
        if content is None:
            return

        name = chapter.get("name", "")
        source_path = chapter.get("source_path", "")
        if source_path:
            source_path = (SRC_DIR / source_path).relative_to(ROOT_DIR)

        previous_end = 0
        lines_so_far = 0

        for block in PYTHON_BLOCK.finditer(content):
            start_line = lines_so_far + content.count("\n", previous_end, block.start(1))
            text = block.group(1)

            blocks.append(
                f"print('Testing file \"{source_path}\", line {start_line}, chapter "
                f"\"{name}\"')\n"
                f"{text}"
            )

            previous_end = block.end()
            lines_so_far = start_line + text.count("\n")

        for sub_item in chapter.get("sub_items", []):
            traverse(sub_item)

    for item in items:
        traverse(item)

    return blocks


def main() -> None:
    if len(sys.argv) >= 2 and sys.argv[1] == "supports":
        # The shell for GitHub Actions on Windows does not handle Unicode.
        # So just don't run in that environment.
        if "GITHUB_ACTION" in os.environ and os.name == "nt":
            sys.exit(1)
        sys.exit(0)

    _, book = json.load(sys.stdin)
    blocks = collect(book.get("sections", []))

    with open(DOCS_DIR / "book.py", mode="w", encoding="utf8") as file:
        file.write(textwrap.dedent(
            """\
            # This script is automatically generated from markdown sources.
            # Please do *not* edit.

            # pyright: reportMissingModuleSource=false

            import os
            import sys
            sys.path.insert(0, os.path.abspath("."))\n\n
            """
        ))

        for index, block in enumerate(blocks):
            file.write(f"def test{index}() -> None:\n    ")
            file.write(
                PYTHON_HIDDEN_LINE
                .sub("\n", block)
                .replace("\n", "\n    ")
                .strip()
            )
            file.write("\n\n\n")

        file.write('if __name__ == "__main__":\n')
        for index in range(len(blocks)):
            file.write(f"    test{index}()\n")
        file.write('    print("happy, happy, joy, joy!")\n')
        file.flush()

    json.dump(book, sys.stdout)


if __name__ == "__main__":
    main()

    n = time.perf_counter_ns() - start_time
    d = 1_000_000
    ms = (n + d // 2) // d
    print(f"pyextractor.py: \x1b[1;90mExtraction of Python code took {ms:,}ms\x1b[m", file=sys.stderr)
