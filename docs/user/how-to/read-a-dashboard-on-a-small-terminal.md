# Read a dashboard on a small terminal

This guide explains the panels that thunderbox drops on a small terminal, and it
gives the steps that return them. thunderbox draws no panel that is less than 3
cells wide or less than 3 cells high. It drops such a panel, and it asks the data
source of that panel for nothing.

## Find the panels that thunderbox dropped

1. Read the status row at the bottom of the terminal.

The status row counts the panels that thunderbox dropped, for example
`The terminal is too small for 1 panel.` A dropped panel leaves an empty space
at its place.

![A terminal of 19 rows holds a dashboard of three panels. thunderbox drops the first panel, and it leaves those two rows empty. The status row reads `The terminal is too small for 1 panel.`](../../media/too_small.gif)

In the picture, the two empty rows at the top are the place of the first panel,
and the status row counts that panel.

## Return a dropped panel

Do one of these steps:

- Make the terminal larger, or make its font smaller. The panel returns, and it
  asks its data source for its data.
- Give the panel a larger share of the grid. Raise the `h` of its field `grid`,
  and lower the `h` of a larger panel in the same column. Then correct the `y`
  of each panel below those two panels.
- Put the panels that you do not read under a row panel, and collapse that row
  panel. The panels that remain then share the free rows.

## Calculate the height that a panel needs

This calculation is correct for a dashboard file that holds no row panel.

1. Find the height of the grid. That height is the largest sum of `y` and `h`
   of a panel.
2. Multiply 3 by the height of the grid, and divide the result by the `h` of the
   panel.
3. Round the result to the next larger whole number. The panels need that
   number of rows of the terminal.
4. Add 1 for the status row. Add 1 more when the dashboard file declares a
   variable, because the row of the variables takes one row.

The dashboard of the picture holds panels of 4, 12, and 12 rows of the grid, so
the height of the grid is 28. The first panel needs 3 × 28 ÷ 4 = 21 rows, and
the terminal therefore needs 22 rows.

The width follows the same rule. A panel of `w` columns of the grid needs a
terminal of 3 × 24 ÷ `w` columns. Round that result to the next larger whole
number as well.

## Read the short texts inside a panel

A panel that has too little room also drops a part of its own data, and it
counts that part:

- A stat panel, a gauge panel, or a bar gauge panel writes
  `2 series do not fit.` or, in a small space, `+2`.
- A table panel writes `2 columns do not fit.` or `+2`.
- A top panel writes `74 rows do not fit.` in its bottom border.

The status row also drops the list of the keys first when a sentence needs the
room.

## The pages for the facts and the reasons

- [Panels that the terminal cannot hold](../explanation/panels-that-the-terminal-cannot-hold.md)
  states why thunderbox drops a panel and asks for nothing.
- [The panels](../reference/panels/README.md) states the short texts of each
  panel.
- [The command line](../reference/command-line.md) states each sentence of the
  status row.
