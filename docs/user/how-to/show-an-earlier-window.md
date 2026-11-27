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
6. Press `f` to make a held window follow the clock behind the present moment.
   Press `f` again to hold the window.

After a move back from a live range, the window stands still. The status row
names the two moments of the window in Coordinated Universal Time. This text is an example:

```
The panels hold 14:00:00 to 14:02:00 UTC. Press f to follow the clock behind, or 0 to return.
```

The sentence stays the same while the window stands still. The status row
writes the short form `Held 14:00:00 to 14:02:00. f follows, 0 returns.` when
the complete sentence does not fit. A window of one day or more also names the
date, as in `01-01 00:00 to 01-03 00:00`. A change of the width keeps the later
end of the window where it is.

A move forward that reaches the present moment makes the time range follow the
clock again. The time range keeps the width that you selected.

## Choose to hold a window or to follow the clock

A time range has one of three modes:

- A live range ends at the present moment.
- A held range has a fixed moment at each end. Its window stands still.
- A following range ends before the present moment, and it moves with the
  clock. Its window moves on, and it keeps the same distance from the present
  moment.

Follow these steps to watch an earlier window move on:

1. Press `Left` to move back. The window is now held.
2. Press `f`. The window now follows the clock behind. The status row writes
   the width of the window and the distance back to it, and the distance stays
   the same.
3. Press `f` again to hold the window. Press `0` to return to the time range of
   the dashboard.

The keys `Left`, `Right`, `-`, and `+` keep the mode that you chose. A move
forward that reaches the present moment returns a live range in each mode.

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
