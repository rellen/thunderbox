# ADR 0032: Each rule of the path of an area holds a test

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/runtime.rs`, `crates/thunderbox-dashboard/src/query.rs` |
| Tests | `a_panel_that_the_renderer_drops_keeps_its_answer`, `a_new_window_puts_a_panel_that_the_renderer_drops_back`, `kept_state_takes_an_error_from_a_panel_that_the_renderer_drops`, `an_area_that_moves_in_and_out_of_the_point_limit_in_use_still_arrives`, `an_area_that_moves_back_to_the_area_in_use_still_arrives`, `an_area_that_costs_nothing_does_not_end_the_wait_of_one_that_costs_a_set`, `taking_an_area_leaves_the_wait_of_that_area_open`, `a_new_area_waits_after_a_caller_dropped_the_area_that_waited`, `a_new_area_waits_after_an_area_that_cost_no_set_of_queries`, `a_panel_that_the_renderer_drops_holds_no_error`, `a_tick_that_takes_an_area_takes_the_error_of_a_dropped_panel_alone` |
| Related | [ADR 0021](0021-a-new-area-waits.md), [ADR 0022](0022-an-area-that-changes-no-point-limit-starts-no-set.md), [ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md), [ADR 0027](0027-a-dropped-panel-keeps-its-answer.md), [ADR 0030](0030-a-set-of-queries-holds-one-job-for-each-panel.md) |

## Context

A change to the path from `DashboardRuntime::set_panel_area` to `run_all` broke
a rule of an earlier change, and every test still passed. Three consecutive
pull requests altered that path. Each one broke a rule that an earlier change
had written, and no test reported the fault. A reader found each fault by a
measurement of the program that runs. No reader found one in the code alone.

| the rule that broke | the change that broke it |
|---|---|
| a panel that the renderer drops keeps the answer that it held | the first design of the skip wrote `PanelState::Loading` in its place |
| a panel that the renderer drops holds no answer of another window, and no error | the skip kept every state that a panel held |
| `AREA_MAX_HOLD` ends the wait of an area | the comparison of the point limits dropped the area that waits |
| `AREA_MAX_HOLD` ends the wait of an area | a tick that took an area which cost no set of queries ended the wait of an area which cost one |
| a new area waits for `AREA_QUIET_TIME` | a caller that dropped the area which waits left the wait open |
| a panel that the renderer drops holds no error | a height that only drops panels started no set of queries |

[ADR 0027](0027-a-dropped-panel-keeps-its-answer.md),
[ADR 0021](0021-a-new-area-waits.md), and
[ADR 0022](0022-an-area-that-changes-no-point-limit-starts-no-set.md) state
those rules.

## Decision

Each rule of the path of an area holds a test. A writer who alters this path
must know two facts.

**The rules of this path live in more than one step.** A rule of `panel_jobs`
also binds `tick`. A rule of `kept_state` also binds
`clear_the_error_of_each_panel_that_the_area_drops`. A change that moves work
from one step to another therefore carries every rule of the step that it left.

**No reader finds these faults in the code alone.** The writer removes each rule
from the code, runs every test, and reads which tests fail. A rule that no
test holds is a rule that the next change will break. Section 4.3 of
[`CLAUDE.md`](../../../CLAUDE.md) states that step for every rule of the
project.

## Consequences

Each of the six rows above now holds at least two tests. The field Tests of
this record names the tests whose names state those rules.

This project has met this fault on this path more often than on any other path.
