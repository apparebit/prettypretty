#!.venv/bin/python

import json
import re
import sys


PYTHON_BLOCK = re.compile(r"(?:^|\n)```py(?:thon)?\n([^`]*)```")


if __name__ == "__main__":
    if len(sys.argv) >= 2 and sys.argv[1] == "supports":
        sys.exit(0)

    context, book = json.load(sys.stdin)

    all_blocks: list[str] = []
    for section in book.get("sections", []):
        chapter = section.get("Chapter", None)
        if chapter is None:
            continue
        content = chapter.get("content", "")
        blocks = PYTHON_BLOCK.findall(content)
        all_blocks.extend(blocks)

    with open("pyextract.py", mode="w", encoding="utf8") as file:
        for block in all_blocks:
            file.write(block)
            file.write("\n\n")
        file.write("print(\"happy, happy, joy, joy!\")")
        file.flush()

    json.dump(book, sys.stdout)
