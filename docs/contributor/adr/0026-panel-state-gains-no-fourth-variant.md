# ADR 0026: `PanelState` gains no fourth variant

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/snapshot.rs`, `crates/thunderbox-widgets/src/lib.rs` |
| Tests | No test holds this decision. The enum `PanelState` holds it. |
| Related | [ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md), [ADR 0027](0027-a-dropped-panel-keeps-its-answer.md), [ADR 0031](0031-a-snapshot-waits-in-a-third-case.md) |

## Context

The runtime asks a data source about no panel that the renderer drops.
[ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md) states
that rule. A snapshot could state that fact about such a panel in a variant of
its own.

`run_all` builds a whole snapshot, and it knows nothing of a layout. A view
holds no data, so one program can draw one snapshot into two areas of different
size.

## Decision

`PanelState` gains no fourth variant for a panel that the renderer drops. Such
a panel holds `PanelState::Loading` until a data source answers it once. It
then holds the answer of that data source.
`thunderbox_widgets::PanelData` gains no fourth variant either.

## Consequences

`PanelState` keeps three variants: `Loading`, `Ready`, and `Error`.
[ADR 0027](0027-a-dropped-panel-keeps-its-answer.md) states which of those a
panel that the renderer drops holds.

## Designs that the project rejected

A fourth variant would carry a fact about one view inside the data of a
dashboard. A snapshot that said `panel 3 is too small` would be true of one
area and false of the other. The variant would also reach every `match` of
every program that holds a thunderbox dashboard. `error_count` would then count
it as no error, exactly as it counts `Loading` as no error.
