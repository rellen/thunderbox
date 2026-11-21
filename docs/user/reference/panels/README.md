# Panels

A panel draws the answer of its queries inside a border. The `kind` field of a
`[[panel]]` table selects the panel. The page
[The dashboard file](../dashboard-file.md#table-panel) lists the fields of
that table.

| kind | page | what the panel draws |
|---|---|---|
| `timeseries` | [Time series panel](timeseries.md) | One line for each series, against time. |
| `stat` | [Stat panel](stat.md) | One large number for each series. |
| `gauge` | [Gauge panel](gauge.md) | One meter for each series, against a range. |
| `bargauge` | [Bar gauge panel](bargauge.md) | One bar for each series, against a range. |
| `table` | [Table panel](table.md) | Every value of the answer, in rows and columns. |
| `top` | [Top panel](top.md) | A list of rows in order by one column, with the largest value first. |
| `row` | [Row panel](row.md) | A line across the whole grid that holds the panels below it. |

## Fields that each kind reads

A cross shows that the kind reads the field. Every kind reads `title`, `kind`,
and `grid`.

| field | `timeseries` | `stat` | `gauge` | `bargauge` | `table` | `top` | `row` |
|---|---|---|---|---|---|---|---|
| `unit` | x | x | x | x | x | x | |
| `decimals` | x | x | x | x | x | x | |
| `min` | x | x | x | x | | | |
| `max` | x | x | x | x | | | |
| `reduce` | | x | x | x | | | |
| `sort` | | | | | | x | |
| `collapsed` | | | | | | | x |
| `[[panel.threshold]]` | | x | x | x | x | x | |
| `[[panel.target]]` | x | x | x | x | x | x | |

## Rules that every panel shares

The rules in this section apply to every kind except `row`. The page
[The row panel](row.md) states the rules of that kind.

### Border and the size

- Each panel draws a border with round corners. The title stands in the top
  edge of the border.
- The border of the selected panel is cyan. The border of every other panel is
  dark grey.
- A panel needs a place of 3 cells across and 3 cells down, or more. A smaller
  panel draws nothing.
- This documentation calls such a panel a panel that the renderer drops. The
  renderer is the part of thunderbox that draws the panels. The status row
  counts such a panel, and thunderbox asks no data source about it.

### Three states that hold no data

| text | colour | condition |
|---|---|---|
| `loading…` | dark grey | The data source has not answered yet. |
| the text of the error | red | The data source returned an error. |
| `no data` | dark grey | The answer holds no value that the panel can draw. |

- The panel draws each text on one line, in the middle of the panel. A text
  that is wider than the panel loses a part at each end.
- A panel that holds no target shows `no data`.
- A new time range returns every panel to `loading…`. A new value of a variable
  does the same.

### Configuration of a column

A data source can attach a unit, decimal places, a range, and thresholds to a
column. The field of the panel replaces the value of the column for each of
these. A panel that states no such field uses the value that the column states.

A panel replaces the whole list of the thresholds of a column when it states
one threshold or more.

### Colour of a series

Each series takes a colour from this list, in order: cyan, magenta, green,
yellow, blue, and light red. The seventh series takes cyan again. A threshold
that a value reaches gives that value the colour of the threshold. The page
[Numbers and units](../numbers-and-units.md#thresholds) states that rule.

### Name of a series

A panel names each series with the name that the `legend` field of its target
builds. For a target that states no pattern, the panel shows the name that the
data source gives. The page [Legend patterns](../legend-patterns.md) states both
rules.

### Short labels

A panel that cannot show every part writes a complete sentence that counts the
parts that it does not show. It writes a short form only when the sentence does
not fit.

| kinds | the complete sentence | the short form |
|---|---|---|
| `stat`, `gauge`, `bargauge` | `1 series does not fit.` or `N series do not fit.` | `+N` |
| `table`, `top` | `1 column does not fit.` or `N columns do not fit.` | `+N` |
| `top` | `1 row does not fit.` or `N rows do not fit.` | `+N` |
| `row` | `The row hides 1 panel.` or `The row hides N panels.` | `+N` |

A panel cuts the short form when the free cells hold less. A full stop then
marks the cut.

The symbol `—` stands in the place of a number that a panel cannot show. The
page [Numbers and units](../numbers-and-units.md#symbol-for-a-value-that-a-panel-cannot-show)
lists where each panel draws it.

## Pages that give the reason

- [Panels that the terminal cannot hold](../../explanation/panels-that-the-terminal-cannot-hold.md)
  explains the panel that the renderer drops.
- [Read a dashboard on a small terminal](../../how-to/read-a-dashboard-on-a-small-terminal.md)
  states what a person can do.
- [Architecture decision record 0007](../../../contributor/adr/0007-the-settings-of-a-panel-win-over-the-column.md)
  states why the field of a panel replaces the value of the column.
