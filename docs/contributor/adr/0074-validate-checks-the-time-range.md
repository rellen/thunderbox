# ADR 0074: `Dashboard::validate` checks the time range

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-21 |
| Source | `docs/contributor/open-findings.md` item 4.3, and [ADR 0061](0061-a-duration-that-does-not-fit-ends-the-program.md) |
| Code | `crates/thunderbox-core/src/time.rs`, the method `TimeRange::validate`. Also `crates/thunderbox-core/src/dashboard.rs`, the method `Dashboard::validate`. |
| Tests | `validate_accepts_a_width_of_exactly_one_second`, `validate_returns_an_error_for_a_width_below_one_second`, `validate_accepts_a_width_of_exactly_ten_years`, `validate_returns_an_error_for_a_width_above_ten_years`, `validate_measures_the_width_of_a_range_that_names_its_later_end_first`, `validate_returns_an_error_for_a_start_that_does_not_fit`, `validate_returns_an_error_for_an_end_that_does_not_fit`, `validate_returns_an_error_for_two_ends_that_lie_further_apart_than_a_number_holds`, and `validate_accepts_the_default_range`, all in `crates/thunderbox-core/src/time.rs`. Also `validate_returns_an_error_for_a_range_wider_than_ten_years`, `validate_accepts_a_range_of_ten_years`, `validate_returns_an_error_for_a_range_narrower_than_one_second`, and `validate_accepts_a_range_of_one_second` in `crates/thunderbox-core/src/dashboard.rs`. Also `a_range_that_is_too_wide_gets_an_error_that_names_the_limit` in `crates/thunderbox/tests/cli.rs`. |
| Related | [ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md), [ADR 0041](0041-validate-reports-the-fault-of-a-variable.md), [ADR 0073](0073-a-duration-that-does-not-fit-returns-an-error.md), [ADR 0075](0075-no-path-of-the-library-panics-on-a-time-value.md), [ADR 0076](0076-a-key-that-would-pass-the-limit-leaves-the-range-as-it-was.md) |

## Context

`Dashboard::validate` read no field of the table `[time]`. The two limits of the
width of a time range, `MIN_SPAN_MS` and `MAX_SPAN_MS`, reached the keys `+` and
`-` through `TimeRange::with_span_ms`. They never reached a dashboard file.
[ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md) states those
keys.

The comment of `MAX_SPAN_MS` said that the arithmetic stays correct. That
sentence held for the path of the keys alone.

A dashboard that held `from = "now-9223372036854775807ms"` loaded with no
error. `QueryRequest::new` then gave every data source a width of `i64::MAX`.
The task of the testdata panel panicked with `attempt to multiply with
overflow`. The message named a data source, and it never named the dashboard.

## Decision

The new public method `TimeRange::validate(&self, now)` checks the time range.
`Dashboard::validate` calls it with the present moment, which `now_ms` returns.
`now` is a count of milliseconds after the Unix epoch.

The method resolves both ends against `now`, and it puts the earlier moment
first. It returns an error in three cases:

- An end is an offset, and the sum of `now` and the offset does not fit in an
  `i64`. The text names the field of that end.
- The range is wider than `MAX_SPAN_MS`, which is 3650 days.
- The range is narrower than `MIN_SPAN_MS`, which is 1000 milliseconds.

A range of exactly one second passes. A range of exactly 3650 days passes.

The three texts are these:

```
The dashboard is not correct. The `[time]` table holds a `from` field of now+9223372036854775807ms. The sum of the present moment and that value does not fit in a count of milliseconds.
The dashboard is not correct. The `[time]` table holds a `from` field of now-3651d and a `to` field of now. That range is wider than 3650 days.
The dashboard is not correct. The `[time]` table holds a `from` field of now-500ms and a `to` field of now. That range is narrower than 1000 milliseconds.
```

The method measures the width in an `i128`. Two ends that each fit in an `i64`
can lie further apart than an `i64` holds. A subtraction in an `i64` would then
overflow.

A range can name its end before its start. The method reads such a range as the
window reads it, with the earlier moment first.

## Consequences

The keys and the dashboard file now keep the same two limits. A person who
presses a key and a person who writes a file meet the same rule.

A range with a fixed end and an end that follows the clock grows as the clock
moves. A dashboard that states a fixed `from` and a `to` of `now` passes today.
It returns an error when more than 3650 days lie between that `from` and the
present moment.
`Dashboard::validate` therefore reads the present moment. It checks the range
at the moment of the load only.

The error comes from the second step of the load, so the program writes it
before it opens the terminal. The error of an offset that does not fit in a
duration comes from the first step. [ADR 0073](0073-a-duration-that-does-not-fit-returns-an-error.md)
states that error.

The fault names the field and the text of the value. [ADR 0077](0077-the-text-form-of-an-offset-keeps-a-unit-that-a-person-can-read.md)
states the form of that text.

A program that builds a `TimeRange` in code can call `TimeRange::validate`
itself. No rule makes that program call it.
[ADR 0075](0075-no-path-of-the-library-panics-on-a-time-value.md) therefore
states that no calculation of the library panics on a time value.

## Measurements

All measurements used a debug build. Each dashboard file holds the lines
`title = "x"` and `[time]`, and the rows below name the other lines. Each
command returned the code 1.

| other lines of the file | the error that the program wrote |
|---|---|
| `from = "now+9223372036854775807ms"` | The first text above. |
| `from = "now-3651d"` | The second text above. |
| `from = "now-500ms"` | The third text above. |
