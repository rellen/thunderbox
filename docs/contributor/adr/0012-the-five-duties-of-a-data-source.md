# ADR 0012: The five duties of a data source

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 10 |
| Code | `crates/thunderbox-source/src/lib.rs`, `crates/thunderbox-core/src/frame.rs`, `crates/thunderbox-core/src/legend.rs`, `crates/thunderbox-source-sysinfo/src/sample.rs`, `crates/thunderbox-source-prometheus/src/lib.rs` |
| Tests | `time_series_returns_an_error_for_columns_of_different_length`, `display_name_returns_the_name_from_the_configuration_first`, `a_column_of_a_core_shows_the_short_name_that_it_showed_before`, `the_cores_come_in_the_order_of_their_position`, `the_descriptor_states_a_limit_that_is_longer_than_the_timeout`, `frames_names_a_series_after_the_query_when_it_has_no_metric_name`, `a_data_source_that_attaches_no_label_obeys_the_pattern_of_a_target` |
| Related | [ADR 0003](0003-agree-on-the-answer-and-not-on-the-question.md), [ADR 0005](0005-one-trait-is-the-seam-of-every-data-source.md), [ADR 0013](0013-one-connection-carries-both-ways-over-ssh.md), [ADR 0017](0017-the-dashboard-applies-the-pattern-of-a-legend.md), [ADR 0019](0019-the-runtime-holds-one-query-to-a-limit.md) |

## Context

Every data source returns the form of answer that
[ADR 0003](0003-agree-on-the-answer-and-not-on-the-question.md) states. A panel
reads that answer without knowledge of its origin. A panel can therefore draw
an answer correctly only when every data source builds it by the same rules.

`thunderbox_core::Reduce` reads a position in the column, and not a moment.
`Reduce::Last` takes the last value. The three panels that show one number can
therefore read an old value from an answer in another order.

`Frame::time_series` returns an error when a number column holds a different
number of rows from the time column. The chart then draws no line for that
frame. The table draws the same frame, and it fills the short column with the
symbol of a value that is not there.
[ADR 0081](0081-the-runtime-returns-an-error-for-a-frame-of-two-lengths.md)
states how the runtime now returns an error for such a frame, before a panel
reads it.

An await point is a place where an `async` function waits for another task. A
caller stops a query at an await point. A data source that holds a thread of
the async runtime, and that awaits nothing, therefore runs to its end whatever
the caller does.

`Field::display_name` writes the labels of a column beside the name of that
column. A column named `0` that carries the label `core` therefore draws as
`0{core="0"}`.

## Decision

A data source has five duties, and it applies no pattern:

1. It gives each column a name. That name is different from the name of every
   other column of the answer.
2. It attaches one label to the column for each property that separates one
   series from another. The host and the job are two examples.
3. It ends every query in a limited time. It returns an error when it cannot
   answer. A data source that reads another machine states its longest time in
   `SourceDescriptor::answer_limit`.
4. It returns the moments of the time column in order, from the oldest to the
   newest.
5. It gives every column of one `Frame` the same number of rows.

The fourth duty holds because of the rule of `Reduce`, and the fifth duty
holds because of the rule of `Frame::time_series`. The context above states
both rules.

The third duty belongs to the data source, and not to the caller, because a
caller stops a query only at an await point.

The second duty asks one more thing of a data source whose column names
already separate one series from another. Such a data source states the short
name in `FieldConfig::display_name`, and a target that holds no pattern then
draws that short name.

## Consequences

**The third duty.** The Prometheus data source keeps this duty already. It
builds its client with `reqwest::Client::builder().timeout(settings.timeout)`,
and it sends the same limit to the server.

The sysinfo data source reads this machine, and the testdata data source makes
its data itself. Neither awaits anything, so neither states a limit, and the
limit of the runtime cannot stop either of them. The limit of the runtime does
nothing for a data source of that kind.

The runtime holds each query to a limit as well, and
[ADR 0019](0019-the-runtime-holds-one-query-to-a-limit.md) states that limit.
The two limits do not compete. A data source states a limit that is a little
longer than its own timeout. The error of that data source therefore reaches
the panel, and that error names the server and the fault. The limit of the
runtime names a wait alone, and it serves a data source that states none.

**The second duty.** The sysinfo data source states a short name, because it
names each column after the position of the core. The Prometheus data source
states none. Every column of one metric carries the same name there, and the
labels alone separate those columns.

The sysinfo data source attaches the label `core` to the signal `cpu.cores`,
and it returns the cores in the order of their position. That signal is the one
signal of that data source which returns more than one series. The store names
each series in the form `cpu.core.0`, and it orders its names by the text. A
machine of ten cores or more therefore gave `cpu.core.0, cpu.core.1,
cpu.core.10, cpu.core.11, cpu.core.2` before this rule, and a panel drew that
order. `Signal::series_names` now reads the number after the last dot and
orders by it, so a person reads `core 0, core 1, core 2`. That change moves the
rows of a bar gauge and of a table.
`docs/user/reference/data-sources/sysinfo.md` records it beside the signal.

**The first duty.** A data source cannot always keep it. The Prometheus data
source names a column after the text of the query when the answer holds no
name of a metric. Every column of that answer then carries the same name. The
labels of the second duty separate those columns, and a person writes a
pattern that reads a label.

**The pattern.** A person states a pattern in the `legend` field of a target.
`thunderbox-dashboard` applies that pattern after the data source answers, and
`thunderbox_core::legend` holds the rule. A `Query` carries no pattern, so a
data source cannot read one and cannot apply one.
[ADR 0017](0017-the-dashboard-applies-the-pattern-of-a-legend.md) records that
decision.

An author of a new data source therefore gets the correct behaviour without a
line of code for it. The reserved word `{{__name__}}` reads the name of the
column, so a data source that attaches no label still obeys a pattern.

This choice also serves the two processes that
[ADR 0013](0013-one-connection-carries-both-ways-over-ssh.md) plans. A pattern
belongs to the side that draws. It will never cross the boundary between the
two processes, so a data source on another machine will need no change.
