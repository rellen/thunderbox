# ADR 0046: A list in the form of `top` is a kind of panel of its own

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-widgets/src/top.rs`, `crates/thunderbox-widgets/src/table.rs`, `crates/thunderbox-core/src/dashboard.rs` |
| Tests | `the_panel_shows_the_first_rows_and_counts_the_rows_that_do_not_fit`, `the_rows_of_two_frames_join_into_one_list`, `the_settings_of_the_panel_describe_the_column_that_gives_the_order`, `a_threshold_of_the_panel_colours_the_column_that_gives_the_order_alone`, `the_panel_shows_the_last_rows_and_puts_the_newest_row_at_the_bottom`, `the_panel_draws_a_column_for_each_field_of_each_frame`, `validate_returns_an_error_for_a_sort_column_on_a_panel_that_is_not_a_top_panel` |
| Related | [ADR 0007](0007-the-settings-of-a-panel-win-over-the-column.md), [ADR 0047](0047-the-process-list-of-sysinfo-keeps-no-past.md), [ADR 0049](0049-the-number-of-a-process-is-text.md), [ADR 0063](0063-a-narrow-top-panel-drops-the-sort-column.md) |

## Context

The program `top` shows the processes of a machine in order by one column. A
panel that draws a list in that form differs from the table panel.

The project examined a field `sort` on the table panel. The top panel differs
from the table panel in three rules, and that design would make each rule
depend on one field:

- The table panel shows the last rows that fit. A row of a time series is a
  moment, and a person reads the newest moment. A list of processes in the
  order of a column must show the first rows instead.
- The table panel puts the columns of two frames side by side, because two
  series of one moment share a row. Two lists of processes share no row. The
  top panel therefore puts the rows of every frame into one list. A column
  joins the column of another frame that shows the same header.
- The table panel applies its settings to every column. The top panel applies
  them to the column that gives the order, and
  [ADR 0007](0007-the-settings-of-a-panel-win-over-the-column.md) states why.

## Decision

A list in the form of `top` is a kind of panel of its own, and not an option of
the table panel. A person who reads `kind = "top"` knows all three rules from
one word.

The two panels still share one layout, because `draw_sheet` in
`crates/thunderbox-widgets/src/table.rs` draws the sheet of both.

## Consequences

`Dashboard::validate` rejects the field `sort` on a panel that is not a top
panel.

## Designs that the project rejected

**A field `sort` on the table panel.** Each of the three rules above would then
depend on that one field.
