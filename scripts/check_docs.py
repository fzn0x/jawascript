"""Runs the example programs in the documentation and checks their output.

A ```ts block that declares `gawe utama` and is followed directly by a
```text block is a program and what it prints. Each one is run with
`jawac run`, and its standard output must match the text block exactly.
Other code blocks are left alone.

    python3 scripts/check_docs.py docs/guide.md
    python3 scripts/check_docs.py --jawac build/bin/jawac docs/*.md

The lit suite runs this on the documentation (tests/Docs).
"""

import argparse
import difflib
import os
import re
import shutil
import subprocess
import sys
import tempfile

FENCE = re.compile(r"^```(\w*)\n(.*?)^```[ \t]*$", re.MULTILINE | re.DOTALL)


def examples(text):
    """Yields (line, program, expected output) for each checked example."""
    blocks = list(FENCE.finditer(text))
    for block, after in zip(blocks, blocks[1:]):
        program_lang, program = block.group(1), block.group(2)
        between = text[block.end():after.start()]
        if (program_lang in ("ts", "jawa") and "gawe utama" in program
                and after.group(1) == "text" and not between.strip()):
            line = text.count("\n", 0, block.start()) + 1
            yield line, program, after.group(2)


def run(jawac, program, workdir):
    path = os.path.join(workdir, "conto.jw")
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(program)
    result = subprocess.run([jawac, "run", path], capture_output=True)
    output = result.stdout.decode("utf-8", "replace").replace("\r\n", "\n")
    errors = result.stderr.decode("utf-8", "replace")
    return result.returncode, output, errors


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("files", nargs="+", help="markdown files to check")
    parser.add_argument("--jawac", default="jawac", help="the compiler to run")
    args = parser.parse_args()

    jawac = shutil.which(args.jawac) or shutil.which(args.jawac + ".exe")
    if not jawac:
        parser.error(f"cannot find the compiler `{args.jawac}`")
    # Windows cannot start a relative path written with forward slashes.
    jawac = os.path.abspath(jawac)
    failures = 0
    with tempfile.TemporaryDirectory() as workdir:
        for name in args.files:
            with open(name, encoding="utf-8") as f:
                text = f.read()
            count = 0
            for line, program, expected in examples(text):
                count += 1
                status, output, errors = run(jawac, program, workdir)
                if status == 0 and output == expected:
                    continue
                failures += 1
                print(f"{name}:{line}: the example does not print what the text says")
                if status != 0:
                    print(f"  jawac exited with status {status}")
                    print("".join("  " + l for l in errors.splitlines(True)))
                diff = difflib.unified_diff(
                    expected.splitlines(True), output.splitlines(True),
                    "documented", "actual")
                print("".join("  " + l for l in diff))
            print(f"{os.path.basename(name)}: {count} examples")
    if failures:
        print(f"{failures} examples failed")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
