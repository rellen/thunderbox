# ADR 0088: The process list names its memory column MemB and states no unit

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-27 |
| Source | new |
| Code | `crates/thunderbox-source-sysinfo/src/process.rs`, `crates/thunderbox-source-testdata/src/lib.rs` |
| Tests | `every_column_holds_one_value_for_each_process`, `the_columns_state_the_widths_of_the_process_list`, `processes_returns_the_columns_of_the_sysinfo_list`, `a_top_panel_gives_the_columns_of_the_process_list_the_widths_that_the_data_source_states` |
| Related | [ADR 0047](0047-the-process-list-of-sysinfo-keeps-no-past.md), [ADR 0083](0083-a-column-can-hold-identifiers.md), [ADR 0086](0086-a-table-rounds-a-number-to-fit-its-column.md), [ADR 0087](0087-a-column-can-take-a-fixed-width-and-the-other-columns-share-the-rest.md) |

## Context

The user asked for a memory column that needs 4 cells, and for the letters K, M,
and G with no `B`. The unit `bytes` writes `855MB`, which needs 5 cells. With no
unit the same number reads `855M`, which needs 4 cells. The header `memory`
needs 6 cells, so it would lose its end in a column of 4 cells. The user chose
the header `MemB`, which fits and says that the values are bytes.

## Decision

1. The sysinfo data source writes these columns of the process list:
   - `pid`, an identifier, with the width 6;
   - `name`, with no width;
   - `cpu`, with the unit `percent` and the width 4;
   - `MemB`, with no unit and the width 4.

   The values of `MemB` stay bytes.
2. The data source testdata writes its `processes` frame in the same shape.
3. Every test and every page that named the column `memory` of the process list
   names `MemB` now.

## Consequences

- A field `sort = "memory"` on a top panel of the process list now returns the
  error `No column of numbers shows the header "memory".` No dashboard of the
  repository states it.
- A panel writes `855M`, `2.1G`, and, by ADR 0086, `12M`. A cell of the column
  never shows the unit `B`.
- A panel field `unit` of a top panel applies to the column that gives the
  order. A top panel that sorts by `MemB` and states `unit = "bytes"` shows
  the unit again.
- A threshold of a top panel still compares bytes, because the values stay
  bytes.

## Measurements

The author took each rule out of the code in turn, and ran
`cargo test --no-fail-fast` for the crates `thunderbox-source-sysinfo`,
`thunderbox-source-testdata`, and `thunderbox-dashboard`.

| rule taken out | the tests that failed |
|---|---|
| the header `MemB` of sysinfo | `every_column_holds_one_value_for_each_process` |
| the unit of `MemB` of sysinfo is absent | `every_column_holds_one_value_for_each_process` |
| a width of sysinfo (`pid`, `cpu`, or `MemB`) | `the_columns_state_the_widths_of_the_process_list` |
| the header `MemB` of testdata | `processes_returns_the_columns_of_the_sysinfo_list` and `a_top_panel_gives_the_columns_of_the_process_list_the_widths_that_the_data_source_states` |
| the unit of `MemB` of testdata is absent | `processes_returns_the_columns_of_the_sysinfo_list` |
| the width of `pid` of testdata | `processes_returns_the_columns_of_the_sysinfo_list` and `a_top_panel_gives_the_columns_of_the_process_list_the_widths_that_the_data_source_states` |
| the width of `cpu` of testdata | `processes_returns_the_columns_of_the_sysinfo_list` |

## Designs that the project rejected

- **Drop the `B` of the unit `bytes` in every panel.** A stat panel and a chart
  would lose the unit. A reader could not tell bytes from a count.
- **Keep the header `memory` and cut it to `mem.`.** The header would end
  with a full stop, and the panel uses that mark for a text that it cut. A
  reader would take the header for a cut text. The header `MemB` fits with no
  cut.
