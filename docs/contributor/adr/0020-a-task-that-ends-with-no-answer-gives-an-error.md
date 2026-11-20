# ADR 0020: A task that ends with no answer gives an error

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/runtime.rs` |
| Tests | These tests hold this decision: `the_end_of_a_task_reports_the_end_of_its_own_set_of_queries`, `the_end_of_a_task_gives_no_error_to_a_panel_that_the_renderer_drops`, and `a_set_of_queries_that_the_runtime_stopped_leaves_no_error`. The tests `a_task_that_ends_with_no_answer_gives_its_panel_an_error`, `a_handle_of_a_runtime_with_no_timer_gives_every_panel_an_error`, and `a_data_source_that_panics_does_not_stop_the_next_tick` hold the error of the task of one panel. [ADR 0045](0045-the-panels-of-one-set-of-queries-run-at-the-same-time.md) states that later rule. |
| Related | [ADR 0012](0012-the-five-duties-of-a-data-source.md), [ADR 0018](0018-a-refresh-keeps-the-answer-of-each-panel.md), [ADR 0019](0019-the-runtime-holds-one-query-to-a-limit.md), [ADR 0030](0030-a-set-of-queries-holds-one-job-for-each-panel.md), [ADR 0045](0045-the-panels-of-one-set-of-queries-run-at-the-same-time.md) |

## Context

A task of a set of queries can end with no answer. Before
[ADR 0045](0045-the-panels-of-one-set-of-queries-run-at-the-same-time.md), a
task ended that way when a data source panicked, and also on an async runtime
that holds no timer. Each panel now runs in a task of its own, and those two
faults end the task of a panel. The task of the set still ends with no answer
in two cases. The async runtime of the caller can stop, or the code that runs
the set can meet a fault.

The runtime wrote `false` into `Snapshot::refreshing` for such a task before
this rule, and it wrote nothing else. Every panel then kept `loading…` for ever
when the first set of queries ended that way. The status row reported no error
and no wait.

## Decision

A task that ends with no answer gives an error to the panels that it asked
about. `ClearOnEnd` is the value that reports the end of the task of a set.
`ClearOnEnd` now writes the error as well as the value `false`, so a person
reads the fault.

Each set of queries carries a generation number, and the runtime raises that
number for each new set. The runtime raises the generation number before it
stops a task, so a task that the runtime stopped writes nothing.

## Consequences

A set of queries asks about the panels that the renderer draws, and it asks
about no other panel. `ClearOnEnd` therefore writes the error into those panels
alone, and it leaves a panel that the renderer drops as it is.
[ADR 0030](0030-a-set-of-queries-holds-one-job-for-each-panel.md) states that
rule in full.

The limit of [ADR 0019](0019-the-runtime-holds-one-query-to-a-limit.md) stops a
query at an await point alone. A data source that holds a thread of the async
runtime, and that awaits nothing, runs to its end. The third duty of
[ADR 0012](0012-the-five-duties-of-a-data-source.md) covers that condition. The
runtime cannot keep a promise for such a data source, and this record does not
make one.
