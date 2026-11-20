# ADR 0002: Write every document in Simplified Technical English

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `CLAUDE.md` section 1 |
| Code | `CLAUDE.md` section 1, `scripts/ste-lint.py`, `.github/workflows/checks.yml` |
| Tests | `python3 scripts/ste-lint.py --selftest` confirms that each check of the script still works. No test checks the standard. |
| Related | [ADR 0001](0001-record-architecture-decisions.md) |

## Context

The project needs one standard for the text of its documents. Simplified
Technical English keeps text short, active, and clear. A reader who does not
speak English as a first language can then read the text correctly. A machine
translation of the text also stays correct.

ASD-STE100 is the name of the specification of that standard. The
specification is not free to read. A future reader therefore cannot find the
rules in it.

Most of the text in this project comes from Claude, a program that writes text
and code. Every fault that this project has found so far sat in text that
Claude wrote.

## Decision

The project writes every piece of documentation in ASD-STE100 Simplified
Technical English.

Section 1 of [`CLAUDE.md`](../../../CLAUDE.md) states the nine rules in
full. A writer applies exactly those rules, and a writer does not search for
the specification. The same section holds these parts as well:

- the one exception, which lets the first line of a doc comment start with a
  verb;
- the list of the texts that the standard covers, and the list of the texts
  that it does not cover;
- the table of the abbreviations that this project defines, in section 1.4;
- the strings that the project did not change, in section 1.5;
- the rule for a short label inside a panel;
- the verbs of two words and their replacements, in section 1.7.

The standard covers each of these texts:

- `README.md`, and every other Markdown file;
- every comment in the code: a `//!` comment, a `///` comment, and a `//`
  comment;
- the `description` field in each `Cargo.toml` file;
- the name of each test, and the message argument of `assert!` and
  `assert_eq!`;
- the text of each error;
- the text that a person reads while the program runs.

## Consequences

No tool checks Simplified Technical English. A reviewer reads the text and
checks it.

The script `scripts/ste-lint.py` finds the common faults, and it does not check
the standard. It finds five kinds of fault:

- a sentence that holds more than 25 words;
- a word that ends in `ing`;
- an em dash;
- the abbreviation `e.g.`;
- the abbreviation `i.e.`.

`.github/workflows/checks.yml` runs both commands of the script, and it runs
them before every other check. One fault in one Markdown file therefore ends
the run, and the workflow then compiles nothing. The build holds the common
faults. The build does not hold the standard.

The script checks four rules, and it checks each of those four only in part.
No tool checks the other five. A writer therefore reads every sentence that a
change adds or alters, before each commit. Section 1.6 of `CLAUDE.md` gives the
five questions that the writer asks of each sentence. A clean report from the
script is not a pass. Rule 2 and rule 6 fail in text that the script calls
clean.

The script reads a file that ends in `.rs` and a file that ends in `.md`, and
it reads no other file. Inside a Rust file it reads a comment line alone.
Inside a Markdown file it drops a title line, a table, and a fence. A reader
therefore checks all nine rules in every other text that the standard covers. The constant `HELP`, each `#[error("...")]` attribute, and the message
of each `assert!` call and each `expect` call are such texts.

A string that a test compares against keeps its form when a change to it would
change what the program does. Section 1.5 of `CLAUDE.md` lists those strings.

Each record of this folder follows the standard, as
[ADR 0001](0001-record-architecture-decisions.md) states.
