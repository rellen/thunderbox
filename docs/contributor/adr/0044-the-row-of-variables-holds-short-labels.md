# ADR 0044: The row of the variables holds short labels

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/variable.rs`, `crates/thunderbox/src/main.rs` |
| Tests | `the_row_names_each_variable_and_marks_the_one_at_the_cursor`, `the_row_of_the_variables_names_each_one_and_lists_the_keys`, `the_status_row_names_a_variable_that_has_no_choice`, `the_status_row_drops_the_sentence_that_a_person_needs_least` |
| Related | [ADR 0041](0041-validate-reports-the-fault-of-a-variable.md), [ADR 0043](0043-the-program-binds-each-key-of-a-variable.md) |

## Context

A program draws the row of the variables in one line, as it draws the row of
the keys. A variable can hold no choice, and a person then needs a sentence
that states the reason.

## Decision

The row of the variables holds short labels, and the status row holds the
sentence.

`Variables::row_text` writes the text `[job: web]  region: eu` for a dashboard
of two variables. The brackets mark the variable at the cursor. Every program
that calls that method therefore writes the same words.

`Variables::error` returns the sentence for a variable that has no choice. The
program writes that sentence in the status row.

## Consequences

The status row already drops the sentence that a person needs least when the
terminal is too narrow. The sentence of a variable therefore follows that rule,
and it needs no rule of its own.
