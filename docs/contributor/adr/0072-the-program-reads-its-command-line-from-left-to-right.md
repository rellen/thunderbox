# ADR 0072: The program reads its command line from left to right

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-21 |
| Source | `docs/contributor/open-findings.md` item 4.6 |
| Code | `crates/thunderbox/src/main.rs`, the functions `parse_arguments` and `write_text` |
| Tests | `parse_arguments_reads_each_command_line_by_its_rules`, `a_name_that_is_not_text_reaches_the_file_system_unchanged`, `write_text_returns_an_error_when_the_write_fails`, `write_text_returns_an_error_when_only_the_flush_fails`, and `write_text_returns_ok_for_a_closed_pipe`, all in `crates/thunderbox/src/main.rs`. Also `an_unknown_option_returns_1_and_names_the_option`, `a_file_name_after_two_dashes_reaches_the_program`, `a_second_file_returns_1_and_names_that_file`, `a_name_that_is_not_text_reaches_the_file_system`, `the_version_to_a_full_device_returns_1`, and `the_help_into_a_closed_pipe_returns_0`, all in `crates/thunderbox/tests/cli.rs` |
| Related | [ADR 0071](0071-the-program-draws-only-on-a-terminal.md) |

## Context

The program read every argument for the options `-h`, `--help`, `-V`, and
`--version`. It then took the first argument as the path of a file, whatever
that argument held. This rule gave six faults. A reader measured each fault
against a debug build:

- The command `thunderbox --colour` wrote `Error: The program cannot read
  --colour.`, and it returned 1. The command `thunderbox -x` gave the same
  message for `-x`. The reason was `No such file or directory`.
- The command `thunderbox -- my-dashboard.toml` wrote `Error: The program
  cannot read --.`. The `README.md` teaches `cargo run -p thunderbox --
  my-dashboard.toml`, so a person who keeps the `--` met that message.
- The command `thunderbox /no/a.toml /no/b.toml` named `/no/a.toml` alone. The
  program dropped the second file with no word.
- The commands `thunderbox board.toml --help` and `thunderbox --colour --help`
  wrote the help text, and they returned 0. The second command hid a fault.
- A file name that is not text, such as `bad\xFF.toml`, made `std::env::args`
  panic. The code was 101.
- The command `thunderbox --help` with the output on a closed pipe panicked
  with `failed printing to stdout: Broken pipe`. The code was 101. The closed
  pipe comes from `thunderbox --help | head -1`.

The block `Options:` of the help text names two options. A reader takes that
block for a promise that the program rejects every other option.

## Decision

The function `parse_arguments` reads the arguments from left to right. The
first argument that decides the result stops it:

- The argument `-h` or `--help` gives the help text.
- The argument `-V` or `--version` gives the version.
- Any other argument that starts with a dash is an unknown option. The function
  returns the error `Error: The program does not know the option OPTION. Run
  thunderbox --help to read the list of the options.`
- A second file name returns the error `Error: The command line names a second
  file, FILE. The program shows one dashboard only.`

The first file name decides nothing, so `board.toml --help` writes the help
text. The option `--colour` before `--help` returns the error for `--colour`.

The argument `--` ends the options. Each argument after it is a file name, so
`thunderbox -- -dash.toml` names the file `-dash.toml`. A second `--` after the
first one is a file name as well.

The program reads its arguments with `std::env::args_os`, and it keeps the path
as an `OsString`. A name that is not text reaches the file system unchanged.

The function `write_text` writes the help text and the version, and it then
flushes the output. A closed pipe is a pipe that no program reads any more. It
is not a fault, because the program that takes the output has all the bytes
that it needs. `head` is such a program. The function returns `Ok` for a closed
pipe, and the program returns 0. Each other fault of the write or of the flush
returns the error `Error: The program cannot write the text to the standard
output.` The program then returns 1.

Each error of the command line returns the code 1. No command line makes the
program panic. The project added no dependency and examined no library. The
program reads each argument itself, as section 1 of `CLAUDE.md` states.

## Consequences

A file name that starts with a dash needs `./` in front of it, or `--` before
it. The usage block of the help text states both ways.

A `--help` that follows a fault never wins over that fault. A `--help` that
follows only a file name still wins. A person who adds an option later must
decide where the option stands in this order.

The program accepts one file. A person who names two files reads an error. The
program no longer drops the second file without a message.

A closed pipe returns 0, so a script that runs `thunderbox --help | head -1`
sees a success. A full disk returns 1.

The help text grew by four lines, and `README.md` and the page
`docs/user/reference/command-line.md` hold the same rules.

## Measurements

All measurements used a debug build.

- **Before the change.** The six commands of the section Context gave the
  results that the section lists.
- **After the change.** Each of those commands gives the result that the
  section Decision states. The command `thunderbox -- my-dashboard.toml` reads
  the file `my-dashboard.toml`. The name `bad\xFF.toml` reaches the file system
  unchanged, and the program reports that it cannot read that file. The command
  `thunderbox --help | head -1` returns 0.
