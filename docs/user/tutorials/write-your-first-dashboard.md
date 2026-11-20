# Write your first dashboard

In this tutorial, you write a dashboard file, and you show it with thunderbox.
You start with one chart, and each step adds one part to the file. The
dashboard reads the test data of this project, so you need no server.

## What you need

This tutorial needs these things:

- the copy of thunderbox from [Watch this machine](watch-this-machine.md);
- a text editor.

Do every step in the folder `thunderbox`.

## Step 1: Draw one chart

1. Make a file with the name `first.toml` in the folder `thunderbox`.
2. Write this text into the file:

   ```toml
   title = "my first dashboard"
   refresh = "1s"

   [time]
   from = "now-1m"
   to = "now"

   [source.test]
   kind = "testdata"

   [[panel]]
   title = "wave"
   kind = "timeseries"
   unit = "percent"
   grid = { x = 0, y = 0, w = 24, h = 8 }

   [[panel.target]]
   source = "test"
   expr = "sine"
   ```

3. Save the file.
4. Run this command:

   ```
   cargo run -p thunderbox -- first.toml
   ```

One chart fills the terminal. It draws a wave that rises and falls once in each
30 seconds, and the wave moves to the left while you watch.

The file tells thunderbox these things:

- The table `[time]` makes each panel show the last minute.
- The table `[source.test]` declares a data source with the name `test`. Its
  kind is `testdata`, which is the test data of this project.
- The table `[[panel]]` declares a chart. The field `grid` gives the chart its
  place. The grid is 24 columns wide, so this chart uses the whole width.
- The table `[[panel.target]]` asks the data source `test` for the signal
  `sine`.

5. Press `q` to stop the program.

## Step 2: Put a number beside the chart

1. In the first panel, change `w = 24` to `w = 16`.
2. Add this text at the end of the file:

   ```toml
   [[panel]]
   title = "now"
   kind = "stat"
   unit = "percent"
   grid = { x = 16, y = 0, w = 8, h = 8 }

   [[panel.target]]
   source = "test"
   expr = "sine"
   ```

3. Save the file, and run the command of step 1 again.

The chart now uses two thirds of the width. A stat panel uses the last third,
and it shows the most recent value of the wave as one large number.

4. Press `q` to stop the program.

## Step 3: Colour the number

1. Add this text at the end of the file:

   ```toml
   [[panel.threshold]]
   value = 50.0
   color = "warning"

   [[panel.threshold]]
   value = 80.0
   color = "critical"
   ```

2. Save the file, and run the command of step 1 again.
3. Watch the number for 30 seconds.

The number is yellow at 50 or more, and it is red at 80 or more. Below 50, the
number keeps the colour of its series.

A `[[panel.threshold]]` table belongs to the `[[panel]]` table above it. These
two thresholds therefore belong to the stat panel.

4. Press `q` to stop the program.

## Step 4: Add a second row

1. Add this text at the end of the file:

   ```toml
   [[panel]]
   title = "walk"
   kind = "gauge"
   unit = "percent"
   min = 0.0
   max = 100.0
   grid = { x = 0, y = 8, w = 24, h = 4 }

   [[panel.target]]
   source = "test"
   expr = "random_walk"
   ```

2. Save the file, and run the command of step 1 again.

A meter now stands below the two panels, across the whole width. The two panels
above start at row 0 of the grid and hold 8 rows, so the meter starts at row 8.
The signal `random_walk` rises and falls slowly.

3. Press `q` to stop the program.

## Step 5: Show three series, and name each one

1. Add the line `series = 3` below the line `kind = "testdata"`.
2. Add the line `legend = "wave {{series}}"` below each line `expr = "sine"`.
3. Add the line `legend = "walk {{series}}"` below the line
   `expr = "random_walk"`.
4. Save the file, and run the command of step 1 again.

The chart now draws three waves. The stat panel shows three numbers, with the
names `wave 0`, `wave 1`, and `wave 2`. The gauge panel shows three meters, with
the names `walk 0`, `walk 1`, and `walk 2`.

The field `legend` holds a pattern, which builds the name of each series. The
data source `testdata` gives each series the label `series`, with the values
`0`, `1`, and `2`. The text `{{series}}` writes the value of that label.

5. Press `q` to stop the program.

Your file now holds this text:

```toml
title = "my first dashboard"
refresh = "1s"

[time]
from = "now-1m"
to = "now"

[source.test]
kind = "testdata"
series = 3

[[panel]]
title = "wave"
kind = "timeseries"
unit = "percent"
grid = { x = 0, y = 0, w = 16, h = 8 }

[[panel.target]]
source = "test"
expr = "sine"
legend = "wave {{series}}"

[[panel]]
title = "now"
kind = "stat"
unit = "percent"
grid = { x = 16, y = 0, w = 8, h = 8 }

[[panel.target]]
source = "test"
expr = "sine"
legend = "wave {{series}}"

[[panel.threshold]]
value = 50.0
color = "warning"

[[panel.threshold]]
value = 80.0
color = "critical"

[[panel]]
title = "walk"
kind = "gauge"
unit = "percent"
min = 0.0
max = 100.0
grid = { x = 0, y = 8, w = 24, h = 4 }

[[panel.target]]
source = "test"
expr = "random_walk"
legend = "walk {{series}}"
```

## What you did

- You declared a data source, and you asked it for two signals.
- You placed three panels on the grid.
- You coloured a value with two thresholds.
- You named each series with a pattern.

## The next step

- Follow [Read a Prometheus server](read-a-prometheus-server.md) to show the
  data of a real server.
- Read [The dashboard file](../reference/dashboard-file.md) for every table and
  every field.
- Read [The panels](../reference/panels/README.md) for each kind of panel.
