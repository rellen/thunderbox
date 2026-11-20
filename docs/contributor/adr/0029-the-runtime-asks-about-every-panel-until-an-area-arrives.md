# ADR 0029: The runtime asks about every panel until an area arrives

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/runtime.rs` |
| Tests | `a_runtime_that_receives_no_area_asks_about_every_panel` |
| Related | [ADR 0021](0021-a-new-area-waits.md), [ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md), [ADR 0028](0028-the-caller-reports-one-area-as-a-size.md), [ADR 0052](0052-the-runtime-owns-the-collapse.md) |

## Context

The runtime asks about no panel that the renderer drops, and the height of the
area decides which panels the renderer drops.
[ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md) states
that rule. The runtime runs its first set of queries before a caller reports an
area, as [ADR 0021](0021-a-new-area-waits.md) states.

## Decision

The runtime asks about every panel until a caller reports an area. It holds
`None` for the area before that report, and it holds no assumed height. The
runtime still asks about no row panel, and about no panel that a collapsed row
panel hides. [ADR 0052](0052-the-runtime-owns-the-collapse.md) states that
rule.

## Consequences

The runtime also asks about every panel for a caller that never reports an
area. Many tests of `crates/thunderbox-dashboard/tests/` are such a caller. A
program that holds a dashboard behind a screen that it does not draw is another
one.

The test `a_runtime_that_receives_no_area_asks_about_every_panel` holds that
rule.

## Designs that the project rejected

A height that this project guessed could be too small. The first picture of a
caller would then hold no panel.
