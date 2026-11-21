# ADR 0080: A dashboard states a refresh above 0 and a known unit

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-21 |
| Source | `docs/contributor/open-findings.md` items 4.4 and 5.3 |
| Code | `crates/thunderbox-core/src/dashboard.rs`, the constant `UNITS` and the method `Dashboard::validate`. Also `crates/thunderbox-dashboard/src/runtime.rs`, the function `DashboardRuntime::new`, and `crates/thunderbox-widgets/src/value.rs`, the function `format_value` |
| Tests | `validate_returns_an_error_that_names_refresh_for_a_refresh_of_0`, `validate_accepts_a_refresh_above_0_and_a_dashboard_with_no_refresh`, `validate_accepts_each_unit_of_the_list`, and `validate_returns_an_error_that_names_a_unit_that_no_rule_knows`, all in `crates/thunderbox-core/src/dashboard.rs`. Also `each_file_under_dashboards_loads` in `crates/thunderbox-core/tests/dashboards.rs`, `new_returns_an_error_for_a_refresh_of_0` in `crates/thunderbox-dashboard/tests/runtime.rs`, and `each_unit_of_the_core_takes_a_rule_of_its_own` and `a_unit_of_a_data_source_that_no_rule_knows_takes_the_format_of_no_unit` in `crates/thunderbox-widgets/src/value.rs` |
| Related | [ADR 0007](0007-the-settings-of-a-panel-win-over-the-column.md), [ADR 0041](0041-validate-reports-the-fault-of-a-variable.md), [ADR 0073](0073-a-duration-that-does-not-fit-returns-an-error.md) |

## Context

Two fields of a dashboard file gave a result that no person asked for, and the
program wrote no message for either result.

**The field `refresh`.** `Dashboard::validate` read the duration of `refresh`
and accepted each value that it could read. `DashboardRuntime::new` then
turned a period of 0 into `None`. That is the value of a dashboard that states
no field `refresh`. The comment of the code gave the reason: a period of 0
would ask the data source without a pause. A person who writes `0s` asks for
the fastest refresh. That person received no refresh on a clock.

**The field `unit`.** The function `format_value` holds one rule of format for
each of four names: `percent`, `bytes`, `bytes/s`, and `seconds`. It gives
every other name the format of a value with no unit. `Dashboard::validate`
read no unit. A typo in a unit therefore loaded, and the panel then looked
like a data source that answers wrongly. The panel wrote no letter of a unit
and no word of a fault.

Item 5.3 also found two rules of format that no page stated. The unit
`percent` multiplies by nothing, and it takes 0 decimals. A value must
therefore already run from 0 to 100. The unit `seconds` changes no scale.

## Decision

The project makes two checks in `Dashboard::validate`, and it keeps two rules
of format.

**A refresh above 0.** `validate` returns an error for a `refresh` of 0, in any
unit. The values `"0s"`, `"0ms"`, and `"0m"` are examples. The message reads
`The dashboard is not correct. The field refresh holds "0s". The value must be
more than 0. A dashboard with no field refresh does not refresh on a clock.`

A dashboard with no field `refresh` does not refresh on a clock. The key `r`
still runs the queries again. A new area or a new time range also starts a set
of queries. `DashboardRuntime::new` calls `validate` first, so its filter for a
period of 0 is gone.

**A known unit.** `thunderbox-core` holds the public constant `UNITS`. Its four
names are `percent`, `bytes`, `bytes/s`, and `seconds`. `validate` returns an
error for a panel whose `unit` the list does not hold. The match is exact, so
`Percent` returns an error, and so does an empty unit. The message reads
`The dashboard is not correct. The panel "cpu" states the unit "byte", and no
rule knows that unit. The known units are percent, bytes, bytes/s, seconds.`

A data source can still state any unit in the configuration of a column. No
check of a dashboard reads that unit. A name outside the list then takes the
format of a value with no unit, as it did before.

**Two rules of format.** The unit `percent` multiplies by nothing and takes 0
decimals, so a ratio of 0.42 draws `0%`. The unit `seconds` changes no scale,
so 0.002 draws `0s` and 86400 draws `86400s`. The page
`docs/user/reference/numbers-and-units.md` states both rules.

## Consequences

A typo in a unit now stops the program with a message that names the panel and
lists the four names. The program no longer draws a panel that looks wrong.

A data source keeps the freedom to state any unit. A column of such a unit
draws with the format of no unit, and no message tells the person.

The list `UNITS` of `thunderbox-core` and the rules of `format_value` in
`thunderbox-widgets` must change together. The two lists live in two crates, so the compiler
does not hold them together. The test
`each_unit_of_the_core_takes_a_rule_of_its_own` holds them together. It fails
when a name of `UNITS` takes the format of no unit.

A dashboard that wants no refresh on a clock omits the field `refresh`. A
person who wants the fastest refresh states a small duration, such as `"1s"`.

The check reads a period of 0 alone. ADR 0073 returns an error for a
duration that does not fit, so no other period reaches the runtime as no
refresh.

The doc comment of `refresh_interval` states that `None` means a dashboard
with no field `refresh`. A program that builds a `Dashboard` in code reaches
`DashboardRuntime::new`, and `new` calls `validate`. Such a program therefore
reads the same error. The test `new_returns_an_error_for_a_refresh_of_0` holds
that rule.

The test `each_file_under_dashboards_loads` loads each file of the folder
`dashboards`. A new dashboard file with a wrong unit now fails that test.

## Measurements

**Before the change.** A dashboard with no field `refresh` loaded. A
dashboard with `refresh` of `"0s"`, `"0ms"`, or `"0m"` also loaded, and each
of the four gave `refresh_interval()` of `None`. A `refresh` of `"1s"` gave
`Some(1s)`.

The call `format_value(1500.0, Some(unit), None)` gave these results:

| unit | text |
|---|---|
| `percent` | `1500%` |
| `byte` | `1.5K` |
| `Percent` | `1.5K` |
| `bytes/sec` | `1.5K` |
| the empty unit | `1.5K` |
| `furlongs` | `1.5K` |

Each of the last five names loaded in a panel, and each drew the text of a
value with no unit.

**After the change.** Each of the three zero values of `refresh` returns the
error above. Each of the five names returns the error above when a panel
states it. A debug build of `thunderbox` writes both errors under the words
`Error: The dashboard in FILE is not correct.` and `Caused by:`.
