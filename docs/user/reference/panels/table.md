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

## Place of the numbers

A column of numbers stands at the right of its column, header and cells. The
field `left` keeps chosen columns at the left. The field holds a list of
headers, and each header is the text that the panel shows. A header of a column
of a frame that has a name includes that name and the full stop, as in
`"cpu.usage"`. A name that matches no header has no effect.

```toml
[[panel]]
title = "the most recent rows for each core"
kind = "table"
left = ["core"]
grid = { x = 0, y = 13, w = 12, h = 6 }
```

A number in a column of the field `left` stands at the left, and the panel
still never cuts that number. The panel drops decimal places from a number that
does not fit, and it shows `—` when no shorter form fits.

## Width of the columns

A column is a fixed column or a flex column. A fixed column has a width in
cells. A flex column has no width. The width of a column never depends on the
text of a cell or of a header. The columns therefore keep their places when the
data refreshes.

A column gets its width from the first of these sources that states one:

1. The field `widths` of the panel, for the header that the panel shows.
2. The hint `width` of the data source for that column.

A column that no source names is a flex column. The data source `sysinfo`
states the widths of the process list. The page
[The data source sysinfo](../data-sources/sysinfo.md#list-of-the-processes)
lists them.

The field `widths` is a table from the text of a header to a number of cells. A
header of a column of a frame that has a name includes that name and the full
stop, as in `"cpu.usage"`. A name that matches no header has no effect. A width
must be 1 or more.

```toml
[[panel]]
title = "processes"
kind = "table"
widths = { pid = 6, cpu = 4 }
grid = { x = 0, y = 13, w = 12, h = 6 }
```

The panel gives the width in these steps:

- One empty cell stands between two columns.
- The columns need the widths of the fixed columns, 4 cells for each flex
  column, and one empty cell between each pair of columns. The panel draws every
  column when they fit.
- A fixed column takes exactly its width. The flex columns share the rest of the
  width inside the border in equal parts. The columns and the empty cells fill
  that width.
- A share can leave a few cells over. The flex columns from the left take one
  more cell each, until no cell remains.
- A panel that has no flex column leaves the spare cells empty at the right of
  the last column.
- A panel that has no fixed column gives each column an equal share.
- Otherwise the panel drops the columns at the right, fixed or flex. It draws
  the largest number of columns that leaves room for one empty cell and for the
  note `+N`. The flex columns among them share the rest of the width. The note
  stands at the right of the header row, and `N` is the number of columns that
  the panel dropped.
- A short column can hold empty cells, and a long text takes a share that is
  too small. The panel cuts a text that is too long. A full stop marks the cut.
- The panel cuts a header that is too long in the same way. A header that is
  wider than a fixed column loses its end.
- The panel never cuts a number. A number that does not fit its column loses
  decimal places, one at each step, until it fits. The number `12.3M` in a column
  of 4 cells shows `12M`. The panel never drops a digit of the whole part, a
  letter of the scale, a sign, or a unit. It shows `—` when the number with no
  decimal place still does not fit. The number `855MB` in a column of 4 cells
  shows `—`. An identifier never loses a decimal place. A number of 7 digits in
  a fixed column of 6 cells shows `—`.
- A panel that is too narrow for one column and its note draws the first
  column and no note. A fixed column takes exactly its width. A flex column
  takes the whole width.
- A panel that is too narrow for the first column draws the border and the
  title alone. A flex column needs 4 cells.

## Fields that the panel reads

| field | what the panel does with it |
|---|---|
| `unit` | The panel writes each number in this unit. |
| `decimals` | The panel writes each number with this number of decimal places. |
| `left` | The panel keeps the columns of these headers at the left. |
| `widths` | The panel gives the columns of these headers a fixed number of cells. |
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
| `+N` | The panel drops columns at the right. `N` is the number of columns that the panel dropped. |

## The panel at a small size

- A panel under 3 cells wide or under 3 cells high draws nothing. The renderer,
  which is the part of thunderbox that draws the panels, drops it.
- A shorter panel shows fewer rows. It keeps the most recent rows.
- A narrower panel drops the columns at the right first.

## Pages that give the reason

- [The top panel](top.md) puts the rows in order by one column, and it shows
  the largest values first.
- [Numbers and units](../numbers-and-units.md) lists the units.
