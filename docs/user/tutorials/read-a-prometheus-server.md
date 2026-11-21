# Read a Prometheus server

In this tutorial, you connect a dashboard to a Prometheus server. The dashboard
shows the memory of the server, and the number of addresses that answer the
server. Then you make the time range wider, and you see older values that the
server keeps.

## What you need

This tutorial needs these things:

- the copy of thunderbox from [Watch this machine](watch-this-machine.md);
- a text editor;
- the program `curl`;
- a Prometheus server that answers at `http://127.0.0.1:9090`.

A Prometheus server collects measurements, and it gives each measurement a name.
Prometheus calls such a named measurement a metric. Start the server with the
configuration file that its release holds. That file tells the server to collect
its own metrics every 15 seconds. This tutorial reads those metrics, so it needs
no other program.

Do every step in the folder `thunderbox`.

## Step 1: Check that the server answers

1. Run this command:

   ```
   curl 'http://127.0.0.1:9090/api/v1/query?query=up'
   ```

The answer starts with `{"status":"success"`. When the command writes an
error, start the server, and run the command again.

## Step 2: Draw the memory of the server

1. Make a file with the name `server.toml` in the folder `thunderbox`.
2. Write this text into the file:

   ```toml
   title = "my server"
   refresh = "15s"

   [time]
   from = "now-30m"
   to = "now"

   [source.prom]
   kind = "prometheus"
   url = "http://127.0.0.1:9090"
   min_step = "15s"

   [[panel]]
   title = "memory of the server"
   kind = "timeseries"
   unit = "bytes"
   grid = { x = 0, y = 0, w = 16, h = 8 }

   [[panel.target]]
   source = "prom"
   expr = "process_resident_memory_bytes"
   ```

3. Save the file.
4. Run this command:

   ```
   cargo run -p thunderbox -- server.toml
   ```

A chart fills the left two thirds of the terminal. It draws the memory that the
server holds, for the last 30 minutes. The line of a server that started less
than 30 minutes ago fills the right part of the chart alone.

The file tells thunderbox these things:

- The field `url` gives the address of the server.
- The field `min_step` holds the smallest distance between two points of a
  chart. This file gives it the value `15s`, because the server collects each
  metric every 15 seconds.
- The field `refresh` makes thunderbox ask the server again every 15 seconds.
- The field `expr` holds a query in PromQL. The data source sends that text to
  the server without a change.

When the data source cannot reach the server, each panel shows an error in
place of its data. On Linux, the program can also stop before it opens the
dashboard, with the error
`No CA certificates were loaded from the system`. Follow
[Give the Prometheus source a CA file](../how-to/give-the-prometheus-source-a-ca-file.md)
when you see that error.

5. Press `q` to stop the program.

## Step 3: Count the addresses that answer

1. Add this text at the end of the file:

   ```toml
   [[panel]]
   title = "addresses that answer"
   kind = "stat"
   grid = { x = 16, y = 0, w = 8, h = 8 }

   [[panel.target]]
   source = "prom"
   expr = "up"
   ```

2. Save the file, and run the command of step 2 again.

A stat panel now stands to the right of the chart, and it shows the number `1`.
The server holds the metric `up` for each address that it collects from. The
value is 1 when that address answered the last collection. The server collects
from its own address alone, so the panel shows one number.

## Step 4: Make the time range wider

1. Press `-`.

The time range is now 1 hour wide, and the axis of time starts at `-60m`. The
chart shows the values that the server keeps for that hour.

2. Press `0`.

The time range returns to the 30 minutes that the file states.

3. Press `q` to stop the program.

## What you did

- You declared a Prometheus data source, and you gave it the address of a
  server.
- You asked the server for two metrics with PromQL.
- You made the time range wider, and the server answered with older values.

## Next step

- Run the dashboard `dashboards/prometheus.toml` of the copy. It reads the
  metrics of the node exporter, which is a program that reports the state of a
  Linux machine to Prometheus. Your server must collect from a node exporter
  before that dashboard shows data.
- Follow [Let a person select a value](../how-to/let-a-person-select-a-value.md)
  to select one machine of the server.
- Read [The data source prometheus](../reference/data-sources/prometheus.md) for
  each field and each option.
