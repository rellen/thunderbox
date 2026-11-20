# ADR 0048: The list of processes reads the machine through a `System` of its own

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-source-sysinfo/src/process.rs`, `crates/thunderbox-source-sysinfo/src/lib.rs` |
| Tests | No test holds this rule. |
| Related | [ADR 0047](0047-the-process-list-of-sysinfo-keeps-no-past.md) |

## Context

A `System` is the value of the `sysinfo` crate that reads the machine. That
crate measures the share of a process against the time of the processor
between two refreshes. A refresh of the processes therefore reads the time of
the processor again.

On Linux, version 0.38.4 of the crate does so in `update_procs_cpu` of
`src/unix/linux/system.rs`. That function calls `refresh_if_needed` on the
processors, and that call reads the processors again when 200 milliseconds
have passed.

If the signal `cpu` shared that `System`, the signal would then measure a
shorter period. That period would start at the last list of processes, and not
at the last sample of the signal.

## Decision

The list of processes reads the machine through a `System` of its own.

## Consequences

A dashboard that shows no list of processes pays nothing, because
`ProcessSampler` reads no process until a query asks for the list.

No test holds this rule. Each sample of the signal `cpu` reads a machine whose
load changes. No test can therefore tell a short period of measurement from a
long one. The comment on `ProcessSampler` states the rule, and this record
states it a second time.
