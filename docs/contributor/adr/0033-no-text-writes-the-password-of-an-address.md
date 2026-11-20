# ADR 0033: No text writes the password of an address

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-source-prometheus/src/lib.rs` |
| Tests | `the_debug_text_holds_no_password`, `the_error_of_an_address_that_is_not_correct_holds_no_password`, `without_password_keeps_an_address_that_holds_no_pair`, `without_password_keeps_a_user_name_that_stands_alone`, `without_password_reads_no_pair_from_a_path`, `without_password_reads_the_last_mark_of_the_pair` |
| Related | None. |

## Context

A person can write a user name and a password into the `url` field of a
Prometheus data source. The client of that data source reads that pair, and it
builds a header of the kind `Basic` from it. The address is therefore one
place that holds a password.

Two texts wrote that address in full before this rule:

- The `Debug` implementation of `Prometheus` wrote
  `http://alice:s3cret@127.0.0.1:9090/`. A log message, a panic message, and
  the report of a test can each hold that text.
- `Prometheus::new` wrote the same address in the error of an address that no
  parser accepts. A person reads that error on the screen when the dashboard
  loads.

## Decision

No text of this project writes the password of an address.

The function `without_password` in
`crates/thunderbox-source-prometheus/src/lib.rs` removes the password from an
address. The `Debug` implementation and `Prometheus::new` both call it. The
function keeps the user name. A user name is no secret, and it tells a reader
which server the message names.

## Consequences

A writer who adds another text that names an address must call that function.
The rule binds each of these texts:

- every `Debug` implementation;
- every `Display` implementation;
- every log message;
- every panic message;
- every error.

No test can find such a text by itself, because a test reads the texts that
the project already wrote.
