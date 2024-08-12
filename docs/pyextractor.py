#!.venv/bin/python

import json
from pathlib import Path
import re
import sys
import textwrap
from typing import Any


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

        source_path = chapter.get("source_path")
        if source_path is None:
            source_path = ""

        previous_end = 0
        lines_so_far = 0

        for block in PYTHON_BLOCK.finditer(content):
            start_line = lines_so_far + content.count("\n", previous_end, block.start(1))
            text = block.group(1)

            blocks.append(f"# {source_path}:{start_line}\n{text}")

            previous_end = block.end()
            lines_so_far = start_line + text.count("\n")

        for sub_item in chapter.get("sub_items", []):
            traverse(sub_item)

    for item in items:
        traverse(item)

    return blocks


def main() -> None:
    if len(sys.argv) >= 2 and sys.argv[1] == "supports":
        sys.exit(0)

    _, book = json.load(sys.stdin)
    blocks = collect(book.get("sections", []))

    with open(Path(__file__).parent / "book.py", mode="w", encoding="utf8") as file:
        file.write(textwrap.dedent(
            """\
            # This script is automatically generated from markdown sources.
            # Please do *not* edit.
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
