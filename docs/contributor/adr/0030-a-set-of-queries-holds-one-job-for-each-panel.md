# ADR 0030: A set of queries holds one job for each panel

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/query.rs`, `crates/thunderbox-dashboard/src/runtime.rs`, `crates/thunderbox-dashboard/src/snapshot.rs` |
| Tests | `a_panel_that_the_renderer_drops_moves_no_other_panel`, `run_all_keeps_the_order_of_the_panels`, `a_panel_that_holds_no_target_returns_no_frame`, `the_end_of_a_task_gives_no_error_to_a_panel_that_the_renderer_drops` |
| Related | [ADR 0020](0020-a-task-that-ends-with-no-answer-gives-an-error.md), [ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md), [ADR 0027](0027-a-dropped-panel-keeps-its-answer.md) |

## Context

The runtime asks no data source about a panel that the renderer drops.
[ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md) states
that rule. A set of queries must still keep the place of such a panel.

`Snapshot::panel` reads a panel at the position that the dashboard gives it. A
list that held no entry for a dropped panel would give panel 1 the answer of
panel 2. The last panel would then hold nothing at all. No test would report
that fault, because `Snapshot::panel` returns a state that waits for an index
past the end of the list.

## Decision

A set of queries holds one job for each panel, and it names the skip.

`run_all` takes a `PanelJob` for each panel, and that value is either
`PanelJob::Ask` or `PanelJob::Skip`. The snapshot then holds one state for each
panel, in the order of the dashboard.

`ClearOnEnd` reports the end of a set of queries that ended with no answer. It
writes the error into the panels that the set asked about, and it leaves a
panel that the renderer drops as it is.

## Consequences

A skipped panel is not a panel of an empty list of targets. `run_panel` gives a
panel of an empty list an answer that holds no frame, and that panel draws
`no data`. A dashboard can declare a panel that holds no target. The dashboard
`ONE_PANEL` of the tests inside `crates/thunderbox-dashboard/src/runtime.rs`
holds such a panel.

No query of a dropped panel ran, so the text of an error from `ClearOnEnd`
would name a fault that the panel never met. `Snapshot::error_count` would also
count a panel that nobody draws. The panel therefore keeps the state that
`DashboardRuntime::start` gave it. That state is the answer that the panel
held, except in two cases. The state is `PanelState::Loading` when the window
changed, and when the panel held an error.
[ADR 0027](0027-a-dropped-panel-keeps-its-answer.md) states that rule.
