# ADR 0039: A stack of charts can share one width

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-core/src/dashboard.rs`, `crates/thunderbox-dashboard/src/view.rs`, `crates/thunderbox-widgets/src/timeseries.rs` |
| Tests | `a_dashboard_that_states_no_rule_lets_each_panel_measure_its_own_labels`, `a_dashboard_that_states_the_rule_gives_one_chart_width_to_a_stack`, `the_rule_leaves_two_panels_of_different_widths_alone`, `a_panel_that_holds_no_data_widens_no_other_panel`, `a_dashboard_can_state_that_a_stack_of_charts_holds_one_width`, `a_width_that_the_caller_states_gives_two_panels_one_chart_width`, `a_width_that_is_too_large_still_gives_two_panels_one_chart_width`, `two_panels_hold_one_chart_width_at_every_width_of_the_area`, `the_limit_gives_no_more_than_a_third_of_the_chart_to_the_labels` |
| Related | [ADR 0008](0008-draw-each-chart-through-a-wrapper-of-the-project.md) |

## Context

A stack is the set of panels that hold the same columns of the grid, one above
another.

A chart keeps a column of cells at its left edge for the labels of the axis of
values. `ratatui` takes the width of that column from the widest label of that
one chart. A panel that shows a share of use writes `100%`, and a panel that
shows a rate writes `8.3GB/s`. The second chart therefore starts three cells
further to the right.

A person who reads one panel above the other then reads two axes of time that
start at different cells. One moment then stands in two places. Grafana had
the same fault.

## Decision

A dashboard can give one width to the charts of a stack, and it does not do so
by default. The `align` field of a dashboard states the rule:

| value | what the renderer does |
|---|---|
| `none` | Each panel measures its own labels. This value is the default value. |
| `column` | Every chart of a stack starts at the same cell. |

The rule is a choice, and not a fix that the project forces. A dashboard that
holds one chart in a column gains nothing from the rule. A person who wants
those cells for the chart, and not for the labels, keeps the default rule.

For a dashboard that states `align = "column"`, `DashboardView::render`
measures every chart panel that it draws, before it draws any of them. It
groups the panels by the pair `(x, w)` of the grid, because two panels of that
pair hold the same cells across the screen. It then passes the widest
measurement of each group to `TimeSeriesPanel::label_width`.
The panel adds spaces at the left of each label of the axis of values, until
every label holds the stated width. `ratatui` then measures the same width in
every chart of the stack.

## Consequences

Two limits belong to this rule:

- `ratatui` gives no more than a third of the width of a chart to those
  labels. A panel that asks for more keeps that third. Every panel of a stack
  holds the same width and the same third, so the charts still start at the
  same cell.
- Two panels of different widths keep their own measurements. Two such charts
  cannot start and end at the same two cells, so the rule leaves them alone.

The measurement reads the data a second time. It does so only for a dashboard
that states `align = "column"`.

The function `label_width_limit` holds the limit of a third.
[ADR 0008](0008-draw-each-chart-through-a-wrapper-of-the-project.md) records
what that limit costs this project.
