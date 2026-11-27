# ADR 0089: The labels of the axis of values stand at the right

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-27 |
| Source | new |
| Code | `crates/thunderbox-core/src/dashboard.rs`, `crates/thunderbox-widgets/src/timeseries.rs`, `crates/thunderbox-dashboard/src/view.rs` |
| Tests | `a_chart_ends_each_label_of_the_axis_of_values_in_the_same_cell_by_default`, `a_chart_starts_each_label_of_the_axis_of_values_in_the_same_cell_for_the_value_left`, `a_stack_keeps_the_chart_width_under_both_values_and_the_labels_follow_the_value`, `pad_labels_puts_the_spaces_on_the_side_away_from_the_alignment`, `a_dashboard_that_states_no_value_labels_ends_each_label_at_the_axis`, `a_panel_that_states_value_labels_left_starts_each_label_at_the_left_edge`, `validate_returns_an_error_for_value_labels_on_a_panel_that_is_not_a_time_series` |
| Related | [ADR 0039](0039-a-stack-of-charts-can-share-one-width.md) |

## Context

A time series chart keeps a column of cells at its left edge for the labels of
the axis of values. The labels of one chart differ in width. `100%`, `50%`, and
`0%` are an example. ratatui drew each label at the left of that column, so the
three labels started in one cell and ended in three cells.

The rule `align = "column"` of ADR 0039 pads each label with spaces at its left.
Only a chart in a stack under that rule therefore showed its labels at the
right. The user asked for the labels at the right, and for an option to choose
the left or the right.

## Decision

1. `thunderbox-core` holds the enum `LabelAlign` with the values `Left` and
   `Right`. The dashboard file writes them in lowercase. The default value is
   `Right`.
2. A panel can state the field `value_labels`. The field holds the value
   `"left"` or the value `"right"`. The writer leaves the field out when the
   panel states none.
3. `Dashboard::validate` returns an error for a panel that is not of the kind
   `timeseries` and states the field. The reader of the file already returns an
   error for any other value, and that error names the value and the two
   choices.
4. `TimeSeriesPanel` holds the method `value_labels`. The default value is
   `LabelAlign::Right`.
5. The chart gives the axis of values the alignment of that value, with
   `Axis::labels_alignment`.
6. The function `pad_labels` of the rule `align = "column"` puts its spaces on
   the side away from the alignment. A label at the right takes its spaces at
   the left, and a label at the left takes its spaces at the right. A chart in
   a stack keeps its width under both values.
7. The function `time_series` of `view.rs` gives each chart the field of its
   panel, or the default value.
8. The axis of time does not change.

## Consequences

- Each chart that states no field now draws its labels at the right. Each
  picture of a chart whose labels differ in width changes.
- A dashboard that wants the old picture writes `value_labels = "left"`.
- A chart in a stack under `align = "column"` keeps the same width and the same
  start cell as before.

## Measurements

The author took each rule out of the code in turn, and ran
`cargo test --no-fail-fast`.

| rule taken out | the tests that failed |
|---|---|
| the default value `Right` becomes `Left` | `a_chart_ends_each_label_of_the_axis_of_values_in_the_same_cell_by_default`, `the_value_right_draws_the_same_chart_as_the_default`, `a_stack_keeps_the_chart_width_under_both_values_and_the_labels_follow_the_value`, `a_dashboard_that_states_no_value_labels_ends_each_label_at_the_axis` |
| the chart ignores the value `Left` | `a_chart_starts_each_label_of_the_axis_of_values_in_the_same_cell_for_the_value_left`, `a_panel_that_states_value_labels_left_starts_each_label_at_the_left_edge` |
| `pad_labels` puts the spaces on the wrong side | `pad_labels_puts_the_spaces_on_the_side_away_from_the_alignment`, `a_stack_keeps_the_chart_width_under_both_values_and_the_labels_follow_the_value` |
| the view does not give the field to the chart | `a_panel_that_states_value_labels_left_starts_each_label_at_the_left_edge` |
| `validate` does not check the kind | `validate_returns_an_error_for_value_labels_on_a_panel_that_is_not_a_time_series` |

No test that existed before this change failed when the default changed. No
earlier test checked the side of the labels.

## Designs that the project rejected

- **Pad every label with spaces in place of `labels_alignment`.** The first
  label of the axis of time can make the column wider than every label of the
  axis of values. The padded labels would then still start at the left of that
  column, and they would not end at the axis.
- **One field for the whole dashboard.** The user asked for an option, and a
  field of a panel follows the form of the other fields of a chart.
