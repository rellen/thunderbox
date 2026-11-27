# Show an earlier window

This guide moves the time range of a dashboard into the past, and it changes
the width of the time range. Every panel shares one time range. A window holds
the two moments that a time range names, and the moment at which the program
read the clock.

## Move the time range with the keys

1. Press `Left` to move the time range back by half of its width.
2. Press `-` to make the time range twice as wide.
3. Press `+` to make the time range half as wide.
4. Press `Right` to move the time range forward by half of its width.
5. Press `0` or `=` to return to the time range that the dashboard file states.

After a move back, the window stands still, and the status row names the two
moments of the window in Coordinated Universal Time. This text is an example:

```
The panels hold 14:00:00 to 14:02:00 UTC. Press 0 to follow the clock.
```

The sentence stays the same while the window stands still. The status row
writes the short form `Held 14:00:00 to 14:02:00. 0 follows.` when the
complete sentence does not fit. A window of one day or more also names the
date, as in `01-01 00:00 to 01-03 00:00`. A change of the width keeps the later
end of the window where it is.

A move forward that reaches the present moment makes the time range follow the
clock again. The time range keeps the width that you selected.

The keys `-` and `+` keep the width between 1 second and 3650 days. A key that
cannot move an end leaves the time range as it was.

## Know which data source holds the past

- The data sources `prometheus` and `testdata` answer for each window.
- The data source `sysinfo` holds nothing from before the program started. A
  window before that moment shows no data from this data source.

## Open a dashboard on another time range

1. Change the table `[time]` of the dashboard file:

   ```toml
   [time]
   from = "now-2h"
   to = "now-1h"
   ```

2. Run the program with your dashboard file.

The keys `0` and `=` return to this time range. The fields `from` and `to` also
take a fixed moment. A fixed moment is a count of milliseconds after the Unix
epoch, as text. The Unix epoch is the start of 1 January 1970, in Coordinated
Universal Time. The text `"1700000000000"` is an example of a fixed moment.

The width of the time range must be between 1 second and 3650 days. The
program returns an error for a file that does not keep these limits, and it
writes that error before it opens the terminal. The page
[The dashboard file](../reference/dashboard-file.md#second-step-dashboardvalidate)
holds the text of each error.

## Pages for the facts and the reasons

- [Time ranges](../reference/time-ranges.md) states the form of a moment, the
  form of a duration, and the limits of the width.
- [The command line](../reference/command-line.md) states the keys and the
  status row.
- [The time range and the window](../explanation/the-time-range-and-the-window.md)
  states why a window stands still after a move.
