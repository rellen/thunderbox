# ADR 0084: Each column of a table takes an equal share of the panel

| field | value |
|---|---|
| Status | Superseded by [ADR 0087](0087-a-column-can-take-a-fixed-width-and-the-other-columns-share-the-rest.md) |
| Recorded | 2026-11-27 |
| Source | new |
| Code | `crates/thunderbox-widgets/src/table.rs` |
| Tests | `equal_widths_gives_each_column_an_equal_share_of_the_room`, `equal_widths_fills_the_room_exactly`, `plan_widths_draws_every_column_in_the_whole_width`, `plan_widths_drops_the_columns_at_the_right_and_leaves_room_for_the_note`, `plan_widths_returns_no_width_when_no_column_fits_with_its_note`, `plan_widths_never_returns_more_than_the_width_of_the_panel`, `the_columns_start_at_the_same_cells_when_the_text_of_the_cells_changes`, `the_table_fills_the_width_of_the_panel`, `the_columns_of_the_panel_start_at_the_same_cells_when_the_text_of_the_cells_changes`, `the_columns_of_the_panel_fill_the_width_of_the_panel` |
| Related | [ADR 0063](0063-a-narrow-top-panel-drops-the-sort-column.md), [ADR 0065](0065-no-panel-cuts-a-number.md), [ADR 0083](0083-a-column-can-hold-identifiers.md) |

## Context

The table panel and the top panel gave each column the width of its widest cell
among the rows on screen. A list of processes changes on each refresh, so the
widest cell changes too. The columns then moved to the left and to the right,
and a reader could not follow one column.

The measurement: a top panel of 32 cells held the columns `pid`, `name`, and
`cpu`. With the name `sh` in each row, the header `name` stood at cell 5. With
the name `a very long process name` in each row, the same header stood at
cell 9. Only the text of the cells changed.

The user asked for columns that flex and fill the width of the panel. Each
column gets a fraction of the width of the panel. The width does not depend on
the content.

## Decision

The widths of the columns depend on two numbers: the width inside the border,
called `available`, and the number of columns, called `n`. These rules hold:

1. The panel draws all `n` columns in `available` cells when
   `n * MIN_COLUMN + (n - 1) * COLUMN_GAP <= available`. `MIN_COLUMN` is 4, and
   `COLUMN_GAP` is 1.
2. Otherwise the panel drops columns at the right. It draws the largest number
   `k`, with `k >= 1`, for which
   `k * MIN_COLUMN + (k - 1) * COLUMN_GAP + COLUMN_GAP` plus the width of the
   note `+{n - k}` is `available` or less. The `k` columns share `available`
   minus that gap and that note.
3. The panel draws one column and no note when no `k` fits and one column of
   `MIN_COLUMN` cells fits. That column takes the whole of `available`.
   The panel draws the border and the title alone when `available` is smaller
   than `MIN_COLUMN`.
4. The columns share the room in equal parts. Each column takes
   `(room - (k - 1) * COLUMN_GAP) / k` cells. The remainder of that division
   gives one more cell to each column from the left, until it runs out. The
   widths and the gaps fill the room exactly.
5. No width reads the text of a cell or of a header. The function
   `plan_widths` takes two numbers, and it takes no text. The method
   `Column::natural_width` no longer exists.
6. The rules for a cell stay. A text that is too long loses its end and takes
   one full stop. A number that does not fit shows `—`, as ADR 0065 states. A
   header that is too long loses its end in the same way as a text.

## Consequences

- A long name loses more of its end in a narrow panel, because it takes an
  equal share and not the share that it needs.
- A short column holds empty cells, because it takes the same share as a long
  column.
- The columns never move while the panel keeps its width.
- The note of the dropped columns always takes the short form `+N`. The long
  sentence needs room that the equal shares must use. The function
  `dropped_text` still writes the sentence for a row of the top panel.
- A panel that is too narrow for one column and its note draws one column and
  no note. The old rule did the same, and this record restores it. The column
  takes the whole width. A panel that is narrower than one column draws the
  border and the title alone.
- ADR 0063 still holds. A narrow top panel can drop the column that gives the
  order.

## Measurements

The old code failed the new tests of the top panel. This command ran them with
the old file `table.rs`:

```
cargo test -p thunderbox-widgets --lib top::
```

The tests `the_columns_of_the_panel_start_at_the_same_cells_when_the_text_of_the_cells_changes`
and `the_columns_of_the_panel_fill_the_width_of_the_panel` failed. The new code
passed every test.

Each rule was taken out in turn. These tests failed:

| rule taken out | the tests that failed |
|---|---|
| 4, the remainder is lost | nine tests: both tests of `equal_widths`, three tests of `plan_widths`, and the four render tests of the columns |
| 4, the remainder goes to the right | six tests: both tests of `equal_widths`, two tests of `plan_widths`, and the two tests that start the columns at the same cells |
| 2, the gap before the note is not counted | `plan_widths_never_returns_more_than_the_width_of_the_panel`, `plan_widths_returns_no_width_when_no_column_fits_with_its_note` |
| 2, the columns ignore the note | four tests, among them `the_panel_draws_the_columns_that_fit_and_the_note_of_the_columns_that_it_dropped` |
| 3, one column stays when none fits | `the_panel_draws_the_border_alone_when_no_column_fits`, and two tests of `plan_widths` |

## Designs that the project rejected

- **The width of the widest cell on screen.** This was the old rule. The
  columns moved on each refresh, as the measurement above shows.
- **The width of the widest value of all rows.** The columns still move when a
  new widest value arrives. A person cannot predict when that happens.
- **A panel that remembers the widest width of each column.** The panel would
  need state between frames. The user asked for widths that come from the width
  of the panel alone.
