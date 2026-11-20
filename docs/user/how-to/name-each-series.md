# Name each series

This guide gives each series of a panel a short name. A panel shows that name in
its legend, beside a bar, or above a number. The name comes from a pattern,
which reads the labels of the series. A label is a pair of a name and a value
that a data source attaches to a series.

## Write a pattern

1. Find the label that separates one series from another. The table below
   names that label for each data source.
2. Add the field `legend` to the target.
3. In that field, write the name of the label between two pairs of brace
   brackets:

   ```toml
   [[panel.target]]
   source = "machine"
   expr = "cpu.cores"
   legend = "core {{core}}"
   ```

4. Run the program with your dashboard file.

This target shows the names `core 0`, `core 1`, and one name for each other core
of the machine.

| data source | the query | the pattern |
|---|---|---|
| `sysinfo` | `cpu.cores` | `core {{core}}` |
| `testdata` | any signal, with the option `series` above 1 | `series {{series}}` |
| `prometheus` | a query that keeps the label `instance` | `{{instance}}` |

## Use the name of the column

1. Write the reserved word `{{__name__}}` in the pattern:

   ```toml
   [[panel.target]]
   source = "prom"
   expr = "up"
   legend = "{{__name__}} on {{job}}"
   ```

A data source returns each series as a column of numbers, and each column has
a name. The reserved word reads that name. A metric is a measurement that a
Prometheus server collects and names. The data source `prometheus` puts the
name of the metric in the name of the column. This target therefore shows
names such as `up on prometheus`.

A query such as `rate(x[5m])` or `sum by (instance) (up)` drops the name of the
metric. The column then takes the text of the query as its name, and
`{{__name__}}` gives one name to every series of that query. Write the name of a
label for such a query.

The data source `testdata` gives every column the text of the query as its name.
Write `{{series}}` for that data source.

## The pages for the facts and the reasons

- [Legend patterns](../reference/legend-patterns.md) states the whole form of a
  pattern.
- [The data sources](../reference/data-sources/README.md) states the labels of
  each data source.
