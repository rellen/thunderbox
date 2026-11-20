# ADR 0053: A panel that appears costs a set of queries

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/runtime.rs` |
| Tests | `expanding_a_row_panel_asks_about_the_panels_that_it_holds`, `collapsing_a_row_panel_asks_about_nothing`, `a_collapse_that_lets_a_small_panel_draw_asks_about_it`, `a_collapsed_row_panel_asks_about_none_of_its_panels`, `a_panel_that_a_row_panel_hides_loses_its_error` |
| Related | [ADR 0022](0022-an-area-that-changes-no-point-limit-starts-no-set.md), [ADR 0027](0027-a-dropped-panel-keeps-its-answer.md), [ADR 0052](0052-the-runtime-owns-the-collapse.md), [ADR 0060](0060-a-height-that-uncovers-a-panel-asks-every-panel.md) |

## Context

A collapse or an expansion of a row panel changes the panels that the renderer
draws. The point limit of a panel is the largest number of points that each
query of that panel returns. A panel that the renderer does not draw holds no
point limit.

## Decision

A panel that appears costs a set of queries. A panel that a collapse hides
costs none.

`DashboardRuntime::toggle_row` compares the point limit of each panel before
and after the change. The runtime starts a set of queries when a panel draws
now and drew nothing before:

- A row panel that a person expands shows such panels.
- A collapse can show one as well, because each panel that remains grows and
  can reach the smallest size that the renderer draws.

A collapse changes no width, so it changes no other point limit.

## Consequences

A panel that a collapsed row panel hides keeps the answer that it held, as a
panel that the renderer drops does.
[ADR 0027](0027-a-dropped-panel-keeps-its-answer.md) states that rule. A panel
that appears therefore draws an old answer until its set of queries answers.

The runtime clears the error of a hidden panel, because no later set of
queries reaches it.
