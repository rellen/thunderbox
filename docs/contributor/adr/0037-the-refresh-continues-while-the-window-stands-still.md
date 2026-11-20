# ADR 0037: The refresh continues while the window stands still

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/runtime.rs`, `crates/thunderbox-source-sysinfo/src/lib.rs` |
| Tests | No test names this rule. |
| Related | [ADR 0018](0018-a-refresh-keeps-the-answer-of-each-panel.md), [ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md), [ADR 0038](0038-a-sysinfo-sample-carries-the-moment-of-the-machine.md) |

## Context

A person can hold the window still with the keys of
[ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md). A window is
the pair of moments that a time range names, and the moment that named them. A
window that stands still names the same two moments on each refresh, so a data
source usually returns the same answer.

## Decision

The refresh continues while a person holds the window still. Three reasons
keep the refresh:

- The store of the sysinfo data source fills only while a query runs. A
  refresh that stopped would leave a hole in that store. That hole would be as
  long as the visit to the past.
- A panel that holds an error asks its data source again on a refresh.
- A data source can change its answer for a window that stands still.
  Prometheus accepts a sample that arrives late, and a rule of that server can
  write a value for a moment that has passed.

The first reason decides the question by itself. A person can read the last
hour for five minutes and then press `0`. That person must not meet a hole of
five minutes at the present moment.

## Consequences

This rule has a cost. A dashboard that refreshes each second asks the data
source for the same window once a second, and it usually learns nothing new.

A person who does not want that load states no `refresh` in the dashboard. The
key `r` then runs the queries when the person presses it.
