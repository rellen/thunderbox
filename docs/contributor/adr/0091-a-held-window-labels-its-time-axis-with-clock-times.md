# ADR 0091: A held window labels its time axis with clock times

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-27 |
| Source | new |
| Code | `crates/thunderbox-widgets/src/timeseries.rs`, `crates/thunderbox-widgets/src/value.rs`, `crates/thunderbox-dashboard/src/view.rs` |
| Tests | `format_clock_writes_the_time_of_day_for_a_span_under_one_day`, `format_clock_writes_the_date_for_a_span_of_one_day_or_more`, `format_clock_reads_a_day_of_a_leap_year`, `format_clock_reads_a_moment_before_the_epoch`, `a_panel_with_clock_labels_writes_the_time_of_each_place`, `a_panel_with_clock_labels_writes_the_date_for_a_window_of_two_days`, `the_clock_labels_of_a_window_do_not_change_when_only_now_changes`, `the_measurement_covers_the_first_clock_label`, `a_held_range_draws_clock_labels`, `a_held_range_draws_the_same_axis_after_a_refresh`, `a_live_range_draws_labels_from_now`, `a_following_range_draws_labels_from_now`, `a_panel_that_draws_a_held_window_labels_its_axis_with_clock_times` |
| Related | [ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md), [ADR 0036](0036-each-label-of-the-time-axis-states-its-distance-from-now.md), [ADR 0037](0037-the-refresh-continues-while-the-window-stands-still.md) |

## Context

A person moves the time range into the past with the left arrow key. The range
then holds a fixed moment at each end, so the window stands still. The refresh
continues (ADR 0037), and the queries run again on the same window.

ADR 0036 makes each label of the axis of time state a distance from the present
moment. The trace of a held window stands still, but the present moment moves.
A person therefore saw the trace stand still while the labels counted further
into the past on each refresh. The labels said that the data changed, and it
did not.

Three terms name the kinds of range. A live range ends at the present moment. A
held range has a fixed moment at each end. A following range follows the clock,
but it ends before the present moment.

## Decision

1. `thunderbox-widgets` holds the enum `TimeLabels`. Its variant `FromNow` is
   the default, and its variant `Clock` states the time of each place.
2. `TimeSeriesPanel::time_labels` sets the variant. With `FromNow` the panel
   draws the labels of ADR 0036. With `Clock` each label is the time of its
   place, in Coordinated Universal Time (UTC). The function
   `format_clock(t, span_ms)` writes the text. It writes `HH:MM:SS` when
   `span_ms` is less than one day. It writes `MM-DD HH:MM` otherwise. The
   function computes the date from the epoch with plain arithmetic, and it adds
   no dependency.
3. `TimeSeriesPanel::measure_label_width` measures the labels of the mode that
   the panel draws. A stack of held charts therefore still starts at one cell
   under ADR 0039.
4. `DashboardView` passes `TimeLabels::Clock` when the range of the snapshot is
   held, which means `!range.moves_with_the_clock()`. It passes
   `TimeLabels::FromNow` otherwise. The measurement of a stack and the renderer
   use the same mode.
5. A live range and a following range keep the labels from now.

## Consequences

- The column of labels of a held chart grows from `-2m` to `14:00:00`, so the
  chart is a few cells narrower.
- The times are in UTC, as the times of the table panel are. A person in
  another time zone must add the offset.
- ADR 0036 stays Accepted. Its rule still holds for a live range and for a
  following range.
- A later change must keep the mode of the measurement equal to the mode of the
  renderer. A difference moves the chart of a stack.

## Designs that the project rejected

- **Freeze the present moment of the window at the moment of the move.** The
  status row and the labels would state a distance from a moment that has
  passed. That statement is false, because the data of the refresh is newer
  than that moment.
- **Write the time in a local time zone.** The project holds no time zone
  database, and a new dependency for one label is not worth its cost.
