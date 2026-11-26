# ADR 0083: A column can hold identifiers

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-27 |
| Source | [ADR 0049](0049-the-number-of-a-process-is-text.md) |
| Code | `crates/thunderbox-core/src/frame.rs`, `crates/thunderbox-widgets/src/panel.rs`, `crates/thunderbox-widgets/src/top.rs`, `crates/thunderbox-source-sysinfo/src/process.rs` |
| Tests | `is_numeric_is_false_for_an_identifier`, `an_ordinary_configuration_writes_no_identifier_key`, `text_writes_every_digit_of_an_identifier_under_a_panel_that_states_a_unit`, `color_for_returns_the_fallback_for_an_identifier_that_reaches_a_threshold`, `the_panel_puts_an_identifier_at_the_right_of_its_column`, `a_panel_that_names_no_column_skips_an_identifier`, `the_column_pid_holds_numbers_and_is_an_identifier` |
| Related | [ADR 0007](0007-the-settings-of-a-panel-win-over-the-column.md), [ADR 0046](0046-a-top-list-is-a-kind-of-panel-of-its-own.md) |

## Context

ADR 0049 wrote the number of a process as text. With that choice, a panel never
wrote `1.2K` for the process 1234. The number also never gave the order of a top
panel. The choice had a cost. A panel stands text at the left of its column.
The column `pid` therefore stood at the left, and every other number stood at
the right.

The column `pid` as an ordinary number would also be wrong. A panel would write
`1.2K` for 1234. A top panel that names no `sort` field would then put the rows
in order by the number of the process.

The project therefore needs to tell two kinds of numbers apart. A number that
a panel can convert to `1.2K` measures something. A number that a panel cannot
convert names something.

## Decision

An identifier is a column of numbers that names a thing and does not measure
it. These rules hold:

1. `FieldConfig` holds a public field `identifier` of the type `bool`, with the
   default `false`. The serialised form of an ordinary column does not change,
   because the field is skipped when it is `false`.
2. An identifier is not a series. `Field::is_numeric` returns `false` for it,
   so `Frame::numeric_fields`, `legend::apply`, and `reduce_frames` skip it.
   The type of the values stays `Values::F64`.
3. A panel writes an identifier with every digit of the number, with no letter
   of the scale, and with no unit. A whole number holds no decimal point. The
   `decimals` of the column applies when the data source states it. The `unit`,
   the `decimals`, and the thresholds of the panel never apply.
   `PanelOptions::text` and `PanelOptions::color_for` hold this rule, so each
   panel follows it. This rule is an exception to ADR 0007.
4. The table panel and the top panel stand an identifier at the right of its
   column, because the type of the values is numbers. The top panel decides
   "stands at the right" from the type of the values, and it decides "can give
   the order" from `is_numeric`. A `sort` field that names an identifier
   returns the error `No column of numbers shows the header "pid".`
5. The data source `sysinfo` writes the column `pid` as a column of numbers
   with `identifier: true`. The data source `testdata` does the same for its
   list of processes.

## Consequences

- A top panel that names no `sort` field still takes the column `cpu`.
- A data source states the hint in the same way as it states the unit. No
  dashboard file holds a field for the hint.
- A new panel that writes a number must call `PanelOptions::text` and
  `PanelOptions::color_for`. A panel that formats a number in its own way
  ignores the hint.
- A caller that needs the type of the values must match on `Field::values`. It
  must not call `Field::is_numeric`, because that method now means "a series".
- Each caller of `is_numeric` and `numeric_fields` means "a series", so each
  skips an identifier. The stat panel, the gauge panel, the bar gauge panel,
  the legend, and the chart read no identifier.
- The table panel and the top panel read the type of the values, so an
  identifier stands at the right of its column.

## Measurements

The tests were written before the code. This command ran them:

```
cargo test --no-fail-fast
```

Before the code, `is_numeric_is_false_for_an_identifier` and
`apply_leaves_the_display_name_of_an_identifier_as_it_was` failed in the crate
`thunderbox-core`. Then five tests of the crate `thunderbox-widgets` failed:
three of `panel`, one of `table`, and one of `top`. After the code, every test
passed.

Each rule was taken out in turn. These tests failed:

| rule taken out | the tests that failed |
|---|---|
| 1, the skip of the serialised field | `an_ordinary_configuration_writes_no_identifier_key` |
| 2, `is_numeric` skips an identifier | `is_numeric_is_false_for_an_identifier`, `apply_leaves_the_display_name_of_an_identifier_as_it_was`, `reduce_frames_never_reduces_an_identifier`, `a_panel_that_names_no_column_skips_an_identifier`, `the_panel_returns_an_error_for_a_sort_column_that_is_an_identifier`, `the_column_pid_holds_numbers_and_is_an_identifier`, and two tests of `sysinfo` and `thunderbox` |
| 3, `PanelOptions::text` writes the digits | `text_writes_every_digit_of_an_identifier_under_a_panel_that_states_a_unit`, `text_applies_the_decimals_of_the_column_to_an_identifier_and_not_those_of_the_panel`, and one test each of `table` and `top` |
| 3, `PanelOptions::color_for` returns the fallback | `color_for_returns_the_fallback_for_an_identifier_that_reaches_a_threshold` |
| 4, the top panel stands an identifier at the right | `the_panel_puts_an_identifier_at_the_right_of_its_column` |
| 4, the top panel skips an identifier for the order | `a_panel_that_names_no_column_skips_an_identifier`, `the_panel_returns_an_error_for_a_sort_column_that_is_an_identifier` |
| 4, the table panel stands a number at the right | `the_panel_puts_an_identifier_at_the_right_and_writes_every_digit`, and two older tests of `table` |
| 5, the data source writes the mark | `the_column_pid_holds_numbers_and_is_an_identifier`, and two tests of `sysinfo` and `thunderbox` |

## Designs that the project rejected

- **Stand each column of text that holds only digits at the right.** The
  alignment would change when one value holds a character that is not a digit.
  A panel would also guess what a column means from its values.
- **A new type of values for identifiers.** Every match on `Values` would
  change for one hint. `FieldConfig` already carries the unit in the same way,
  so the hint belongs there.
