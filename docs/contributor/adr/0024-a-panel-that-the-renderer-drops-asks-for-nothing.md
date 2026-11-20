# ADR 0024: A panel that the renderer drops asks for nothing

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-widgets/src/panel.rs`, `crates/thunderbox-dashboard/src/grid.rs`, `crates/thunderbox-dashboard/src/runtime.rs`, `crates/thunderbox-dashboard/src/view.rs` |
| Tests | `the_runtime_asks_about_exactly_the_panels_that_the_renderer_draws`, `a_panel_that_the_renderer_drops_asks_a_data_source_for_nothing`, `a_column_of_eight_panels_drops_these_panels_on_each_terminal`, `the_default_dashboard_drops_a_panel_only_under_a_terminal_of_ten_rows` |
| Related | [ADR 0023](0023-each-panel-asks-for-the-points-of-its-own-width.md), [ADR 0025](0025-the-rows-of-the-grid-belong-to-the-dashboard.md), [ADR 0026](0026-panel-state-gains-no-fourth-variant.md), [ADR 0027](0027-a-dropped-panel-keeps-its-answer.md), [ADR 0054](0054-the-program-binds-enter-and-space.md), [ADR 0061](0061-a-duration-that-does-not-fit-ends-the-program.md) |

## Context

The renderer draws no panel under 3 cells wide or under 3 cells high. This
record calls such a panel a panel that the renderer drops.

The runtime knew the width of the panel area alone before this change, and it
knew no height. It therefore asked a data source about every panel, and a
person paid for a query whose answer nobody read.

The number 3 lived in seven places before this change: five panels of
`thunderbox-widgets`, and two lines of `view.rs`.

The functions `panel_size` and `place_panel` held the arithmetic of the grid
before the project added the row panels. A row panel moves each panel below
it, so the place of one panel now depends on the other panels.

## Decision

A panel that the renderer drops asks a data source for nothing.

`thunderbox_widgets::draws_a_panel` now holds the smallest size. That function
returns `true` when a panel of a given size holds a cell for its data. The rule
belongs to that crate, because every panel of that crate draws nothing at a
smaller size. The number 3 now lives in one constant of
`crates/thunderbox-widgets/src/panel.rs`. Every one of the seven places now
reads the function. The top panel and each later caller read it as well.

The private module `grid` of `thunderbox-dashboard` holds the arithmetic that
gives a panel its size:

- `rows_of_grid` returns the rows of the grid of a dashboard;
- `places` returns the place of every panel, in terminal cells.

One function therefore returns every place. `DashboardView::render` reads
`places` for the place of a panel. `DashboardRuntime::point_limits` reads it
for the point limit of that same panel. Each of the two then gives that size to
`draws_a_panel`.

## Consequences

The test `the_runtime_asks_about_exactly_the_panels_that_the_renderer_draws`
holds the two parts together. It draws the dashboard into a buffer and counts
the titles that the renderer wrote. It then counts the queries that reached a
data source. The two counts must agree at each height from 3 rows to 30 rows.

The change saves a query only on a terminal that already hides part of the
dashboard. The section "Measurements" below gives the numbers.

The case that saves every query is a caller that reports an area of no cells. A
dashboard behind a tab that a person hid is such a caller, and it paid for
every query of every refresh before this change.

This change does not do three things. Each one is work for a later version:

1. The renderer leaves the rows of a dropped panel empty. The section
   "Measurements" below holds a dashboard of eight panels. On a terminal of 24
   rows, a reader of that dashboard loses 2 of the 23 rows of the body to a
   panel that nobody sees. A later version can give
   those rows to the panels that the renderer draws.
2. The key `Tab` still selects a panel that the renderer drops. A person then
   presses a key and the screen does not change. `DashboardView::handle_key`
   holds no area today, so a rule that skips such a panel needs the area of the
   last frame. [ADR 0054](0054-the-program-binds-enter-and-space.md) records
   this limit again.
3. `Dashboard::validate` bounds `x` and `w` against the columns of the grid,
   and it bounds neither `y` nor `h`. The grid states no number of rows, so
   this project has no bound to check against. `rows_of_grid` saturates, which
   means that it holds a sum at the largest number of its type. No such
   dashboard therefore ends the program. That rule holds on the path of the grid
   alone. `parse_duration_ms` of `crates/thunderbox-core/src/time.rs` ends with
   `Ok(n * mult)`, and that multiplication does not saturate.
   `Dashboard::validate` calls that function for the `refresh` field. A
   `refresh` of `20000000000w` therefore ends the program with an overflow of
   an integer before the program opens the terminal. An overflow is a result
   that does not fit in its type. A build for release wraps the number instead,
   and the dashboard then refreshes at a period that nobody asked for.
   [ADR 0061](0061-a-duration-that-does-not-fit-ends-the-program.md) records
   that fault, with its measurement.

## Measurements

This table holds a dashboard of eight panels in one column, on a terminal of
110 cells. The program keeps one row of that terminal for the status row, so
the panels hold one row fewer. The test
`a_column_of_eight_panels_drops_these_panels_on_each_terminal` holds the same
numbers.

| rows of the terminal | panels drawn | queries saved |
|---|---|---|
| 25 and above | 8 | 0 of 8 |
| 24 | 7 | 1 of 8 |
| 23 | 6 | 2 of 8 |
| 22 | 5 | 3 of 8 |
| 21 | 4 | 4 of 8 |
| 20 | 3 | 5 of 8 |
| 19 | 2 | 6 of 8 |
| 18 | 1 | 7 of 8 |
| 17 and below | 0 | 8 of 8 |

The default dashboard holds seven panels and eight queries, because the panel
`network` holds two targets. It saves nothing on a terminal of 10 rows or more.
The renderer drops these panels on a smaller terminal:

| rows of the terminal | panels dropped | queries saved |
|---|---|---|
| 10 and above | 0 of 7 | 0 of 8 |
| 9 | 2 of 7 | 2 of 8 |
| 8 | 5 of 7 | 6 of 8 |
| 7 and below | 7 of 7 | 8 of 8 |

The panels of one row of the dashboard hold the same rows of the grid, so the
renderer drops them together. A terminal of 9 rows drops the two panels of the
first row, which are `cpu` and `cpu now`. A terminal of 8 rows also drops the
three panels of the second row. The test
`the_default_dashboard_drops_a_panel_only_under_a_terminal_of_ten_rows` holds
the counts of the panels. It counts no query.

The `too_small` scene of `scripts/record-demo.py` records a terminal that hides
part of the dashboard. That scene runs a dashboard of three panels on a
terminal of 90 cells by 19 rows. The renderer drops the first panel, so the
runtime saves 1 query of 3. Every other scene of this project records a
terminal that is large enough for every panel, and each one saves no query.
