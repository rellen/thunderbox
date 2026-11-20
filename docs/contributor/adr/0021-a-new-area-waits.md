# ADR 0021: A new area waits, and the next set of queries carries it

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/runtime.rs` |
| Tests | `a_run_of_new_areas_costs_one_set_of_queries`, `a_dashboard_that_states_no_refresh_interval_still_receives_a_new_area`, `an_area_that_never_stands_still_still_reaches_the_data_source`, `the_wait_of_an_area_ends_inside_the_longest_hold`, `an_area_that_moves_back_to_the_area_in_use_still_arrives`, `an_area_that_moves_in_and_out_of_the_point_limit_in_use_still_arrives`, `a_new_area_waits_after_a_caller_dropped_the_area_that_waited`, `a_new_area_waits_after_an_area_that_cost_no_set_of_queries`, `a_set_of_queries_ends_the_wait_of_an_area`, `an_area_that_changes_no_point_limit_starts_no_wait`, `the_first_area_that_a_caller_reports_reaches_the_data_source_at_once` |
| Related | [ADR 0018](0018-a-refresh-keeps-the-answer-of-each-panel.md), [ADR 0022](0022-an-area-that-changes-no-point-limit-starts-no-set.md), [ADR 0028](0028-the-caller-reports-one-area-as-a-size.md), [ADR 0029](0029-the-runtime-asks-about-every-panel-until-an-area-arrives.md), [ADR 0032](0032-each-rule-of-the-path-of-an-area-holds-a-test.md) |

## Context

`DashboardRuntime::set_panel_area` started a set of queries at once before this
rule. A caller reports the area on every frame. A person who dragged the corner
of a terminal window therefore started and stopped a set on every frame. A data
source on another machine received one request for each frame, and the runtime
stopped every request except the last.

## Decision

A new area waits, and the next set of queries carries it.

The runtime now holds a new area. `tick` gives that area to a new set of
queries when no set of queries runs, and when one of these two conditions
holds:

- the area has stood still for `AREA_QUIET_TIME`, which is 250 milliseconds;
- the area has waited for `AREA_MAX_HOLD`, which is one second.

This record calls that first time the quiet time. This step of `tick` comes
before the check of the refresh interval. A dashboard that states no refresh
interval therefore receives a new area as well.

The runtime records the moment when the wait of an area began, and a later
change does not start that wait again. `area_wait_since` holds that moment.

The width and the height wait together, in one field of the runtime.

The first area that a caller reports reaches the data source at once, when
that area changes a point limit.

## Consequences

**The longest hold.** The second condition serves a caller whose area never
stands still. A panel area that grows and shrinks by one cell is one example,
and a scroll bar that appears and disappears makes one. The quiet time alone
would hold such an area for as long as it moved. The point limit would then
stay wrong for that whole time.

**The start of a wait.** The runtime writes the moment into `area_wait_since`
when an area that costs a set of queries arrives and no wait is open. An area
that changes no point limit makes no point limit wrong, so it opens no wait. A
person can move the bottom edge of a window for longer than `AREA_MAX_HOLD`.
Such a caller would otherwise leave a wait open. The next area that changed a
point limit would then reach a data source at once.

**The end of a wait.** The runtime writes `None` at two steps:

- a set of queries that starts writes it, because that set carries the area
  which waited;
- `end_the_wait_of_a_settled_area` writes it when no area waits and the area
  of the caller has stood still for `AREA_QUIET_TIME`.

**Two steps that end no wait.** Two steps leave no area in the wait, and
neither one ends the wait by itself:

- `tick` takes an area that costs no set of queries;
- `set_panel_area` drops the area that waits when a caller reports the area in
  use again.

A caller that still moves would otherwise start the wait again on its next
report. That rule matters, because a caller can report a size that needs no
set of queries. [ADR 0022](0022-an-area-that-changes-no-point-limit-starts-no-set.md)
states which sizes those are. A caller can report one such size on one frame,
and a size that needs a set on the next frame. Such a caller would otherwise
hold an area on one frame and drop it on the next frame. Each new hold would
start the wait again, so neither the quiet time nor `AREA_MAX_HOLD` could pass.
That area would then never reach a data source.

The runtime held that fault before this rule, for the one size that equals the
area in use. The tests
`an_area_that_moves_back_to_the_area_in_use_still_arrives` and
`an_area_that_moves_in_and_out_of_the_point_limit_in_use_still_arrives` hold
both shapes to the longest hold.

The quiet time ends the wait, and a step that leaves no area in the wait does
not. A caller that reports the area in use again would otherwise leave the
wait open for ever. Every later area would then reach a data source on its
first frame. A person who drags a corner would pay a set of queries for a width
that the drag does not keep. Three tests hold the end of a wait:

- `a_new_area_waits_after_a_caller_dropped_the_area_that_waited` holds the
  quiet time after a caller drops the area that waits;
- `a_new_area_waits_after_an_area_that_cost_no_set_of_queries` holds the quiet
  time after `tick` takes an area that costs no set of queries;
- `a_set_of_queries_ends_the_wait_of_an_area` holds the start of a set of
  queries.

**One field for both numbers.** A person who drags the corner of a window
changes both numbers, and the two numbers rarely stand still on one frame. Two
fields, each with a wait of its own, would therefore cost two sets of queries
for one drag. `area_is_due` also asks that no set of queries runs, so the
second of those two sets would wait for the first to answer. The test
`a_run_of_new_areas_costs_one_set_of_queries` drags a corner across thirty
frames and holds the cost at one set.

**The quiet time and the interval.** The quiet time does not grow with the
refresh interval, because the two answer different questions. The interval
states how fresh a person wants the numbers. The quiet time states how long a
person waits after a change of the size of the window. A dashboard of
`refresh = "10s"` would hold a new area for ten seconds under any rule that
read the interval.

**The first area.** The runtime runs its first set of queries before any caller
draws a frame. That set carries `ASSUMED_WIDTH`, which is a width of 80 cells,
and it asks about every panel. The first report of a caller is not a change of
the size of the window. The first picture must hold the correct
number of points. `ASSUMED_WIDTH` therefore serves one set of queries alone,
which is the set that runs before a caller knows the area.

That first report costs nothing when it changes no point limit. The runtime
compares the point limit of each panel, and it starts a set only when one of
those limits changed. A first area that is 80 cells wide, and that drops no
panel, therefore starts no second set of queries.
