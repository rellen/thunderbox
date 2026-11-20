# ADR 0052: The runtime owns the collapse, and each snapshot carries it

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/runtime.rs`, `crates/thunderbox-dashboard/src/snapshot.rs`, `crates/thunderbox-dashboard/src/grid.rs`, `crates/thunderbox-dashboard/src/view.rs` |
| Tests | `a_set_of_queries_that_started_before_a_collapse_keeps_the_collapse`, `the_renderer_draws_no_panel_that_a_collapsed_row_panel_hides`, `the_runtime_asks_about_exactly_the_panels_that_the_renderer_draws`, `a_new_time_range_keeps_the_row_panels_that_a_person_collapsed` |
| Related | [ADR 0016](0016-a-program-that-holds-a-dashboard-holds-four-rules.md), [ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md), [ADR 0050](0050-a-row-panel-holds-panels-by-their-place.md), [ADR 0053](0053-a-panel-that-appears-costs-a-set-of-queries.md), [ADR 0054](0054-the-program-binds-enter-and-space.md) |

## Context

A person collapses a row panel to hide the panels that it holds. The renderer
must hide those panels, and the runtime must ask no data source about them.
Both parts must therefore read one state.

## Decision

The runtime owns the collapse, and each snapshot carries it.
`DashboardRuntime::toggle_row` changes the state, and `Snapshot::collapsed`
carries it to the renderer. The renderer and the runtime then hide the same
panels.

The runtime publishes each snapshot through a channel, and it writes each
change of the collapse into that channel at once. A collapse that only hides
panels starts no set of queries. A set that started before the collapse can
therefore end after it. The function `publish` keeps the collapse that the
channel holds, and it never takes the collapse of the set.

## Consequences

A version of the code without that rule of `publish` let such a set expand the
row panel again. The test
`a_set_of_queries_that_started_before_a_collapse_keeps_the_collapse` holds the
rule.

## Designs that the project rejected

**The shape of the marker.** In that shape the view holds the state, and the
program reports it to the runtime on every frame.
[ADR 0016](0016-a-program-that-holds-a-dashboard-holds-four-rules.md) states
that shape for the marker. A program that forgot that report would draw an
expanded panel that no query ever reached. That panel would read `loading…` for
ever, and nothing would name the fault.

The project took the shape of the time range in
[ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md). In that
shape the runtime holds the state, and the program calls the runtime from a
key press. That shape does less harm when a program forgets it. A program that
passes no key to `row_key` cannot collapse a row panel, and every panel still
holds an answer.
