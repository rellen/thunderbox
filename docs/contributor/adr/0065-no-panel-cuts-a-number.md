# ADR 0065: No panel cuts a number

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-21 |
| Source | new |
| Code | `crates/thunderbox-widgets/src/stat.rs`, `crates/thunderbox-widgets/src/gauge.rs`, `crates/thunderbox-widgets/src/bargauge.rs`, `crates/thunderbox-widgets/src/table.rs` |
| Tests | `the_panel_draws_the_symbol_in_place_of_a_number_that_does_not_fit` and `the_panel_draws_a_number_that_fills_the_tile_exactly` of `stat.rs`, `the_panel_draws_no_number_that_does_not_fit` and `label_text_returns_an_empty_text_for_a_number_that_does_not_fit` of `gauge.rs`, `row_parts_drops_the_text_of_the_value_in_a_narrow_row` of `bargauge.rs`, `cell_text_returns_nothing_for_a_number_that_does_not_fit` and `the_panel_draws_no_number_that_it_cannot_show_in_full` of `table.rs` |
| Related | [ADR 0046](0046-a-top-list-is-a-kind-of-panel-of-its-own.md) |

## Context

A panel cuts a text that does not fit, and it ends the cut text with a full
stop. A reader then knows that the text continues. That rule suits a name. It
does not suit a number, because a reader takes the full stop for a decimal
point.

The stat panel cut a number before this record. The value 1234.5678 with 4
decimal places reads `1.2346K`, and that text takes 7 cells. A tile of 6 cells
drew `1.234.`, which states a number 1000 times too small.

A narrow tile is common. The stat panel adds a second column of tiles when
each column holds 8 cells, and the last cell of each tile then stays empty.
A tile of 8 cells then holds 7 cells for a number.

The gauge panel, the bar gauge panel, and the table panel already refused to
cut a number. No record stated that rule.

## Decision

The project cuts no number. Each panel draws the whole text of a number, or it
draws no number:

- The stat panel draws the symbol `—` in the place of the number.
- The gauge panel draws the bar with no number.
- The bar gauge panel drops the text of the value, and it keeps the bar.
- The table panel and the top panel draw the symbol `—` in the cell.

A text that fills its space exactly fits, and the panel draws it whole.

## Consequences

A narrow panel can show less than a wide panel. A reader then sees no number,
and not a wrong number.

The stat panel keeps its own rules for the large digits. It draws the large
form only when the whole text fits, so this rule binds its plain text alone.

Each new kind of panel follows this rule. It also holds a test that fails when
the code loses the rule.

## Measurements

The command `cargo test --no-fail-fast` ran once for each change below. Each
change removed the rule from one panel.

| panel | the change | the tests that failed |
|---|---|---|
| stat | `whole_number` returns the text whatever its width | 1 |
| stat | `<=` becomes `<` in `whole_number` | 1 |
| gauge | `label_text` returns the text whatever its width | 2 |
| bar gauge | `row_parts` keeps the width of the value in a narrow row | 3 |
| table | `cell_text` never returns `None` | 2 |
