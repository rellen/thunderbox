# ADR 0042: The runtime asks for the choices of a variable one time

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/runtime.rs`, `crates/thunderbox-dashboard/src/query.rs`, `crates/thunderbox-dashboard/src/variable.rs`, `crates/thunderbox-source/src/lib.rs` |
| Tests | `a_variable_takes_its_choices_from_a_data_source`, `a_data_source_that_holds_no_choice_for_a_variable_gives_a_sentence`, `the_runtime_stops_the_task_of_a_variable_when_it_drops`, `take_choices_holds_the_value_that_the_dashboard_states`, `load_choices_returns_the_values_that_a_data_source_holds`, `load_choices_names_a_data_source_that_holds_no_choice_for_a_variable` |
| Related | [ADR 0041](0041-validate-reports-the-fault-of-a-variable.md), [ADR 0043](0043-the-program-binds-each-key-of-a-variable.md), [ADR 0045](0045-the-panels-of-one-set-of-queries-run-at-the-same-time.md) |

## Context

A variable can take its choices from a data source. The choices of a variable
change rarely. A question on each refresh would reach the data source as often
as the panels do.

## Decision

The runtime asks a data source for the choices of a variable one time, when it
starts.

The runtime holds a task of its own for that question, so
`DashboardRuntime::new` returns at once and blocks nothing.
`DashboardRuntime::tick` takes the answer. A caller that drives `tick`
therefore needs no other call.

The first set of queries runs before that answer arrives. A variable that
states a `current` value holds that value from the first moment. A variable
that states none holds no value until the answer arrives, and its panels hold
an error until then. The runtime starts a second set of queries only when the
answer changes a value.

The field `SourceDescriptor::supports_label_values` reports whether a data
source holds choices at all. A variable whose data source holds no choice takes
a sentence that states the reason, and a program shows that sentence.

## Consequences

The runtime never asks again. A program that needs new choices drops the
runtime and makes a new one.

The runtime stops the task of that question when a program drops the runtime.
`load_choices` asks the data source of every variable at the same time, as
[ADR 0045](0045-the-panels-of-one-set-of-queries-run-at-the-same-time.md)
states.
