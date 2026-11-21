# Data sources

A data source answers the queries of the panels. A dashboard declares each data
source that it needs in a `[source.NAME]` table. The `kind` field of that table
selects the data source.

| kind | page | what the data source reads |
|---|---|---|
| `sysinfo` | [Sysinfo data source](sysinfo.md) | This machine: the processor, the memory, the network, and the processes. |
| `prometheus` | [Prometheus data source](prometheus.md) | A Prometheus server, with a range query. |
| `testdata` | [Testdata data source](testdata.md) | Values that this project makes, for a test and for a picture. |

## Table `[source.NAME]`

The example below is a part of a dashboard file, and it does not load alone.
The file also needs a `title`.

```toml
[source.prom]
kind = "prometheus"
url = "http://127.0.0.1:9090"
min_step = "15s"
```

| part | what it states |
|---|---|
| `NAME` | The name of the data source. A target and a variable name the data source in their `source` field. |
| `kind` | The kind of the data source. The file must state this field. |
| `url` | The address of the server. Only the data source `prometheus` reads this field. |
| any other field | An option of the data source. The page of each kind lists the options of that kind. |

- A data source returns an error for an option that it does not know.
- The data sources `sysinfo` and `testdata` ignore the field `url`.
- A dashboard can declare several data sources, of one kind or of several
  kinds. The guide
  [Show two data sources on one dashboard](../../how-to/show-two-data-sources-on-one-dashboard.md)
  uses this rule.

## Kinds that a build holds

The program holds `sysinfo` and `testdata` in every build. It holds `prometheus`
only when the build holds the feature `prometheus`. That feature is on by
default.

A table that names another kind gives this error. The program writes it under
the line `The program cannot make the data sources of the dashboard.`

```
The dashboard is not correct. the data source "NAME" has the type "KIND", and this program makes these types: sysinfo, testdata, prometheus
```

A build without the feature `prometheus` does not name `prometheus` in that
list.

## A comparison of the data sources

| fact | `sysinfo` | `prometheus` | `testdata` |
|---|---|---|---|
| the options | none | `name`, `min_step`, `timeout` | `seed`, `series` |
| the longest wait for one query | 60 seconds | 5 seconds more than the `timeout` | 60 seconds |
| the choices of a variable | the name of each query | each value of one label | none |
| the label of a series | `core`, on the signal `cpu.cores` alone | each label of the server, except `__name__` | `series` |
| the data source attaches a unit to each column of numbers | yes | no | yes |

### The point limit

Each panel asks for the points of its own width. The point limit is the width of
the panel in cells, times the sub-cells across one cell of the glyph. A sub-cell
is a part of a cell that the glyph can fill alone. Braille and octant hold 2
sub-cells across one cell, and half block and block hold 1.

Each data source of this project returns that number of points or fewer.

### The longest wait

A data source can state the longest time that one query takes. thunderbox waits
60 seconds for a data source that states no time. A query that takes longer
than that time gives its panel this error:

```
The data source "NAME" did not answer in 60 seconds.
```

thunderbox runs the panels of one set of queries at the same time. One data
source that does not answer therefore stops no other panel.

### Columns of one frame

Each column of one frame must hold the same number of rows. thunderbox checks
every column of each frame before a panel draws the answer. A frame of two
lengths holds columns with different numbers of rows, and it gives its panel
this error:

```
The data source "NAME" answered a frame with a column of SHORT rows and a column of LONG rows. Each column of one frame must hold the same number of rows.
```

Every type of panel draws that error, and the status row counts it. The three
data sources of this project never answer such a frame.

A program that gives frames to a panel directly, with no dashboard, meets no
such check. A chart then draws `no data` for a frame of two lengths.

### Choices of a variable

A variable that states a `query` takes its choices from a data source. Only a
data source that offers choices can answer that query. Every other data source
gives the variable this reason:

```
The data source "NAME" returns no choice for a variable.
```

The page [Variables](../variables.md#choices-that-a-data-source-holds)
states the rule.

## Pages that give the reason

- [How thunderbox works](../../explanation/how-thunderbox-works.md) explains a
  data source, a frame, and a panel.
- [Why each panel asks for its own points](../../explanation/why-each-panel-asks-for-its-own-points.md)
  explains the point limit.
- [Architecture decision record 0011](../../../contributor/adr/0011-add-a-data-source-as-code-behind-a-feature.md)
  states why a feature holds a data source.
- [Architecture decision record 0012](../../../contributor/adr/0012-the-five-duties-of-a-data-source.md)
  states the five duties of a data source.
- [Architecture decision record 0081](../../../contributor/adr/0081-the-runtime-returns-an-error-for-a-frame-of-two-lengths.md)
  states why a frame of two lengths gives its panel an error.
