# Show the busiest processes

This guide adds a top panel to a dashboard file. The panel lists the processes
of this machine, and the busiest process comes first.

## Add the panel

1. Declare the data source `sysinfo`, if the dashboard file does not declare it:

   ```toml
   [source.machine]
   kind = "sysinfo"
   ```

2. Add a panel of the kind `top` that asks for the query `processes`:

   ```toml
   [[panel]]
   title = "the busiest processes"
   kind = "top"
   sort = "cpu"
   grid = { x = 0, y = 0, w = 24, h = 12 }

   [[panel.threshold]]
   value = 85.0
   color = "critical"

   [[panel.target]]
   source = "machine"
   expr = "processes"
   ```

3. Change the field `grid` to the place that the panel needs on your grid.
4. Run the program with your dashboard file:

   ```
   thunderbox my-dashboard.toml
   ```

The panel shows one row for each process. The column `cpu` gives the order, and
a share of 85 or more is red. The bottom border counts the rows that do not fit.
Each column takes an equal share of the width of the panel, so the columns stay
where they are when the list refreshes.

The data source `sysinfo` keeps no list from the past. The panel therefore shows
`no data` while the window stands still in the past.

## Put the process with the most memory first

1. Change the field `sort` to `"memory"`.
2. Change the bound of the threshold to a count of bytes:

   ```toml
   [[panel.threshold]]
   value = 1000000000.0
   color = "critical"
   ```

The panel now puts the process that holds the most memory first. A process
that holds 1000000000 bytes or more is red in the column `memory`.

The thresholds and the field `unit` of a top panel describe the column of the
field `sort` alone. [The top panel](../reference/panels/top.md) states the
rules of the order and of each column.

## Keep the number of the process at the left

A top panel stands the column `pid` at the right, as it stands each column of
numbers. To keep that column at the left, add the field `left` to the panel:

```toml
[[panel]]
title = "the busiest processes"
kind = "top"
sort = "cpu"
left = ["pid"]
grid = { x = 0, y = 0, w = 24, h = 12 }

[[panel.target]]
source = "machine"
expr = "processes"
```

The field `left` holds the header of each column that stands at the left. A name
that matches no header has no effect. [The top panel](../reference/panels/top.md)
states the rule.

## Show a list that is not of this machine

1. Declare the data source `testdata`:

   ```toml
   [source.test]
   kind = "testdata"
   ```

2. Change the field `source` of the target to `"test"`.

The data source `testdata` answers the query `processes` with a list of twelve
invented processes. That list has the same four columns. Use it for a picture or
for a test that must show nothing of your machine.

## Pages for the facts and the reasons

- [The top panel](../reference/panels/top.md) states the fields `sort` and
  `threshold` of a top panel.
- [The data source sysinfo](../reference/data-sources/sysinfo.md) states the
  four columns of the list.
- [The data source testdata](../reference/data-sources/testdata.md) states the
  invented list.
