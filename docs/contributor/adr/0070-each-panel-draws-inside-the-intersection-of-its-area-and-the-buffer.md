# ADR 0070: Each panel draws inside the intersection of its area and the buffer

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-21 |
| Source | `docs/contributor/open-findings.md` item 2.6 |
| Code | `crates/thunderbox-widgets/src/timeseries.rs`, `crates/thunderbox-widgets/src/stat.rs`, `crates/thunderbox-widgets/src/gauge.rs`, `crates/thunderbox-widgets/src/bargauge.rs`, `crates/thunderbox-widgets/src/table.rs`, `crates/thunderbox-widgets/src/top.rs`, `crates/thunderbox-widgets/src/row.rs`, `crates/thunderbox-widgets/src/lib.rs` |
| Tests | `each_panel_draws_the_intersection_of_the_area_and_the_buffer` and `the_row_panel_draws_the_intersection_of_the_area_and_the_buffer`, both in `crates/thunderbox-widgets/tests/area.rs` |
| Related | [ADR 0004](0004-design-for-the-way-inside-another-program-first.md), [ADR 0015](0015-both-ways-into-another-program-are-in-scope.md), [ADR 0016](0016-a-program-that-holds-a-dashboard-holds-four-rules.md), [ADR 0028](0028-the-caller-reports-one-area-as-a-size.md) |

## Context

The crate `thunderbox-widgets` is the smaller of the two ways to put thunderbox
inside another program. A program that embeds a panel passes an area and a
buffer to `render`. The program can pass any area. The area can reach past the
buffer, and the buffer can start away from the origin.

Only `GaugePanel` and `BarGaugePanel` took the intersection of the area and the
buffer before this decision. The `Chart` of ratatui writes each cell with no
check, so it panics with `index outside of buffer`. A `Block` and a `Line` of
ratatui clip their own writes.

## Decision

Each of the seven renderers of `thunderbox-widgets` starts its `render` with
`let area = area.intersection(buf.area);`. The renderers are `TimeSeriesPanel`,
`StatPanel`, `GaugePanel`, `BarGaugePanel`, `TablePanel`, `TopPanel`, and
`RowPanel`. Only then does the renderer ask `draws_a_panel`. The row panel asks
for a width and a height above zero.

Each panel therefore draws only inside the intersection. It draws there exactly
what it draws when the caller gives it that intersection.

## Consequences

A new panel must keep this rule. Follow these steps:

1. Take the intersection first.
2. Ask `draws_a_panel` about the intersection, and not about the area.
3. Add the panel to the test `area.rs`.

A panel that calls a helper which writes a cell must also pass that helper the
intersection. A check inside each write is not enough. The panel plans its
layout from the area that it holds, so the layout must start from the
intersection.

## Measurements

The test `crates/thunderbox-widgets/tests/area.rs` drew six panels that show
data. It used four states of the data: a frame of two series, no frame,
`PanelData::Loading`, and `PanelData::Error`. It used four cases, given as the
buffer and then the area:

1. `Rect::new(0, 0, 40, 10)` and `Rect::new(20, 5, 40, 10)`. The area reaches
   past the right edge and the lower edge.
2. `Rect::new(10, 4, 40, 10)` and `Rect::new(0, 0, 40, 10)`. The buffer starts
   away from the origin.
3. `Rect::new(0, 0, 20, 6)` and `Rect::new(0, 0, 80, 30)`. The area is much
   larger than the buffer.
4. `Rect::new(0, 0, 20, 6)` and `Rect::new(30, 10, 40, 10)`. The two share no
   cell.

Of the 96 draws, 49 broke the rule before the change.

- The time series panel with a frame panicked in all 4 cases. In the other 3
  states it drew a buffer that differs from the intersection in 3 of 4 cases.
- The stat panel, the table panel, and the top panel each differed in 3 of 4
  cases in every state. The stat panel drew the lower part of its large digits
  over its lower border. The table panel drew its columns over its right border
  and its lower border.
- The gauge and the bar gauge drew exactly the intersection.
- The row panel, collapsed and expanded, broke the rule in 4 of its 8 draws. It
  did not panic.

After the change, all draws pass.

The test compares each draw against a draw at the intersection. It also reads
the title on the top row of the intersection, because two draws that are wrong
in the same way are equal.

## Designs that the project rejected

- **The caller keeps the area inside the buffer.** The program `thunderbox`
  does so. A program that embeds a panel can pass any area, and the time series
  panel then panics.
- **Each write checks the buffer.** `StatPanel` did so inside its helper `put`.
  No panic follows. The panel computes its layout from the whole area,
  however, so the stat panel drew its digits over its own border.
