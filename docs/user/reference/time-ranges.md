# Time ranges

This page states the form of a moment and the form of a duration. It also
states the limits of the width of a time range, and the form of a label of the
axis of time. The program returns an error for a dashboard file that does not
keep a limit of this page.

A time range names two moments, a start and an end. A window holds three
moments: the start and the end that a time range names, and the present moment
at which the program read the time range. Every panel of a dashboard shares one
time range. This dashboard file shows the last 15 minutes:

```toml
title = "the last 15 minutes"
refresh = "5s"

[time]
from = "now-15m"
to = "now"
```

## Form of a moment

The fields `from` and `to` of the table `[time]` each hold a moment. A moment is
a text in one of these forms:

| form | what it names | example |
|---|---|---|
| `now` | The present moment. | `"now"` |
| `now-DURATION` | A moment before the present moment. | `"now-5m"` |
| `now+DURATION` | A moment after the present moment. | `"now+1h"` |
| a whole number | A fixed moment, as a count of milliseconds after the Unix epoch. The Unix epoch is midnight at the start of 1 January 1970, in Coordinated Universal Time. | `"1700000000000"` |

- The moment is always a text, between two quotation marks. The program
  rejects a whole number that stands without quotation marks.
- The program ignores the spaces at the start and at the end of the text. It
  also ignores the spaces between the sign and the duration, so it accepts
  `"now- 5m"`.
- The program rejects a space between `now` and the sign, as in `"now - 5m"`. It
  also rejects a space inside the duration, as in `"now-5 m"`.
- The default value of `from` is `"now-5m"`. The default value of `to` is
  `"now"`.
- A time range follows the clock when one end or more starts with `now`. A
  time range of two fixed moments names one window for ever.
- A time range can name its end before its start. The window then puts the
  earlier moment first.

## Form of a duration

A duration is a text that holds a whole number and then a unit, with no space
between them.

| unit | the length |
|---|---|
| `ms` | one millisecond |
| `s` | one second |
| `m` | one minute |
| `h` | one hour |
| `d` | one day |
| `w` | one week, which is 7 days |

- The text `"90s"` and the text `"5m"` are both correct.
- The program rejects a text with no unit, as `"5"`.
- The program rejects a number that holds a decimal point, as `"1.5s"`.
- The program rejects a minus sign, as `"-5s"`.
- The program rejects a unit in capital letters, as `"5M"`.
- The program ignores the spaces at the start and at the end of the text.
- The program returns an error for a duration that is larger than
  9223372036854775807 milliseconds, as `"20000000000w"`.

These fields hold a duration:

| field | the table | what the duration states |
|---|---|---|
| `refresh` | the top level | The time between two sets of queries. |
| `from`, `to` | `[time]` | The distance from the present moment, after `now-` or `now+`. |
| `min_step` | a `[source.NAME]` table of the kind `prometheus` | The smallest distance between two points. |
| `timeout` | a `[source.NAME]` table of the kind `prometheus` | The longest time that the data source waits for one answer. |

The field `refresh`, each offset of the table `[time]`, and the options
`min_step` and `timeout` follow the same rules. The program returns an error
for a duration that does not keep them. It writes that error before it opens
the terminal.

A duration of the wrong form gives an error of this form:

```
The time value "1.5s" is not correct. The unit ".5s" is not known. The known units are ms, s, m, h, d, and w.
```

A duration that is larger than 9223372036854775807 milliseconds gives an error
of this form:

```
The time value "20000000000w" is not correct. A duration cannot be larger than 9223372036854775807 milliseconds.
```

A whole number of more than 19 digits, as in `"99999999999999999999s"`, gives
the same error. The program reads no unit for such a number.

The reference of each field states the place of the error:

