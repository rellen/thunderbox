# ADR 0085: A panel can keep a column of numbers at the left

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-27 |
| Source | new |
| Code | `crates/thunderbox-core/src/dashboard.rs`, `crates/thunderbox-widgets/src/table.rs`, `crates/thunderbox-widgets/src/top.rs`, `crates/thunderbox-dashboard/src/view.rs` |
| Tests | `a_table_panel_and_a_top_panel_read_the_columns_that_stand_at_the_left`, `validate_returns_an_error_for_a_left_column_on_a_panel_that_is_not_a_table_or_a_top`, `validate_returns_an_error_for_a_left_column_with_an_empty_name`, `the_panel_keeps_a_column_of_numbers_at_the_left_when_the_dashboard_names_it`, `the_panel_never_cuts_a_number_in_a_column_that_stands_at_the_left`, `the_panel_matches_a_left_name_with_the_header_that_includes_the_name_of_the_frame`, `the_column_pid_of_a_panel_that_names_no_left_column_stands_at_the_right`, `the_panel_keeps_the_column_pid_at_the_left_when_the_dashboard_names_it`, `a_table_panel_and_a_top_panel_of_a_dashboard_keep_the_columns_that_the_dashboard_names_at_the_left` |
| Related | [ADR 0065](0065-no-panel-cuts-a-number.md), [ADR 0083](0083-a-column-can-hold-identifiers.md), [ADR 0084](0084-each-column-of-a-table-takes-an-equal-share-of-the-panel.md) |

## Context

A table panel and a top panel stand a column of numbers at the right, header and
cells. A reader compares such a column by its last digit. Some numbers are
better at the left. The column `pid` is an example, because a reader scans it as
a name. The panel cannot know which column a person wants at the left, so the
dashboard file must say.

## Decision

1. A column of numbers stands at the right, header and cells, in the table
   panel and in the top panel. This rule is as it was.
2. A panel of the dashboard file can state the field `left`. It is a list of
   text, and each text is the header of a column. The default is an empty list.
   The struct `Panel` of `crates/thunderbox-core/src/dashboard.rs` holds the
   field. The writer leaves the field out when the list is empty.
3. `Dashboard::validate` returns an error in two cases:
   - a panel that is not of the kind `table` or `top` states `left`;
   - a panel states `left` with a header that is empty, or that holds spaces
     alone.
4. `TablePanel::left` and `TopPanel::left` take a list of headers. A column
   whose header equals one of the names stands at the left, header and cells,
   even when it holds numbers. The header is the text that the panel shows,
   after `plain` and after the name of the frame and the full stop. A name that
   matches no header has no effect.
5. A column that stands at the left still counts as a column of numbers for the
   rule of ADR 0065. The panel never cuts a number of that column, and it shows
   `—` in the place of a number that does not fit.
6. `DashboardView` passes the list of the panel to both panels.

## Consequences

- A person can keep one column at the left and another at the right.
- A misspelled name has no effect and returns no error, because the panel
  cannot know the headers of the data before the data arrives.
- The rule of the width does not change. The field `left` changes the place of
  the text inside a column, and it never changes a width.

## Measurements

Each rule was taken out in turn. These tests failed:

| rule taken out | the tests that failed |
|---|---|
| 4, no name matches a header | two tests of `table`, two tests of `top`, and the test of `thunderbox-dashboard` |
| 4, the table panel skips the list | `the_panel_keeps_a_column_of_numbers_at_the_left_when_the_dashboard_names_it` and `the_panel_matches_a_left_name_with_the_header_that_includes_the_name_of_the_frame` of `table`, and the test of `thunderbox-dashboard` |
| 4, the top panel skips the list | two tests of `top`, and the test of `thunderbox-dashboard` |
| 5, a column at the left cuts a number | `the_panel_never_cuts_a_number_in_a_column_that_stands_at_the_left` |
| 6, the view passes no list to the table panel, or none to the top panel | `a_table_panel_and_a_top_panel_of_a_dashboard_keep_the_columns_that_the_dashboard_names_at_the_left` |
| 3, no error for another kind | `validate_returns_an_error_for_a_left_column_on_a_panel_that_is_not_a_table_or_a_top` |
| 3, no error for an empty name | `validate_returns_an_error_for_a_left_column_with_an_empty_name` |

## Designs that the project rejected

- **One switch for the whole panel.** A switch cannot keep one column at the
  left and another at the right.
