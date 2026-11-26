# Top panel

The kind `top` draws a list of rows in order by one column, with the largest
value first. The program `top` shows the processes of a machine in this form.
The panel knows nothing of a process. A row can describe a process, a disk, or
any other thing.

The example below is a part of a dashboard file, and it does not load alone.
The file also needs a `title`, and a table `[source.machine]` of the kind
`sysinfo`.

```toml
[[panel]]
title = "processes"
kind = "top"
sort = "cpu"
grid = { x = 12, y = 13, w = 12, h = 6 }

[[panel.threshold]]
value = 85.0
color = "critical"

[[panel.target]]
source = "machine"
expr = "processes"
```

The guide
[Show the busiest processes](../../how-to/show-the-busiest-processes.md) adds
this panel to a dashboard.

## Data that the panel needs

A data source answers a query with frames. A frame is a table of columns, and
each column holds one value for each row.

- Each row of a frame describes one thing. The panel needs no time column.
- The panel needs one column of numbers or more to put the rows in order.
- The panel puts the rows of every frame into one list.
- The header of a column is the name of the column, after the pattern of the
  `legend` field. A column of one frame joins the column of another frame that
  shows the same header.
- A frame that holds no column for a header shows `—` in that column.
- The panel shows `no data` when every frame holds no row.

## Order of the rows

- The field `sort` names the column that gives the order. The name is the text
  of the header of that column.
- A panel that states no `sort` field takes its first column of numbers that is
  not an identifier. An identifier cannot give the order, because it names a
  thing and does not measure it. The column `pid` of the data source `sysinfo`
  is an identifier.
- The row that holds the largest number comes first.
- Two rows that hold the same number keep the order of the data. Such rows
  therefore stay in their places on each refresh.
- A row that holds no number in that column comes last.
- A line under the header marks the column that gives the order.
- The panel shows the first rows that fit. A note at the right of the bottom
  border counts the rows that do not fit.

The panel shows an error in place of the rows when no column of numbers shows
the header that `sort` names. The error reads
`No column of numbers shows the header "NAME".` A `sort` field that names an
identifier returns the same error.

The header holds the name after the pattern of the `legend` field. The field
`sort` names that header. The page [Legend patterns](../legend-patterns.md)
states the pattern.

## Configuration of the columns

- The fields `unit` and `decimals` and the thresholds of the panel apply to the
  column that gives the order. They apply to no other column.
- Every other column keeps the configuration that its data source states. A
  list of processes holds a share of the processor beside a count of bytes, and
  one unit cannot describe both.

## Cells and the columns

The panel shares the layout of the [table panel](table.md):

- The panel writes each cell in the same form as the table panel.
- A number stands at the right of its column, and so does an identifier. Every
  other value stands at the left.
- The field `left` keeps chosen columns at the left, header and cells, even
  when they hold numbers. The field holds a list of headers, as in
  `left = ["pid"]`. A name that matches no header has no effect. The panel
  still never cuts a number in such a column.
- A cell shows `—` for a hole in the data, and for a value that is not finite.
- Each column takes an equal share of the width inside the border, as in the
  table panel. The text of a cell never changes the width of a column. The
  section "Width of the columns" of the [table panel](table.md) states the
  rule.
- The example below keeps the column `pid` at the left. The example is a part
  of a dashboard file, and it does not load alone.

  ```toml
  [[panel]]
  title = "processes"
  kind = "top"
  sort = "cpu"
  left = ["pid"]
  grid = { x = 12, y = 13, w = 12, h = 6 }

  [[panel.target]]
  source = "machine"
  expr = "processes"
  ```

- The panel drops the columns at the right that have no room. The note `+N` at
  the right of the header row counts them.
- The panel can drop the column that gives the order in a narrow panel.
  [Architecture decision record 0063](../../../contributor/adr/0063-a-narrow-top-panel-drops-the-sort-column.md)
  records that fault.

## Fields that the panel reads

| field | what the panel does with it |
|---|---|
| `sort` | The panel puts the rows in order by the column of this header. |
| `left` | The panel keeps the columns of these headers at the left. |
| `unit` | The panel writes the column that gives the order in this unit. |
| `decimals` | The panel writes the column that gives the order with this number of decimal places. |
| `[[panel.threshold]]` | The panel colours each number of the column that gives the order with the colour of the threshold that it reaches. |
| `[[panel.target]]` | The panel puts the rows of the answer of each target into the list. |

The panel ignores the fields `min`, `max`, and `reduce`. The program rejects
the field `sort` on a panel of every other kind. It rejects the field `left` on
a panel of every kind except `table` and `top`.

## Texts that the panel can draw

| text | condition |
|---|---|
| `loading…` | The data source has not answered yet. |
| the text of the error | The data source returned an error. |
| `No column of numbers shows the header "NAME".` | No column of numbers shows the header that `sort` names. |
| `no data` | Every frame holds no row. |
| `—` | A cell holds a hole, a value that is not finite, or a number that does not fit. |
| `1 row does not fit.` or `N rows do not fit.` | The panel shows fewer rows than the list holds, and the sentence fits the bottom border. |
| `+N` in the header row | The panel drops columns at the right. `N` is the number of columns that the panel dropped. |
| `+N` in the bottom border | The sentence about the rows does not fit. |

The panel writes the text `No column of numbers shows the header "NAME".` in
red, as it writes every error.

## The panel at a small size

- A panel under 3 cells wide or under 3 cells high draws nothing. The renderer,
  which is the part of thunderbox that draws the panels, drops it.
- A shorter panel shows fewer rows. It keeps the rows with the largest numbers.
- A narrower panel drops the columns at the right first.
- A panel that is too narrow for one column and its note draws the first
  column and no note. That column takes the whole width.

## Pages that give the reason

- [Architecture decision record 0046](../../../contributor/adr/0046-a-top-list-is-a-kind-of-panel-of-its-own.md)
  states why this list is a kind of panel of its own.
- [The data source sysinfo](../data-sources/sysinfo.md#list-of-the-processes)
  states the columns of the query `processes`.
