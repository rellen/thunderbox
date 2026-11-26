# ADR 0087: A column can take a fixed width, and the other columns share the rest

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-27 |
| Source | new |
| Code | `crates/thunderbox-core/src/frame.rs`, `crates/thunderbox-core/src/dashboard.rs`, `crates/thunderbox-widgets/src/table.rs`, `crates/thunderbox-widgets/src/top.rs`, `crates/thunderbox-dashboard/src/view.rs` |
| Tests | `plan_widths_gives_a_fixed_column_exactly_its_width`, `plan_widths_shares_the_rest_among_the_flex_columns_and_gives_the_remainder_to_the_left`, `plan_widths_leaves_the_spare_cells_empty_when_every_column_is_fixed`, `plan_widths_drops_a_column_at_the_right_when_the_fixed_widths_do_not_fit`, `plan_widths_draws_the_first_column_alone_when_no_column_fits_with_its_note`, `plan_widths_gives_the_same_widths_as_before_when_no_column_is_fixed`, `plan_widths_never_returns_more_than_the_width_of_the_panel_with_fixed_columns`, `the_panel_gives_a_column_the_width_that_its_data_source_states`, `the_widths_of_the_panel_replace_the_widths_of_the_data_source`, `the_panel_gives_the_process_columns_the_widths_that_the_data_source_states`, `the_panel_takes_the_width_of_the_first_field_of_a_header`, `a_table_panel_and_a_top_panel_of_a_dashboard_give_the_columns_the_widths_that_the_dashboard_names`, `validate_returns_an_error_for_widths_on_a_panel_that_is_not_a_table_or_a_top`, `validate_returns_an_error_for_a_width_of_a_column_with_an_empty_name`, `validate_returns_an_error_for_a_width_of_0`, `a_width_survives_a_round_trip_through_toml` |
| Related | [ADR 0007](0007-the-settings-of-a-panel-win-over-the-column.md), [ADR 0083](0083-a-column-can-hold-identifiers.md), [ADR 0084](0084-each-column-of-a-table-takes-an-equal-share-of-the-panel.md), [ADR 0086](0086-a-table-rounds-a-number-to-fit-its-column.md) |

## Context

ADR 0084 gave each column of a table panel and of a top panel an equal share of
the panel. The user then looked at the process list and wrote this:

```text
pid only ever needs 6 cells, cpu only ever needs 4, mem only ever needs 4 (only
show K, M, G not KB, MB, GB) - so can we enhance the flex to fix some cols to
cell numbers and give any flex to the others based on fraction
```

An equal share gives the column `name` the same room as the column `pid`. The
name needs the room, and the number of a process does not. The user chose these
answers:

- The data source states the widths of its own columns. A panel field can
  replace them, as a panel unit replaces a column unit (ADR 0007).
- The memory column of the process list loses its unit. ADR 0088 records that
  decision.

## Decision

1. `thunderbox_core::FieldConfig` has the field `width`, of the type
   `Option<u16>`. It is the number of cells that the column takes in a table
   panel or a top panel. Every other panel ignores it. A column with no width
   serialises as before.
2. A panel of a dashboard file has the field `widths`. It is a table from the
   text of a header to a number of cells, as in
   `widths = { pid = 6, cpu = 4, MemB = 4 }`. It applies to the kinds `table`
   and `top`.
3. `Dashboard::validate` returns an error in three cases:
   - a panel that is not of the kind `table` or `top` states `widths`;
   - a header is empty, or it holds spaces alone;
   - a width is 0.
4. `TablePanel::widths` and `TopPanel::widths` take the table of the panel. The
   width of a column is the first of these that exists: the entry of the panel
   for the header that the panel shows, then the `width` of the `FieldConfig` of
   the column, then none. The top panel takes the first field of a header, as it
   does for every other hint.
