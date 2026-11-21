# Prometheus data source

The data source `prometheus` reads a Prometheus server. It answers each query
with a range query of the Prometheus HTTP API. The program holds this data
source when the build holds the feature `prometheus`, and that feature is on by
default.

The example below is a part of a dashboard file, and it does not load alone.
The file also needs a `title`.

```toml
[source.prom]
kind = "prometheus"
url = "http://127.0.0.1:9090"
min_step = "15s"
timeout = "30s"
```

The tutorial [Read a Prometheus server](../../tutorials/read-a-prometheus-server.md)
connects a dashboard to a server.

## Fields and the options

| field | type | default | what it states |
|---|---|---|---|
| `url` | text | `"http://127.0.0.1:9090"` | The address of the server. |
| `name` | text | `"Prometheus"` | The name in the description of this data source. The program shows this name on no screen today. |
| `min_step` | duration | `"1s"` | The smallest distance between two points. |
| `timeout` | duration | `"30s"` | The longest time that the data source waits for one answer. |

- Set `min_step` to the interval at which the server collects the metric. A
  smaller step gives a chart that holds more holes than values.
- The data source raises a `min_step` under 1 second to 1 second.
- The data source raises a `timeout` of `"0s"` to 1 millisecond.
- The data source also asks the server to stop a query after the `timeout`. It
  sends that time in milliseconds, in the form `30000ms`, and it sends 1
  millisecond or more.

The page [Time ranges](../time-ranges.md#form-of-a-duration) states the form
of a duration.

### Errors of the options

| fault | the text of the error |
|---|---|
| The table holds another option. | `a prometheus source has no option "NAME"; it takes url, name, min_step, and timeout` |
| An option holds a value that is not text. | `the NAME of a prometheus source must be a text value` |
| A duration has a form that the program cannot read. | `The time value "TEXT" is not correct.` and the reason |
| The address is not correct. | `the address "TEXT" of a prometheus source is not correct.` and the reason |
| The program cannot build a client for HTTP. | `thunderbox cannot build a client.` and the reason |

The program writes `The dashboard is not correct.` before each text of the
table, except before the text of a duration. The message of each of these
errors starts with the line
`Error: The program cannot make the data sources of the dashboard.`

## Address

- The address can hold a user name and a password, before the sign `@`. The
  data source then sends them to the server in a header of the kind `Basic`.
- No text of the data source writes the password. Each error and each message
  writes the user name, and no password.

### File of certificates on Linux

On Linux, the data source reads the certificate of each certificate authority
from the machine. A certificate authority is an organization that signs the
certificate of a server.

- The data source reads the certificates when the program makes the data
  source.
- The data source returns an error when the machine holds no such certificate.
  It does so for an address that starts with `http://` as well.
- That error ends with `No CA certificates were loaded from the system`.
- The environment variable `SSL_CERT_FILE` can name a file of such
  certificates. An environment variable is a value that the shell gives to each
  program that it starts.

The guide
[Give the Prometheus source a CA file](../../how-to/give-the-prometheus-source-a-ca-file.md)
repairs this error.
[Architecture decision record 0064](../../../contributor/adr/0064-plain-http-needs-no-ca-file.md)
records the question for an address of plain HTTP.

## Query

The `expr` field of a target holds a query in PromQL. The data source removes
the spaces at the start and at the end of that text. It sends the rest to the
server without a change.

- A query that holds no text gives the panel the error
  `The data source returned an error. the query holds no text`.
- The data source sends a range query, and the server returns a range of
  values for such a query.
- A server that does not answer, or that rejects the query, also gives the panel
  an error. That error starts with `The data source returned an error.`

### Step of the query

The step is the distance between two points. The data source selects the step
from the point limit of the panel, in these four steps:

1. It divides the width of the window by the point limit.
2. It raises the result to `min_step`, when the result is smaller.
3. It raises the result to the next round step of this list: 1, 2, 5, 10, 15,
   and 30 seconds; 1, 2, 5, 10, 15, and 30 minutes; 1, 2, 3, 6, and 12 hours;
   1, 2, 7, and 30 days.
4. It raises the step again until the number of points fits the point limit.
   The data source asks for 11000 points or fewer, because a server refuses a
   query of more points.

A step longer than 30 days is a whole number of 30 days. The data source moves
each end of the window to the point of the step at that end or before it. The
times of the points therefore stay the same while the window moves.

## Answer

A frame is a table of columns, and each column holds one value for each row.
The answer of a query holds one frame:

- The name of the frame is the text of the query.
- The first column holds one time for each point of the step, and its name is
  `time`.
- Each other column holds one series of the answer. The name of the column is
  the name of the metric.
- A series that holds no name of a metric takes the text of the query as its
  name. The query `sum by (instance) (up)` is an example.
- Each column carries every label of its series, except `__name__`.
- A point that holds no sample of the server holds a hole. A value that is not
  finite also becomes a hole.
- The data source attaches no unit and no range to a column. State the field
  `unit` on the panel.
- A query that matches no series returns no frame. The panel then shows
  `no data`.

The data source returns an error for an answer of another kind. The panel
writes `The data source returned an error.` before each text of this table.

| the answer of the server | the text of the error |
|---|---|
| a single value for each series | `the server returned a single value for each series, and not a range of values` |
| one number | `the server returned one number, and not a range of values` |

## A variable of this data source

A variable that states a `query` on this data source names one label in the
`expr` field. The data source returns every value of that label as the choices.

The example below is a part of a dashboard file, and it does not load alone.
The file also needs a `title`, and a table `[source.prom]` of the kind
`prometheus`.

```toml
[[variable]]
name = "instance"
label = "the machine"
query = { source = "prom", expr = "instance" }
```

A variable whose query holds no text gives the reason
`The data source returned an error. a variable of a prometheus source needs the name of a label`.
The status row writes that reason after the sentence about the variable.

## The longest wait

The data source states a longest wait of 5 seconds more than the `timeout`. The
client stops a request at the `timeout`, and the data source then reports an
error that names the fault. The 5 seconds give that error time to arrive.

## Pages that give the reason

- [Architecture decision record 0009](../../../contributor/adr/0009-the-data-source-reduces-the-points.md)
  states why the data source reduces the points.
- [Architecture decision record 0033](../../../contributor/adr/0033-no-text-writes-the-password-of-an-address.md)
  states why no text writes the password of an address.
- [Name each series](../../how-to/name-each-series.md) writes a pattern for the
  labels of this data source.
