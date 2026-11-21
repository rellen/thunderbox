# ADR 0078: A duration that saturates hides the mistake

| field | value |
|---|---|
| Status | Rejected |
| Recorded | 2026-11-21 |
| Source | [ADR 0061](0061-a-duration-that-does-not-fit-ends-the-program.md) |
| Code | No code holds a rejected design. |
| Tests | No test reads this record. |
| Related | [ADR 0073](0073-a-duration-that-does-not-fit-returns-an-error.md), [ADR 0025](0025-the-rows-of-the-grid-belong-to-the-dashboard.md) |

## Context

[ADR 0061](0061-a-duration-that-does-not-fit-ends-the-program.md) asked whether
`parse_duration_ms` returns an error for a number that does not fit, or whether
it saturates. An operation saturates when it returns the largest or the
smallest number of its type in place of a result that does not fit.

The grid of a dashboard already saturates. `Dashboard::validate` adds `x` and
`w` with `u16::saturating_add`, as
[ADR 0025](0025-the-rows-of-the-grid-belong-to-the-dashboard.md) states. The
project examined the same rule for a duration. [ADR 0073](0073-a-duration-that-does-not-fit-returns-an-error.md)
records the design that the project took instead.

## Decision

The project does not take this design. `parse_duration_ms` returns an error for
a duration that does not fit.

## Consequences

A person who proposes this design again must first answer the fault that the
next section names.

## Designs that the project rejected

- **`parse_duration_ms` saturates.** The function would return `i64::MAX` for
  each duration that does not fit. That is 9223372036854775807 milliseconds,
  which is about 292 million years. A person who writes
  `refresh = "20000000000w"` by mistake would get a dashboard that never
  refreshes. The program would write no message. An offset of the table
  `[time]` would give the same result: a window that the person did not write,
  and no message. The grid of a dashboard saturates a value that code
  calculates. A duration is a value that a person writes. The person can
  correct the value only when the program names it. No reader built this
  design, so the fault comes from arithmetic and not from a measurement.
