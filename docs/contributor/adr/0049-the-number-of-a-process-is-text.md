# ADR 0049: The number of a process is text

| field | value |
|---|---|
| Status | Superseded by [ADR 0083](0083-a-column-can-hold-identifiers.md) |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-source-sysinfo/src/process.rs` |
| Tests | `a_sample_lists_this_process`, `the_rows_come_in_the_order_of_the_number_of_each_process` |
| Related | [ADR 0046](0046-a-top-list-is-a-kind-of-panel-of-its-own.md) |

## Context

A panel shortens a large number. A column of numbers would therefore write
`1.2K` for the process 1234.

A top panel that names no `sort` column puts its rows in order by its first
column of numbers. The number of a process would then give the order, and that
order means nothing to a reader.

## Decision

The number of a process is text. The sysinfo data source writes the column
`pid` as a column of text.

## Consequences

A top panel that names no `sort` column puts the list of processes in order by
the column `cpu`. That column is the first column of numbers.

The two tests that the field `Tests` names read the column `pid` through the
helper `pids`. That helper fails for a column that does not hold text.
