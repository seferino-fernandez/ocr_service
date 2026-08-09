#!/usr/bin/env python3
"""Compare one HTTP Recognition response with the bundled Tesseract reference."""

import argparse
import difflib
import json
from pathlib import Path
import sys


MINIMUM_SIMILARITY = 0.95


def fail(message: str) -> None:
    print(f"Recognition comparison failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def read_snapshot_text(path: Path) -> str:
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("text: "):
            value = json.loads(line.removeprefix("text: "))
            if isinstance(value, str):
                return value
            fail(f"snapshot text is not a string: {path}")

    fail(f"snapshot has no text field: {path}")


def read_response_text(path: Path) -> str:
    try:
        response = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        fail(f"response is not valid JSON: {error}")

    if not isinstance(response, dict):
        fail("response JSON is not an object")

    text = response.get("text")
    if not isinstance(text, str) or not text.strip():
        fail("response has no non-whitespace text")
    return text


def normalized(text: str) -> str:
    return " ".join(text.split())


def print_difference(expected: str, actual: str) -> None:
    difference = difflib.unified_diff(
        expected.splitlines(),
        actual.splitlines(),
        fromfile="bundled-reference",
        tofile="container-recognition",
        lineterm="",
    )
    print("Recognition drift (bundled reference -> container response):")
    print("\n".join(difference))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("reference", type=Path)
    parser.add_argument("response", type=Path)
    args = parser.parse_args()

    expected = read_snapshot_text(args.reference)
    actual = read_response_text(args.response)
    similarity = difflib.SequenceMatcher(
        None,
        normalized(expected),
        normalized(actual),
        autojunk=False,
    ).ratio()

    print(
        "Recognition similarity: "
        f"{similarity:.2%} (minimum {MINIMUM_SIMILARITY:.2%})"
    )
    if expected != actual:
        print_difference(expected, actual)

    if similarity < MINIMUM_SIMILARITY:
        fail("similarity is below the required minimum")

    print("Recognition comparison passed")


if __name__ == "__main__":
    main()
