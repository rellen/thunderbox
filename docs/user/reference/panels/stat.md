# The stat panel

The kind `stat` draws one large number for each series. Each number stands in a
tile of its own.

The example below is a part of a dashboard file, and it does not load alone.
The file also needs a `title`, and a table `[source.machine]` of the kind
`sysinfo`.

```toml
[[panel]]
title = "cpu now"
kind = "stat"
unit = "percent"
decimals = 0
grid = { x = 16, y = 0, w = 8, h = 7 }

[[panel.threshold]]
value = 85.0
color = "critical"

[[panel.target]]
source = "machine"
expr = "cpu"
```

## The data that the panel needs

A data source answers a query with frames. A frame is a table of columns, and
each column holds one value for each row.

- Each column of numbers of each frame is one series. The panel needs no time
  column.
- The panel reduces each series to one number. The field `reduce` names the
  step.
- The panel drops a series that holds no value that the step can use.
- The panel shows `no data` when no series remains.

## What a tile holds

A tile holds these parts, from the top to the bottom:

1. The name of the series. The tile shows the name only when the panel holds
   two series or more. The tile must also hold 2 rows or more.
2. The number. The panel draws the number in large block characters when the
   tile has room for them. It draws plain text in every other tile.
3. A small chart of the series. The tile draws the chart only when 3 rows or
   more remain below the number. The chart takes 3 rows or 4 rows.

The colour of the number and of the small chart is the colour of the threshold
that the number reaches. A number that reaches no threshold takes the colour of
its series.

### The large number

- Each large character takes 3 cells across and 3 rows. One empty column stands
  between two large characters.
- The large form covers the digits, the decimal point, and the minus sign.
- The panel writes the unit and the letter of the scale as plain text. That
  text stands on the bottom row, beside the large characters.
- The panel writes plain text for a number that starts with another character.
  The symbol `—` is an example.

### The small chart

- The chart draws one bar for each point of the series. The most recent point
  stands at the right edge.
- The chart draws the most recent points that fit its width.
- The chart draws a light shade, `░`, in the place of a hole in the data.
- The glyph `block` gives each bar 3 levels. Every other glyph gives each bar 9
  levels.
- The fields `min` and `max` of the panel set the range of the chart. A panel
  that states neither takes the range of the column.
- When the column states no range either, the chart takes the smallest and the
  largest value of the series. It adds zero to that range.

## The arrangement of the tiles

- The panel puts as many tiles across as fit at 8 cells each, and as many tiles
  down as fit at 2 rows each. A panel of one tile gives that tile the whole
  space inside the border.
- One empty column stands between two tiles across.
- Every tile follows the plan of the smallest tile. Each tile therefore draws
  the same parts.
- The panel cuts a name that is too long. A full stop marks the cut.
- The panel can hold too few tiles for every series. The last row then holds a
  note that counts the series that the panel drops. The note needs a panel of 2
  rows or more inside the border.

## The fields that the panel reads

| field | what the panel does with it |
|---|---|
| `unit` | The panel writes each number in this unit. |
| `decimals` | The panel writes each number with this number of decimal places. |
| `min` | The panel puts this value at the bottom of the small chart. |
| `max` | The panel puts this value at the top of the small chart. |
| `reduce` | The panel reduces each series to one number with this step. |
| `[[panel.threshold]]` | The panel colours each number and its small chart with the colour of the threshold that the number reaches. |
| `[[panel.target]]` | The panel shows one tile for each series of the answer of each target. |

## The texts that the panel can draw

| text | condition |
|---|---|
| `loading…` | The data source has not answered yet. |
| the text of the error | The data source returned an error. |
| `no data` | No series holds a value that the step can use. |
| `—` | The number is not finite. The sum of two very large numbers is an example. |
| `1 series does not fit.` or `N series do not fit.` | The panel holds too few tiles for every series, and the sentence fits the width. |
| `+N` | The panel holds too few tiles for every series, and the sentence does not fit the width. |

## The panel at a small size

- A panel under 3 cells wide or under 3 cells high draws nothing. The renderer,
  which is the part of thunderbox that draws the panels, drops it.
- A tile that becomes smaller drops the small chart first.
- A tile that has no room for the large characters then writes the number as
  plain text.
- A tile of one row shows the number and no name. The number is the last part
  that a tile keeps.

## The pages that give the reason

- [Colour a value that reaches a bound](../../how-to/colour-a-value-that-reaches-a-bound.md)
  adds thresholds to a panel.
- [Numbers and units](../numbers-and-units.md) lists the units and the steps of
  `reduce`.
