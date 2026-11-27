# ADR 0096: A filled chart of several series chooses one series for each cell

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-27 |
| Source | new |
| Code | `crates/thunderbox-widgets/src/timeseries.rs` |
| Tests | `the_owner_of_a_cell_is_the_series_whose_line_crosses_it_before_any_fill`, `the_owner_of_a_cell_that_two_lines_cross_is_the_later_series`, `the_owner_of_a_cell_with_no_line_is_the_series_whose_area_starts_lowest`, `the_owner_of_a_cell_goes_to_the_later_series_when_two_areas_start_at_one_row`, `a_series_with_no_area_in_a_cell_cannot_own_the_cell`, `a_cell_that_holds_no_data_has_no_owner`, `a_cell_with_no_line_has_no_line_owner`, `the_colour_that_names_a_series_returns_the_series`, `a_filled_chart_shows_the_second_series_on_the_bottom_row_outside_the_burst_of_the_first`, `a_filled_chart_shows_the_first_series_above_the_second_series_inside_the_burst`, `a_filled_chart_shows_the_fill_of_a_lower_series_below_its_line_and_the_higher_series_above`, `the_order_of_two_series_does_not_change_a_cell_that_holds_no_line_of_both`, `a_line_shows_in_the_fill_of_another_series_that_starts_higher_in_the_column`, `a_cell_that_the_line_of_a_series_crosses_equals_the_cell_of_its_area_alone`, `the_legend_lists_each_series_once_and_no_data_cell_replaces_a_cell_of_the_legend`, `each_name_of_the_legend_has_the_colour_of_its_series`, `a_legend_that_does_not_fit_leaves_no_legend_of_one_name_in_the_chart`, `a_legend_that_fits_leaves_the_cells_outside_the_legend_as_a_chart_with_no_legend`, `a_series_with_a_hole_draws_no_fill_in_the_columns_of_the_hole_in_a_chart_of_two_series`, `every_marker_composes_a_chart_of_two_series_and_shows_both_colours`, `each_marker_draws_the_glyphs_of_its_own_in_a_composed_chart`, `a_filled_panel_drawn_twice_into_one_buffer_draws_the_cells_of_one_draw`, `a_buffer_that_held_other_cells_gives_the_cells_of_a_blank_buffer_where_the_chart_draws_data`, `a_chart_of_more_than_256_series_shows_the_last_series_where_all_lines_share_a_cell`, `a_composed_chart_of_the_shape_of_rx_and_tx_draws_the_cells_of_the_first_version`, `a_composed_chart_of_three_series_with_the_marker_half_block_draws_the_cells_of_the_first_version`, `a_composed_chart_of_three_series_with_a_legend_draws_the_cells_of_the_first_version`, `a_chart_that_is_not_filled_draws_the_cells_that_one_chart_of_lines_draws` |
| Related | [ADR 0082](0082-a-filled-chart-draws-each-line-over-every-fill.md), [ADR 0094](0094-the-program-draws-a-frame-only-when-the-screen-can-change.md) |

This record supersedes [ADR 0090](0090-a-filled-chart-draws-no-line-over-the-fill-of-its-last-series.md).

## Context

ratatui draws each dataset of a `Chart` as a layer of its own. A later layer
replaces the glyph and the colour of each cell that it paints. A cell holds one
colour.

[ADR 0082](0082-a-filled-chart-draws-each-line-over-every-fill.md) and
[ADR 0090](0090-a-filled-chart-draws-no-line-over-the-fill-of-its-last-series.md)
chose an order of layers: every fill, then the line of each series but the
last. Two faults remain for a filled chart of two series or more.

1. The line layer of a series replaces every cell that it shares with the last
   series. The default dashboard draws `network.rx` and then `network.tx`. Both
   series stay near 0 most of the time, so both lie in the bottom row of
   cells. The line of `rx` wins each shared cell. The chart shows `tx` only
   under the bursts of `rx`, where the line of `rx` is higher. A user saw this
   and asked for a fix.
