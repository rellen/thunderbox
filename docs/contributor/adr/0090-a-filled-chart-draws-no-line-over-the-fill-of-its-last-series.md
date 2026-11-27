# ADR 0090: A filled chart draws no line over the fill of its last series

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-27 |
| Source | new |
| Code | `crates/thunderbox-widgets/src/timeseries.rs` |
| Tests | `a_filled_chart_of_one_series_draws_a_fill_below_its_line_in_the_bottom_row`, `a_filled_chart_of_one_series_draws_the_same_cells_as_its_fill_alone`, `a_filled_chart_shows_a_series_that_lies_below_a_later_series`, `a_filled_chart_shows_the_series_that_lies_above_an_earlier_series`, `a_line_chart_shows_each_series_that_lies_below_a_later_series` |
| Related | [ADR 0082](0082-a-filled-chart-draws-each-line-over-every-fill.md) |

## Context

[ADR 0082](0082-a-filled-chart-draws-each-line-over-every-fill.md) made a
filled chart draw every `Area` dataset first. It then drew one `Line` dataset
for each run of each series.

ratatui draws each dataset as a layer of its own. A later layer replaces the
glyph and the colour of each cell that it paints. The method `Context::marker`
in `canvas.rs` of ratatui-widgets 0.3.2 starts such a layer. The `Line` layer
of a series therefore erased the dots of its fill in each cell that the line
crosses.

A series with low values lost its whole fill, because the fill fits in the
cells of the line. A high part of a series kept the fill below the cells of the
line. A reader saw one block of fill, and a bare line where the values are low.

A reader measured the fault on one series of 60 points. The values stay low,
near 5, and one bump rises to between 40 and 55. The panel is 60 cells by 10
cells and it is filled.

- The bottom row of the plot drew only the glyphs of a line, for example
  `⠒⠒⠉⠑⠒⠢⠤⠤`.
- The panel held 588 braille dots.

## Decision

A filled chart draws the `Line` datasets for every series but the last.

- The panel builds the `Area` datasets for every run of every series, as
  before.
- The panel then appends one `Line` dataset for every run of every series but
  the last series.
- An `Area` dataset draws its points and its line in its own layer. The last
  fill therefore holds the line of its own series, and no layer of a line
  covers it.

## Consequences

- A chart of one series draws exactly one `Area` dataset for each run, as it
  did before ADR 0082. The chart keeps every dot of its fill.
- A series that lies below a later series still shows its line over the fill
  of the later series.
- In a chart of several series, a cell where the line of a lower series
  crosses a fill still shows the line alone. The fill loses the dots of that
  cell.
- A change to the way that the panel builds datasets must keep the order: every
  fill, then the line of each series but the last.

## Measurements

The reader drew the same panel as in the context, with the change.

- The panel holds 800 braille dots.
- The bottom row of the plot draws full cells, for example `⣶⣶⣿⣷⣶⣦⣤⣤`.

The reader then put the old line pass back, which draws a line for every
series. The command was `cargo test --no-fail-fast`.

- Two tests failed:
  `a_filled_chart_of_one_series_draws_a_fill_below_its_line_in_the_bottom_row`
  and `a_filled_chart_of_one_series_draws_the_same_cells_as_its_fill_alone`.
- The three tests of ADR 0082 still passed.

The second test builds its reference with the `Chart` and the `Dataset` of
ratatui. The reference holds the `Area` datasets alone, and it uses the axes
and the labels of the panel.

## Designs that the project rejected

- Draw every fill and every line in one layer. ratatui starts a new layer for
  each dataset of a `Chart`. A canvas of our own would need its own axes, its
  own labels, and its own legend.
- Draw no line pass at all. A series that lies below a later series would
  vanish under the later fill. This is the fault of GitHub issue #15, and it
  returns.
