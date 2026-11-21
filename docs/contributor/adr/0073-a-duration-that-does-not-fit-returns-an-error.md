# ADR 0073: A duration that does not fit returns an error

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-21 |
| Source | [ADR 0061](0061-a-duration-that-does-not-fit-ends-the-program.md), and `docs/contributor/open-findings.md` items 4.1 and 4.3 |
| Code | `crates/thunderbox-core/src/time.rs`, the function `parse_duration_ms` |
| Tests | `parse_duration_ms_returns_an_error_for_a_duration_that_does_not_fit`, `parse_duration_ms_reads_a_duration_at_the_limit_of_the_number`, `the_error_for_a_large_duration_names_the_input_and_the_limit`, `parse_relative_returns_an_error_for_an_offset_that_does_not_fit`, and `parse_relative_reads_the_largest_offset`, all in `crates/thunderbox-core/src/time.rs`. Also `validate_returns_an_error_for_a_refresh_duration_that_does_not_fit` and `validate_returns_an_error_for_a_start_that_does_not_fit` in `crates/thunderbox-core/src/dashboard.rs`. Also `a_refresh_duration_that_does_not_fit_gets_an_error_and_no_panic` and `a_start_that_does_not_fit_gets_an_error_and_no_panic` in `crates/thunderbox/tests/cli.rs`. No test names a `min_step` or a `timeout` that does not fit. |
| Related | [ADR 0061](0061-a-duration-that-does-not-fit-ends-the-program.md), which this record supersedes. Also [ADR 0074](0074-validate-checks-the-time-range.md), [ADR 0075](0075-no-path-of-the-library-panics-on-a-time-value.md), and [ADR 0078](0078-a-duration-that-saturates-hides-the-mistake.md) |

## Context

[ADR 0061](0061-a-duration-that-does-not-fit-ends-the-program.md) recorded a
fault and left one question open. A dashboard file could end the program with
an overflow. An overflow is a result that does not fit in its type.
`parse_duration_ms` ended with `Ok(n * mult)`, and that multiplication had no
check.

Four calls reach that function:

- `Dashboard::validate` calls it for the field `refresh`.
- `parse_relative` calls it for each offset of the table `[time]`. The forms
  are `now-DURATION` and `now+DURATION`. The TOML reader runs `parse_relative`
  before it runs `validate`.
- The data source `prometheus` calls it for the option `min_step`.
- The data source `prometheus` calls it for the option `timeout`.

A build without `--release` panicked, and the code was 101. A build for release
wrapped the number. The panic named no field, so a person could not find the
fault from its message.

[ADR 0061](0061-a-duration-that-does-not-fit-ends-the-program.md) asked one
question. Does `parse_duration_ms` return an error for a number that does not
fit, or does it saturate? An operation saturates when it returns the largest
or the smallest number of its type in place of a result that does not fit.

## Decision

`parse_duration_ms` returns an error for a duration that does not fit in an
`i64`. An `i64` is a whole number of 64 bits that has a sign. The function
multiplies the number by the length of the unit with `i64::checked_mul`. It
returns an error when that call returns `None`.

The error is an `Error::Time`. Its text names the input and the limit:

```
The time value "20000000000w" is not correct. A duration cannot be larger than 9223372036854775807 milliseconds.
```

The limit is `i64::MAX`, which is 9223372036854775807 milliseconds. A duration
of exactly that length is correct.

The rule covers each caller, because each caller calls the same function:

- the field `refresh`;
- the offset of each end of the table `[time]`;
- the option `min_step` of the data source `prometheus`;
- the option `timeout` of the data source `prometheus`.

Each caller returns the error to its own caller. The program writes it before
it opens the terminal.

## Consequences

A person who writes a large number by mistake reads a message that names the
text. A dashboard that holds such a number never starts.

The error of an offset of `[time]` comes inside the error of the TOML reader.
That error names the line and the column of the field. The error of `refresh`
comes from `Dashboard::validate`. The error of `min_step` and `timeout` comes
from the step that makes the data sources.

The limit of a duration is far above the limit of the width of a time range.
[ADR 0074](0074-validate-checks-the-time-range.md) states that second limit.

A new function that reads a duration from text calls `parse_duration_ms`, and
it returns the error of that function. It never multiplies the number by a unit
itself.

A number that an `i64` cannot hold gets the same text, before the function
reads the unit. `9223372036854775808ms` is such a number.

This rule covers a value that a person writes. A calculation on a value that
code already holds follows another rule.
[ADR 0075](0075-no-path-of-the-library-panics-on-a-time-value.md) states it.

[ADR 0061](0061-a-duration-that-does-not-fit-ends-the-program.md) is now
superseded. [ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md)
names the same fault in its list of three things that a change did not do.

## Measurements

All measurements used a debug build. Each dashboard file holds the line
`title = "x"`, and the rows below name the other lines.

| other lines of the file | before the change | after the change |
|---|---|---|
| `refresh = "20000000000w"` | `attempt to multiply with overflow`, and the code 101 | The message above, and the code 1 |
| `[time]` and `from = "now-20000000000w"` | The same panic, and the code 101 | The same message inside the error of the TOML reader, and the code 1 |
| `[time]` and `to = "now+20000000000w"` | The same panic, and the code 101 | The same message inside the error of the TOML reader, and the code 1 |
| `[source.p]`, `kind = "prometheus"`, `url = "http://localhost:9090"`, and `min_step = "20000000000w"` | The same panic, and the code 101 | The same message, under `The program cannot make the data sources of the dashboard.`, and the code 1 |
| the same `[source.p]` table, with `timeout = "20000000000w"` instead of `min_step` | No measurement | The same message as for `min_step`, and the code 1 |

The column `before the change` for the rows `from`, `to`, and `min_step` comes
from item 4.1 of `docs/contributor/open-findings.md`. That item names the call
for `timeout` and states no measurement for it. The change that wrote this
record removed item 4.1. The command
`git log -p -- docs/contributor/open-findings.md` shows its text.

## Designs that the project rejected

The project examined one other design. In that design, `parse_duration_ms`
saturates. [ADR 0078](0078-a-duration-that-saturates-hides-the-mistake.md)
names the fault of that design.
