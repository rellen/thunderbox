# ADR 0034: A dashboard reads more than one data source

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-core/src/dashboard.rs`, `crates/thunderbox-source/src/lib.rs`, `crates/thunderbox-dashboard/src/query.rs` |
| Tests | `one_data_source_that_returns_an_error_changes_one_panel_only`, `build_makes_one_data_source_for_each_declaration` |
| Related | [ADR 0003](0003-agree-on-the-answer-and-not-on-the-question.md), [ADR 0005](0005-one-trait-is-the-seam-of-every-data-source.md), [ADR 0015](0015-both-ways-into-another-program-are-in-scope.md) |

## Context

The plan of the project states what to build, and when. That plan once listed
a dashboard that reads more than one data source under `Later`. The project
then found that the code already held that feature.

## Decision

Each target of a panel names its own data source. A dashboard can therefore
read more than one data source, and that design cost no work.

Three parts of the code hold the decision:

- `Dashboard::sources` is a map from a name to a declaration.
- `thunderbox_source::Registry` is a map from that same name to a data source.
- `run_panel` reads `sources.get(&target.source)` for each target. It does not
  read the map once for each panel, and it does not read the map once for each
  dashboard.

## Consequences

The rule of [ADR 0003](0003-agree-on-the-answer-and-not-on-the-question.md)
supplied this result at no cost. That rule states: agree on the form of the
answer, and do not agree on the form of the question. Every data source answers
with values of the type `Frame`, so a panel needs no knowledge of the data
source that answered. Two data sources on one dashboard are then two entries of
one map.

A data source that returns an error changes the panels of that data source
alone. [ADR 0015](0015-both-ways-into-another-program-are-in-scope.md) already
states that rule for one data source, and the rule holds for several.
