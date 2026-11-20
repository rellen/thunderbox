# ADR 0018: A refresh keeps the answer of each panel

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/runtime.rs`, `crates/thunderbox-dashboard/src/snapshot.rs`, `crates/thunderbox/src/main.rs` |
| Tests | `a_refresh_keeps_the_answer_that_each_panel_holds`, `a_new_time_range_puts_every_panel_back_to_the_state_that_has_no_answer`, `a_time_range_that_does_not_change_keeps_the_answer_that_each_panel_holds`, `a_new_area_and_a_new_marker_keep_the_answer_that_each_panel_holds`, `tick_starts_nothing_while_a_set_of_queries_waits_for_an_answer`, `a_dashboard_whose_panel_holds_no_target_reports_its_set_of_queries`, `a_snapshot_reports_the_set_of_queries_that_runs_now`, `the_status_row_reports_a_set_of_queries_that_takes_a_long_time` |
| Related | [ADR 0019](0019-the-runtime-holds-one-query-to-a-limit.md), [ADR 0020](0020-a-task-that-ends-with-no-answer-gives-an-error.md), [ADR 0021](0021-a-new-area-waits.md), [ADR 0027](0027-a-dropped-panel-keeps-its-answer.md), [ADR 0029](0029-the-runtime-asks-about-every-panel-until-an-area-arrives.md), [ADR 0037](0037-the-refresh-continues-while-the-window-stands-still.md), [ADR 0042](0042-the-runtime-asks-for-the-choices-of-a-variable-once.md), [ADR 0045](0045-the-panels-of-one-set-of-queries-run-at-the-same-time.md) |

## Context

Before this rule, the dashboard returned every panel to the state that has no
answer on each refresh. The default dashboard refreshes each second, so a reader
met the word `loading…` in every panel many times a minute. The comment beside
the code said that this step separates an old answer from a new one.

That reason does not hold. `run_all` builds one whole `Snapshot`, and the
runtime publishes it in one call. Two panels can therefore never come from two
different sets of queries. A set of queries is the whole group of queries that
one refresh sends.

A window is the pair of moments that a time range names, and the moment that
named them. The range `the last five minutes` names a new window on each
refresh. `thunderbox_core::TimeWindow` holds all three values, and a panel needs
all three. A person who moves the range makes the end of the window and the
present moment two different moments.

## Decision

A refresh keeps the answer that each panel holds. The runtime returns a panel to
the state that has no answer only when that answer describes another window.

`DashboardRuntime::set_time_range` returns every panel to the state that has
no answer when the new range differs from the range before. Those answers
describe another window, and a reader would take them for the new window. The
runtime compares the two expressions of the range, and not the two windows that
they name. A range that says `the last five minutes` therefore keeps its
answers.

The runtime writes `true` into `Snapshot::refreshing` before it starts a set of
queries, and the end of that set writes `false`. The task of a set of queries
writes that `false` even when it publishes no answer.

`DashboardRuntime::tick` starts no set of queries for the refresh interval, and
none for a new area, while a set of queries waits for an answer.

## Consequences

The field `refreshing` is not a count of the queries. A dashboard whose panels
hold no target sends a set that holds no query, and the field reports that set
as it reports any other.

`Snapshot::is_refreshing` reads the field. The program draws a status row from
it, and the row reports a set of queries that waits for one second or longer.
A data source that answers at once therefore changes no text.

The task of a set of queries can end with no answer. The async runtime of the
caller can stop, or the code that runs the set can meet a fault. The runtime
must not wait for a set of queries that no longer runs. That is the reason that
the end of the task writes `false` with no answer as well.
[ADR 0020](0020-a-task-that-ends-with-no-answer-gives-an-error.md) states what
else the end of the task writes then. A data source that panics ends the task
of its own panel alone, and the set still publishes its answer.
[ADR 0045](0045-the-panels-of-one-set-of-queries-run-at-the-same-time.md)
states that rule.

`DashboardRuntime::refresh_age` reports the time that the set which runs now
has waited, and the program reads that time. The row therefore reports a data
source that answers slowly, and it never reports the age of the frames of the
program.

A data source that is slower than the interval now finishes one set of queries.
A tick for the refresh interval no longer stops a set of queries before its
answer arrives. These calls still stop the set that runs:

- a call to `refresh`;
- a call to `set_marker` with a new marker;
- a call to `set_time_range`;
- a call to `set_variables` that changes a value;
- a call to `toggle_row` that makes a panel appear;
- the first call to `set_panel_area`, when that call changes a point limit, as
  [ADR 0021](0021-a-new-area-waits.md) states;
- a tick that takes the choices of a variable and changes a value, as
  [ADR 0042](0042-the-runtime-asks-for-the-choices-of-a-variable-once.md)
  states.

[ADR 0019](0019-the-runtime-holds-one-query-to-a-limit.md) states why every set
of queries now ends.
