# ADR 0063: A narrow top panel drops the column that gives the order

| field | value |
|---|---|
| Status | Proposed |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 13 item 8 |
| Code | `crates/thunderbox-widgets/src/table.rs`, `crates/thunderbox-widgets/src/top.rs`, `crates/thunderbox-source-sysinfo/src/process.rs`, `crates/thunderbox/src/default.toml` |
| Tests | No test holds a narrow top panel. |
| Related | [ADR 0007](0007-the-settings-of-a-panel-win-over-the-column.md), [ADR 0046](0046-a-top-list-is-a-kind-of-panel-of-its-own.md), [ADR 0049](0049-the-number-of-a-process-is-text.md) |

## Context

A top panel draws its sheet with `draw_sheet`. That function drops the columns
at the right that have no room.

The list of processes holds the column `cpu` as its third column. A top panel
that is 12 cells wide therefore draws the column `pid` alone, and the note `+3`
beside it. The rows still come in the order of the column `cpu`, and no reader
can see that column.

The default dashboard gives the panel 55 cells on a terminal of 110 cells, and
40 cells on a terminal of 80 cells. The panel keeps all four columns at both
widths, and it cuts a long name. A person therefore meets this fault only in a
dashboard that holds a narrow top panel.

## Decision

The project has not taken a decision. `draw_sheet` drops the columns at the
right today, whatever column gives the order.

## Consequences

A narrow top panel can hide the column that gives the order. The rows then
follow an order that the panel does not show.

## The question that remains

The work that remains keeps the order of a narrow top panel visible to a
reader.

The column that gives the order can stand at the right of another column. The
next design answers one question. Which of those two columns does a narrow top
panel drop first?
