# ADR 0047: The list of processes of sysinfo keeps no past

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-source-sysinfo/src/process.rs`, `crates/thunderbox-source-sysinfo/src/lib.rs` |
| Tests | `a_window_that_does_not_hold_the_sample_returns_the_columns_and_no_row`, `a_window_in_the_past_returns_no_process` |
| Related | [ADR 0038](0038-a-sysinfo-sample-carries-the-moment-of-the-machine.md), [ADR 0046](0046-a-top-list-is-a-kind-of-panel-of-its-own.md), [ADR 0048](0048-the-process-list-reads-the-machine-through-its-own-system.md) |

## Context

The store of the sysinfo data source holds a series over time. A row of the
list of processes describes one process, and not one moment of a series.

## Decision

The list of processes of the sysinfo data source keeps no past. The data source
keeps the list of the last sample alone.

A query for a window that does not hold the moment of that sample returns the
four columns and no row. The panel then reports `no data`.

## Consequences

A list of the present moment inside a window of the past would be false. The
test `a_window_that_does_not_hold_the_sample_returns_the_columns_and_no_row`
holds that rule.
