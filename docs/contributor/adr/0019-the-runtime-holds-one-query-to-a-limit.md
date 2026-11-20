# ADR 0019: The runtime holds one query to a limit

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/query.rs`, `crates/thunderbox-source/src/lib.rs` |
| Tests | `a_data_source_that_never_answers_gives_its_panel_an_error`, `a_data_source_that_never_answers_stops_no_other_panel`, `a_data_source_that_never_answers_does_not_stop_the_next_tick`, `a_data_source_that_states_no_limit_takes_the_limit_of_the_runtime`, `a_data_source_that_states_a_limit_keeps_that_limit`, `a_handle_of_a_runtime_with_no_timer_gives_every_panel_an_error` |
| Related | [ADR 0012](0012-the-five-duties-of-a-data-source.md), [ADR 0015](0015-both-ways-into-another-program-are-in-scope.md), [ADR 0016](0016-a-program-that-holds-a-dashboard-holds-four-rules.md), [ADR 0018](0018-a-refresh-keeps-the-answer-of-each-panel.md), [ADR 0020](0020-a-task-that-ends-with-no-answer-gives-an-error.md), [ADR 0045](0045-the-panels-of-one-set-of-queries-run-at-the-same-time.md) |

## Context

A data source that never answered stopped every later tick before this rule,
because `Snapshot::refreshing` never returned to `false`.
[ADR 0018](0018-a-refresh-keeps-the-answer-of-each-panel.md) states why `tick`
starts no set for the refresh interval while a set of queries waits.

## Decision

The runtime holds one query to a limit, and it holds no set of queries to one.
`run_panel` wraps each call to `DataSource::query` in `tokio::time::timeout`. A
query that passes the limit gives its own panel an error, and that error names
the data source and the wait. Every other panel keeps its answer.

The limit sits on one query for three reasons:

- A limit on the whole set would discard the answer of every panel that had
  already answered. [ADR 0015](0015-both-ways-into-another-program-are-in-scope.md)
  promises that a slow query changes one panel alone.
- A person reads the name of the data source that did not answer. A set of
  queries that the runtime stopped names none.
- A dashboard that states no refresh interval starts no set of queries for the
  clock. A limit inside `tick` would never reach such a dashboard.

The value of the limit comes from the data source, and never from the refresh
interval. `SourceDescriptor::answer_limit` holds it, and
`thunderbox_dashboard::DEFAULT_ANSWER_LIMIT` of 60 seconds serves a data source
that states none.

## Consequences

Each query now ends, so the task of the set ends as well, and the field
`refreshing` returns to `false`. A program that drives `tick` alone needs no
other step to end a wait.

That choice has a cost. One set of queries takes the longest of the limits of
its panels, because `run_all` runs the panels at the same time. A dashboard of
four panels, where each panel names a data source that never answers, holds
one set of queries for that one limit. `tick` starts no set for the refresh
interval in that time, and it gives no new area to a data source in that time
either. A limit on the whole set would cut almost none of that time. It would
also cut a dashboard whose four panels each answer slowly and correctly. The project therefore keeps the
limit on one query.

The refresh interval and the limit answer different questions. The refresh
interval states how fresh a person wants the numbers, and a dashboard can state
`1ms`. The limit states how long a data source can take.

The runtime needs the timer of tokio. The caller must supply a `Handle` of an
async runtime that holds that timer. `#[tokio::main]` and `Runtime::new` each
build such an async runtime. A caller that builds its own calls `enable_time`
or `enable_all` on the builder. On an async runtime that holds no timer, the
task of each panel panics, and each panel then holds an error.

The timer is not the only need. The async runtime must also run a task on a
thread other than the thread that draws. A runtime of the flavour
`current_thread` holds the timer, and every panel on it still reads `loading…`
for ever. [ADR 0016](0016-a-program-that-holds-a-dashboard-holds-four-rules.md)
states that rule in full, as the first of its four rules.
`#[tokio::main]` and `Runtime::new` each call `Builder::new_multi_thread`, so an
async runtime from either one holds that rule as well. A `#[tokio::main]` that
names `flavor = "current_thread"` calls `Builder::new_current_thread` instead,
and it breaks the rule.

The limit stops a query at an await point alone. A data source that holds a
thread of the async runtime, and that awaits nothing, runs to its end. The
third duty of [ADR 0012](0012-the-five-duties-of-a-data-source.md) covers that
condition.
