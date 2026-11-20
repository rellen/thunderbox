# ADR 0051: A row panel takes one line, and a collapse gives its rows to the other panels

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/grid.rs` |
| Tests | `a_dashboard_with_no_row_panel_keeps_every_place`, `each_row_panel_takes_one_line_at_every_height`, `a_collapsed_row_panel_hides_the_panels_that_it_holds`, `the_panels_below_a_collapsed_row_panel_move_up_and_every_panel_grows`, `every_place_stays_inside_the_area_and_no_two_places_overlap` |
| Related | [ADR 0025](0025-the-rows-of-the-grid-belong-to-the-dashboard.md), [ADR 0050](0050-a-row-panel-holds-panels-by-their-place.md), [ADR 0053](0053-a-panel-that-appears-costs-a-set-of-queries.md) |

## Context

A row panel holds the panels below it on the grid, and a person collapses it to
hide them. A collapse frees the rows of the grid that the hidden panels held.

## Decision

A row panel takes one line, and a collapse gives its rows to the other panels.

`grid::places` gives each row panel one line of the terminal. It scales the
other rows of the grid onto the lines that remain. The rows that a collapsed
row panel holds leave that scale. The panels below therefore move higher on the
screen, and every panel that remains grows.

A person chose this behaviour, and rejected two others. In the first, the
panels below keep their size. In the second, the space stays empty.

## Consequences

A dashboard that holds no row panel keeps every place that it held before the
project added row panels. The test
`a_dashboard_with_no_row_panel_keeps_every_place` compares the two layouts at
every height from 0 rows to 70 rows.
