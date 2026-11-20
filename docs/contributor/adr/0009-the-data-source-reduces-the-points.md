# ADR 0009: The data source reduces the points

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 6 |
| Code | `crates/thunderbox-core/src/query.rs`, `crates/thunderbox-dashboard/src/grid.rs`, `crates/thunderbox-source-prometheus/src/step.rs`, `crates/thunderbox-source-prometheus/src/convert.rs` |
| Tests | `max_data_points_multiplies_the_width_by_the_sub_cells`, `for_request_raises_the_step_to_the_smallest_step_of_the_dashboard`, `for_request_raises_the_step_to_a_round_value`, `for_request_puts_both_ends_on_the_step`, `for_request_stays_inside_the_limit_of_the_server`, `for_request_never_returns_more_points_than_the_panel_asked_for`, `the_point_limit_of_the_panel_becomes_the_step_of_the_query`, `frames_leaves_a_hole_where_a_series_has_no_sample` |
| Related | [ADR 0007](0007-the-settings-of-a-panel-win-over-the-column.md), [ADR 0023](0023-each-panel-asks-for-the-points-of-its-own-width.md), [ADR 0062](0062-downsamples-promises-a-step-that-no-code-takes.md) |

## Context

A panel can draw a limited number of points. A data source can return many
more. Grafana is a dashboard program that runs in a web browser, and it asks
the data source to reduce the number of points.

A panel that is N cells wide and that uses a braille marker can address 2N
positions. [ADR 0007](0007-the-settings-of-a-panel-win-over-the-column.md)
describes the markers.

## Decision

The data source reduces the number of points, as Grafana does. Each panel asks
for this number of points:

```
max_data_points = width_in_cells * sub_cells_across
```

The width in that formula is the width of one panel. It is not the width of the
area that holds every panel. thunderbox divides the grid of the dashboard onto
the cells of the terminal, and each panel then holds a width of its own. A
panel of 8 grid columns of 24 holds about 37 cells of a terminal of 110 cells,
and a panel of 16 columns holds about 73 cells.

The point limit of each panel therefore differs from the point limit of the
panel beside it. The function `panel_width` of
`crates/thunderbox-dashboard/src/grid.rs` returns the width of one panel, and
both the renderer and the runtime read that one function.
[ADR 0023](0023-each-panel-asks-for-the-points-of-its-own-width.md) records
that decision.

The runtime computes the number again when the area of the panels changes. An
area that changes a point limit then reaches the data source with a later set
of queries. [ADR 0021](0021-a-new-area-waits.md) and
[ADR 0022](0022-an-area-that-changes-no-point-limit-starts-no-set.md) state
when that set starts. Each data source turns the number into the form that it
needs. The `step` of Prometheus and `time_bucket` in SQL are two examples of
such a form.

## Consequences

The Prometheus data source does this work today. It turns the point limit into
the step of a range query. It raises that step in three ways, and it never
lowers it:

- it raises the step to the smallest step that the dashboard permits, which is
  the `min_step` option;
- it raises the step to the next round value, such as 2 seconds or 5 minutes.
  It then moves both ends of the range onto that step. The times of the points
  therefore stay still while the range moves, and a chart that refreshes does
  not shake;
- it raises the step again until the number of points fits both the panel and
  the limit of 11000 points that a Prometheus server holds.

The server returns a point for every step, and it returns no point where the
series holds no value. The data source builds the time column from the step,
and it puts each sample in its own place. A place that holds no sample keeps a
hole in the data, so a chart shows a gap and not a straight line.

No code of thunderbox reduces the number of points that a data source returns.
The field `SourceDescriptor::downsamples` promises that reduction for a data
source that cannot reduce the points itself. Every read of that field sits
inside a test module. `Cargo.lock` holds no crate that reduces a series.
[ADR 0062](0062-downsamples-promises-a-step-that-no-code-takes.md) records
that absence, and it asks whether this project takes the reduction.

The design that takes the reduction would run the crate `minmaxlttb` in
thunderbox **only when the data source cannot reduce the points itself**. A CSV
file and a plain SQL table are examples. The LTTB algorithm keeps the shape of
a signal. A system monitor instead shows the smallest and the largest value in
each column. That form never hides a short, high peak. That design would offer
both forms, and it would use the smallest and the largest value for a signal
that behaves like a meter.

[The research](../research.md) records the crates for that work. `minmaxlttb`
0.1.4 first selects the smallest and the largest value in each group, and it
then runs the LTTB algorithm. It is much faster than `lttb` 0.2.0 at the sizes
that matter here. That older crate made its last release in 2020, and it is
small enough to copy.
