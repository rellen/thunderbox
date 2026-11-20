# ADR 0050: A row panel holds panels by their place on the grid

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/grid.rs`, `crates/thunderbox-core/src/dashboard.rs` |
| Tests | `a_row_panel_holds_each_panel_below_it_up_to_the_next_row_panel`, `a_dashboard_reads_a_row_panel_and_the_state_that_it_starts_in`, `validate_returns_an_error_for_a_panel_that_crosses_the_line_of_a_row_panel`, `validate_returns_an_error_for_a_row_panel_that_does_not_span_the_grid`, `validate_returns_an_error_for_a_row_panel_that_holds_a_target` |
| Related | [ADR 0051](0051-a-row-panel-takes-one-line.md), [ADR 0052](0052-the-runtime-owns-the-collapse.md), [ADR 0053](0053-a-panel-that-appears-costs-a-set-of-queries.md), [ADR 0054](0054-the-program-binds-enter-and-space.md) |

## Context

A row panel draws one line, and it draws no data. It holds the panels below it
on the grid, and a person collapses it to hide them.

Grafana moves the panels of a row into that row when the row collapses. It
removes them from the row again when the row expands. Its file therefore
changes shape with the state of the screen.

## Decision

A row panel holds the panels below it by their place on the grid. The file
never puts a panel inside a row panel.

thunderbox keeps one flat list of panels. A row panel holds each panel below
its line, up to the line of the next row panel. The field `collapsed` states
the first state of a row panel alone, and the program never writes the file.
The function `owners` in `crates/thunderbox-dashboard/src/grid.rs` holds that
rule.

## Consequences

`Dashboard::validate` rejects a panel that crosses the line of a row panel,
because such a panel would belong to two places.

`Dashboard::validate` also rejects two other faults of a row panel:

- a row panel that does not span the whole width of the grid with one row;
- a row panel that holds a target.

## Designs that the project rejected

**The panels of a row inside the row, as Grafana does.** The file would then
change shape with the state of the screen.
