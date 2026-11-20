# The data source testdata

The data source `testdata` makes values of its own. It needs no server, and it
reads nothing of the machine. Most pictures of this project show the values of
this data source.

The example below is a part of a dashboard file, and it does not load alone.
The file also needs a `title`.

```toml
[source.test]
kind = "testdata"
series = 3
```

The data source has two properties:

- The data does not change between two runs. One window and one seed always
  give the same values.
- Each value of `random_walk`, `sine`, and `gappy` depends on its own moment
  alone. A wider panel therefore shows the same shape at a higher resolution.
  The query `ramp` reads the window as well, and the query `processes` reads
  the end of the window.

## The options

| option | type | default | what it states |
|---|---|---|---|
| `seed` | whole number, 0 or more | `195939070` | A number that selects one set of signals. Two data sources that have a different seed make a different signal. |
| `series` | whole number, 0 or more | `1` | The number of series that one query returns. The data source raises 0 to 1. |

- The data source ignores the field `url`.
- Any other field gives the error
  `The dashboard is not correct. a testdata source has no option "NAME"; it takes seed and series`.
- A value that is not a whole number of 0 or more gives the error
  `The dashboard is not correct. the NAME of a testdata source must be a whole number that is 0 or more`.
- The program writes each of these errors under the line
  `The program cannot make the data sources of the dashboard.`

## The queries

The `expr` field of a target holds the name of one query. The data source
ignores the spaces around the name.

| query | what it returns |
|---|---|
| `random_walk` | A signal that moves slowly between 0 and 100. Each series moves in its own way. |
| `sine` | A smooth wave between 5 and 95. One period lasts 30 seconds. Each series starts at a different point of the wave. |
| `ramp` | A straight rise from 0 at the start of the window to 100 at the end of the window. |
| `gappy` | A signal that moves slowly between 0 and 100, with a hole of 10 seconds in each minute. The hole covers the seconds from 30 to 40 of each minute of the clock. |
| `processes` | A list of 12 invented processes. The section below states the list. |

A query of another name gives this error in the panel:

```
The data source returned an error. The data source testdata knows no query "NAME". The known queries are random_walk, sine, ramp, gappy, and processes.
```

## The answer of a signal

A frame is a table of columns, and each column holds one value for each row.
Each query except `processes` is a signal. The answer of a signal holds one
frame:

- The name of the frame is the text of the query.
- The first column holds the time of each point, and its name is `time`.
- The frame holds one column for each series. The `series` option sets the
  number of series.
- The name of each such column is the text of the query. Every series of one
  answer therefore carries the same name.
- Each such column carries the label `series`. The label holds `0` for the first
  series, `1` for the second series, and so on.
- Each such column carries the unit `percent` and the range 0 to 100.
- The answer holds exactly the point limit of the panel. The points stand at an
  even distance apart, from the start of the window.

Write `{{series}}` in the `legend` field to give each series a different name.
The page [Legend patterns](../legend-patterns.md) states the rule.

## The list of the processes

The query `processes` returns one frame, named `processes`. The frame has the
shape of the list of the data source `sysinfo`, so a top panel can draw it.

| column | what it holds | the type of the column | unit |
|---|---|---|---|
| `pid` | An invented number of the process. | text | none |
| `name` | The name of the process. | text | none |
| `cpu` | A share of the processor that moves slowly. | numbers | `percent` |
| `memory` | An amount of memory that moves slowly. | numbers | `bytes` |

The list holds these processes, in this order:

| `name` | `pid` | the largest share of the processor | the mean memory |
|---|---|---|---|
| `init` | `1` | 2 | 12 million bytes |
| `shell` | `212` | 4 | 6 million bytes |
| `editor` | `423` | 25 | 180 million bytes |
| `compiler` | `634` | 190 | 900 million bytes |
| `browser` | `845` | 60 | 2100 million bytes |
| `database` | `1056` | 45 | 1400 million bytes |
| `server` | `1267` | 35 | 350 million bytes |
| `indexer` | `1478` | 80 | 260 million bytes |
| `backup` | `1689` | 30 | 120 million bytes |
| `logger` | `1900` | 6 | 40 million bytes |
| `player` | `2111` | 15 | 210 million bytes |
| `terminal` | `2322` | 8 | 90 million bytes |

- A share of 100 is one whole core. The process `compiler` therefore reaches
  almost two cores.
- The memory of each process moves between 0.8 and 1.2 times its mean.
- Each value depends on the seed and on the end of the window alone. A window
  in the past therefore shows the list of that moment.

## A variable of this data source

The data source offers no choice to a variable. A variable that states a
`query` on this data source holds no choice. The status row then writes this
sentence:

```
The variable "NAME" has no choice. The data source "SOURCE" returns no choice for a variable.
```

Give such a variable fixed `options` instead. The dashboard
`dashboards/variables.toml` holds an example.

## The longest wait

The data source states no longest wait. thunderbox therefore waits 60 seconds.

## The pages that give the reason

- [Write your first dashboard](../../tutorials/write-your-first-dashboard.md)
  reads this data source.
- [How thunderbox works](../../explanation/how-thunderbox-works.md) explains a
  frame.
