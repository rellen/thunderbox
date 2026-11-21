# ADR 0060: A height that uncovers a panel asks every panel

| field | value |
|---|---|
| Status | Proposed |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 13 item 4 |
| Code | `crates/thunderbox-dashboard/src/runtime.rs`, `crates/thunderbox-dashboard/src/grid.rs`, `crates/thunderbox-dashboard/src/view.rs`, `crates/thunderbox/src/main.rs`, `scripts/record-demo.py` |
| Tests | `a_height_that_uncovers_a_panel_starts_a_set_of_queries`, `a_height_that_only_drops_panels_starts_no_set_of_queries` |
| Related | [ADR 0021](0021-a-new-area-waits.md), [ADR 0022](0022-an-area-that-changes-no-point-limit-starts-no-set.md), [ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md), [ADR 0027](0027-a-dropped-panel-keeps-its-answer.md), [ADR 0029](0029-the-runtime-asks-about-every-panel-until-an-area-arrives.md), [ADR 0053](0053-a-panel-that-appears-costs-a-set-of-queries.md) |

## Context

The renderer draws no panel under 3 cells wide or under 3 cells high. The
runtime asks a data source for nothing for such a panel, as
[ADR 0024](0024-a-panel-that-the-renderer-drops-asks-for-nothing.md) states. A
new height of the terminal can uncover such a panel.

A height that uncovers a panel starts a set of queries, and that set asks about
every panel that the renderer draws. Only the panel that appeared needs a new
answer.

The point limit of a panel is the largest number of points that each query of
that panel returns. The runtime holds no point limit for a panel that the area
before dropped. It therefore cannot know whether the answer that such a panel
holds has the right shape. Every other panel holds the width that it held, and
therefore the answer that it needs.

### Height of a step

A better design would save the queries of every other panel. That saving
follows the number of panels that stand one above the other. A dashboard of P
equal panels in one column loses its first panel at about 3P rows. The last
column of this table counts the queries of the step that costs the most:

| panels in one column | rows where a panel goes | queries today |
|---|---|---|
| 4 | 8 to 11 | 4 |
| 8 | 16 to 23 | 8 |
| 12 | 24 to 35 | 12 |

The height of a step is not 3P in general. `places` scales each panel by its
own `h` against `rows_of_grid`. The step of a dashboard therefore sits at about
3 times `rows_of_grid`, divided by the smallest `h` of that dashboard. The
writer of the dashboard states both numbers.

`TOO_SMALL_DASHBOARD` of `scripts/record-demo.py` is a dashboard of this
repository, and the `too_small` scene draws it. Its first panel holds 4 grid
rows of 28. That panel appears at 21 rows of the body, which is the area that
holds the panels. A body of 21 rows needs a terminal of 22 rows. A dashboard
that holds one short strip above two tall charts therefore uncovers a panel at
a height that a person uses every day.

### Cost of a drag

A person who drags the edge of a terminal window changes the height one row at
a time. A drag costs much more than one step. `AREA_QUIET_TIME` is 250
milliseconds. A drag that rests longer than that on each row settles on each
row. It therefore starts a whole set of queries on each row.

## Decision

The project has not taken a decision. A height that uncovers a panel starts a
set of queries that asks about every panel that the renderer draws.

The project designed two ways to take the saving, and it took neither. It then
examined three more ways, and it took none of them. A reader measured a fault
in each of the five. The section "Designs that the project rejected" names
each one.

## Consequences

Each step of a drag that settles costs one whole set of queries today. A
dashboard that holds many panels in one column pays the most.

## Measurements

A reader measured a drag against the runtime, in the order that the loop of
`crates/thunderbox/src/main.rs` uses. Each number is a count of queries:

| the drag | 100 milliseconds a row | 300 milliseconds a row | 500 milliseconds a row | a runtime that asks only what it needs |
|---|---|---|---|---|
| 8 panels, 16 rows to 24 rows | 8 | 12 | 36 | 8 |
| 12 panels, 24 rows to 36 rows | 23 | 24 | 78 | 12 |

A drag of 500 milliseconds a row costs the sum 1 + 2 + ... + P. A person who
drags an edge deliberately rests that long on a row. The wait of
`AREA_QUIET_TIME` joins the rows of a fast drag alone.

## Designs that the project rejected

### First design: ask about the panel that appeared

The first design sends a set of queries that asks about the panel which
appeared. That set keeps the answer of every other panel.

The fault: `DashboardView::render` gives one window to every chart.
[ADR 0027](0027-a-dropped-panel-keeps-its-answer.md) states why. A panel that
kept its frames would draw a line that stops before the right edge of that
window. A person reads that shape as a data source that died. A stat, a gauge
and a table give a person no sign at all.

### Second design: hold the moment of the snapshot still

The second design holds `Snapshot::updated_at` still for such a set, so that
every panel of the snapshot describes one window.

The fault: that moment then never moves again on a dashboard that states no
refresh interval, because each later area holds it still once more. The design
also gives a data source a moment in the past as the present moment, and
`QueryRequest::now` names the present moment.

### Third design: give each answer its own moment

The third design gives each answer its own moment. In that design,
`PanelState::Ready` carries the moment that its set of queries asked about. A
new method `Snapshot::panel_window` builds the window of one panel from that
moment, and `view.rs` gives each chart its own window. A chart that holds an
older answer then draws an axis that ends before the present moment. Its line
reaches the right edge of that axis.

The fault: no sign of the older answer exists at the heights where the design
saves a query. A chart of 3 cells draws a border and a line, and it draws no
axis of time at all. Every height at which a panel appears gives the panels
around it 3 cells or 4. The design depends on the axis of time alone, and that
axis is absent at each of those heights. A stat, a gauge, a bar gauge and a
table draw no axis at any height.

### Fourth design: one window and two sets of queries

The fourth design keeps one window and sends two sets of queries. A set that an
area starts asks about the window that the snapshot already names, as two
fixed moments. It gives the true present moment to each data source.
`TimeRange::to_absolute` and `QueryRequest::new` already hold that shape. A
person who moves the time range into the past sends that shape today.

This design holds two faults:

- The set still asks about every panel that the renderer draws. It therefore
  saves nothing at any step of a drag, and the drag holds the cost.
- The chart writes `now` at the right edge of a window of any age.

### Fifth design: remember the point limit of each panel

The fifth design remembers the point limit of the last query of each panel. A
panel that appears at the width that it held then needs nothing.

The fault: two other steps write a `PanelState` without that memory.
`clear_the_error_of_each_panel_that_the_area_drops` writes
`PanelState::Loading`, and `ClearOnEnd` writes `PanelState::Error`. Each one
leaves the memory as it was.

Take a panel that held an error, that a height then dropped, and that a height
then uncovered. That panel draws `loading…` until the next refresh. On a
dashboard that states no refresh interval, the panel draws `loading…` for
ever.

## The question that remains

The work that remains saves the queries of each panel that did not appear. A
slow drag gains the most, because it costs the sum 1 + 2 + ... + P today.

A sixth design must answer two questions, and the first one is new:

1. Where does a person read that one panel of a screen is older than the rest?
   The answer must hold for a panel of 3 cells, which draws no axis. It must
   also hold for a stat, a gauge, a bar gauge and a table, which draw no axis
   at any size.
2. How does a set of queries that an area starts save a query at each step of
   a drag? A saving at the last step alone leaves the cost of a drag as it is.
