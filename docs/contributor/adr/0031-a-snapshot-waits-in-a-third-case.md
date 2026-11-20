# ADR 0031: `Snapshot::is_loading` gains a third case

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/snapshot.rs`, `crates/thunderbox-dashboard/src/view.rs`, `crates/thunderbox/src/main.rs` |
| Tests | `a_new_window_puts_a_panel_that_the_renderer_drops_back`, `a_panel_that_the_renderer_drops_keeps_its_answer`, `the_count_of_the_dropped_panels_holds_the_panels_that_the_renderer_leaves_empty`, `the_count_of_the_dropped_panels_holds_the_panels_that_are_under_three_cells`, `the_status_row_reports_the_panels_that_the_terminal_is_too_small_for` |
| Related | [ADR 0016](0016-a-program-that-holds-a-dashboard-holds-four-rules.md), [ADR 0018](0018-a-refresh-keeps-the-answer-of-each-panel.md), [ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md), [ADR 0027](0027-a-dropped-panel-keeps-its-answer.md) |

## Context

`Snapshot::is_loading` returns `true` while every panel holds
`PanelState::Loading`. The runtime asks no data source about a panel that the
renderer drops. No answer can therefore arrive for such a panel while the
renderer drops it.

A person who read an empty screen learned nothing before this change.

## Decision

`Snapshot::is_loading` gains a third case, and that case has no end of its own.
The doc comment of the method names three cases, and the third is the whole
time in which the renderer drops every panel. When every panel holds
`PanelState::Loading`, the method returns `true` for as long as the renderer
drops every panel.

A program that draws a message about a small terminal reads
`DashboardView::dropped_panels`. It reads that method whatever
`Snapshot::is_loading` returns.

## Consequences

**Where the third case reaches.** The third case reaches a terminal that was
too small before the first answers arrived. It also reaches a terminal that is
too small when the runtime returns every panel to the state that has no
answer. `DashboardRuntime::set_time_range` does so for a new time range, and
`DashboardRuntime::set_variables` does so for a new value of a variable.
`kept_state` of `crates/thunderbox-dashboard/src/runtime.rs` then returns
`PanelState::Loading` for each panel that the renderer drops, whatever state
that panel held. The test
`a_new_window_puts_a_panel_that_the_renderer_drops_back` holds that rule.

**Where it does not reach.** The third case does not reach a terminal that
shrinks after the panels answered. The runtime keeps the answer of a panel that
the renderer drops, so each panel stays at `PanelState::Ready` and the method
returns `false`. The test `a_panel_that_the_renderer_drops_keeps_its_answer`
holds that rule. A panel that held an error is the one exception.
[ADR 0027](0027-a-dropped-panel-keeps-its-answer.md) states why such a panel
loses its error.

**The program.** `Snapshot::is_loading` alone therefore cannot tell a program
that the terminal is too small. `crates/thunderbox/src/main.rs` reads
`DashboardView::dropped_panels`, and the doc comment of `Snapshot::is_loading`
states this rule. Such a program must hold four other rules as well.
[ADR 0016](0016-a-program-that-holds-a-dashboard-holds-four-rules.md) states
them.

**The count.** `DashboardView::dropped_panels` returns the number of panels
that the renderer drops in an area. It reads the rule that
`DashboardView::render` reads, so the count describes the screen beside it. The
status row of the program that runs alone now writes one sentence from that
count. That sentence reads `The terminal is too small for 3 panels.`

Two tests hold that count against the renderer. The test
`the_count_of_the_dropped_panels_holds_the_panels_that_the_renderer_leaves_empty`
draws a dashboard into a buffer at many sizes of a terminal. It then counts the
panels that hold no cell, and that number must equal the count. A rule that
reaches `DashboardView::render` alone therefore fails this test. The test
`the_count_of_the_dropped_panels_holds_the_panels_that_are_under_three_cells`
states the smallest size a second time, and it reads an area that does not
start at the first cell.
