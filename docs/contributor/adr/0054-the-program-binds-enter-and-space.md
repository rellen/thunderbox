# ADR 0054: The program binds `Enter` and `Space`

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/row.rs`, `crates/thunderbox-dashboard/src/view.rs`, `crates/thunderbox/src/main.rs`, `crates/thunderbox-dashboard/examples/full_embed.rs` |
| Tests | `enter_and_space_toggle_the_row_panel_that_a_person_selected`, `a_selection_that_holds_no_row_panel_leaves_the_key_to_the_program`, `another_key_toggles_nothing`, `enter_on_a_selected_row_panel_reaches_the_runtime`, `tab_skips_the_panels_that_a_collapsed_row_panel_hides` |
| Related | [ADR 0015](0015-both-ways-into-another-program-are-in-scope.md), [ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md), [ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md), [ADR 0043](0043-the-program-binds-each-key-of-a-variable.md), [ADR 0052](0052-the-runtime-owns-the-collapse.md) |

## Context

A person selects a row panel with the key `Tab`, and a second key then
collapses it or expands it. A program that holds a thunderbox dashboard keeps
its own bindings for every key that thunderbox ignores.
[ADR 0015](0015-both-ways-into-another-program-are-in-scope.md) states that
rule.

## Decision

The program binds `Enter` and `Space`. thunderbox consumes neither.

`thunderbox_dashboard::row_key` maps a key to the row panel that the selection
holds, as `time_key` maps a key to a range in
[ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md). It answers
only while the selection holds a row panel, so the program keeps both keys at
every other time. `DashboardView::handle_key` reports `Consumed::No` for both.

## Consequences

The key `Tab` skips each panel that a collapsed row panel hides.
`DashboardView::handle_key` reads the snapshot for that rule.

The key `Tab` still selects a panel that the renderer drops.
[ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md) holds a
list of three things that a change did not do, and that list records this
limit as its second item.
