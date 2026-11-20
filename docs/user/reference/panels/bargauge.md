# The bar gauge panel

The kind `bargauge` draws one row for each series. Each row holds the name of
the series, a bar, and the number. A reader compares the rows and sees the
largest series. The panel serves a large number of series. The
[gauge panel](gauge.md) serves a small number.

The example below is a part of a dashboard file, and it does not load alone.
The file also needs a `title`, and a table `[source.machine]` of the kind
`sysinfo`.

```toml
[[panel]]
title = "cores"
kind = "bargauge"
unit = "percent"
min = 0.0
max = 100.0
grid = { x = 8, y = 7, w = 8, h = 6 }

[[panel.threshold]]
value = 80.0
color = "critical"

[[panel.target]]
source = "machine"
expr = "cpu.cores"
legend = "core {{core}}"
```

## The data that the panel needs

A data source answers a query with frames. A frame is a table of columns, and
each column holds one value for each row.

- Each column of numbers of each frame is one series. The panel needs no time
  column.
- The panel reduces each series to one number. The field `reduce` names the
  step.
- The panel drops a series that holds no value that the step can use. It also
  drops a series whose number is not finite.
- The panel shows `no data` when no series remains.
- The panel keeps the order of the series that the frames give. A row therefore
  stays in its place when the data source answers again.

## What a row holds

A row holds these parts, from the left to the right:

1. The name of the series, and one space. The name takes a third of the row or
   less. Every name takes the width of the longest name, so every bar starts in
   the same column.
2. The bar. The bar fills one whole cell for each whole cell of its length. It
   then draws one part of a cell, in eighths of a cell. A light shade, `░`,
   fills the rest of the range.
3. One space, and the text of the number. Every number ends in the same column.

- The bar takes the colour of the threshold that the number reaches. A number
  that reaches no threshold takes the colour of its series.
- A number below the range gives an empty bar. A number above the range gives a
  full bar.
- The panel shows the name for one series as well.

## The range of a bar

The panel finds each end of the range with the first rule that applies:

1. The field `min` of the panel sets the smallest value, and the field `max`
   sets the largest value.
2. A data source can attach a range to a column. The panel then takes that
   range.
3. The panel takes the smallest and the largest number across every series in
   every other condition. It adds zero to that range.

## The width of the parts

- The bar keeps 3 cells or more.
- To keep those cells, the panel drops the name first. It drops the text of the
  number second.
- The panel never cuts the text of a number. A reader would read a cut number
  as a different number.
- The panel cuts a name that is too long. A full stop marks the cut.
- The panel measures each name in cells, and not in characters. A character of a
  Chinese script or of a Japanese script takes two cells.

## The rows that do not fit

- A panel can hold more series than rows. It then draws one bar on each row
  except the last row. The last row holds a note that counts the series that
  the panel drops.
- A panel of one row inside the border draws one bar, and it draws no note.

## The fields that the panel reads

| field | what the panel does with it |
|---|---|
| `unit` | The panel writes each number in this unit. |
| `decimals` | The panel writes each number with this number of decimal places. |
| `min` | The panel puts this value at the left end of each bar. |
| `max` | The panel puts this value at the right end of each bar. |
| `reduce` | The panel reduces each series to one number with this step. |
| `[[panel.threshold]]` | The panel colours each bar with the colour of the threshold that the number reaches. |
| `[[panel.target]]` | The panel shows one row for each series of the answer of each target. |

## The texts that the panel can draw

| text | condition |
|---|---|
| `loading…` | The data source has not answered yet. |
| the text of the error | The data source returned an error. |
| `no data` | No series holds a finite number. |
| `1 series does not fit.` or `N series do not fit.` | The panel holds more series than rows, and the sentence fits the width. |
| `+N` | The panel holds more series than rows, and the sentence does not fit the width. |

## The panel at a small size

- A panel under 3 cells wide or under 3 cells high draws nothing. The renderer,
  which is the part of thunderbox that draws the panels, drops it.
- A narrow panel drops the names first, and then the numbers. The bars stay.

## The pages that give the reason

- [Colour a value that reaches a bound](../../how-to/colour-a-value-that-reaches-a-bound.md)
  adds thresholds to a panel.
- [Name each series](../../how-to/name-each-series.md) writes the pattern of
  the `legend` field.
