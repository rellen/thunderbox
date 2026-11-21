# ADR 0076: A key that would pass the limit leaves the range as it was

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-21 |
| Source | `docs/contributor/open-findings.md` item 4.3, and [ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md) |
| Code | `crates/thunderbox-core/src/time.rs`, the methods `TimeRange::panned` and `TimeRange::with_span_ms`. Also `crates/thunderbox-dashboard/src/time.rs`, the function `time_key`. |
| Tests | `panned_returns_the_range_as_it_was_when_an_end_would_pass_the_limit`, `panned_returns_the_range_as_it_was_when_the_range_is_wider_than_a_number`, `panned_returns_the_range_as_it_was_when_the_present_moment_is_at_the_limit`, `with_span_ms_returns_the_range_as_it_was_when_the_earlier_end_would_pass_the_limit`, and `no_method_of_a_time_value_panics_at_the_limit_of_the_number`, all in `crates/thunderbox-core/src/time.rs`. Also `many_presses_of_the_keys_keep_the_range_inside_the_limits` and `no_key_panics_for_a_range_at_the_limits_of_the_number` in `crates/thunderbox-dashboard/src/time.rs`. |
| Related | [ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md), [ADR 0074](0074-validate-checks-the-time-range.md), [ADR 0075](0075-no-path-of-the-library-panics-on-a-time-value.md), [ADR 0079](0079-a-key-that-clamps-an-end-changes-the-width.md) |

## Context

A terminal repeats a key for as long as a person presses it.
[ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md) states that a
move is half of the width of the range, and that a scale is a factor of two. A
person who presses a key for a long time can therefore carry an end of the
range to the limit of an `i64`. An `i64` is a whole number of 64 bits that has
a sign.

`TimeRange::panned` serves the keys `Left` and `Right`.
`TimeRange::with_span_ms` serves the keys `-` and `+`. Before this change, these
two methods had two faults:

- `RelativeTime::resolve` added the present moment and an offset with no check.
  An overflow is a result that does not fit in its type. A range with an offset
  of `i64::MAX` made `panned` panic.
- `RelativeTime::moved` held an end at the limit with `saturating_add`. An end
  that reached the limit then moved by less than the other end, so the width of
  the range changed with no message.

The section "Measurements" gives the numbers.

## Decision

`TimeRange::panned` returns the range as it was when a move would carry an end
past the limit of an `i64`. `TimeRange::with_span_ms` returns the range as it
was when the earlier end would pass that limit.

`panned` also returns the range as it was when the range is wider than an
`i64` holds. `with_span_ms` returns the range as it was when the new range
would have another width than the caller asked for.

The two methods do not clamp the end. A clamp holds a value at a limit.
[ADR 0079](0079-a-key-that-clamps-an-end-changes-the-width.md) names the fault
of that design.

`time_key` then returns `None`, because the range did not change. A key that
changes nothing starts no new set of queries.
[ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md) states that
rule.

## Consequences

The width of the range never changes because an end reached the limit of the
number. A move keeps the width, and a scale gives the width that the caller
asked for, or it changes nothing.

A range that starts inside the two limits of
[ADR 0074](0074-validate-checks-the-time-range.md) stays inside them after each
key. The test `many_presses_of_the_keys_keep_the_range_inside_the_limits`
presses `-` 200 times, then `Left` 10000 times, then `Right` 10000 times, and
then `+` 200 times. After each press the range passes `TimeRange::validate`.

A person who presses a key at the limit sees no change on the panels. The
program shows no message for that case.

## Measurements

A reader copied the code of the commit before this change into a folder of its
own. The reader added a test to `crates/thunderbox-core/src/time.rs` and ran
`cargo test -p thunderbox-core` on a debug build. The sizes are in
milliseconds.

| call | result before this change |
|---|---|
| `panned(NOW, 1)` on a range from `now+9223372036854775807ms` to `now` | The test panicked at `time.rs:268:32` with `attempt to add with overflow`. |
| `panned(NOW, -1)` on a range from `i64::MIN` to `i64::MIN + 1000` | A range from `i64::MIN` to `i64::MIN + 999`. The width fell from 1000 to 999. |
| `with_span_ms(NOW, 2000)` on the same range | A range from `i64::MIN` to `i64::MIN + 1000`. The caller asked for the width 2000, and the width was 1000. |

After this change, the tests in the field `Tests` show that each call returns
the range as it was.

## Designs that the project rejected

The project examined one other design. In that design, the two methods clamp the
end at the limit. [ADR 0079](0079-a-key-that-clamps-an-end-changes-the-width.md)
names the fault of that design.
