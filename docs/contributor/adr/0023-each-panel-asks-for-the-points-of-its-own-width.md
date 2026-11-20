# ADR 0023: Each panel asks for the points of its own width

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/grid.rs`, `crates/thunderbox-dashboard/src/runtime.rs`, `crates/thunderbox-dashboard/src/query.rs`, `crates/thunderbox-dashboard/src/view.rs` |
| Tests | `each_panel_asks_for_the_points_of_its_own_width`, `the_place_of_a_panel_holds_the_width_that_the_point_limit_reads`, `run_all_gives_the_queries_of_a_panel_the_point_limit_of_that_panel`, `the_point_limit_follows_the_width_of_the_panel_and_the_marker`, `the_default_dashboard_holds_these_widths_on_a_terminal_of_110_cells` |
| Related | [ADR 0009](0009-the-data-source-reduces-the-points.md), [ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md), [ADR 0028](0028-the-caller-reports-one-area-as-a-size.md), [ADR 0030](0030-a-set-of-queries-holds-one-job-for-each-panel.md) |

## Context

A dashboard holds a grid of 24 columns, and the renderer divides that grid onto
the cells of the terminal. A panel of 8 columns therefore holds about a third
of the screen.

Every panel asked for the points of the whole panel area before this change. A
panel of 8 columns asked for about three times the points that it draws. The
runtime held one number, it read that number one time, and `run_all` gave that
one number to every panel.

`DashboardRuntime::set_panel_area` carried the name `set_panel_width` before
this change, and then the name `set_panel_area_width`. Both callers of that
method always reported the width of the whole area, so the first name named
the wrong thing.

## Decision

Each panel asks its data source for the points of its own width.

The function `panel_width` of `crates/thunderbox-dashboard/src/grid.rs` returns
the width of one panel. The runtime reads that function for the point limit of
each panel. The renderer reads the same function inside `places` of the same
file, for the place of that same panel. One function returns both numbers, so
a panel cannot ask for the points of a width that it does not draw.

`run_all` took one point limit for the whole set of queries. It now takes one
`PanelJob` value for each panel, and
[ADR 0030](0030-a-set-of-queries-holds-one-job-for-each-panel.md) states that
type. A job that asks a data source holds one `PanelQueries` value. That value
holds the targets of a panel beside the point limit of that same panel. The two
fields travel together, so no panel can take the limit of another panel.

Every kind of panel follows this one rule:

- a chart draws a point at each position that the marker can address inside its
  own width;
- a stat panel draws a small chart of the most recent points, across its own
  width;
- a gauge and a bar gauge read the most recent point of the answer under the
  default reduce step `last`. A data source returns that point at every limit.
  `History::range` of the sysinfo data source always takes the newest sample,
  and a Prometheus server returns the last step before the end of the range. A
  coarser step moves that point by less than the width of one step. A reduce
  step such as `max` or `sum` reads every point, so its number can follow the
  point limit;
- a top panel shows the first rows that fit in its height. Its rows describe
  things and not moments. The sysinfo data source and the testdata data source
  therefore read no point limit for the query `processes`;
- a table shows the last rows that fit in its height. The point limit of a
  narrow table can be smaller than that number of rows, and such a table then
  shows fewer rows. The renderer draws no panel under 3 cells wide, and a table of 3
  cells asks for 6 points with a braille marker.

The width of a panel counts the border of that panel. It also counts the column
of cells that a chart keeps for the labels of the axis of values. A panel
therefore asks for a few points more than it draws, and never for fewer.
thunderbox keeps that margin.

## Consequences

The test `the_place_of_a_panel_holds_the_width_that_the_point_limit_reads`
holds the two numbers together at every width from 0 cells to 300 cells.

A reader of the newest name of `set_panel_area` asks no question about which
panel the number describes.
[ADR 0028](0028-the-caller-reports-one-area-as-a-size.md) states why the third
name is the last one.

The default dashboard asks for half of the points that it asked for before. The
section "Measurements" below gives the numbers.

A panel that asked for fewer points than it draws would lose a shape that the
data holds. That is the reason for the margin of the border and of the labels.

## Measurements

This table holds the default dashboard on a terminal of 110 cells. The column
`before` holds the points that a panel asked for before this change. The column
`after` holds the points that it asks for now.

| panel | kind | grid columns | cells | before | after |
|---|---|---|---|---|---|
| cpu | timeseries | 16 | 73 | 220 | 146 |
| cpu now | stat | 8 | 37 | 220 | 74 |
| memory | gauge | 8 | 36 | 220 | 72 |
| cores | bargauge | 8 | 37 | 220 | 74 |
| network | timeseries | 8 | 37 | 220 | 74 |
| the most recent rows for each core | table | 24 | 110 | 220 | 220 |
| the whole dashboard | | | | 1320 | 660 |

Two numbers of that table need an explanation. The panel `memory` holds 36
cells and the panel `cores` holds 37 cells, and both of them hold 8 grid
columns. `scale` multiplies before it divides, so the width of a panel follows
`x` as well as `w`. The panel on the right takes the remainder of the division. The panel of
the table holds the number that it held before, because that panel holds the
whole width of the area.

The dashboard held six panels in three rows, and the sum of the widths of one
row is the width of the area.

The table above records the default dashboard of the time of this rule. A later
change gave the right half of the last row to the panel `processes`. The panel
of the table now holds 12 grid columns. Those columns hold 55 cells, and the
panel asks for 110 points. The panel of the processes holds the other 55 cells.
The sum of the dashboard stays 660 points. The test
`each_panel_asks_for_the_points_of_its_own_width` keeps the grid of the table
above, because it records the measurement of this rule.

## Designs that the project rejected

A rule that gave the gauge, the bar gauge, the top panel, and the table a limit
of their own would be a second rule. A reader of the code would then need to
know which panel follows which rule. `docs/user/reference/data-sources/README.md`
could also no longer state one sentence about every panel.
