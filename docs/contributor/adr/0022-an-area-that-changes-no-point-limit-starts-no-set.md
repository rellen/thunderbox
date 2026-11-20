# ADR 0022: An area that changes no point limit starts no set of queries

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/runtime.rs` |
| Tests | `an_area_that_changes_no_point_limit_starts_no_set_of_queries`, `a_height_that_only_drops_panels_starts_no_set_of_queries`, `a_panel_that_the_terminal_grows_to_hold_asks_a_data_source_again`, `a_height_that_uncovers_a_panel_starts_a_set_of_queries`, `an_area_that_costs_no_set_of_queries_still_reaches_the_next_comparison`, `a_tick_that_takes_an_area_takes_the_error_of_a_dropped_panel_alone`, `a_tick_that_changes_no_panel_wakes_no_reader`, `a_new_marker_after_a_new_area_sends_one_set_of_queries_that_carries_both`, `an_answer_of_an_older_set_of_queries_never_replaces_a_newer_answer` |
| Related | [ADR 0021](0021-a-new-area-waits.md), [ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md), [ADR 0027](0027-a-dropped-panel-keeps-its-answer.md), [ADR 0032](0032-each-rule-of-the-path-of-an-area-holds-a-test.md), [ADR 0060](0060-a-height-that-uncovers-a-panel-asks-every-panel.md) |

## Context

The width of an area decides the point limit of each panel. The height decides
which panels the renderer drops, and it decides nothing else that a query
carries. An area that changes neither would therefore ask each data source for
exactly what the set of queries before asked.

## Decision

An area that changes no point limit starts no set of queries.

`DashboardRuntime::limits_for` returns the point limit that each panel would
hold on any area, and `point_limits` reads it for the area in use. When the
area that waits is due, `tick` compares the list of that area against the list
of the area in use. `pending_area_costs_a_set` holds that comparison. `tick`
starts a set of queries when the two lists differ, and it takes the area
without a set when they agree.

The comparison reads the panels that the new area draws, and it reads no other
panel. A panel that the new area drops asks a data source for nothing. A point
limit that such a panel loses therefore costs no set of queries.

A panel that the new area draws and that the area before dropped costs a set of
queries. The runtime holds no point limit for such a panel, so it cannot know
whether the answer that the panel holds has the right shape.

`tick` holds the comparison that takes a later area or starts a set for it.
`set_panel_area` takes no later area on that comparison. It reads the
comparison for two other steps alone: the first area that a caller reports,
and the start of a wait, which [ADR 0021](0021-a-new-area-waits.md) states.

The runtime writes an area that costs no set of queries into the field `area`
all the same.

## Consequences

A height that hides one panel of a column leaves every other panel of that
column with the answer that it holds. A dashboard of eight panels in one column
now sends no query for that height. The test
`a_panel_that_the_terminal_grows_to_hold_asks_a_data_source_again` holds the
case of a panel that appears, and
`a_height_that_only_drops_panels_starts_no_set_of_queries` holds the other
one. [ADR 0060](0060-a-height-that-uncovers-a-panel-asks-every-panel.md)
records the cost of the case of a panel that appears.

**The error of a dropped panel.** A set of queries takes the error of each panel
that the renderer drops, and `kept_state` states that rule.
[ADR 0027](0027-a-dropped-panel-keeps-its-answer.md) records it. A height that
only drops panels starts no such set, so `tick` must take that error itself.
`clear_the_error_of_each_panel_that_the_area_drops` does so. It writes into the
channel only when it changes a panel. It takes no error from a panel that the
renderer draws. `Snapshot::error_count` would otherwise count an error that no
later set of queries can clear.

**The place of the comparison.** The wait of an area keeps every rule that
[ADR 0021](0021-a-new-area-waits.md) states, because `tick` holds the
comparison. A comparison inside `set_panel_area` would clear the area that
waits on each frame that needs nothing. The wait of a caller that moves between
the two sizes would then never end.

**The field `area`.** That field carries the point limits of the set of queries
that runs now, and an area that changes no limit carries the same ones. A
runtime that dropped such an area would compare every later area against an
area that no caller reports.

**An error that stays.** A height that changes nothing no longer asks a data
source again, so it no longer clears the error of a panel that returned one. A
dashboard that states a refresh interval clears such an error at that interval.
A dashboard that states none clears it on one of six steps:

- the key `r`;
- a new marker;
- a new time range;
- a new choice of a variable;
- an area that changes a point limit;
- a collapse or an expansion of a row panel that makes a panel appear.

**Each start takes the area.** Each start of a set of queries takes the area
that waits, whatever started that set. `refresh`, which the key `r` calls,
carries the newest area. So do `set_marker`, `set_time_range`,
`set_variables`, `toggle_row`, and `tick`, when each one starts a set. A change
of the marker after a change of the area sends one set of queries, and that set
carries both.

**The generation.** Each task carries the generation of the set of queries that
started it, and it publishes its answer only while that generation still runs.
Each task reads the generation and writes its snapshot inside one call to
`send_if_modified`. The lock of the channel therefore holds the check and the
publish together.

## Measurements

A dashboard of two panels on a terminal of 80 cells by 40 rows sent 2 queries.
A change to 80 cells by 41 rows sent 2 more queries before this rule, and each
one carried the same point limit of 80. It now sends none.

A dashboard of eight panels in one column sent 7 queries for a height that
hides one panel, before this rule. It now sends none.
