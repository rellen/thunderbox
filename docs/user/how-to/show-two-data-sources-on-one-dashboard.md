# Show two data sources on one dashboard

This guide declares two data sources in one dashboard file, and it shows the
data of both. The example reads this machine and a Prometheus server.

## Declare both data sources

1. Add one table `[source.NAME]` for each data source. Give each table a
   different name:

   ```toml
   [source.machine]
   kind = "sysinfo"

   [source.prom]
   kind = "prometheus"
   url = "http://127.0.0.1:9090"
   min_step = "15s"
   ```

2. In each target, write the name of a data source in the field `source`:

   ```toml
   [[panel]]
   title = "this machine"
   kind = "timeseries"
   unit = "percent"
   grid = { x = 0, y = 0, w = 12, h = 8 }

   [[panel.target]]
   source = "machine"
   expr = "cpu"

   [[panel]]
   title = "the server"
   kind = "timeseries"
   unit = "bytes"
   grid = { x = 12, y = 0, w = 12, h = 8 }

   [[panel.target]]
   source = "prom"
   expr = "process_resident_memory_bytes"
   ```

3. Run the program with your dashboard file.

The left panel reads this machine, and the right panel reads the server. A data
source that returns an error changes its own panels alone. Every other panel
continues to draw.

## Show both data sources in one panel

1. Declare a third data source, of the kind `testdata`:

   ```toml
   [source.test]
   kind = "testdata"
   ```

2. Give one panel a target for each data source:

   ```toml
   [[panel.target]]
   source = "machine"
   expr = "cpu"
   legend = "this machine"

   [[panel.target]]
   source = "test"
   expr = "sine"
   legend = "test data"
   ```

The panel draws the series of both targets. When the data source of one target
returns an error, the whole panel shows that error.

## Correct the name of a data source

The program stops before it draws when a target names a data source that the
file does not declare. This text is an example of the fault:

```
panel "this machine" refers to undeclared source "mchine"
```

1. Correct the field `source` of that target, or declare the data source.

## Pages for the facts and the reasons

- [The data sources](../reference/data-sources/README.md) states the table
  `[source.NAME]` and each kind of data source.
- [The dashboard file](../reference/dashboard-file.md) states the field `source`
  of a target.
