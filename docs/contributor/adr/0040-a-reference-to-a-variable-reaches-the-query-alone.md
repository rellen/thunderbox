# ADR 0040: A reference to a variable reaches the query alone

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-core/src/variable.rs`, `crates/thunderbox-core/src/dashboard.rs`, `crates/thunderbox-dashboard/src/query.rs` |
| Tests | `apply_puts_the_value_of_a_variable_in_the_place_of_its_name`, `apply_reads_the_text_one_time`, `apply_writes_one_mark_for_two_marks`, `apply_keeps_a_mark_that_starts_no_name`, `is_name_accepts_a_name_that_a_query_can_refer_to`, `is_name_rejects_a_name_that_a_query_cannot_refer_to`, `to_query_puts_the_value_of_a_variable_into_the_text_of_the_query`, `validate_returns_an_error_for_a_variable_query_that_refers_to_a_variable`, `the_value_that_a_variable_holds_reaches_the_data_source` |
| Related | [ADR 0017](0017-the-dashboard-applies-the-pattern-of-a-legend.md), [ADR 0041](0041-validate-reports-the-fault-of-a-variable.md), [ADR 0042](0042-the-runtime-asks-for-the-choices-of-a-variable-once.md), [ADR 0043](0043-the-program-binds-each-key-of-a-variable.md) |

## Context

A dashboard can declare a variable, and a person selects one of its choices. A
person writes `$job` in the `expr` field of a target. thunderbox then puts the
value that the person selected in the place of that reference.

## Decision

A reference to a variable reaches the text of a query, and it reaches nothing
else. `thunderbox_core::variable` holds the rule, and `Target::to_query`
applies it.

That method takes the values as a parameter. A caller therefore cannot build
the query of a target without them. No caller can send the name of a variable
to a data source by mistake.

The `legend` field holds no reference. A pattern names a series, and
`thunderbox_core::legend` holds that rule alone.
[ADR 0017](0017-the-dashboard-applies-the-pattern-of-a-legend.md) states that
rule.

The rule reads the text one time. A value that itself holds a reference
reaches the data source as it is, so the rule cannot loop.

The form of a reference follows three rules:

- The text `$$` writes one `$`.
- Every other `$` stays as it is.
- A name starts with a letter or an underscore, and it then holds letters,
  digits, and underscores.

The project holds no data source of SQL today. Such a data source would
therefore receive the text `$1` without a change.

## Consequences

`Dashboard::validate` rejects a variable whose own query refers to a variable.
That dashboard asks for a rule that this version does not hold.

A chained variable is a variable whose query reads the value of another
variable. A chained variable needs an order between the variables, and a
later version must supply that order.