2. A line layer replaces the dots of the fill in each cell that the line
   crosses. [ADR 0090](0090-a-filled-chart-draws-no-line-over-the-fill-of-its-last-series.md)
   states this consequence. The fill of a later series also hides the fill of
   each earlier series below it.

No order of layers repairs both faults, because each layer replaces a whole
cell.

## Decision

A filled chart of two series or more draws its plot through ratatui several
times into scratch buffers. A scratch buffer is a buffer that the panel draws
into and does not show. Each scratch buffer starts blank. It holds no cell of
the real buffer, so a cell that the real buffer held before the draw changes no
choice. The panel then chooses one series for each cell. The panel draws no
point itself. Every marker therefore works, and ratatui still chooses the place
of the plot.

A chart that is not filled, and a chart of one series, draw as before through
one `Chart`. The line pass of ADR 0082 and ADR 0090 leaves the code.

All charts below share the same block, axes, bounds, labels, label alignment,
and area. ratatui therefore puts the plot in the same cells each time.

1. **Reference.** The panel draws a `Chart` with no dataset into a scratch
   buffer, and it calls that buffer `empty`. A data cell of any other scratch
   buffer is a cell that differs from the same cell of `empty`.
2. **One area buffer for each series.** For series `k`, the panel draws a
   `Chart` that holds only the `Area` datasets of that series into `area[k]`.
   There is one dataset for each run. Each dataset has `fill_to_y` set to the
   bottom of the axis, the colour and the marker of the series, and no name. An
   `Area` dataset draws the points, the line, and the fill in one layer. A cell
   of `area[k]` therefore holds the dots of the line and of the fill together.
3. **One buffer for the lines of all series.** The panel draws one `Chart` that
   holds the `Line` datasets of every series, in the order of the series, with
   no name. The panel gives series `k` a colour that names it in this buffer
   alone. The palette of a theme repeats after a few series, so the palette
   cannot name a series. The colour is `Rgb`, and its three bytes hold the
   number `k`. ratatui draws a later dataset over an earlier dataset. The
   colour of a cell therefore names the last series that has a line in that
   cell. A
   marker of half blocks can put two series in one cell. The foreground holds
   the upper half, and the background holds the lower half. The later of the two
   series owns that cell. The colour can name 16,777,216 series. A dashboard
   cannot hold so many, because the panel keeps one buffer for each series. The
   buffer of the lines is called `lines`.
4. **The owner of each cell.** The function `owner` chooses the owner of a cell
   in this order:
   1. `lines` can hold a data cell there. Then the owner is the series that the
      colour of that cell names. A line wins over a fill. Between two lines,
      the later series wins.
   2. Otherwise, some series can have a data cell in its area buffer there.
      Then the owner is the one among them whose area starts lowest in that
      column of cells. For each series, take the smallest row of its data cells
      in `area[k]` in that column. The row at the top of the screen has the
      smallest number. Choose the series with the largest such row. That series
      has the nearest line above the cell, so its fill is in front. When two
      series tie, the later series wins.
   3. Otherwise the cell has no owner.
5. **The frame.** The panel draws one `Chart` with one dataset for each series.
   Each dataset holds the name and the colour of the series and no point. The
   panel draws that chart twice: into a blank scratch buffer that it calls
   `frame`, and into the real buffer. ratatui draws the border, the axes, the
   labels, and the legend each time. A cell of `frame` that differs from `empty`
   belongs to the legend. Next, the panel visits each cell that has an owner
   `k` and does not belong to the legend. It writes the cell of `area[k]` into
   the real buffer. The write replaces the whole cell, which includes the
   symbol, the foreground, and the background. The panel never compares a cell
   of the real buffer with a scratch cell. A cell that the real buffer held
   before the draw therefore changes nothing. The legend stays on top.

The function `owner` draws nothing. A test calls it with plain values.

## Consequences

- A cell that a line crosses shows that line together with the fill of the same
  series in that cell. No line removes the dots of its own fill.
