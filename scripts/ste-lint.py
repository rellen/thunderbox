#!/usr/bin/env python3
"""Find the common faults of Simplified Technical English.

This script does not check the standard. No tool can do that, and CLAUDE.md
section 1 states so. The script finds five faults that a writer makes often:

- a sentence that holds more than 25 words;
- a word that ends in "ing";
- an em dash;
- the abbreviation "e.g.";
- the abbreviation "i.e.".

A reviewer still reads the text. This script only shortens the search.

The script reads the prose and nothing else. It ignores Rust code. It also
ignores each of these:

- the code inside a doc-test fence;
- the code inside a Markdown fence;
- a Markdown table;
- a span between two back quotes.

CLAUDE.md states that the standard does not cover any of those.

Run the script with no argument to check the whole project:

    python3 scripts/ste-lint.py

Run it with a path to check one file or one directory:

    python3 scripts/ste-lint.py crates/thunderbox-core

The script writes one line for each fault, and each line starts with a file name
and a line number. The script returns 0 when it finds no fault. It returns 1
when it finds one fault or more.

Run the self test to confirm that each check still works:

    python3 scripts/ste-lint.py --selftest
"""

import re
import sys
from pathlib import Path

# The longest sentence that the rules permit. Rule 1 sets 20 words for a
# sentence that gives an instruction, and 25 words for a sentence that
# describes something. The script cannot tell the two apart, so it uses the
# larger number and reports only a clear fault.
MAX_WORDS = 25

# Rule 3 permits an -ing form inside a technical name. This set holds the names
# that the project uses. Add a word here only when the word is a technical name
# or a noun, and never to hide a verb.
ALLOWED_ING = {
    "operating",  # as in "operating system"
    "warning",  # the name of a threshold in the theme
    "string",  # a Rust type
    "during",
    "according",
    "nothing",
    "something",
    "anything",
    "everything",
}

SKIP_DIRS = {".git", "target", "node_modules"}


def strip_spans(text, in_span):
    """Remove each span between two back quotes, and return the new state.

    A span can start on one line and end on the next line. The caller therefore
    passes the state of the last line back into this function.
    """
    out = []
    for character in text:
        if character == "`":
            in_span = not in_span
            if not in_span:
                out.append("X")
            continue
        if not in_span:
            out.append(character)
    return "".join(out), in_span


def prose_lines(path):
    """Return each line of prose, as a pair of a line number and the text.

    The function removes the marks of the comment or of the Markdown. It also
    removes every part that the standard does not cover.
    """
    out = []
    in_fence = False
    in_span = False
    suffix = path.suffix
    for number, line in enumerate(path.read_text(errors="replace").split("\n"), 1):
        if suffix == ".rs":
            match = re.match(r"\s*(///|//!|//)\s?(.*)", line)
            if not match:
                continue
            text = match.group(2)
        elif suffix == ".md":
            text = line
        else:
            continue
        stripped = text.strip()
        if stripped.startswith("```"):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        if suffix == ".md" and (stripped.startswith("|") or stripped.startswith("#")):
            continue
        # A span between two back quotes holds code, or the name of an item.
        # Such a span can cross a line break, so the state carries forward.
        text, in_span = strip_spans(text, in_span)
        # A pair of asterisks marks bold text. The marks break a sentence.
        text = text.replace("**", "").replace("*", "")
        out.append((number, text))
    return out


def find_faults(path):
    """Return a list of faults. Each fault is a line number and a message."""
    faults = []
    lines = prose_lines(path)

    for number, text in lines:
        for pattern, message in (
            ("—", "This line holds an em dash. Rule 8 forbids it."),
            (r"\be\.g\.", 'This line holds "e.g.". Rule 9 asks for "for example".'),
            (r"\bi\.e\.", 'This line holds "i.e.". Rule 9 asks for "that is".'),
        ):
            if re.search(pattern, text):
                faults.append((number, message))
        for match in re.finditer(r"\b([A-Za-z]{4,}ing)\b", text):
            word = match.group(1).lower()
            if word not in ALLOWED_ING:
                faults.append(
                    (number, f'The word "{match.group(1)}" is an -ing form. Rule 3 forbids it.')
                )

    # A sentence can cross several lines, so the check needs a stream of words
    # that remembers the line of each word.
    words = []
    for number, text in lines:
        if not text.strip():
            words.append(None)  # A blank line ends a paragraph.
            continue
        if re.match(r"\s*[-*]\s+", text):
            words.append(None)  # A new list item starts a new sentence.
            text = re.sub(r"\s*[-*]\s+", "", text, count=1)
        for word in text.split():
            words.append((number, word))

    sentence = []
    for item in words + [None]:
        ends = item is None or item[1].endswith((".", ":", ";", "!", "?"))
        if item is not None:
            sentence.append(item)
        if ends and sentence:
            counted = [w for _, w in sentence if re.search(r"[A-Za-z]", w)]
            if len(counted) > MAX_WORDS:
                start = sentence[0][0]
                preview = " ".join(w for _, w in sentence)[:70]
                faults.append(
                    (
                        start,
                        f"This sentence holds {len(counted)} words. "
                        f"Rule 1 permits {MAX_WORDS}. Text: {preview}",
                    )
                )
            sentence = []
    return faults


def files_under(root):
    """Return every Rust file and every Markdown file below a path."""
    root = Path(root)
    if root.is_file():
        return [root]
    found = []
    for path in sorted(root.rglob("*")):
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        if path.suffix in (".rs", ".md") and path.is_file():
            found.append(path)
    return found


SELFTEST_CASES = [
    ("an em dash", "//! A line that holds an em dash — here.", "em dash"),
    ("e.g.", "//! A line that holds e.g. here.", "e.g."),
    ("i.e.", "//! A line that holds i.e. here.", "i.e."),
    ("an -ing form", "//! A line that shows how the code is working.", "-ing form"),
    (
        "a long sentence",
        "//! " + " ".join(["word"] * 30) + ".",
        "30 words",
    ),
]


def selftest():
    """Check that each test still finds the fault that it looks for."""
    import tempfile

    failures = 0
    with tempfile.TemporaryDirectory() as directory:
        for name, content, expected in SELFTEST_CASES:
            path = Path(directory) / "case.rs"
            path.write_text(content + "\n")
            messages = " ".join(m for _, m in find_faults(path))
            if expected in messages:
                print(f"ok   the check for {name} works")
            else:
                print(f"FAIL the check for {name} found nothing. Text: {messages}")
                failures += 1
        # A span between two back quotes must not raise a fault.
        path = Path(directory) / "case.rs"
        path.write_text("//! The name `working_thing` is code, and not prose.\n")
        if find_faults(path):
            print("FAIL the checker read a span between two back quotes")
            failures += 1
        else:
            print("ok   the checker ignores a span between two back quotes")
        # Such a span can also cross a line break.
        path.write_text("//! The text `Returns the value,\n//! panicking here` is code.\n")
        if find_faults(path):
            print("FAIL the checker read a span that crosses a line break")
            failures += 1
        else:
            print("ok   the checker ignores a span that crosses a line break")
    return 1 if failures else 0


def main():
    arguments = sys.argv[1:]
    if arguments and arguments[0] == "--selftest":
        return selftest()
    roots = arguments or ["."]
    total = 0
    for root in roots:
        for path in files_under(root):
            for number, message in find_faults(path):
                print(f"{path}:{number}: {message}")
                total += 1
    if total:
        noun = "fault" if total == 1 else "faults"
        print(f"\n{total} possible {noun}. A reviewer must read each one.")
        return 1
    print("The script found none of the common faults.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
