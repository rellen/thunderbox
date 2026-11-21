# ADR 0071: The program draws only on a terminal

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-21 |
| Source | `docs/contributor/open-findings.md` items 4.7 and 7.2 |
| Code | `crates/thunderbox/src/main.rs`, the function `run` |
| Tests | `the_program_returns_1_when_no_stream_is_a_terminal` and `the_program_returns_1_when_the_output_is_a_file_and_the_input_is_a_terminal`, both in `crates/thunderbox/tests/cli.rs` |
| Related | [ADR 0016](0016-a-program-that-holds-a-dashboard-holds-four-rules.md) |

## Context

The program promised to send no control sequence to a program that reads its
output. The promise held only for a dashboard that fails to load. A dashboard
that loads reached `ratatui::init()`, and that function panics when it finds no
terminal.

A second fault hid behind the first. The program drew into a file when only the
output was a file and the input was a terminal. No code of the program checked a
stream.

## Decision

The function `run` loads the dashboard, makes the data sources, and starts the
runtime. It then checks `std::io::stdout().is_terminal()`.

When the output is not a terminal, the program writes this line to the standard
error, writes nothing to the standard output, and returns 1:

```
Error: The program draws a dashboard only on a terminal. The standard output of the program is not a terminal.
```

The program then calls `ratatui::try_init()` in place of `ratatui::init()`.
When `try_init` returns an error, the program calls `ratatui::restore()`, and it
returns the error with the context `The program cannot start the terminal.`. The
reason follows after `Caused by:`. A terminal can accept raw mode and then
refuse the alternate screen. The restore returns the terminal to its first state
in that case.

`try_init` installs the same panic hook as `init`. A dashboard that is not there,
or that holds a fault, still reports its own error first. The options `--help`
and `--version` still write to a pipe.

## Consequences

The program writes no control sequence to a file or to a pipe. A person who
redirects the output reads one sentence of the program, and not a panic of
ratatui.

The program never returns 101 for a start with no terminal. A panic of the
program still returns 101.

The check stands after the dashboard and the data sources. A fault in the
dashboard therefore reaches the person before the check does.

## Measurements

Both cases used a debug build, the default dashboard, and macOS.

- **Case 1.** No terminal on any stream, with the command
  `thunderbox > out.bin < /dev/null`. Before the change, `out.bin` held 8
  bytes: `ESC [ ? 1 0 4 9 l`. That sequence leaves the alternate screen, and the
  panic hook of ratatui wrote it. The error output held the panic
  `failed to initialize terminal: Os { code: 6, kind: Uncategorized, message:
  "Device not configured" }`. The code was 101. On Linux the same panic names
  `No such device or address`.
- **Case 2.** A pseudo-terminal of 80 columns and 24 rows on the input and on
  the error output, and a file on the output. Before the change, the program
  wrote 8,365 bytes into the file in 3 seconds. The pseudo-terminal received 0
  bytes. Only the key `q` ended the program, with the code 0.

After the change, both cases return 1, and the output receives 0 bytes. In case
2 the pseudo-terminal receives the one line above, which holds 112 bytes with
the line end.

## Designs that the project rejected

- **Check the input.** Case 1 then returns 1. Case 2 passes the check, so the
  program draws into the file while the terminal stays in raw mode.
- **Check the error output.** This design has the same fault as the check of the
  input, because case 2 holds a terminal on the error output.
- **Keep `ratatui::init()`.** Case 1 then gives a panic, the code 101, and 8
  bytes on the output.