- Each series shows its fill from its line down to the next line below it. The
  lowest series shows its fill down to the bottom of the axis.
- Where two lines share a cell, only the later series shows.
- The legend, the axes, and the labels are the ones that one `Chart` draws.
- The order of the series changes only a cell that two lines share, and a tie.
- A draw into a buffer that holds an earlier draw gives the same cells as a draw
  into a blank buffer. A program that keeps one `Buffer` between frames therefore
  shows no part of the earlier frame in a cell that holds data.
- A filled chart of `n` series draws `n + 4` charts for each frame. These are
  the reference, one area chart for each series, one chart of all the lines,
  the blank frame, and the real frame. Each frame of a filled chart of eight
  series, on a panel of 110 cells by 32 cells, takes about 1.8 milliseconds.
  The old order of layers took about 0.86 milliseconds.
  [ADR 0094](0094-the-program-draws-a-frame-only-when-the-screen-can-change.md)
  keeps the count of frames low.
- A change to the way that the panel builds datasets must keep this rule. The
  panel must not draw a line over the fill of its own series.

## Measurements

The reader ran `cargo test --no-fail-fast` for each step.

- The tests of the list above hold the rule. The reader wrote the tests of the
  first version first, and the function `owner` returned `None` for every cell.
  Nine tests failed. Five of them were tests of the function `owner`. The other
  four were
  `a_filled_chart_shows_the_second_series_on_the_bottom_row_outside_the_burst_of_the_first`,
  `a_filled_chart_shows_the_fill_of_a_lower_series_below_its_line_and_the_higher_series_above`,
  `the_order_of_two_series_does_not_change_a_cell_that_holds_no_line_of_both`,
  and `a_cell_that_the_line_of_a_series_crosses_equals_the_cell_of_its_area_alone`.
  All tests passed after the change.
- A review of the first version found two faults. Its scratch buffers started
  as a copy of the real cells, and no test held four other rules of steps 1, 2,
  3, and 5. The repair changed the scratch buffers and the buffer of the lines.
  The three tests that start with `a_composed_chart` hold the cells of the first
  version on a blank buffer, and they passed after the repair.
- The three tests of ADR 0082 and the two tests of ADR 0090 still pass. The
  decision makes none of their claims false.
