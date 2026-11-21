# ADR 0079: A key that clamps an end changes the width

| field | value |
|---|---|
| Status | Rejected |
| Recorded | 2026-11-21 |
| Source | new |
| Code | No code holds a rejected design. |
| Tests | No test reads this record. The test `panned_returns_the_range_as_it_was_when_an_end_would_pass_the_limit` holds the design that the project took. |
| Related | [ADR 0076](0076-a-key-that-would-pass-the-limit-leaves-the-range-as-it-was.md), [ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md) |

## Context

[ADR 0076](0076-a-key-that-would-pass-the-limit-leaves-the-range-as-it-was.md)
records the design that the project took. That design serves a key that would
carry an end of the range past the limit of an `i64`. The project also examined
one other design. This record names the fault of that design.

A clamp holds a value at a limit. In the rejected design, a key moves each end
by the count that it asks for. The key then clamps an end that passed the
limit. The code held this design in part before this change.

## Decision

The project does not take this design. The methods `TimeRange::panned` and
`TimeRange::with_span_ms` return the range as it was when a move would carry an
end past the limit.

## Consequences

A person who proposes this design again must first answer the fault that the
next section names.

## Measurements

The code of the commit before this change already clamped an end. The function
`RelativeTime::moved` holds an end at the limit with `saturating_add`. A reader
ran two calls against that code in a debug build. The section "Measurements" of
[ADR 0076](0076-a-key-that-would-pass-the-limit-leaves-the-range-as-it-was.md)
names the method of the reader.

| call | result |
|---|---|
| `panned(NOW, -1)` on a range from `i64::MIN` to `i64::MIN + 1000` | The start stayed at `i64::MIN`, and the end moved to `i64::MIN + 999`. The width fell from 1000 to 999. |
| `with_span_ms(NOW, 2000)` on the same range | The width was 1000. The caller asked for 2000. |

## Designs that the project rejected

- **A clamp of the end at the limit.** A move keeps the width of the range, and
  that is the rule of
  [ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md). A clamp
  moves one end by less than the count that the key asks for, so the width
  changes. The first row of the table above shows the fault. A person who
  presses the key for a long time then makes the range narrower with a key that
  must keep the width. The second row shows the same fault for the keys `-` and
  `+`: a scale that clamps the earlier end gives another width than the caller
  asked for. The design therefore does not keep a rule of the keys, and it gives
  the person no gain.
