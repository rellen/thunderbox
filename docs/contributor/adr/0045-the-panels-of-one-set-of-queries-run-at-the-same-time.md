# ADR 0045: The panels of one set of queries run at the same time

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/query.rs`, `crates/thunderbox-dashboard/src/runtime.rs`, `crates/thunderbox-source/src/lib.rs` |
| Tests | `run_all_runs_the_panels_of_one_set_at_the_same_time`, `run_all_keeps_the_order_of_the_panels`, `load_choices_asks_every_variable_at_the_same_time`, `the_runtime_stops_the_task_of_each_panel_when_it_drops`, `a_refresh_stops_the_task_of_each_panel_of_the_set_before`, `a_data_source_that_panics_gives_its_panel_an_error`, `a_data_source_that_panics_leaves_every_other_panel_with_its_answer`, `a_data_source_that_panics_gives_its_variable_no_choice` |
| Related | [ADR 0012](0012-the-five-duties-of-a-data-source.md), [ADR 0019](0019-the-runtime-holds-one-query-to-a-limit.md), [ADR 0020](0020-a-task-that-ends-with-no-answer-gives-an-error.md), [ADR 0042](0042-the-runtime-asks-for-the-choices-of-a-variable-once.md) |

## Context

A set of queries is the whole group of queries that one refresh sends. The
runtime ran the panels of one set one after another before this change. A
dashboard of four panels, where each data source takes one second, answered in
about four seconds.

A data source that panicked ended the whole set with no answer before this
change. Every panel then held one sentence about the set.

## Decision

The runtime runs the panels of one set of queries at the same time. `run_all`
builds one job for each panel that the renderer draws, and `run_together`
spawns one task for each job.

### No new dependency

The crate takes no new dependency for this work. `tokio::task::JoinSet` comes
from the tokio that the workspace already holds. No crate of this workspace
names `futures` or `futures-util` in its `Cargo.toml` file. `futures-util`
reaches `Cargo.lock` through other libraries alone.

A spawned task holds no borrow. Each job therefore takes a copy of each value
that it reads:

- the registry;
- the time range;
- the present moment;
- the targets and the point limit of its own panel;
- the value of each variable.

`Registry` holds each data source in an `Arc`, which is a pointer that several
owners share. A copy of the registry therefore copies no data source.

### Order of the answers

The answers keep the order of the panels. Each task carries the position of
its own job, and `run_together` writes the answer of that task at that
position. `Snapshot::panel` reads a panel at the position that the dashboard
gives it.

### Stop of a set

The drop of a `JoinSet` stops every task that the set holds. A future is the
value that an `async` function returns, and a task runs it. The task of a set
of queries holds that `JoinSet` inside the future of `run_all`. The stop of
that one task therefore reaches the task of each panel.

`DashboardRuntime` stops the task of a set when it starts a new set, and when a
program drops the runtime. A refresh starts a new set. Two tests hold that
rule, and each one counts the queries that run now. The two tests are
`the_runtime_stops_the_task_of_each_panel_when_it_drops` and
`a_refresh_stops_the_task_of_each_panel_of_the_set_before`.

### A panic

A data source that panics ends the task of its own panel. That panel takes an
error of its own, and every other panel keeps its answer. The limit of one
query in [ADR 0019](0019-the-runtime-holds-one-query-to-a-limit.md) already
followed this rule, and a panic now follows it as well.

### Choices of a variable

`load_choices` follows the same rule for the variables. The runtime asks the
data source of every variable at the same time, and it returns the answers in
the order of the variables.

## Consequences

A dashboard that reads a server on another machine gains the most from this
change. The wait of such a panel is the wait of a network.

A dashboard that reads this machine gains nothing. The sysinfo data source
takes a lock over its whole answer, so the panels of such a dashboard cannot
run at the same time. That dashboard pays the cost of one task for each panel.
That cost is a fraction of a millisecond.

The stop reaches a task at an await point alone. An await point is a place in
the code where a task can pause and give its thread to another task. The async
runtime is the part of tokio that runs each task on a thread. A data source
that holds a thread of the async runtime, and that awaits nothing, runs to its
end. [ADR 0012](0012-the-five-duties-of-a-data-source.md) states that
condition as the third duty of a data source.

This change raises the cost of a data source that does not keep that duty. One
panel of such a data source ran at a time before this change, and every panel
of the set runs now. The section "Measurements" gives the numbers. The sysinfo
data source reads this machine, and the testdata data source makes its own
data. Each one answers in about one millisecond, so a person waits for no such
query.

One data source can now receive several queries at once. The trait already
asks for `Send` and for `Sync`, and it takes `&self`. A data source therefore
already permitted that call. The Prometheus data source holds one client of
the crate `prometheus-http-query`, and that client holds one `reqwest::Client`.
A `reqwest::Client` sends several requests at once over one pool of
connections. The sysinfo data source and the testdata data source hold no
connection.

## Measurements

- A dashboard of four panels, where each data source takes one second,
  answered in about four seconds before this change. It answers in about one
  second now.
- A test drove six panels on a data source that holds a thread and awaits
  nothing. Three queries ended after the stop before this change, and six
  queries ended after the stop with this change.

## Designs that the project rejected

- **A `Vec` of `JoinHandle`.** The drop of such a value would stop the task of
  no panel. The queries of the set before would then reach a data source beside
  the queries of the new set.
- **A function that returns the answers in the order of their arrival.** That
  function would put the answer of one panel in the place of another.
