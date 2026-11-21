# Change the glyph of a chart

This guide changes the glyph that a chart draws a point with. A terminal cell
holds one character, and some characters divide the cell into smaller parts. A
glyph of such characters lets a chart draw more points than the terminal has
cells.

## Change the glyph while the program runs

1. Press `m`.
2. Press `m` again until the charts draw the glyph that you want.

The key `m` selects the glyphs in this order, and it then returns to the first
glyph:

| glyph | parts of one cell | what to know |
|---|---|---|
| `braille` | 2 across, 4 down | Most fonts hold these characters. The program starts with this glyph. |
| `octant` | 2 across, 4 down | No gap separates one cell from the next. The font must hold the characters of Unicode 16. |
| `halfblock` | 1 across, 2 down | Every font holds these characters. |
| `block` | 1 across, 1 down | A plain terminal can show this character. |

The key changes the glyph of each time series panel, and of the small chart of
each stat panel. The panels then ask their data sources again, because the
glyph changes the number of points that a panel can draw.

## Repair a chart that draws boxes

A chart that draws a box in place of each point uses a font that does not hold
the characters of the glyph.

1. Press `m` until the chart draws the glyph `halfblock` or the glyph `block`.

Every font holds the characters of those two glyphs.

## Set the glyph in your own program

The program `thunderbox` starts with the glyph `braille` each time, and the
dashboard file holds no field for the glyph. A program that holds thunderbox
selects the glyph itself:

- A program that draws a whole dashboard calls
  `DashboardView::new().with_marker(Marker::Block)`. It also passes
  `view.marker()` to `DashboardRuntime::set_marker` on each pass of its loop.
- A program that draws one panel calls `.marker(Marker::Block)` on a
  `TimeSeriesPanel` or a `StatPanel`.

The type `Marker` is `thunderbox_widgets::Marker`. The crate
`thunderbox-dashboard` also offers it as `thunderbox_dashboard::widgets::Marker`.

## Pages for the facts and the reasons

- [The time series panel](../reference/panels/timeseries.md) states how a chart
  draws a point.
- [Why each panel asks for its own points](../explanation/why-each-panel-asks-for-its-own-points.md)
  states why the glyph changes the number of points.
- [The command line](../reference/command-line.md) states the key `m`.
