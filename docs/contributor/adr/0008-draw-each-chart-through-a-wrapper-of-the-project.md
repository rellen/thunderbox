# ADR 0008: Draw each chart through a wrapper of the project

| field | value |
|---|---|
| Status | Accepted, not built |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 5, and section 13 item 3 |
| Code | `crates/thunderbox-widgets/src/timeseries.rs` |
| Tests | No test holds this decision, because the project has not built the wrapper. The test `the_limit_gives_no_more_than_a_third_of_the_chart_to_the_labels` holds the rule that `label_width_limit` copies from ratatui. |
| Related | [ADR 0006](0006-use-ratatui-crossterm-and-tokio.md), [ADR 0007](0007-the-settings-of-a-panel-win-over-the-column.md), [ADR 0039](0039-a-stack-of-charts-can-share-one-width.md) |

## Context

The `Chart` type that ratatui supplies will not meet every later need. A chart
can need a time axis that ends at the right, a legend in a chosen place, and a
logarithmic axis. [ADR 0006](0006-use-ratatui-crossterm-and-tokio.md) lists
those limits of ratatui.

`chart.rs` is the file of `ratatui-widgets` that holds `Chart`. A copy of
`chart.rs` must follow every change that ratatui makes to that file.

## Decision

A panel renderer must not build a `ratatui_widgets::chart::Chart` value. Each
renderer must draw a chart through an internal type `TimeGraph`. That wrapper
lets the project replace the code that draws a chart in one place.

The project does not copy `chart.rs` before a need appears.

## Consequences

The code does not follow this decision yet. The wrapper does not exist.
`crates/thunderbox-widgets/src/timeseries.rs` builds a
`ratatui_widgets::chart::Chart` value, and it calls no internal `TimeGraph`
type.

The rule that gives one width to a stack of charts shows the cost of the
absence of a wrapper. [ADR 0039](0039-a-stack-of-charts-can-share-one-width.md)
states that rule. The function `label_width_limit` holds a rule of `ratatui-widgets`
version 0.3.2. That rule gives no more than a third of the width of a chart to
the labels at the left edge. It measures the space inside the border of the
panel. The documentation of `ratatui` does not state that rule. A comment
inside a private method of `chart.rs` holds it. A release that changes the
third would cut a label of this project.

The wrapper would hold such a rule in one place. The project will build the
wrapper when a second rule of that kind appears.