5. A column that has a width is a **fixed** column. A column that has none is a
   **flex** column. `plan_widths` takes the fixed width of each column, and
   it follows these rules:
   1. The columns need the sum of the fixed widths, `MIN_COLUMN` cells for each
      flex column, and one `COLUMN_GAP` between each pair of columns.
   2. The panel draws all columns when they fit in `available`. Each fixed
      column takes exactly its width. The flex columns share the rest in equal
      parts, and the remainder gives one more cell to each flex column from the
      left. A panel that has no flex column leaves the spare cells empty at the
      right of the last column.
   3. Otherwise the panel drops columns at the right, fixed or flex. It draws
      the largest number `k`, with `k >= 1`, of the first columns that fit with
      one `COLUMN_GAP` and the note `+{n - k}`. The flex columns among them
      share the rest after the note, as in rule 2.
   4. Otherwise the panel draws the first column with no note, when it fits. A
      fixed column needs its width and takes exactly that width. A flex column
      needs `MIN_COLUMN` cells and takes the whole of `available`.
   5. Otherwise the panel draws the border and the title alone.
   6. A panel that has no fixed column follows ADR 0084 exactly.
   7. No width reads the text of a cell. A header that is wider than a fixed
      column loses its end, by the rule that already holds.
6. A width of 0 in a data source becomes 1. A column of no cell shows no text.

## Consequences

- The number of a process of 7 digits, which Linux allows, shows `—` in 6
  cells. ADR 0065 forbids a cut number, and an identifier never rounds.
- A process above 999 percent shows `—` in 4 cells. A percent has no decimal
  place to round away.
- A panel field can widen either column. A dashboard that needs the wider
  `pid` writes `widths = { pid = 8 }`.
- A fixed column wider than the panel is dropped, as any other column is.
- The panel field names a header. A header that no column shows has no effect,
  as for `left`.
- ADR 0084 is superseded. Its rules 1 to 6 still hold for a panel that has no
  fixed column, and the tests of ADR 0084 pass with no change of their numbers.

## Measurements

The widths of the example: four columns with the widths 6, none, 4, and 4 in 40
cells give 6, 23, 4, and 4. The sum is 37 and the three gaps make 40.

The author took each rule out of the code in turn, and ran
`cargo test --no-fail-fast` for the crates `thunderbox-core`,
`thunderbox-widgets`, and `thunderbox-dashboard`.

| rule taken out | the tests that failed |
|---|---|
| a fixed column takes its width (every column is flex) | 5 tests: `the_panel_gives_a_column_the_width_that_its_data_source_states`, `the_widths_of_the_panel_replace_the_widths_of_the_data_source`, and the three process tests of `top` |
| the flex columns share the rest | 46 tests, among them every `plan_widths` test |
| the remainder goes to the left flex column | 10 tests, among them `plan_widths_shares_the_rest_among_the_flex_columns_and_gives_the_remainder_to_the_left` |
| the panel field replaces the width of the data source | `the_widths_of_the_panel_replace_the_widths_of_the_data_source` of `table`, and the same rule of `top` |
| the spare cells stay empty | `plan_widths_leaves_the_spare_cells_empty_when_every_column_is_fixed` and 2 more |
| the top panel reads the width of the first field | the 3 width tests of `top` |
| the view passes `widths` to the panel | `a_table_panel_and_a_top_panel_of_a_dashboard_give_the_columns_the_widths_that_the_dashboard_names` alone. Each test of `thunderbox-widgets` passed. |
| `validate` rejects `widths` on another kind | `validate_returns_an_error_for_widths_on_a_panel_that_is_not_a_table_or_a_top` |
| `validate` rejects an empty name | `validate_returns_an_error_for_a_width_of_a_column_with_an_empty_name` |
| `validate` rejects a width of 0 | `validate_returns_an_error_for_a_width_of_0` |

## Designs that the project rejected

- **Widths in the panel field alone.** Every dashboard would repeat the widths
  of the process list. The data source knows what its own columns hold.
- **Widths in the data source alone.** A dashboard could not change them. A
  narrow panel or a long process number needs a different width.
- **A fixed width that never drops below the width of its header.** The user
  chose exact widths. A header that is wider than its column loses its end, as
  any other text does.
