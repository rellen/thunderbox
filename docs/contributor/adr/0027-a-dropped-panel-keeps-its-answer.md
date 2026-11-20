# ADR 0027: A panel that the renderer drops keeps its answer

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/runtime.rs`, `crates/thunderbox-dashboard/src/query.rs`, `crates/thunderbox-dashboard/src/snapshot.rs` |
| Tests | `a_panel_that_the_renderer_drops_keeps_its_answer`, `a_new_window_puts_a_panel_that_the_renderer_drops_back`, `a_panel_that_the_renderer_drops_holds_no_error`, `kept_state_takes_an_error_from_a_panel_that_the_renderer_drops` |
| Related | [ADR 0018](0018-a-refresh-keeps-the-answer-of-each-panel.md), [ADR 0022](0022-an-area-that-changes-no-point-limit-starts-no-set.md), [ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md), [ADR 0026](0026-panel-state-gains-no-fourth-variant.md), [ADR 0030](0030-a-set-of-queries-holds-one-job-for-each-panel.md), [ADR 0032](0032-each-rule-of-the-path-of-an-area-holds-a-test.md) |

## Context

The runtime asks no data source about a panel that the renderer drops.
[ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md) states
that rule. A set of queries must still write a state for such a panel.

A set that wrote `Loading` into such a panel would draw `loading…` there until
the terminal grows again. A person who makes a terminal smaller and then larger
would meet that text in every panel. The answers of the panels that the
renderer still draws would stand beside it.

## Decision

A panel that the renderer drops keeps the answer that it held. The set of
queries that skips the panel writes that answer again, and `PanelJob::Skip`
carries it.

`runtime::kept_state` states the two answers that a panel must not keep, and it
returns `PanelState::Loading` in their place:

1. **An answer of another window.** `DashboardRuntime::set_time_range` reports
   `Kept::Nothing` for a range that differs, and the answers of the set before
   then describe the window before. `DashboardRuntime::set_variables` reports
   `Kept::Nothing` as well when the value of a variable changes. The answers of
   the set before then describe another question.
2. **An error.** A panel that the renderer drops reaches no data source, so no
   later set of queries can clear an error that it holds.

## Consequences

**The first case.** The doc comment of `Snapshot` promises that the fields
`panels`, `range`, `variables` and `updated_at` of one snapshot come from one
set of queries. [ADR 0018](0018-a-refresh-keeps-the-answer-of-each-panel.md)
relies on that promise. An answer of an older window would break that promise,
and it would also mislead a person. The snapshot holds one window for every
panel. A chart of older frames would draw a line that stops before
the right edge of that window. A person reads that shape as a data source that
died.

**The second case.** `Snapshot::error_count` would count that error for as long
as the terminal stays small, and the status row would name a fault that nobody
can repair. A height that only drops panels starts no set of queries, so
[ADR 0022](0022-an-area-that-changes-no-point-limit-starts-no-set.md) gives
`tick` the same step.

**The cost.** This rule breaks the promise of one snapshot for a panel that the
renderer drops. The answer of such a panel comes from the set of queries that
last asked about it. The doc comment of `Snapshot` names that cost, and it
states that one exception.

## Designs that the project rejected

The project examined the other choice, which writes `PanelState::Loading` into
a dropped panel. That choice keeps the promise of one snapshot. It also draws
`loading…` in every panel that the terminal grows to hold again, and it draws
that text until one set of queries answers. A person who makes a terminal
smaller and then larger meets that text on every change of the size.

The project chose the rule that keeps the answer. The window of that answer is
the window of the snapshot, and `kept_state` holds the two cases in which it is
not.
