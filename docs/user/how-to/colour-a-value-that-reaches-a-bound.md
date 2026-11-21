# Colour a value that reaches a bound

This guide adds thresholds to a panel. A threshold holds a bound and a colour. A
value that reaches the bound takes that colour.

## Add the thresholds

1. Add one `[[panel.threshold]]` table for each bound, below the `[[panel]]`
   table of the panel:

   ```toml
   [[panel]]
   title = "now"
   kind = "stat"
   unit = "percent"
   grid = { x = 0, y = 0, w = 8, h = 8 }

   [[panel.threshold]]
   value = 50.0
   color = "warning"

   [[panel.threshold]]
   value = 80.0
   color = "critical"

   [[panel.target]]
   source = "test"
   expr = "sine"
   ```

2. Write the bound in the field `value`, in the unit of the value. Write `80.0`
   for 80 percent, and `1000000000.0` for a thousand million bytes.
3. Write `warning` or `critical` in the field `color`.
4. Run the program with your dashboard file.

The value takes the colour of the largest bound that it reaches. The colour
`warning` is yellow, and the colour `critical` is red. A value that reaches no
bound keeps its usual colour. A stat panel, a gauge panel, and a bar gauge
panel draw that value in the colour of its series. A table panel and a top
panel draw it in grey.

## Select a panel that shows the colour

These kinds of panel colour a value:

- `stat`, which colours the number;
- `gauge`, which colours the meter;
- `bargauge`, which colours the bar;
- `table`, which colours each cell;
- `top`, which colours the column that gives the order, and no other column.

A panel of the kind `timeseries` draws no colour of a threshold.

## Pages for the facts and the reasons

- [Numbers and units](../reference/numbers-and-units.md) states the rule of the
  thresholds, and the colours that the theme knows.
- [The panels](../reference/panels/README.md) states the fields that each kind
  of panel reads.
