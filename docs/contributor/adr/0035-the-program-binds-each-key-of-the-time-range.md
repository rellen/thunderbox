# ADR 0035: The program binds each key of the time range

| field | value |
|---|---|
| Status | Superseded by [ADR 0093](0093-a-person-can-follow-the-clock-behind-the-present.md) |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-core/src/time.rs`, `crates/thunderbox-dashboard/src/time.rs`, `crates/thunderbox-dashboard/src/view.rs`, `crates/thunderbox/src/main.rs`, `crates/thunderbox-dashboard/examples/full_embed.rs` |
| Tests | `the_left_key_moves_the_range_back_by_half_of_its_width`, `the_right_key_moves_the_range_forward_by_half_of_its_width`, `a_move_that_reaches_the_present_follows_the_clock_and_keeps_the_width`, `the_minus_key_makes_the_range_twice_as_wide`, `the_plus_key_makes_the_range_half_as_wide`, `the_zero_key_returns_the_range_that_the_dashboard_states`, `a_key_that_changes_nothing_asks_for_no_range`, `a_move_forward_that_passes_the_present_moment_stops_at_it`, `a_move_forward_follows_the_clock_again_after_the_clock_moved_on`, `a_move_back_holds_still_a_range_that_ends_in_the_future`, `a_move_back_and_a_move_forward_differ_for_a_range_that_ends_in_the_future`, `the_view_uses_no_key_of_the_time_range`, `a_key_that_the_program_binds_moves_the_time_range`, `a_key_that_changes_nothing_starts_no_set_of_queries`, `each_key_of_the_time_range_moves_the_range` |
| Related | [ADR 0015](0015-both-ways-into-another-program-are-in-scope.md), [ADR 0036](0036-each-label-of-the-time-axis-states-its-distance-from-now.md), [ADR 0037](0037-the-refresh-continues-while-the-window-stands-still.md), [ADR 0043](0043-the-program-binds-each-key-of-a-variable.md), [ADR 0054](0054-the-program-binds-enter-and-space.md) |

## Context

A person moves the time range into the past with keys, and a person scales the
width of the range with keys. A program that holds a thunderbox dashboard keeps
its own bindings for every key that thunderbox ignores.
[ADR 0015](0015-both-ways-into-another-program-are-in-scope.md) states that
rule.

A window is the pair of moments that a time range names, and the moment that
named them. A person who studies the past needs a picture that stands still.
No data source of this project answers a question about the future.

`DashboardRuntime::set_time_range` starts a new set of queries on every call.
A terminal repeats a key for as long as a person presses it.

## Decision

The program binds every key of the time range. thunderbox consumes none of
them.

`DashboardView::handle_key` consumes three keys alone: `Tab`, `Shift+Tab`, and
`m`. It reports `Consumed::No` for every key of the time range. A program that
holds a thunderbox dashboard therefore keeps the left arrow key, the right arrow
key, `-`, `+`, `0`, and `=` for itself.

One type holds the arithmetic, and one function holds the policy:

- `thunderbox_core::TimeRange` holds every operation on a range: `window`,
  `resolve`, `span_ms`, `moves_with_the_clock`, `to_absolute`, `moved`,
  `panned`, and `with_span_ms`.
- `thunderbox_dashboard::time_key` offers a default map from one key to one new
  range. It states the policy: a move is half of the width, a scale is a
  factor of two, and `0` and `=` return the range that the dashboard states.

The function consumes nothing, and a program that wants another map ignores
it. The program reads the key of its terminal, and it turns that key into a
`Key`. The program then decides whether to call the function. The program that
runs alone and `full_embed.rs` each do that in one short block. No program
holds a copy of the policy.

### Rules of a move

A move fixes a moment at each end of the range, so the window stands still.

A move forward that would carry the end of the window to the present moment,
or past it, stops at the present moment. `time_key` then returns a range of the
same width that follows the clock, and the panels follow the clock again.

A move forward also returns to the clock when it stops closer to the present
moment than the length of one move. The program reads the clock again for each
key press. A move back and then a move forward would otherwise leave a window a
few milliseconds behind the present moment. That window would look like a window
that follows the clock, and it would never move again.

A move back holds the window still in every condition. A dashboard can state a
range that ends after the present moment, in the form `now+5m`. The move back
fixes both ends of such a range as well, so the two keys never return the same
range.

### A key that changes nothing

`time_key` returns `None` for a key that asks for the range that the runtime
already holds. A key that changes nothing would otherwise stop each set of
queries before it answers, because each call to
`DashboardRuntime::set_time_range` starts a new set.

## Consequences

A program calls `time_key` from a key press, and never on every frame. A call
on every frame would stop and start a set of queries on every frame.

A program that wants another map ignores `time_key`, and it still uses the
arithmetic of `TimeRange`. A change to the policy therefore changes `time_key`
alone.

[ADR 0043](0043-the-program-binds-each-key-of-a-variable.md) and
[ADR 0054](0054-the-program-binds-enter-and-space.md) follow the same rule for
the keys of a variable and for the keys of a row panel.
