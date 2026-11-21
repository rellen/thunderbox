# ADR 0075: No path of the library panics on a time value

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-21 |
| Source | `docs/contributor/open-findings.md` items 4.1 and 4.3 |
| Code | `crates/thunderbox-core/src/time.rs`, `crates/thunderbox-core/src/query.rs`, `crates/thunderbox-source-prometheus/src/step.rs`, `crates/thunderbox-source-testdata/src/lib.rs`, `crates/thunderbox-source-sysinfo/src/history.rs`, `crates/thunderbox-source-sysinfo/src/sample.rs`, `crates/thunderbox-widgets/src/value.rs`, `crates/thunderbox/src/main.rs` |
| Tests | `no_method_of_a_time_value_panics_at_the_limit_of_the_number`, `resolve_holds_the_value_at_the_limit_of_the_number`, and `a_window_that_is_wider_than_the_number_reports_the_largest_width` in `crates/thunderbox-core/src/time.rs`. Also `new_does_not_panic_for_a_range_at_the_limit_of_the_number` and `span_ms_does_not_panic_for_ends_in_the_wrong_order` in `crates/thunderbox-core/src/query.rs`. Also `for_request_returns_quickly_for_the_widest_range` and `for_request_does_not_panic_for_the_limits_of_the_number` in `crates/thunderbox-source-prometheus/src/step.rs`. Also `timestamps_does_not_panic_for_the_widest_range`, `timestamps_returns_no_more_than_the_largest_number_of_points`, and `query_does_not_panic_for_a_range_at_the_limits_of_the_number` in `crates/thunderbox-source-testdata/src/lib.rs`. Also `format_window_does_not_panic_for_the_limits_of_the_number` in `crates/thunderbox-widgets/src/value.rs`, and `age_of_does_not_panic_for_a_moment_at_the_limits_of_the_number` in `crates/thunderbox/src/main.rs`. |
| Related | [ADR 0016](0016-a-program-that-holds-a-dashboard-holds-four-rules.md), [ADR 0073](0073-a-duration-that-does-not-fit-returns-an-error.md), [ADR 0074](0074-validate-checks-the-time-range.md) |

## Context

[ADR 0074](0074-validate-checks-the-time-range.md) makes the dashboard file keep
two limits. A file is not the only way to a time value. A program that embeds
the crates can build a `TimeRange` or a `QueryRequest` in code. The fields of
both types are public. No rule makes such a program call `Dashboard::validate`,
and [ADR 0016](0016-a-program-that-holds-a-dashboard-holds-four-rules.md) lists
no such rule.

The library therefore met a time value that no check had read. Several
additions, subtractions, and multiplications on milliseconds could then
overflow. An overflow is a result that does not fit in its type.
`clippy::missing_panics_doc` reads `panic!`, `unwrap`, and `expect`, and it
reads no arithmetic. No lint reported these places.

## Decision

No path of the library panics on a time value. Each addition, subtraction, and
multiplication on milliseconds does one of two things:

- It saturates. An operation saturates when it returns the largest or the
  smallest number of its type in place of a result that does not fit.
- It uses a wider number, an `i128` or a `u64`, so that the result fits.

These places keep the rule:

- `RelativeTime::resolve` and `TimeWindow::span_ms`;
- `QueryRequest::new` and `QueryRequest::span_ms`;
- the step grid of the data source `prometheus`;
- the times of the points of the data source `testdata`;
- the range of the history of the data source `sysinfo`, and the sample of its
  network;
- `format_window` of `thunderbox-widgets`;
- `age_of` of the program.

The step grid keeps two more rules. It stops its search for a step after 2000
turns, and it then takes a step that it calculates. A range of the width of the
whole number would otherwise need a billion turns. The data source `testdata`
returns at most 100000 points for one query. A caller can ask for as many points
as a `u32` holds, and that number is far above the width of any terminal.

### The rule that separates this record from ADR 0073

A value that a person writes returns an error, because the person can correct
it. [ADR 0073](0073-a-duration-that-does-not-fit-returns-an-error.md) states
that rule. A calculation on a value that code already holds saturates, because
no person reads a message at that place.

## Consequences

A program that builds a range from any numbers draws a window, and it does not
panic. The window can mean little. A range at the limit of the number gives a
width at the limit of the number.

A later change that adds arithmetic on milliseconds must saturate, or it must
use a wider number. No lint reads arithmetic, so the tests hold this rule. The
test `no_method_of_a_time_value_panics_at_the_limit_of_the_number` calls the
methods of `TimeRange` and `RelativeTime` with seven numbers. They are both
limits of an `i64`, the number next to each limit, and -1, 0, and 1.

A request of ordinary width never reaches the limit of 2000 turns of the step
grid. The grid of such a request holds the smallest round step that fits.
