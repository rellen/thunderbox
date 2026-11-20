# ADR 0010: The program that runs alone is a dashboard over sysinfo

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 7 |
| Code | `crates/thunderbox/src/main.rs`, `crates/thunderbox/src/default.toml`, `crates/thunderbox-source-sysinfo` |
| Tests | `the_dashboard_inside_the_program_is_correct`, `the_dashboard_inside_the_program_reads_this_machine`, `the_dashboard_inside_the_program_shows_every_type_of_panel`, `the_default_dashboard_draws_the_names_that_it_drew_before` |
| Related | [ADR 0004](0004-design-for-the-way-inside-another-program-first.md), [ADR 0036](0036-each-label-of-the-time-axis-states-its-distance-from-now.md), [ADR 0037](0037-the-refresh-continues-while-the-window-stands-still.md) |

## Context

[ADR 0004](0004-design-for-the-way-inside-another-program-first.md) names four
ways to run thunderbox. One of them is the program that runs alone. A program
of that kind could take a separate path through the code, with a look of its
own.

## Decision

The program that runs alone is **thunderbox with the built-in `sysinfo` data
source and a default dashboard**. Its look is a theme and a set of panels. It is
not a separate path through the code. The same renderer, the same frames, and
the same panels serve every way to run thunderbox.

## Consequences

This arrangement also gives a demonstration that needs one command.

The look has one limit. The axis of time covers the window that a person asked
for, and not the data that a data source returned.
[ADR 0036](0036-each-label-of-the-time-axis-states-its-distance-from-now.md)
states that rule. The sysinfo data source holds nothing from before the program
started. The first minutes of every run therefore draw a short line at the
right edge. That picture is the truth about the data source, and the truth
comes before the look.

`crates/thunderbox/src/default.toml` holds the default dashboard, and
`main.rs` reads it when a person names no file.
