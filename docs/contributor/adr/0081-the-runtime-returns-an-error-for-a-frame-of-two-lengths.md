# ADR 0081: The runtime returns an error for a frame of two lengths

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-22 |
| Source | `docs/contributor/open-findings.md` item 3.3 |
| Code | `crates/thunderbox-dashboard/src/query.rs`, `crates/thunderbox-source/src/lib.rs`, `crates/thunderbox-widgets/src/timeseries.rs` |
| Tests | `a_number_column_shorter_than_the_time_column_gives_the_panel_an_error`, `a_column_of_text_or_of_true_and_false_values_of_another_length_gives_an_error`, `an_answer_whose_second_frame_holds_two_lengths_gives_the_panel_an_error`, `a_frame_of_one_length_a_frame_of_zero_rows_and_a_frame_with_no_column_pass`, `a_frame_of_two_lengths_gives_its_panel_an_error_that_the_count_reads` |
| Related | [ADR 0012](0012-the-five-duties-of-a-data-source.md), [ADR 0019](0019-the-runtime-holds-one-query-to-a-limit.md) |

## Context

A frame is a table of columns. A frame of two lengths holds columns with
different numbers of rows. [ADR 0012](0012-the-five-duties-of-a-data-source.md)
gives a data source five duties, and the fifth duty forbids a frame of two
lengths. No code checked that duty. `run_panel` gave each answer to the panel
as it was.

Each panel then read a frame of two lengths in its own way:

- The time series panel calls `Frame::time_series`, and that method returns an
  error for such a frame. `collect` and `last_value` drop the error, so the
  chart draws `no data`.
- The stat panel, the gauge panel and the bar gauge panel read the column of
  numbers alone. Each one reduces that column to one number, and it draws that
  number.
- The table panel takes the number of rows from the longest column. It draws
  `—` in each cell of the short column.

No panel held an error, so `Snapshot::error_count` counted nothing, and the
status row reported nothing. A person read three answers on one dashboard, and
no message named the data source.

## Decision

`run_panel` checks each frame of each answer before it keeps the answer. The
check reads every column of the frame: the time column, each column of numbers,
each column of text, and each column of true and false values. A frame of two
lengths gives the panel `PanelState::Error`. An answer of several frames gives
the error when one frame holds two lengths.

A frame of one length passes. A frame of zero rows passes, and so does a frame
with no column.

The error names the data source of the target, the length of the shortest
column, and the length of the longest column. It also states the rule. A data
source `bad` that answers a time column of 4 rows and a column of numbers of 3
rows gives this error:

```
The data source "bad" answered a frame with a column of 3 rows and a column of 4 rows. Each column of one frame must hold the same number of rows.
```

## Consequences

Every panel draws the error, and `Snapshot::error_count` counts it. The error
names the data source, so a person knows which data source to correct.

An answer that keeps the fifth duty changes no panel. The three data sources of
this repository keep it, so no picture of `docs/media` changes.

The check belongs to the runtime, and not to a panel. A panel of
`thunderbox-widgets` receives frames with no name of a data source, so its
message could name no data source. The status row also counts the state of the
runtime, and it reads no text that a panel draws.

A program that gives frames to a panel of `thunderbox-widgets` directly meets
no such check. Each panel then reads a frame of two lengths as the context above
states. The crate comment of `thunderbox-source` states that limit. The blocks
of `collect` and `last_value` that drop the error of `Frame::time_series` stay,
and a comment beside each one names the frames that it skips.

The check reads the length of each column once. It reads no value.

## Measurements

A test drew each panel at 40 cells by 8 cells, against two answers, before this
record. `Snapshot::error_count` returned 0 for each answer.

| answer | time series | stat, gauge, bar gauge | table |
|---|---|---|---|
| a time column of 4 rows, and `usage` of 3 rows: 10, 50, 20 | `no data` | `20` | 4 rows, and `—` under `usage` in the last row |
| a time column of 3 rows, and `usage` of 4 rows: 10, 50, 20, 80 | `no data` | `80` | 4 rows, and `—` under `time` in the last row |

The command `cargo test --no-fail-fast` ran once for each change below. Each
change removed one rule from the code.

| the change | the tests that failed |
|---|---|
| `run_panel` does not call the check. | 4 |
| The check reads the columns of numbers alone. | 4 |
| The check reads the first frame of an answer alone. | 1 |
| The message names no length. | 3 |
| The check gives a frame of zero rows an error. | 2 |

One of the two tests of the last change belongs to the program:
`the_default_dashboard_draws_the_names_that_it_drew_before`. A data source of
this repository therefore answers a frame of zero rows to the default
dashboard. The rule for a frame of zero rows keeps that dashboard correct.
