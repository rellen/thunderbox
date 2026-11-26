# Legend patterns

A pattern builds the name of each series. A data source answers a query with
frames. A frame is a table of columns, and a series is a column of numbers. A
person writes the pattern in the `legend` field of a target. The dashboard applies the pattern after the data
source answers, so a data source never reads a pattern.

The guide [Name each series](../how-to/name-each-series.md) writes a pattern for
each data source.

The example below is a part of a dashboard file, and it does not load alone. It
is a `[[panel.target]]` table of a panel, and it names each core of the machine:

```toml
[[panel.target]]
source = "machine"
expr = "cpu.cores"
legend = "core {{core}}"
```

## Form of a pattern

| part of the pattern | what it gives |
|---|---|
| `{{label}}` | The value of the label `label` of the column. |
| `{{__name__}}` | The name of the column. |
| any other text | The text itself. |

These rules apply to a pattern:

- The dashboard ignores the spaces around a name inside the brackets. The
  pattern `{{ job }}` gives the same name as `{{job}}`.
- The reserved word `{{__name__}}` reads the name of the column. It never reads
  a label of the name `__name__`.
- A label that the column does not hold gives an empty text. The pattern
  `a{{nope}}b` gives `ab`.
- The dashboard removes the spaces at the start and at the end of the name.
- A pattern that gives an empty name changes nothing. The panel then shows the
  name that the data source gave.
- A pattern names a column of numbers alone. A time column and a column of text
  keep the name that the data source gave.
- A pattern replaces a name that the data source suggests.
- The pattern reaches each column of numbers of each frame of its own target. It
  reaches no column of another target.
- A pattern holds no reference to a variable. The text `$job` in a pattern
  stays as it is.

## Cases that stay as text

A name between two pairs of brace brackets is the name of a label, and the name
of a label holds no brace bracket. Two kinds of pattern are therefore not
correct:

- a pattern that holds no second pair of brackets after a first pair;
- a pattern that holds a brace bracket inside the name of a label.

The dashboard keeps such a pattern as plain text, from that pair of brackets to
the end.

| pattern | the name that the panel shows |
|---|---|
| `cpu {{job` | `cpu {{job` |
| `{{{instance}}}` | `{{{instance}}}` |
| `{{a{{b}}}}` | `{{a{{b}}}}` |
| `cpu of {{job}} and {{{instance}}}` | `cpu of node and {{{instance}}}` |

The last row reads a column that holds the label `job` with the value `node`.

## Name with no pattern

For a target that states no pattern, a panel shows the first of these names
that applies:

1. the name that the data source suggests for the column;
2. the name of the column, and then its labels between two brace brackets, as
   `up{instance="web-1", job="node"}`;
3. the name of the column alone, when the column holds no label.

The labels of the second form stand in the alphabetical order of their names.

## Names and the labels of each data source

| kind | the name of a column | the labels |
|---|---|---|
| `sysinfo` | The name of the query. The query `cpu.cores` names each column after the position of its core: `0`, `1`, and so on. The query `processes` names its columns `pid`, `name`, `cpu`, and `MemB`. | `core` on the query `cpu.cores` alone. |
| `prometheus` | The name of the metric. A series with no name of a metric takes the text of the query. | Every label of the series, except `__name__`. |
| `testdata` | The text of the query, for every series of one answer. The query `processes` names its columns `pid`, `name`, `cpu`, and `MemB`. | `series`, which holds `0`, `1`, and so on. The query `processes` attaches no label. |

These patterns give each series a name of its own:

| kind | the query | the pattern | the names |
|---|---|---|---|
| `sysinfo` | `cpu.cores` | `core {{core}}` | `core 0`, `core 1` |
| `prometheus` | `sum by (instance) (rate(node_network_receive_bytes_total[5m]))` | `{{instance}}` | one name for each machine |
| `testdata` | `random_walk` | `series {{series}}` | `series 0`, `series 1` |

The pattern `{{__name__}}` gives one name to every series of an answer of
`testdata`. It does the same for a query of `prometheus` that drops the name of
the metric, as `sum by (instance) (up)` does.

## Where a name appears

| kind | the place of the name |
|---|---|
| `timeseries` | the legend |
| `stat` | the top row of each tile |
| `gauge` | the row above each meter |
| `bargauge` | the left part of each row |
| `table` | the header of each column |
| `top` | the header of each column |

A top panel reads the header after the pattern. The field `sort` names that
header.

## Pages that give the reason

- [Architecture decision record 0017](../../contributor/adr/0017-the-dashboard-applies-the-pattern-of-a-legend.md)
  states why the dashboard applies the pattern.
- [Architecture decision record 0012](../../contributor/adr/0012-the-five-duties-of-a-data-source.md)
  states the duties of a data source for the name and the labels of a column.
