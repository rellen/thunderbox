# ADR 0041: `Dashboard::validate` reports the fault of a variable

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-core/src/dashboard.rs`, `crates/thunderbox-dashboard/src/query.rs` |
| Tests | `validate_returns_an_error_for_a_variable_name_that_no_query_can_refer_to`, `validate_returns_an_error_for_two_variables_that_carry_one_name`, `validate_returns_an_error_for_a_variable_that_states_no_choice`, `validate_returns_an_error_for_a_variable_that_states_its_choices_two_times`, `validate_returns_an_error_for_a_current_value_that_is_not_an_option`, `validate_returns_an_error_for_a_variable_that_names_an_undeclared_source`, `validate_returns_an_error_for_a_variable_query_that_refers_to_a_variable`, `validate_returns_an_error_for_a_panel_that_refers_to_an_undeclared_variable`, `a_target_that_names_a_variable_which_holds_no_value_gives_the_panel_an_error`, `a_panel_names_the_variable_that_holds_no_value` |
| Related | [ADR 0016](0016-a-program-that-holds-a-dashboard-holds-four-rules.md), [ADR 0040](0040-a-reference-to-a-variable-reaches-the-query-alone.md), [ADR 0042](0042-the-runtime-asks-for-the-choices-of-a-variable-once.md) |

## Context

A person can write a fault into a variable, or into a reference to a variable.
A person who writes `$jbo` for `$job` is an example. Without a check, the data
source would receive a query that it cannot parse. That person would then read
the error of the data source, and that error names no variable.

## Decision

`Dashboard::validate` reports the fault of a variable, and it reports it before
the program draws. The method rejects each of these:

- a name that no query can refer to;
- two variables that carry one name;
- a variable that states no choice;
- a variable that states its choices in two places;
- a `current` value that is not one of the options;
- a data source that the dashboard does not declare;
- a query of a variable that refers to a variable;
- a target that refers to a variable which the dashboard does not declare.

The check of a data source applies only to a dashboard that declares at least
one data source.
[ADR 0016](0016-a-program-that-holds-a-dashboard-holds-four-rules.md) states
that condition.

The error of the load is the error that the program writes when it loads the
dashboard. That error and the error of a panel answer two different questions:

- The error of the load says that the dashboard is wrong.
- The error of a panel says that the answer did not arrive.

## Consequences

A variable that holds no value gives its panels an error that names the
variable. Only a variable that takes its choices from a data source reaches
that state, in two conditions:

- The dashboard states no `current` value, and the data source has returned no
  choice yet. The first set of queries meets this condition, and the condition
  stays when the data source returns an error.
- The data source returns an empty list of choices.

Without that error, the panel would ask the data source about the name itself,
and the answer of that question means nothing.
