# Time range and the window

Every panel of a dashboard shares one time range. This page answers these
questions:

- What does a time range name?
- What are the three modes of a time range?
- What does a label of the axis of time state, for each kind of range?
- Why does a window in the past still refresh?

The page gives no steps. [Show an earlier window](../how-to/show-an-earlier-window.md)
gives the keys, and [Time ranges](../reference/time-ranges.md) gives the form of
a moment and of a duration. An architecture decision record (ADR) states one
decision of the project and the reasons for it. This page summarises each idea
and links to the ADR that holds the detail.

## A time range and a window

A time range names the period that every panel shows. The default dashboard
states the range from `now-2m` to `now`. That text names no fixed moment. It
names a period that follows the clock. A saved dashboard therefore shows "the
last two minutes" on every day that a person opens it.

A window is the pair of moments that a time range names, and the moment that
named them. The runtime reads the clock one time for each set of queries. A set
of queries is the whole group of queries that one refresh sends. The runtime
then calculates the two ends of the range, and it keeps the present moment
beside them. Each data source receives that one window.

A window holds three moments, and not two. A person can move the time range into
the past. The end of the window and the present moment are then two different
moments. A chart needs both of them, and so does the sysinfo data source. The
sections below explain why.

## A window that stands still

The keys of the time range move the range back and forward. They also make the
range wider or narrower. A move back from a live range fixes a moment at each
end of the range. The window then stands still, and every panel shows the same
period on each refresh. A person who studies the past needs a picture that does
not move. The section on the three modes of a range states the other case.

A move forward that reaches the present moment returns a range that follows the
clock again. That range keeps the width that the person chose. No data source of
this project answers a question about the future.

One move is half of the width of the range. A move forward that stops less than
one move before the present moment also returns to the clock. The clock moves
between two key presses. A move back and then a move forward would otherwise
leave the window a few milliseconds behind the present moment. Such a window
would look as if it follows the clock, but it would never move again.

A change of the width holds the later end of the range still. That end also
keeps its kind, so a range that follows the clock still follows the clock after
the change.

A new time range returns every panel to the state that has no answer. The old
answers describe another window, and a person would read them as answers for
the new window. Each panel therefore shows `loading…` until its new answer
arrives.

A key that asks for the range that the dashboard already holds asks for nothing.
A terminal repeats a key while a person presses it. Each repeat would otherwise
stop the set of queries that runs, and the dashboard would never complete a
refresh.

The program `thunderbox` binds every key of the time range itself. The layer of
the dashboard, which another program can hold, consumes none of them. That
layer offers the function `time_key`, which maps a key to a new range. The
program decides which key reaches that function.
[ADR 0035](../../contributor/adr/0035-the-program-binds-each-key-of-the-time-range.md)
states the decision, and
[Two ways into another program](two-ways-into-another-program.md) explains the
reasons.

## Three modes of a range: live, held, and following

A time range is in one of three modes:

- A live range ends at the present moment. Its end is `now`, or an offset of 0
  or more. The window moves with the clock, and the right edge of every chart
  is the present moment.
- A held range has a fixed moment at each end. Its window stands still.
- A following range follows the clock, but it ends before the present moment.
  Each end is an offset before the present moment, such as `now-10m`. The
  window moves with the clock and keeps the same distance from it.

A move back from a live range holds the window. A person who studies an event
needs the picture to stand still. A held window that a person moves stays held.
A following window that a person moves keeps its mode. The keys `-` and `+`
keep the mode as well.

The key `f` changes the mode between a held range and a following range. A person who holds a
window sees the same moments on each refresh. After the key `f`, the same
window follows the clock. The person then sees a period of the same width move
on, and the distance back to it stays the same. The key `f` again holds
the window at the moments that it names at that time. A live range already
follows the clock, so the key `f` does nothing there. The key `0` returns the
range of the dashboard from every mode.

A move forward that reaches the present moment returns a live range in each
mode. The status row states the mode. A held window writes its two moments. A
following window writes its width and its distance, and the sentence stays the
same on each refresh because the distance stays the same.
[ADR 0093](../../contributor/adr/0093-a-person-can-follow-the-clock-behind-the-present.md)
states the decision.

## A live window states distances, and a held window states clock times

A chart writes labels under its axis of time. A live range ends at the present
moment. A following range follows the clock, but it ends before the present
moment. A held range holds a fixed moment at each end. For a live range and for a
following range, each label states the distance from the present moment to that
place on the axis. The label `-30s` marks the
moment 30 seconds before the present moment. The label `now` appears only where
the axis reaches the present moment.

The chart once measured each label from the newest point of the data. The right
label then always read `now`. A person who moved the range one hour into the
past still read `now` at the right edge, and that label was false. The chart now
reads the window, and the window holds the present moment. A window that ended
one hour ago therefore writes no label that reads `now`.

The unit of a label comes from the distance between two labels, and not from the
value of one label. Two labels of one axis therefore differ. The chart then
corrects that unit in two conditions:

- The chart takes a shorter unit when the value would round to zero. The text
  `-0m` would state both a distance and no distance.
- The chart takes a longer unit when the value would need four digits. The text
  `-61000ms` does not fit a narrow panel.

[ADR 0036](../../contributor/adr/0036-each-label-of-the-time-axis-states-its-distance-from-now.md)
states that rule.

