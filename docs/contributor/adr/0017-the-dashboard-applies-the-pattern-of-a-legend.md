# ADR 0017: The dashboard applies the pattern of a legend

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-core/src/legend.rs`, `crates/thunderbox-core/src/dashboard.rs`, `crates/thunderbox-core/src/query.rs`, `crates/thunderbox-dashboard/src/query.rs` |
| Tests | `a_data_source_that_attaches_no_label_obeys_the_pattern_of_a_target`, `the_sysinfo_data_source_obeys_a_pattern_with_no_change_of_its_own`, `a_pattern_reads_a_label_that_the_series_holds`, `each_target_of_a_panel_reads_its_own_pattern` |
| Related | [ADR 0003](0003-agree-on-the-answer-and-not-on-the-question.md), [ADR 0012](0012-the-five-duties-of-a-data-source.md) |

## Context

A person states a pattern in the `legend` field of a target. The pattern
builds the name of each series.

Only one data source of three obeyed the pattern before this rule. The other
two ignored the `legend` field. `thunderbox_core::Query` held the pattern then,
so every data source received one, and each author had to remember to apply
it.

## Decision

A data source never reads the pattern of the `legend` field. The dashboard
applies that pattern after the data source answers. The pattern names a
series, and a name is a matter for the side that draws.

The pattern stays on `thunderbox_core::Target`, and `Target::to_query` drops
it. `thunderbox_core::legend` holds the one rule, and `run_panel` is the one
place that applies it. That place sees both a target and the answer of that
target.

## Consequences

The compiler rejects a data source that reads a pattern, and a comment no
longer has to ask for the correct behaviour.

[ADR 0012](0012-the-five-duties-of-a-data-source.md) states what this rule
gives the author of a new data source.
