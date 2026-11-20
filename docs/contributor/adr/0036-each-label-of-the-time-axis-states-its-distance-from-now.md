# ADR 0036: Each label of the axis of time states its distance from the present moment

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-widgets/src/timeseries.rs`, `crates/thunderbox-widgets/src/value.rs`, `crates/thunderbox-dashboard/src/view.rs` |
| Tests | `format_from_now_names_the_present_moment`, `format_from_now_selects_the_unit_from_the_distance_between_two_labels`, `format_from_now_writes_no_count_of_zero`, `format_from_now_writes_no_count_of_four_digits`, `a_window_that_ended_in_the_past_labels_no_place_as_now`, `a_panel_that_draws_a_window_in_the_past_writes_no_label_that_reads_now`, `a_panel_that_draws_a_window_in_the_past_labels_no_place_as_now`, `a_window_that_the_data_covers_in_part_leaves_the_rest_of_the_axis_empty`, `a_point_outside_the_window_still_widens_the_axis_of_time`, `a_value_past_the_limit_still_widens_the_axis` |
| Related | [ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md), [ADR 0037](0037-the-refresh-continues-while-the-window-stands-still.md), [ADR 0038](0038-a-sysinfo-sample-carries-the-moment-of-the-machine.md) |

## Context

The panel measured every label from the newest point of the data before this
change, so the right label always read `now`. A person who moved the range one
hour into the past then read `now` at the right edge. That label was not
correct.

## Decision

Each label of the axis of time states the distance from the present moment to
the place of that label.

`TimeSeriesPanel::window` takes the window. A window is the pair of moments
that a time range names, and the moment that named them. The window therefore
holds the present moment. `thunderbox_widgets::format_from_now` writes each
label.

The function selects the unit of a label from the distance between two labels,
and not from the value of one label. Two labels of one axis therefore differ,
unless the axis has no width.

The function then corrects that unit against the value, in two conditions:

- It takes a shorter unit while the value rounds to zero, because the text
  `-0m` states both a distance and no distance.
- It takes a longer unit while the value needs four digits, because the text
  `-61000ms` does not fit a narrow panel.

A unit can be longer than the distance between two labels. A label in such a
unit keeps one or two decimal places, so the two labels still differ.

The axis covers the window and every point.

## Consequences

A window that a data source covers in part fills one part of the panel, and
the rest of the panel stays empty. That picture is the truth about the data
source.

The axis of values follows the same rule. It covers every value, even a value
past a limit that the panel states. The test
`a_value_past_the_limit_still_widens_the_axis` records that rule.
