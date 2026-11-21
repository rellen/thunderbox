# ADR 0077: The text form of an offset keeps a unit that a person can read

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-21 |
| Source | new |
| Code | `crates/thunderbox-core/src/time.rs`, the table `UNITS`, the function `largest_exact_unit`, and the `Display` implementation of `RelativeTime` |
| Tests | `display_writes_the_largest_unit_that_divides_an_offset`, `display_does_not_panic_for_the_limits_of_the_number`, `parse_relative_reads_each_text_that_display_writes`, and `serialize_writes_the_text_that_display_writes`, all in `crates/thunderbox-core/src/time.rs`. Also `validate_returns_an_error_for_a_start_that_does_not_fit` and `validate_returns_an_error_for_an_end_that_does_not_fit` in the same file. |
| Related | [ADR 0073](0073-a-duration-that-does-not-fit-returns-an-error.md), [ADR 0074](0074-validate-checks-the-time-range.md) |

## Context

The `Display` implementation of `RelativeTime` wrote an offset in milliseconds
alone. The offset `now-5m` gave the text `now-300000ms`.

[ADR 0074](0074-validate-checks-the-time-range.md) adds errors that name a field
of the table `[time]`, and each error holds the text of that field. A person who
wrote `now-3651d` would then read `now-315446400000ms`. The person cannot find
that text in the file.

`Serialize` writes the same text as `Display`, so a change to `Display` changes
both.

## Decision

The `Display` implementation of `RelativeTime` writes an offset in the largest
unit that divides the offset exactly. The offset `-300000` gives `now-5m`. The
offset `-90000` gives `now-90s`. The offset `-1500` gives `now-1500ms`. An
offset of zero keeps the form `now+0ms`.

One table, `UNITS`, holds the six units and their lengths. `parse_duration_ms`
reads the names from that table, and `Display` writes them from the same table.
The two therefore cannot differ.

`Serialize` writes the same text. `parse_relative` reads each text that
`Display` writes, and it returns the same offset.

## Consequences

A person can find the text of an error that names a field of `[time]` in the
file, when the person wrote the largest unit. A file that writes `now-300s`
gets `now-5m` in a message. The two texts name the same offset.

A new unit takes one row of the table `UNITS`. The new unit then serves the
reader and the writer.

The offset `i64::MIN` gives `now-9223372036854775808ms`, because no larger unit
divides it. `Display` does not panic for it.
