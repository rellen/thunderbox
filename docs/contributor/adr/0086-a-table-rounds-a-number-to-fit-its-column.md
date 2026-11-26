# ADR 0086: A table rounds a number to fit its column

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-27 |
| Source | new |
| Code | `crates/thunderbox-widgets/src/table.rs`, `crates/thunderbox-widgets/src/top.rs` |
| Tests | `the_panel_drops_decimal_places_of_a_number_that_does_not_fit_its_column`, `the_panel_draws_the_symbol_for_a_number_that_has_no_shorter_form_that_fits`, `the_panel_drops_one_decimal_place_at_each_step_down_to_none`, `the_panel_never_rounds_an_identifier`, `the_panel_takes_the_next_letter_when_a_rounded_number_reaches_1000`, `cell_text_returns_the_first_shorter_text_that_fits`, `shorter_texts_round_with_the_format_and_never_cut_a_digit` |
| Related | [ADR 0065](0065-no-panel-cuts-a-number.md), [ADR 0084](0084-each-column-of-a-table-takes-an-equal-share-of-the-panel.md) |

## Context

A memory column of 4 cells is narrow. The number `12.3M` needs 5 cells. ADR 0065
forbids a cut number, so the cell showed `—` for a number that a reader could
read in a shorter form. The number `12M` states the same size with one place
less.

## Decision

1. A cell of numbers holds the full text of its number, as before. It also holds
   the shorter texts of the same number. Each shorter text has one decimal place
   fewer than the text before it, down to no decimal place. The function `entry`
   builds them, and the field `shorter` of `Entry` holds them.
2. `cell_text` draws the first text of the full text and the shorter texts that
   fits the column. It returns nothing when no text fits, and the panel then
   draws `—`, as before.
3. A shorter text never drops a digit of the whole part, a letter of the scale,
   a sign, or a unit. It removes decimal places only. The number `1.3G` in 2
   cells shows `1G`. The number `855MB` in 4 cells shows `—`.
4. An identifier never rounds (ADR 0083). It has no shorter text.
5. The function `format_value` writes each shorter text. A number therefore
   rounds as `format!("{v:.*}")` rounds it. The letter of the scale can change
   after a number rounds. The number 999.9K with no decimal place is `1M`.
6. Only the table panel and the top panel apply this rule. A stat panel, a
   gauge, a bar gauge, and a chart do not change.

## Consequences

- A reader sees fewer decimal places in a narrow column.
- The panel still never cuts a whole digit, and ADR 0065 still holds.
- The format decides how a number rounds, so a half rounds as Rust rounds it. The
  number 1.25G with one decimal place shows `1.2G`.

## Measurements

The new tests failed before the code, apart from the two tests of the rules
that the old code already held (`—` and an identifier). Each rule was then taken
out in turn.

| rule taken out | the tests that failed |
|---|---|
| 1 and 2, the panel draws the full text only | `cell_text_returns_the_first_shorter_text_that_fits`, `the_panel_drops_decimal_places_of_a_number_that_does_not_fit_its_column` and `the_panel_drops_one_decimal_place_at_each_step_down_to_none` and `the_panel_takes_the_next_letter_when_a_rounded_number_reaches_1000` of `table`, and `the_panel_drops_decimal_places_of_a_number_that_does_not_fit_its_column` of `top` |
| 4, an identifier rounds | `the_panel_never_rounds_an_identifier` |

## Designs that the project rejected

- **Change the automatic decimal places of the format for every panel.** A stat
  panel and a chart would lose a place that they have room for.
- **Cut the text.** ADR 0065 rejects a cut number.
