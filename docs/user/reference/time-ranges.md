# Time ranges

This page states the form of a moment and the form of a duration. It also
states the limits of the width of a time range, and the form of a label of the
axis of time.

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

## The form of a moment

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

## The form of a duration

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
- A duration of a very large number, as `"20000000000w"`, can end the program
  with the code 101. A build for release does not stop. It takes a wrong
  duration instead.
  [Architecture decision record 0061](../../contributor/adr/0061-a-duration-that-does-not-fit-ends-the-program.md)
  records that fault.

These fields hold a duration:

| field | the table | what the duration states |
|---|---|---|
| `refresh` | the top level | The time between two sets of queries. |
| `from`, `to` | `[time]` | The distance from the present moment, after `now-` or `now+`. |
| `min_step` | a `[source.NAME]` table of the kind `prometheus` | The smallest distance between two points. |
| `timeout` | a `[source.NAME]` table of the kind `prometheus` | The longest time that the data source waits for one answer. |

A duration of the wrong form gives an error of this form:

```
The time value "1.5s" is not correct. The unit ".5s" is not known. The known units are ms, s, m, h, d, and w.
```

## The limits of the width

| limit | the width |
|---|---|
| `MIN_SPAN_MS` | 1 second |
| `MAX_SPAN_MS` | 3650 days, which is about ten years |

- The keys `-` and `+` keep the width of the time range between these two
  limits.
- The key `+` stops at 1 second. The key `-` stops at 3650 days.
- The limits apply to the keys alone. The dashboard file can state a time
  range outside them.

The page [The command line](command-line.md#the-rules-of-the-keys-of-the-time-range)
states every rule of the keys of the time range.

## The labels of the axis of time

A time series panel writes three labels under its axis of time. Each label
states the distance from the present moment to that place on the axis.

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

## The sentence of the status row

The status row names a window that ends before the present moment. It writes a
sentence of this form:

```
The panels show 5 minutes that ended 1 hour ago.
```

The sentence writes each length in seconds, minutes, hours, or days. It rounds
each count to a whole number.

## The pages that give the reason

- [The time range and the window](../explanation/the-time-range-and-the-window.md)
  explains a window and the labels of the axis of time.
- [Show an earlier window](../how-to/show-an-earlier-window.md) moves the time
  range with the keys.
- [Architecture decision record 0037](../../contributor/adr/0037-the-refresh-continues-while-the-window-stands-still.md)
  states why the refresh continues while the window stands still.
