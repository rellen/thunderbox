# ADR 0007: The settings of a panel win over the configuration of a column

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 5 |
| Code | `crates/thunderbox-widgets/src/panel.rs`, `crates/thunderbox-widgets/src/table.rs`, `crates/thunderbox-widgets/src/top.rs`, `crates/thunderbox-widgets/src/row.rs` |
| Tests | `the_panel_wins_over_the_column_configuration`, `color_for_reads_the_thresholds_of_the_panel_first`, `the_panel_reads_the_unit_of_the_panel_and_not_of_the_column`, `the_settings_of_the_panel_describe_the_column_that_gives_the_order`, `a_threshold_of_the_panel_colours_the_column_that_gives_the_order_alone` |
| Related | [ADR 0005](0005-one-trait-is-the-seam-of-every-data-source.md), [ADR 0008](0008-draw-each-chart-through-a-wrapper-of-the-project.md), [ADR 0046](0046-a-top-list-is-a-kind-of-panel-of-its-own.md), [ADR 0050](0050-a-row-panel-holds-panels-by-their-place.md), [ADR 0051](0051-a-row-panel-takes-one-line.md) |

## Context

The parts that ratatui supplies do most of the work of a panel:

- The time series panel draws with `Chart`, `GraphType::Area`, and
  `Dataset::fill_to_y`.
- The stat panel draws its small chart with `Sparkline`.
- The gauge panel draws its bar with `Gauge`.
- The table panel draws with `Table`.

No panel uses `BarChart`, `LineGauge`, or `Canvas` today.

A marker is the set of characters that a chart draws a point with. A person
selects the marker. A terminal cell holds one character, and some characters
divide the cell into smaller parts. This project calls each such part a
sub-cell. The marker `braille` is the default, and it gives 2 sub-cells across
and 4 down. The marker `octant` gives the same sub-cells with no space between
two cells, and fewer fonts hold its characters. The marker `halfblock` gives 1
sub-cell across and 2 down. The marker `block` gives one sub-cell, for a plain
terminal or for a font that lacks the other characters.

A `Dataset` holds one `Style` for a whole series, so a point cannot have its
own colour. A gradient would colour each point by its height, from the bottom
of the chart to the top. Two methods can give that result:

- a chart draws on a `Canvas`, with one batch of points for each band of
  colour;
- a panel changes the colour of each cell in the `Buffer` after the chart
  draws.

The project has not done that work yet.

`thunderbox-widgets` holds one renderer for each type of panel. The project has
six types of panel that show data:

| type | what it draws | the question that it answers |
|---|---|---|
| `TimeSeriesPanel` | a chart of values against time | How did this value move? |
| `StatPanel` | one large number for each series | What is the value now? |
| `GaugePanel` | a meter, with the range written out | How full is it? |
| `BarGaugePanel` | one named bar for each series | Which of these is the largest? |
| `TablePanel` | the columns of the frames | What are the numbers? |
| `TopPanel` | the rows of the frames, in order by one column | Which of these uses the most? |

Three of the six show one number for each series. A `Reduce` step turns a
column of numbers into that one number. The step lives in `thunderbox-core`,
because a dashboard names it. `thunderbox-transform` will hold the rest of the
pipeline that [ADR 0005](0005-one-trait-is-the-seam-of-every-data-source.md)
describes.

A data source and a panel each state how to show a number. A data source fills
the `FieldConfig` of each column, and a person writes the panel.

## Decision

Each of these six renderers takes the same three arguments, which are a title,
the data, and a theme. Each of them also takes a `PanelOptions` value through
its method `options`. That value carries these settings:

- the name of the unit;
- the number of decimal places;
- the range of a meter, as a smallest value and a largest value;
- the step that reduces a column of numbers to one number;
- the thresholds.

**The panel wins.** Where the settings of the panel and the `FieldConfig` of a
column disagree, the panel applies its own settings. A dashboard can therefore
override the unit that a data source states. The sysinfo data source states a
unit for each column, and the Prometheus data source states none. The
`PanelOptions` type holds that rule, so six renderers apply it in one way.

**A top panel applies its settings to the column that gives the order.** A list
of processes holds a share of the processor beside a count of bytes. A unit of
the panel that reached every column would write the count of bytes as a share.
A threshold of 85 would also colour every count of bytes, because each count
passes 85. The settings of a top panel therefore describe the column that the
field `sort` names. Every other column keeps the settings that its data source
states. The table panel applies its settings to every column, because a table
of one signal holds one unit.

**The shared parts sit in one module.** `crates/thunderbox-widgets/src/panel.rs` holds
the border, the three states that a panel reports, the reduce step, and the
step that cuts a name to a width.

**The table panel and the top panel share a second module.**
`crates/thunderbox-widgets/src/table.rs` holds the layout of a sheet of rows and
columns, and its function `draw_sheet` draws a sheet for both panels. The two
panels differ in the rows that they select. The top panel also draws a line
under one header, and a note on its bottom border.

**A row panel draws no data.** `RowPanel` draws the line of a row panel, and it
takes a title and a theme alone. A row panel holds the panels below it on the
grid, and a person collapses it to hide them.
[ADR 0050](0050-a-row-panel-holds-panels-by-their-place.md) and
[ADR 0051](0051-a-row-panel-takes-one-line.md) record its rules. The table of
the types above therefore omits it. Each count of the types of panel in the
documents of this project omits it as well.

## Consequences

A renderer that repeated a shared part would differ from the others after a
later change. A new renderer therefore calls the parts of `panel.rs`, and it
copies none of them.

A change to the rule of a top panel changes `top.rs` alone. A change to the
rule "the panel wins" changes `PanelOptions`, and every renderer then follows
it.
