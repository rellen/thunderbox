# ADR 0043: The program binds each key of a variable

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/variable.rs`, `crates/thunderbox-dashboard/src/view.rs`, `crates/thunderbox-dashboard/src/runtime.rs`, `crates/thunderbox/src/main.rs`, `crates/thunderbox-dashboard/examples/full_embed.rs` |
| Tests | `the_key_that_follows_takes_the_choice_that_follows`, `the_key_before_takes_the_choice_before`, `the_key_of_a_variable_moves_the_cursor_to_the_variable_that_follows`, `a_key_that_changes_nothing_asks_for_no_selection`, `the_view_uses_no_key_of_a_variable`, `a_key_that_moves_the_cursor_starts_no_set_of_queries`, `a_key_takes_the_choice_that_follows_and_the_data_source_reads_it`, `a_new_value_puts_every_panel_back_to_the_state_that_has_no_answer`, `each_key_of_a_variable_changes_the_selection` |
| Related | [ADR 0015](0015-both-ways-into-another-program-are-in-scope.md), [ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md), [ADR 0042](0042-the-runtime-asks-for-the-choices-of-a-variable-once.md), [ADR 0044](0044-the-row-of-variables-holds-short-labels.md), [ADR 0054](0054-the-program-binds-enter-and-space.md) |

## Context

A dashboard can declare several variables, and a person selects a value for
each one with keys. A program that holds a thunderbox dashboard keeps its own
bindings for every key that thunderbox ignores.
[ADR 0015](0015-both-ways-into-another-program-are-in-scope.md) states that
rule.

## Decision

The program binds every key of a variable. thunderbox consumes none of them.

`thunderbox_dashboard::variable_key` offers a default map from one key to a
new selection. It states the policy:

- `v` moves the cursor to the variable that follows;
- `]` takes the choice that follows;
- `[` takes the choice before.

`DashboardView::handle_key` reports `Consumed::No` for all three, so a program
that wants those keys keeps them. This rule is the rule of the time range in
[ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md), and the two
functions have the same shape.

`Variables` holds the cursor beside the choices. A dashboard that declares
several variables therefore needs no key for each one. The row of the
variables marks the one that a key changes.

`DashboardRuntime::set_variables` starts a new set of queries only when a value
differs from the value before. A key that moves the cursor asks a data source
for nothing.

## Consequences

The runtime returns every panel to the state that has no answer when a value
changes. The answers of the set before describe another question.