- The reader took out each rule in turn. These tests failed:

  | rule that the reader took out | failed tests |
  |---|---|
  | a line wins over a fill | `the_owner_of_a_cell_is_the_series_whose_line_crosses_it_before_any_fill`, `a_line_shows_in_the_fill_of_another_series_that_starts_higher_in_the_column`, and two tests of `a_composed_chart` |
  | the later of two lines wins | `the_owner_of_a_cell_that_two_lines_cross_is_the_later_series`, and one test of `a_composed_chart` |
  | the later of two lines wins when the later series owns the lower half of a cell | `the_owner_of_a_cell_that_two_lines_cross_is_the_later_series` |
  | the area that starts lowest wins | `the_owner_of_a_cell_with_no_line_is_the_series_whose_area_starts_lowest`, `the_owner_of_a_cell_goes_to_the_later_series_when_two_areas_start_at_one_row`, `a_filled_chart_shows_the_fill_of_a_lower_series_below_its_line_and_the_higher_series_above`, `a_chart_of_more_than_256_series_shows_the_last_series_where_all_lines_share_a_cell`, and two tests of `a_composed_chart` |
  | a tie goes to the later series | `the_owner_of_a_cell_goes_to_the_later_series_when_two_areas_start_at_one_row`, `a_chart_of_more_than_256_series_shows_the_last_series_where_all_lines_share_a_cell`, and two tests of `a_composed_chart` |
  | the cell comes from the area buffer, and not from the buffer of the lines | eleven tests, among them `a_cell_that_the_line_of_a_series_crosses_equals_the_cell_of_its_area_alone`, `a_filled_chart_shows_the_fill_of_a_lower_series_below_its_line_and_the_higher_series_above`, and the three tests of `a_composed_chart` |
  | the legend stays on top | `the_legend_lists_each_series_once_and_no_data_cell_replaces_a_cell_of_the_legend`, `each_name_of_the_legend_has_the_colour_of_its_series`, `a_legend_that_fits_leaves_the_cells_outside_the_legend_as_a_chart_with_no_legend`, and one test of `a_composed_chart` |
  | the scratch buffers start blank | `a_filled_panel_drawn_twice_into_one_buffer_draws_the_cells_of_one_draw`, `a_buffer_that_held_other_cells_gives_the_cells_of_a_blank_buffer_where_the_chart_draws_data` |
  | the write replaces the whole cell | `a_buffer_that_held_other_cells_gives_the_cells_of_a_blank_buffer_where_the_chart_draws_data`, and one test of `a_composed_chart` |
  | the scratch datasets hold no name | `a_legend_that_fits_leaves_the_cells_outside_the_legend_as_a_chart_with_no_legend`, and one test of `a_composed_chart` |
  | the scratch buffers draw the marker of the panel | `each_marker_draws_the_glyphs_of_its_own_in_a_composed_chart`, and one test of `a_composed_chart` |
  | each name of the legend holds the colour of its series | `each_name_of_the_legend_has_the_colour_of_its_series`, and one test of `a_composed_chart` |
  | the colour that names a series holds one byte | `the_colour_that_names_a_series_returns_the_series`, `a_chart_of_more_than_256_series_shows_the_last_series_where_all_lines_share_a_cell` |
  | a colour that `empty` holds names no series | `a_cell_with_no_line_has_no_line_owner` |

  The first version of the unit test for the first rule passed with that rule
  taken out, because the area rule gave the same answer. The reader changed
  the values of that test. The reader also wrote the test
  `a_line_shows_in_the_fill_of_another_series_that_starts_higher_in_the_column`,
  and both tests then failed with the rule taken out.
  The line datasets of the buffer `lines` hold no name too. A name there
  changes no cell. The legend of that chart stands in the cells of the real
  legend, and the legend rule keeps those cells. No test can tell the two apart.
- The reader timed `render` of a filled chart of eight series on a panel of
  110 cells by 32 cells. Each series was a random walk of 300 points. The
  loop ran 1000 draws into a new buffer with a release build, after 50 draws
  to warm up. Three loops gave the same time within 1 percent. The code of
  commit b35320a took 0.86 milliseconds for each draw. The code of this
  record takes 1.81 milliseconds for each draw. A chart of one series, and a
  chart that is not filled, still use one `Chart` and cost what they cost
  before.

A reader looks at a chart of the shape of `rx` and `tx`. The rows below show
the colour of each cell inside the axes. The digit `0` is `rx`, and the digit
`1` is `tx`. The border and the labels of the values keep their own
characters. The bottom row shows `tx` outside the burst, and `rx` stands above
`tx` inside the burst.

```text
│ 42│                            00000000                            │
│   │                            00000000                            │
│   │                           0000000000                           │
│   │                           0000000000                           │
│ 21│                           0000000000                           │
│   │                           0000000000                           │
│   │                           0000000000                           │
│  0│1111111111111111111111111111111111111111111111111111111111111111│
```

## Designs that the project rejected

- Keep the order of layers of ADR 0090. The fault of this record then stays:
  `tx` hides under `rx`, and the chart shows `tx` only under the bursts of
  `rx`.
- Draw a line pass for every series. A line layer then removes the dots of its
  own fill. This is the fault of ADR 0082 that ADR 0090 repaired for the last
  series alone.
- Draw one chart of lines for each series, and read the owner of a cell from
  that chart. The first version did this. A chart of eight series then takes
  2.22 milliseconds for each draw, against 1.81 milliseconds for one chart of
  all the lines. Both versions choose the same owner for each cell.
- Add a field of the panel that turns the fill off. The field avoids the fault
  and does not repair it. A person must then choose between the fill and the
  second series.
- Draw a canvas of our own that paints each dot. The canvas would copy the
  layout, the axes, and the marker code of ratatui.
