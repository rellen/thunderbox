# ADR 0025: The rows of the grid belong to the dashboard

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/grid.rs`, `crates/thunderbox-core/src/dashboard.rs` |
| Tests | `the_tallest_panel_states_the_rows_of_the_grid`, `a_dashboard_that_spans_past_the_grid_holds_a_number_of_rows`, `validate_returns_an_error_for_a_panel_that_starts_past_the_grid` |
| Related | [ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md), [ADR 0028](0028-the-caller-reports-one-area-as-a-size.md) |

## Context

The renderer and the runtime both need the place of each panel, and
[ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md) gives that
work to the private module `grid` of `thunderbox-dashboard`. That arithmetic
needs a number of rows for the grid.

`place_panel` added `y` and `h` with a plain addition before this change. A
dashboard of `y = 65535` then ended the program, and `Dashboard::validate`
accepts such a dashboard. That fault reached the renderer alone before this
change.

## Decision

The rows of the grid belong to the dashboard, and not to the area.
`rows_of_grid` reads `dashboard.panels` for that number, and it reads nothing
of the area. The area supplies its height alone, and `scale` divides the rows
of the dashboard onto that height.

`rows_of_grid` adds `y` and `h` with `u16::saturating_add`.
`Dashboard::validate` now adds `x` and `w` with `u16::saturating_add` as well.

## Consequences

The runtime takes no number of rows from the area. It reads the height of the
area alone for that arithmetic.

The runtime now reads the same arithmetic as the renderer. The fault of
`y = 65535` would therefore reach `DashboardRuntime::new` and the task of a set
of queries. The change that made this rule therefore removed the fault.

The check of `x` and `w` ended the program on a panel of `x = 65535` before the
change. It now returns an error.
