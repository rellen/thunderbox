# ADR 0038: A sample of sysinfo carries the moment of the machine

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-core/src/query.rs`, `crates/thunderbox-source-sysinfo/src/lib.rs`, `crates/thunderbox-dashboard/src/snapshot.rs`, `crates/thunderbox-widgets/src/value.rs`, `crates/thunderbox/src/main.rs` |
| Tests | `a_sample_carries_the_present_moment_and_not_the_end_of_the_range`, `new_keeps_the_present_moment_beside_the_range`, `a_snapshot_that_holds_no_answer_names_no_window`, `format_window_names_the_width_and_the_distance_back_in_time`, `the_status_row_names_a_window_that_ended_before_the_present_moment`, `the_status_row_says_nothing_about_a_range_that_follows_the_clock`, `the_status_row_keeps_the_key_of_the_range_beside_the_sentence` |
| Related | [ADR 0005](0005-one-trait-is-the-seam-of-every-data-source.md), [ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md), [ADR 0036](0036-each-label-of-the-time-axis-states-its-distance-from-now.md), [ADR 0037](0037-the-refresh-continues-while-the-window-stands-still.md) |

## Context

A person can move the window one hour into the past. A window is the pair of
moments that a time range names, and the moment that named them.

The sysinfo data source stamped each new sample with `to` before this change.
It then stamped every later sample with a moment one hour old, while a person
studied a window one hour in the past. The store then held its samples in the
wrong order, and a chart drew a line that jumped backwards.

The sysinfo data source holds nothing from before the program started. A
window far in the past therefore leaves every panel of that data source with
`no data`.

## Decision

A sample of the sysinfo data source carries the moment when the machine
produced it. `QueryRequest` holds `now` beside `from` and `to`, and the data
source stamps a new sample with `now`.

The status row of the program names the window while that window ends before
the present moment. It writes this sentence:

```
The panels show 5 minutes that ended 1 hour ago.
```

The row reads `Snapshot::window`, which is the window that the panels draw.
The row and the axis of time therefore name one window. The row names no
window before the first set of queries answers.

The row is the one place that can tell a person why every panel reports
`no data`. `thunderbox_widgets::format_window` writes that sentence. Another
program that holds a dashboard can call that function, and it then writes the
same words. A server for SSH, which the project plans, will call it too.

## Consequences

That report has one limit. No part of thunderbox states the earliest moment
that a data source holds. Only the data source knows that moment, and
`SourceDescriptor` has no field for it.

A person therefore joins two facts: the row names the window, and each panel
reports that it has no data. The full report needs a new field on
`SourceDescriptor`. That change belongs to the seam of every data source,
which [ADR 0005](0005-one-trait-is-the-seam-of-every-data-source.md) states.