A held window needs another rule. Its trace stands still on each refresh. A
label that states a distance from the present moment would count further into
the past, while the trace did not move. That change would tell a person that
the data changed, and it did not. A held window therefore labels its axis with
the time of each place, as `14:00:00`. The time is in Coordinated Universal Time
(UTC). A window that is one day wide or wider writes the date as well, as
`01-02 14:02`. The labels then stand still with the trace.

[ADR 0091](../../contributor/adr/0091-a-held-window-labels-its-time-axis-with-clock-times.md)
states that rule.

## The axis covers the window, and not only the data

The axis of time covers the whole window that a person asked for. It does not
shrink to fit the data that a data source returned. A data source that holds
only part of the window therefore fills only part of the panel. The rest of the
panel stays empty.

The empty part tells the truth about the data source. A chart that stretched the
data across the whole panel would hide the gap. A person would then read a
short period as a long one.

## Why the sysinfo data source shows nothing from before the program started

The machine reports its state at one moment, and it keeps no past. The sysinfo
data source therefore keeps its own store of samples in memory. It takes a new
sample when a query arrives, so the store fills only while the program runs. The
distance between two samples is about the refresh interval of the dashboard. It
is never less than 250 milliseconds, because the data source takes no sample
closer than that to the sample before. A dashboard of six panels therefore
takes one sample for each refresh, and not six.

The store holds at most 4096 samples of each series. A full store drops its
oldest sample for each new sample. A dashboard that refreshes each second
therefore keeps a little more than one hour of samples.

Two effects follow. In the first minutes of each run, each chart draws a short
line at its right edge. The rest of the window holds no sample. A window far in
the past holds no sample at all, and each panel then shows `no data`. The
list of processes keeps no past at all. It shows `no data` for each window that
does not hold the most recent sample.

The status row is the one place that can tell a person why. While the window
stands still in the past, the row names the two moments of that window. It also
names the keys that follow the clock behind and that return the range of the
dashboard:

```
The panels hold 14:00:00 to 14:02:00 UTC. Press f to follow the clock behind, or 0 to return.
```

The sentence holds no distance from the present moment. The picture of a held
window stands still, so the sentence stands still too. A following window ends
before the present moment, and it moves on each refresh. The row then writes
the width of that window and the distance back to it:

```
The panels show 5 minutes that ended 1 hour ago. Press f to hold, or 0 to return.
```

The row and the axis of time read the same window, so the two always agree. No
part of thunderbox knows the earliest moment that a data source holds. A person
therefore reads two facts together: the row names the window, and each panel
reports `no data`.

A sample carries the moment when the machine produced it. The data source once
stamped each new sample with the end of the window. While a person studied a
window one hour in the past, each new sample carried a moment one hour old. The
store then held its samples out of order, and a chart drew a line that jumped
backwards. Each query now carries the present moment beside the window, and the
data source stamps each new sample with that moment.
[ADR 0038](../../contributor/adr/0038-a-sysinfo-sample-carries-the-moment-of-the-machine.md)
states that rule.

## Why the refresh continues while the window stands still

A window that stands still names the same two moments on each refresh. A person
can therefore expect thunderbox to stop the refresh. thunderbox continues it,
for three reasons:

- The store of the sysinfo data source fills only while a query runs. A refresh
  that stopped would leave a hole in that store. The hole would be as long as
  the visit to the past.
- A refresh is the way that a panel which holds an error tries again.
- A data source can change its answer for a window that stands still. A
  Prometheus server can receive a late sample. A rule of that server can also
  write a value for a moment in the past.

The first reason decides the question by itself. A person can read the last hour
for five minutes and then press `0`. That person must not meet a hole of five
minutes at the present moment.

The cost is real. A dashboard that refreshes each second asks for the same
window once a second, and most of those answers hold nothing new. A person who
does not want that load writes no `refresh` field in the dashboard. The queries
then run only after a change, or when the person presses the key `r`.
[ADR 0037](../../contributor/adr/0037-the-refresh-continues-while-the-window-stands-still.md)
states the decision.

## Where to read more

These records hold the detail:

- [ADR 0035](../../contributor/adr/0035-the-program-binds-each-key-of-the-time-range.md):
  the program binds each key of the time range. ADR 0093 supersedes it.
- [ADR 0093](../../contributor/adr/0093-a-person-can-follow-the-clock-behind-the-present.md):
  a person can follow the clock behind the present moment.
- [ADR 0036](../../contributor/adr/0036-each-label-of-the-time-axis-states-its-distance-from-now.md):
  a live range and a following range label the axis of time with distances from
  the present moment.
- [ADR 0091](../../contributor/adr/0091-a-held-window-labels-its-time-axis-with-clock-times.md):
  a held window labels its axis of time with clock times.
- [ADR 0037](../../contributor/adr/0037-the-refresh-continues-while-the-window-stands-still.md):
  the refresh continues while the window stands still.
- [ADR 0038](../../contributor/adr/0038-a-sysinfo-sample-carries-the-moment-of-the-machine.md):
  a sample of sysinfo carries the moment of the machine.

These pages give the facts:

- [The command line](../reference/command-line.md) gives each key and each
  sentence of the status row.
- [The data sources](../reference/data-sources/README.md) describes the store of
  the sysinfo data source.
- [How thunderbox works](how-thunderbox-works.md) explains why a new time range
  clears the panels.