- [The dashboard file](dashboard-file.md#faults-that-the-program-rejects) states
  the error of `refresh` and of the table `[time]`.
- [The Prometheus data source](data-sources/prometheus.md#errors-of-the-options)
  states the error of `min_step` and of `timeout`.

## Limits of the width

| limit | the width |
|---|---|
| `MIN_SPAN_MS` | 1 second |
| `MAX_SPAN_MS` | 3650 days, which is about ten years |

- The keys `-` and `+` keep the width of the time range between these two
  limits.
- The key `+` stops at 1 second. The key `-` stops at 3650 days.
- The table `[time]` of a dashboard file must keep the same limits. A width of
  exactly 1 second is correct, and a width of exactly 3650 days is correct.
- The program puts the earlier moment first. The width of a time range does
  not depend on the order of the two moments.
- A time range with a fixed start and an end of `"now"` grows as the clock
  moves. The program checks the width once, when it reads the file.
- The sum of the present moment and an offset must fit in a count of
  milliseconds. A `from` of `"now+9223372036854775807ms"` does not fit.
- The program returns an error for a file that does not keep a limit. The
  texts of these errors are in the table below.
- A key that would carry an end past the largest or the smallest count of
  milliseconds leaves the time range as it was. The panels stay as they are,
  and the program starts no new set of queries.

### Errors of the width

The program writes one of these texts for a dashboard file that does not keep a
limit. `FIELD` is `from` or `to`. Each other capital word stands for a moment
of the file.

| fault | the text of the error |
|---|---|
| The sum of the present moment and an offset does not fit in a count of milliseconds. | `` The `[time]` table holds a `FIELD` field of VALUE. The sum of the present moment and that value does not fit in a count of milliseconds. `` |
| The width is larger than 3650 days. | `` The `[time]` table holds a `from` field of FROM and a `to` field of TO. That range is wider than 3650 days. `` |
| The width is smaller than 1 second. | `` The `[time]` table holds a `from` field of FROM and a `to` field of TO. That range is narrower than 1000 milliseconds. `` |
| A duration is larger than 9223372036854775807 milliseconds. | `The time value "TEXT" is not correct. A duration cannot be larger than 9223372036854775807 milliseconds.` |

The program writes `The dashboard is not correct.` before each of the first
three texts. The page [The dashboard file](dashboard-file.md#second-step-dashboardvalidate)
states the place of each text in the output.

The program writes a moment in the largest unit that divides it exactly. A
file that holds `"now-300s"` therefore shows `now-5m` in an error. Both texts
name the same moment.

The page [The command line](command-line.md#rules-of-the-keys-of-the-time-range)
states every rule of the keys of the time range.

## Labels of the axis of time

Three kinds of range exist:

- A live range ends at the present moment, as `now-5m` to `now` does.
- A following range follows the clock, but it ends before the present moment,
  as `now-1h` to `now-55m` does.
- A held range holds a fixed moment at each end. The left arrow key makes such
  a range.

A time series panel writes three labels under its axis of time. A live range
and a following range give labels that state the distance from the present
moment to that place on the axis. A held range gives labels that state the time
of each place. The section "Labels of a held range" below describes that form.

| label | what it states |
|---|---|
| `now` | The present moment. |
| `-30s` | 30 seconds before the present moment. |
| `+5m` | 5 minutes after the present moment. |
| `-61.0s` | 61 seconds before the present moment. The label holds one decimal place, so that two labels of one axis differ. |

- The unit of a label is `ms`, `s`, `m`, `h`, or `d`.
- The panel selects the largest unit that is not longer than the distance
  between two labels. Two labels of one axis therefore differ.
- The panel takes a shorter unit when the count would round to zero. It writes
  `-1s`, and never `-0m`.
- The panel takes a longer unit when the count would need four digits.
- A window that ends before the present moment holds no label `now`.

### Labels of a held range

A held range holds a fixed moment at each end, so its window stands still. The
panel then writes the time of each place, in Coordinated Universal Time (UTC).
The labels do not change on a refresh, because the window does not change.

| label | what it states |
|---|---|
| `14:02:10` | The time of day, for a window that is narrower than one day. |
| `01-02 14:02` | The month, the day, and the time, for a window that is one day wide or wider. |

- The project holds no time zone database, so a label never states local time.
  The table panel also writes UTC times.
- A live range and a following range keep the labels that state a distance.
- The column of labels at the left edge of a held chart is wider. The label
  `14:00:00` holds more characters than the label `-2m` does.

## The sentence of the status row

The status row names a window that ends before the present moment. A held range
has a fixed moment at each end. The row then writes the two moments in
Coordinated Universal Time. It also names the keys that follow the clock behind
and that return:

```
The panels hold 14:00:00 to 14:02:00 UTC. Press f to follow the clock behind, or 0 to return.
```

The row writes the short form
`Held 14:00:00 to 14:02:00. f follows, 0 returns.` when the sentence does not
fit. A window that is one day wide or wider writes each moment
as `MM-DD HH:MM`.

A following range ends before the present moment, and it moves with the clock.
It writes a sentence of this form:

```
The panels show 5 minutes that ended 1 hour ago. Press f to hold, or 0 to return.
```

The sentence writes each length in seconds, minutes, hours, or days. It rounds
each count to a whole number.

## Pages that give the reason

- [The time range and the window](../explanation/the-time-range-and-the-window.md)
  explains a window and the labels of the axis of time.
- [Show an earlier window](../how-to/show-an-earlier-window.md) moves the time
  range with the keys.
- [Architecture decision record 0037](../../contributor/adr/0037-the-refresh-continues-while-the-window-stands-still.md)
  states why the refresh continues while the window stands still.
