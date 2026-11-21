# ADR 0061: A duration that does not fit ends the program

| field | value |
|---|---|
| Status | Superseded by [ADR 0073](0073-a-duration-that-does-not-fit-returns-an-error.md) |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 13 item 6 |
| Code | `crates/thunderbox-core/src/time.rs`, `crates/thunderbox-core/src/dashboard.rs`, `crates/thunderbox/src/main.rs` |
| Tests | No test holds a duration that does not fit. |
| Related | [ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md), [ADR 0025](0025-the-rows-of-the-grid-belong-to-the-dashboard.md) |

## Context

A dashboard file can end the program with an overflow of an integer. An
overflow is a result that does not fit in its type.

`parse_duration_ms` of `crates/thunderbox-core/src/time.rs` ends with
`Ok(n * mult)`. That multiplication does not saturate, and it returns no
error. An operation that saturates returns the largest value of its type in
place of a result that does not fit. `Dashboard::validate` calls that function
for the `refresh` field.

Twelve lines below that call, a comment states the opposite rule for the grid.
The addition there saturates, because a panel must not end the program. The
rule holds on the path of the grid alone.

`validate` never reads the `[time]` table, so the fields `from` and `to` reach
the same fault through a different path. The parser of a moment calls
`parse_duration_ms` for the forms `now-<duration>` and `now+<duration>`.
`Dashboard::from_toml` runs that parser before it calls `validate`.

`clippy::missing_panics_doc` reads `panic!`, `unwrap` and `expect`, and it
reads no arithmetic. The lint therefore reports no fault for
`parse_duration_ms`.

## Decision

The project has not taken a decision. `parse_duration_ms` multiplies with no
check today.

## Consequences

A build without `--release` panics before the program opens the terminal. At
that point, `crates/thunderbox/src/main.rs` promises a message and a stop. The
message of the panic names no field of the dashboard, so a person cannot find
the fault from it.

A build for release wraps the number instead. The dashboard then refreshes at
a period that nobody asked for, or it never refreshes. The value of the
measurement below wraps to a negative number, and the runtime turns the
refresh off for a period that is not positive.

[ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md) records
the same fault in its list of three things that a change did not do.

## Measurements

A dashboard of two lines sets `refresh = "20000000000w"`. The program writes
`attempt to multiply with overflow`, and it returns the code 101.

## The question that remains

The `refresh` field and the `[time]` table both reach `parse_duration_ms`. One
change to that function therefore repairs both paths, and the program then
keeps the promise of a message and a stop.

The next design answers one question. Does `parse_duration_ms` return an error
for a number that does not fit, or does it saturate as the grid does? The two
answers differ for a person who writes a large number by mistake.
