# Numbers and units

This page states how a panel writes a number. It lists the units, the decimal
places, the steps of the field `reduce`, the thresholds, and the symbol `—`.

A terminal has few columns, so a panel keeps the text of a number short. It
writes `1.2G` in place of `1234567890`.

## The units

The field `unit` of a panel names the unit. A data source can also attach a
unit to a column, and the field of the panel replaces it.

| unit | the form of the text | examples |
|---|---|---|
| `percent` | The number, and then `%`. The number takes no letter of the scale, and no decimal place by default. | `43%`, `0%` |
| `bytes` | The number with a letter of the scale, and then `B`. | `999B`, `1.5KB`, `2MB`, `3GB` |
| `bytes/s` | The number with a letter of the scale, and then `B/s`. | `1.2MB/s`, `8.3GB/s` |
| `seconds` | The number, and then `s`. The number takes two decimal places or fewer by default, and it takes no letter of the scale. | `1.5s`, `0.12s`, `3600s` |
| any other name | The number with a letter of the scale. The panel writes no name of a unit. | `1.5K`, `150K` |
| no unit | The number with a letter of the scale. | `43`, `150K`, `-1.5K` |

The panel knows these four names alone: `percent`, `bytes`, `bytes/s`, and
`seconds`.

### The letters of the scale

| letter | the factor |
|---|---|
| none | 1 |
| `K` | 1000 |
| `M` | 1000 times `K` |
| `G` | 1000 times `M` |
| `T` | 1000 times `G` |
| `P` | 1000 times `T` |

- The panel divides the number by 1000 while the number is 1000 or more, up to
  the letter `P`.
- The panel then rounds the number to its decimal places. A number that rounds
  to 1000 or more takes the next letter. The unit `bytes` therefore writes `1KB`
  for 999.9 bytes, and not `1000B`.
- The whole part of a number therefore holds three digits at most. The letter
  `P` is the one exception, because no larger letter exists. A number of 1000
  `P` or more keeps the letter `P`, as `1500P` does.
- The letters count in steps of 1000, and not of 1024. The unit `bytes`
  therefore writes `1.5KB` for 1500 bytes.
- A negative number keeps its minus sign, as `-1.5K` does.

## The decimal places

The field `decimals` of a panel states the number of decimal places. It holds a
whole number from 0 to 255. A data source can also attach a number to a column,
and the field of the panel replaces it.

When the panel or the column states a number, the text holds exactly that number
of decimal places. The panel removes no zero.

| value | unit | `decimals` | text |
|---|---|---|---|
| 42 | `percent` | 2 | `42.00%` |
| 1500 | `bytes` | 2 | `1.50KB` |
| 2.6 | `seconds` | 0 | `3s` |
| 7 | none | 1 | `7.0` |

When neither states a number, the panel selects the number itself:

| the form of the text | the decimal places |
|---|---|
| `percent` | 0 |
| `seconds` | 2 or fewer |
| a number with no letter of the scale | 0 |
| a number with a letter of the scale, under 100 | 1 or fewer |
| a number with a letter of the scale, 100 or more | 0 |

The panel then removes each zero at the end of the decimal places. It also
removes the decimal point when no decimal place remains. A number under 1000
with no unit therefore takes no decimal place. The panel writes `0` for 0.4, and
`43` for 42.7. State `decimals` for a panel of small numbers.

## The steps of `reduce`

The stat panel, the gauge panel, and the bar gauge panel show one number for
each series. The field `reduce` names the step that reduces the column of
numbers to that number.

| step | the number that the step returns |
|---|---|
| `last` | The most recent value. This step is the default. |
| `first` | The oldest value. |
| `min` | The smallest value. |
| `max` | The largest value. |
| `mean` | The mean of every value. |
| `sum` | The sum of every value. |
| `count` | The number of values that the column holds. |

- Each step ignores a hole in the data, and it ignores a value that is not
  finite.
- Each step returns no number for a column that holds no value that it can use.
  The panel then drops that series.
- The step `count` is the one exception. It returns 0 for such a column.
- The steps `last` and `first` read the position of a value in the column. A
  data source returns its values from the oldest to the newest.

## The thresholds

A `[[panel.threshold]]` table states one bound, and the colour of a value that
reaches the bound. The example below is a part of a dashboard file, and it does
not load alone. It belongs to a `[[panel]]` table, and it states two
thresholds:

```toml
[[panel.threshold]]
value = 60.0
color = "warning"

[[panel.threshold]]
value = 85.0
color = "critical"
```

- A value reaches a bound when the value is equal to the bound or larger.
- The panel takes the threshold with the largest bound that the value reaches.
- A value that reaches no bound keeps its usual colour.
- The order of the tables in the file does not matter.
- A panel replaces the whole list of the thresholds of a column when it states
  one threshold or more.

| `color` | the colour |
|---|---|
| `warning` | yellow |
| `critical` | red |
| any other name | the usual colour of the value |

The panels apply the colour of a threshold in these places:

| kind | the part that takes the colour | the usual colour |
|---|---|---|
| `stat` | The number and its small chart. | the colour of the series |
| `gauge` | The bar. | the colour of the series |
| `bargauge` | The bar. | the colour of the series |
| `table` | The text of each number. | grey |
| `top` | The text of each number of the column that gives the order. | grey |

The panels `timeseries` and `row` read no threshold.

The guide
[Colour a value that reaches a bound](../how-to/colour-a-value-that-reaches-a-bound.md)
adds thresholds to a panel.

## The symbol for a value that a panel cannot show

The symbol `—` stands in the place of a number that a panel cannot show. The
symbol is not a mark of punctuation.

| kind | the condition |
|---|---|
| `stat` | The number that the step returns is not finite. The sum of two very large numbers is an example. |
| `stat` | The number does not fit its tile. The panel never cuts a number. |
| `table`, `top` | The cell holds a hole, or a value that is not finite. |
| `table`, `top` | The number does not fit its column. The panel never cuts a number. |
| `table` | A frame holds fewer rows than another frame. The symbol fills the cells above the rows of the shorter frame. |
| `top` | A frame holds no column for a header that another frame holds. |

The gauge panel and the bar gauge panel drop a series whose number is not
finite, so they never draw the symbol.

## The pages that give the reason

- [Architecture decision record 0007](../../contributor/adr/0007-the-settings-of-a-panel-win-over-the-column.md)
  states why the field of a panel replaces the value of the column.
- [The panels](panels/README.md) lists the fields that each kind reads.
