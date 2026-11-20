# ADR 0062: `SourceDescriptor::downsamples` promises a step that no code takes

| field | value |
|---|---|
| Status | Proposed |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 13 item 7 |
| Code | `crates/thunderbox-source/src/lib.rs`, `Cargo.lock` |
| Tests | No test holds this decision. Every read of the field sits inside a test module. |
| Related | [ADR 0009](0009-the-data-source-reduces-the-points.md) |

## Context

The comment on the field `downsamples` stated that the caller reduces the
number of points when a data source cannot. Every read of the field sits
inside a test module, and no code outside a test reads it. `Cargo.lock` holds
no crate that reduces a series.

[ADR 0009](0009-the-data-source-reduces-the-points.md) and
`docs/contributor/research.md` each promised the same step.

## Decision

The project has not taken a decision. Until it does, three texts state that no
such code exists:

- the comment on the field;
- [ADR 0009](0009-the-data-source-reduces-the-points.md);
- `docs/contributor/research.md`.

Each of the three texts names this record.

The row `LTTB` of section 1.4 of `CLAUDE.md` defines an abbreviation. It
promises no step, so it stays.

## Consequences

The point limit of a panel is the largest number of points that each query of
that panel returns. A panel would draw all 50000 points today if its data
source returned 50000 points for a point limit of 160. No data source of this
project does so today, because each one reduces the points itself.

## The question that remains

The work that remains is a step that reduces the points of a data source that
cannot reduce them itself. Without that step, a panel draws every point that
such a data source returns.

The next design answers one question. Does this project build the reduction,
or does it remove the field from the interface until it builds the reduction?
