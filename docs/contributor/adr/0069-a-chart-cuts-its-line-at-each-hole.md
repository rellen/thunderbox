# ADR 0069: A chart cuts its line at each hole

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-21 |
| Source | new |
| Code | `crates/thunderbox-widgets/src/timeseries.rs` |
| Tests | `a_line_chart_leaves_the_columns_over_a_hole_empty`, `a_filled_chart_leaves_the_columns_over_a_hole_empty`, `a_run_of_one_point_between_two_holes_draws_a_cell`, `the_legend_lists_each_series_once_when_each_series_holds_a_hole`, `a_line_chart_joins_two_values_with_a_gap_in_time_between_them`, `collect_keeps_a_gap_in_time_inside_one_run`, `collect_cuts_a_series_at_each_value_that_is_not_finite` |
| Related | [ADR 0009](0009-the-data-source-reduces-the-points.md) |

## Context

A hole is a place in a series where the data source sent no value. The data
source prometheus and the data source testdata each send a hole as `None`.
[ADR 0009](0009-the-data-source-reduces-the-points.md) states that a chart then
shows a gap, and not a straight line.

The time series panel broke that promise. `collect` dropped each hole, and it
kept the points on both sides of the hole in one list. The panel gave that list
to ratatui as one `Dataset`, and ratatui joins each point of a `Dataset` to the
next point. The chart therefore drew a line across each hole, and a filled
chart also filled the area below that line. The line showed values that no data
source sent.

The small chart of the stat panel already marked a hole with the shade `░`.
Two panels therefore drew one hole in two ways.

## Decision

`collect` cuts each series at its holes. A run is the stretch of points between
two holes, and each run is a range of the points of its series. The panel gives
ratatui one `Dataset` for each run. The first `Dataset` of a series alone takes
the name of the series, because the legend lists each `Dataset` that holds a
name.

The change takes three choices:

1. **The columns over a hole stay empty, with no mark.** Each value of a chart
   draws at least one dot, so an empty column cannot show a value. A reader
   therefore reads an empty column as a place with no data. The small chart of
   the stat panel needs its shade for a different reason. A bar at the bottom
   of its range draws an empty cell, so an empty cell there can be a value.
2. **No field of a dashboard file selects the join.** A line across a hole
   shows values that no data source sent, so no dashboard needs it. A data
   source that wants a line with no break can fill its holes itself. A field
   would also add a key to the file format and to its reference. That key would
   select a chart that is less true.
3. **Only `None` and a value that is not finite make a hole. A gap in time does
   not.** The panel does not know the interval at which a data source collects
   its values. A program that embeds the panel can send a point at any moment.
   A rule on time would need a limit that no frame states. The data source
   knows where it holds no value, and it marks that place with `None`.

## Consequences

A series with many holes gives ratatui many datasets. Each `Dataset` borrows a
slice of the points of its series, so the change copies no point.

A run of one point draws that point alone. ratatui draws each point of a
`Dataset`, and it joins two points only when the `Dataset` holds two or more.

A hole adds nothing to the bounds of the axes. `last_value` ignores a hole, as
it did before this record.

The chart draws a line across each gap in time. A data source that holds no
value for a moment marks that moment with `None`.

## Measurements

A panel of 62 cells by 12 rows drew a series of 30 points, one each second: 10
points at 80, then 10 holes, then 10 points at 20. Before this record, the line
chart drew a diagonal from 80 to 20 across about 20 columns. The filled chart
also filled each cell below that diagonal. A value that is not finite gave the
same picture as `None`.

The command `cargo test --no-fail-fast` ran once for each change below. Each
change removed one rule from the code.

| the change | the tests that failed |
|---|---|
| The panel gives ratatui one `Dataset` for each series. | 2 |
| Each run takes the name of the series. | 1 |
| `collect` keeps a value that is not finite. | 3 |
| `collect` cuts a run at a gap in time of more than 1 second. | 2 |
| The panel drops each run of one point. | 1 |
