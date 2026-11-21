# Gauge panel

The kind `gauge` draws one meter for each series. A meter is a bar against a
range, and it shows how full a thing is. The panel serves a small number of
series. The [bar gauge panel](bargauge.md) serves a large number.

The example below is a part of a dashboard file, and it does not load alone.
The file also needs a `title`, and a table `[source.machine]` of the kind
`sysinfo`.

```toml
[[panel]]
title = "memory"
kind = "gauge"
unit = "percent"
min = 0.0
max = 100.0
grid = { x = 0, y = 7, w = 8, h = 6 }

[[panel.threshold]]
value = 90.0
color = "critical"

[[panel.target]]
source = "machine"
expr = "memory"
```

## Data that the panel needs

A data source answers a query with frames. A frame is a table of columns, and
each column holds one value for each row.

- Each column of numbers of each frame is one series. The panel needs no time
  column.
- The panel reduces each series to one number. The field `reduce` names the
  step.
- The panel drops a series that holds no value that the step can use. It also
  drops a series whose number is not finite.
- The panel shows `no data` when no series remains.

## What a meter holds

A meter holds these parts, from the top to the bottom:

1. The name of the series. Every meter shows its name when the panel draws two
   meters or more and the shortest meter holds 2 rows or more.
2. The bar. The bar takes every row that remains.
3. The range row. This row holds the smallest value of the range at the left,
   and the largest value at the right.

The panel makes one decision for every meter. All the meters therefore hold the
same parts.

### Bar

- A light shade, `░`, marks the whole range of the meter. The filled part of the
  bar covers that shade.
- The filled part takes the colour of the threshold that the number reaches. A
  number that reaches no threshold takes the colour of its series.
- The text of the number stands in the middle of the bar. The bar shows no text
  when the whole text does not fit the width.
- A number below the range gives an empty bar. A number above the range gives a
  full bar.

### Range row

- Every meter holds this row when the shortest meter holds 2 rows or more for
  the bar and the range row together.
- A meter leaves the row empty when the two numbers and one space do not fit
  its width.

## Range of a meter

The panel finds each end of the range with the first rule that applies:

1. The field `min` of the panel sets the smallest value, and the field `max`
   sets the largest value.
2. A data source can attach a range to a column. The panel then takes that
   range.
3. The panel takes the smallest and the largest number across every series in
   every other condition. It adds zero to that range.

The panel gives a width of 1 to a range that has no width. A panel that states
`min` and `max` keeps one range from one refresh to the next. A reader can then
compare two refreshes.

## Arrangement of the meters

- The panel puts the meters side by side when all of these conditions hold:
  - the panel holds 2 to 4 series;
  - the width inside the border is 12 cells or more for each series;
  - that width is 3 times the height inside the border, or more.
- One empty cell stands between two meters side by side.
- The panel puts the meters one above the other in every other condition. Each
  meter takes an equal share of the height.
- A panel can hold more series than rows. It then draws one meter on each row
  except the last row. The last row holds a note that counts the series that
  the panel drops.
- A panel of one row inside the border holds one meter and the short form of the
  note. The note stands at the right end of the row. A row that is too narrow
  holds the note alone.
- The panel cuts a name that is too long. A full stop marks the cut.

## Fields that the panel reads

| field | what the panel does with it |
|---|---|
| `unit` | The panel writes the number and the range row in this unit. |
| `decimals` | The panel writes the number and the range row with this number of decimal places. |
| `min` | The panel puts this value at the left end of each meter. |
| `max` | The panel puts this value at the right end of each meter. |
| `reduce` | The panel reduces each series to one number with this step. |
| `[[panel.threshold]]` | The panel colours the bar with the colour of the threshold that the number reaches. |
| `[[panel.target]]` | The panel shows one meter for each series of the answer of each target. |

## Texts that the panel can draw

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
- A meter that becomes shorter drops its range row first, and then its name.
  The bar keeps one row or more.
- A bar that is narrower than its number shows no number.

## Pages that give the reason

- [Colour a value that reaches a bound](../../how-to/colour-a-value-that-reaches-a-bound.md)
  adds thresholds to a panel.
- [Numbers and units](../numbers-and-units.md) lists the units and the steps of
  `reduce`.
