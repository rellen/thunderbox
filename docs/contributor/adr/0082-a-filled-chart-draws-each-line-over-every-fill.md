# ADR 0082: A filled chart draws each line over every fill

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-27 |
| Source | GitHub issue #15 (code review finding 48) |
| Code | `crates/thunderbox-widgets/src/timeseries.rs` |
| Tests | `a_filled_chart_shows_a_series_that_lies_below_a_later_series`, `a_filled_chart_shows_the_series_that_lies_above_an_earlier_series`, `a_line_chart_shows_each_series_that_lies_below_a_later_series` |
| Related | [ADR 0069](0069-a-chart-cuts-its-line-at-each-hole.md) |

## Context

The time series panel builds one dataset for each run of each series.
[ADR 0069](0069-a-chart-cuts-its-line-at-each-hole.md) explains the
runs. A filled chart gave each dataset the graph type `Area`, with a fill down
to the bottom of the axis.

ratatui draws each dataset as one layer, in series order. A later layer
replaces the glyph and the colour of each cell that it paints. The area of a
later series that lies above an earlier series therefore covered every cell of
the earlier series. The earlier series vanished, and the legend still listed
it.

Each chart of a dashboard is filled, because `DashboardView::time_series` never
calls `filled`. The default dashboard lost a series in the panel `network` and
in the panel `cpu`.

A reader measured the fault on a panel of 60 cells by 12 cells. The first
series lies below the second series. The filled chart drew 0 cells in the
colour of the first series, and 424 cells in the colour of the second series.
The same frame with `filled(false)` gave 53 cells of each colour.

## Decision

A filled chart draws every fill first, and it draws every line after all the
fills.

- The panel builds the `Area` datasets for every run of every series, in series
  order. The first run of each series carries the name for the legend.
- The panel then appends one `Line` dataset for every run of every series, in
  the same order. Each `Line` dataset has the colour and the marker of its
  series, and it has no name.
- A chart that is not filled draws the `Line` datasets alone, as before.

## Consequences

- Each series shows its line. The legend still lists each series once.
- The fill of a series that lies below a later series stays hidden under the
  fill of the later series. A reader reads the lower series by its line.
- A cell where a line crosses a later fill shows the glyph and the colour of
  the line.
- A filled chart holds twice as many datasets as before.
- A change to the way that the panel builds datasets must keep the order: every
  fill, then every line.

## Measurements

The command was `cargo test -p thunderbox-widgets --lib a_filled_chart_shows`.
The input was the frame of the tests, drawn on a panel of 60 cells by 12 cells
with no legend.

- Before the change, the first test failed with this message: `Cells of the
  first colour: 0. Cells of the second colour: 424.` The second test passed.
- After the change, all three tests passed.
- With the second pass of `Line` datasets taken out, `cargo test --no-fail-fast`
  failed one test of the workspace:
  `a_filled_chart_shows_a_series_that_lies_below_a_later_series`.

## Designs that the project rejected

- Fill only a chart that holds one series. The look of a chart would then
  change with the number of series that a query returns. The method `filled`
  also documents that it fills below each line.
- Draw the fills in order of height. The order differs from one column to the
  next, and a dataset of ratatui draws one layer for the whole chart.
