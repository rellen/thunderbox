# Time series panel

The kind `timeseries` draws one line for each series, against time. The panel
fills the area below each line.

The example below is a part of a dashboard file, and it does not load alone.
The file also needs a `title`, and a table `[source.machine]` of the kind
`sysinfo`.

```toml
[[panel]]
title = "cpu"
kind = "timeseries"
unit = "percent"
grid = { x = 0, y = 0, w = 16, h = 8 }

[[panel.target]]
source = "machine"
expr = "cpu"
```

## Data that the panel needs

A data source answers a query with frames. A frame is a table of columns, and
each column holds one value for each row.

- A frame must hold one time column, and one column of numbers or more.
- Each column of one frame must hold the same number of rows. A frame of two
  lengths gives the panel an error. The page
  [Data sources](../data-sources/README.md#columns-of-one-frame) gives the
  text of that error.
- The panel ignores a frame that holds no time column, or no column of numbers.
  It reports no error for such a frame.
- Each column of numbers is one series.
- The panel draws no point for a hole in the data. It also draws no point for a
  value that is not finite. It draws no zero in the place of either.
- The line stops at each hole, and it starts again after the hole. The columns
  over a hole stay empty. A filled chart fills no column over a hole.
- A single value between two holes draws one point.
- A gap in time between two values is not a hole. The line joins those two
  values. A data source that holds no value for a moment sends a hole for it.

## What the panel draws

- The axis of time covers the whole window. A data source that covers part of
  the window fills part of the panel.
- The axis of time holds three labels. Each label states the distance from the
  present moment, as `-5m`, `-2m`, and `now` do. The page
  [Time ranges](../time-ranges.md#labels-of-the-axis-of-time) states the
  form.
- The axis of values holds three labels: the value at the bottom, the value in
  the middle, and the value at the top.
- Each series takes the next colour of a fixed list of colours. The page
  [The panels](README.md#colour-of-a-series) gives that list.
- The panel shows a legend in its top right corner only when each of these
  conditions holds:
  - the panel holds two series or more;
  - the panel is 30 cells wide or more;
  - the legend needs a quarter of the width of the chart or less, and a quarter
    of the height of the chart or less.
- The glyph of the chart follows the key `m`. The guide
  [Change the glyph of a chart](../../how-to/change-the-glyph-of-a-chart.md)
  describes each glyph.

## Ends of the axis of values

The panel finds each end of the axis with the first rule that applies:

1. The field `min` of the panel sets the bottom end, and the field `max` sets
   the top end.
2. A data source can attach a smallest value and a largest value to a column.
   The axis then reaches at least those values. A value past such a limit still
   widens the axis.
3. The data sets the end in every other condition. The axis adds 5 percent of
   the height of the data above the largest value. When the smallest value is
   above zero, the axis also adds 5 percent below that value, and it stops at
   zero.

The data source `sysinfo` attaches the range 0 to 100 to each share of use. A
chart of such a share therefore reaches 100.

## The point limit

The point limit of the panel is its width in cells, times the sub-cells across
one cell of the glyph. The width includes the border. A sub-cell is a part of a
cell that the glyph can fill alone. Braille and octant hold 2 sub-cells across
one cell. Half block and block hold 1. A panel of 80 cells that draws with
braille therefore asks for 160 points.

The page
[Why each panel asks for its own points](../../explanation/why-each-panel-asks-for-its-own-points.md)
explains the rule.

## Fields that the panel reads

| field | what the panel does with it |
|---|---|
| `unit` | The panel writes the labels of the axis of values in this unit. A panel that states no unit takes the unit of the first series. |
| `decimals` | The panel writes the labels of the axis of values with this number of decimal places. A panel that states no number takes the number of the first series. |
| `min` | The panel puts this value at the bottom of the axis of values. |
| `max` | The panel puts this value at the top of the axis of values. |
| `[[panel.target]]` | The panel draws the answer of each target. |

The panel ignores the field `reduce` and the thresholds.

The field `align` of the dashboard also applies to this panel. The page
[The dashboard file](../dashboard-file.md#field-align) states that rule.

## Texts that the panel can draw

| text | condition |
|---|---|
| `loading…` | The data source has not answered yet. |
| the text of the error | The data source returned an error. |
| `no data` | No frame holds a value that the panel can draw. A frame that holds no time column is one example. |

The legend names each series. The page
[Legend patterns](../legend-patterns.md) states the rule of a name.

## The panel at a small size

- A panel under 3 cells wide or under 3 cells high draws nothing. The renderer,
  which is the part of thunderbox that draws the panels, drops it.
- A panel under 30 cells wide shows no legend. A short panel also shows no
  legend, because the legend needs a quarter of the height of the chart or
  less.
- Under the rule `align = "column"`, the column of the labels takes a third of
  the width inside the border, or less.

## Pages that give the reason

- [The time range and the window](../../explanation/the-time-range-and-the-window.md)
  explains what a label of the axis of time states.
- [Architecture decision record 0036](../../../contributor/adr/0036-each-label-of-the-time-axis-states-its-distance-from-now.md)
  states why each label names its distance from the present moment.
- [Architecture decision record 0039](../../../contributor/adr/0039-a-stack-of-charts-can-share-one-width.md)
  states why a stack of charts can share one width.
- [Architecture decision record 0069](../../../contributor/adr/0069-a-chart-cuts-its-line-at-each-hole.md)
  states why the line stops at each hole.
