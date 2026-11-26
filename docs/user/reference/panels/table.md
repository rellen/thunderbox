# Table panel

The kind `table` draws every value of the answer, in rows and columns. The panel
reduces no column to one number.

The example below is a part of a dashboard file, and it does not load alone.
The file also needs a `title`, and a table `[source.machine]` of the kind
`sysinfo`.

```toml
[[panel]]
title = "the most recent rows for each core"
kind = "table"
unit = "percent"
decimals = 1
grid = { x = 0, y = 13, w = 12, h = 6 }

[[panel.target]]
source = "machine"
expr = "cpu.cores"
legend = "core {{core}}"
```

## Data that the panel needs

A data source answers a query with frames. A frame is a table of columns, and
each column holds one value for each row.

- The panel accepts a frame of any shape.
- The panel makes one column for each column of each frame, in the order of the
  frames.
- The header of a column is the name of the column, after the pattern of the
  `legend` field. The page [Legend patterns](../legend-patterns.md) states the
  rule of that name.
- Two frames or more can hold columns, and a frame can have a name. The header
  of each column of such a frame then starts with the name of the frame and a
  full stop, as `cpu.usage` does.
- The panel shows `no data` when every frame holds no row.

## Rows

- The header takes the first row inside the border. The data takes every other
  row.
- The panel shows the most recent rows that fit, and it drops the oldest rows.
  It writes no note for the rows that it drops.
- The panel keeps the order of the data, so the newest row stands at the
  bottom.
- Two frames can hold a different number of rows. The panel puts the last row
  of each frame at the bottom. The symbol `—` fills the cells above the rows of
  the shorter frame.

## Cells

| the type of the column | the text of a cell | the place of the text |
|---|---|---|
| time | The time of day as `HH:MM:SS`, on the clock of Coordinated Universal Time. | left |
| numbers | The number in the unit and with the decimal places of the panel. | right |
| numbers that are an identifier | Every digit of the number, with no letter of the scale and no unit. The unit, the decimal places, and the thresholds of the panel do not apply. | right |
| text | The text. Each control character becomes a space. | left |
| true or false | `true` or `false` | left |

- A cell shows `—` for a hole in the data, and for a value that is not finite.
- A cell shows `—` when its column holds no value at that row.
- A number takes the colour of the threshold that it reaches. An identifier
  takes no threshold. Every other value
  is grey, and the symbol `—` is dark grey.

## Width of the columns

- One empty cell stands between two columns.
- Each column takes the width of its widest cell, the header included, when
  every column fits.
- A narrow panel gives each column 4 cells, or fewer cells for a column that
  needs fewer. It then shares the rest of the width between the columns, in
  proportion to the cells that each column still needs.
- The panel drops the columns at the right that have no room. A note at the
  right of the header row counts them.
- The panel cuts a text that is too long. A full stop marks the cut.
- The panel never cuts a number. It shows `—` in the place of a number that does
  not fit its column.
- A panel that is too narrow for one column draws the border and the title
  alone.

## Fields that the panel reads

| field | what the panel does with it |
|---|---|
| `unit` | The panel writes each number in this unit. |
| `decimals` | The panel writes each number with this number of decimal places. |
| `[[panel.threshold]]` | The panel colours each number with the colour of the threshold that the number reaches. |
| `[[panel.target]]` | The panel shows the columns of the answer of each target. |

The panel ignores the fields `min`, `max`, and `reduce`.

## Texts that the panel can draw

| text | condition |
|---|---|
| `loading…` | The data source has not answered yet. |
| the text of the error | The data source returned an error. |
| `no data` | Every frame holds no row. |
| `—` | A cell holds a hole, a value that is not finite, or a number that does not fit. |
| `1 column does not fit.` or `N columns do not fit.` | The panel drops columns at the right, and the sentence fits the free cells of the header row. |
| `+N` | The panel drops columns at the right, and the sentence does not fit the free cells of the header row. |

## The panel at a small size

- A panel under 3 cells wide or under 3 cells high draws nothing. The renderer,
  which is the part of thunderbox that draws the panels, drops it.
- A shorter panel shows fewer rows. It keeps the most recent rows.
- A narrower panel drops the columns at the right first.

## Pages that give the reason

- [The top panel](top.md) puts the rows in order by one column, and it shows
  the largest values first.
- [Numbers and units](../numbers-and-units.md) lists the units.
