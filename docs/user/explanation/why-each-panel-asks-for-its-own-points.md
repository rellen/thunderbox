# Why each panel asks for its own points

A chart can draw only a limited number of points across its width. thunderbox
asks each data source for that number of points, and for no more. This page
answers these questions:

- How does thunderbox find the number of points?
- Why does a narrow panel ask for fewer points?
- Which part of the system reduces the points?
- Why does a new size of the terminal wait a moment?

An architecture decision record (ADR) states one decision of the project and
the reasons for it. This page summarises each idea and links to the ADR that
holds the detail.

## The point limit

The point limit is the largest number of points that a panel asks a data source
for. Each query of a panel carries the point limit of that panel.

A terminal cell holds one character. Some characters divide the cell into
smaller parts, and this documentation calls each part a sub-cell. The marker is
the set of characters that a chart draws its points with. The braille marker
divides each cell into 2 sub-cells across and 4 sub-cells down. A chart can
therefore draw 2 points in each column of cells.

The point limit is the width of the panel in cells, multiplied by the number of
sub-cells across one cell:

| marker | sub-cells across one cell | the point limit of a panel of 37 cells |
|---|---|---|
| `braille` | 2 | 74 |
| `octant` | 2 | 74 |
| `halfblock` | 1 | 37 |
| `block` | 1 | 37 |

That number matches what the panel can show. A panel that asked for more points
would receive points that it cannot draw. Two points would then fall into one
sub-cell, and the data source and the network would carry the second point for
no reason. A panel that asked for fewer points would lose a shape that the data
holds. A short, high peak can sit between two points.

The width of a panel counts its border. It also counts the column of cells that
holds the labels of the axis of values. A panel therefore asks for a few points
more than it draws, and never for fewer. The project keeps that margin, for the
reason above.

A new marker changes the point limit, so a new marker starts a new set of
queries. A set of queries is the whole group of queries that one refresh sends.
[Change the glyph of a chart](../how-to/change-the-glyph-of-a-chart.md) gives
the steps.

## Why a narrow panel asks for fewer points

A dashboard places each panel on a grid of 24 columns. The renderer divides that
grid onto the cells of the terminal, so each panel holds a width of its own. The
runtime reads the width of each panel, and not the width of the whole screen.

The default dashboard on a terminal of 110 cells shows the effect. The chart
`cpu` holds 16 columns of the grid, which are 73 cells. It asks for 146 points.
The stat panel `cpu now` beside it holds 8 columns, which are 37 cells. It asks
for 74 points.

Each panel once asked for the points of the whole width. The stat panel then
asked for 220 points, which is about three times the points that it can draw.
[ADR 0023](../../contributor/adr/0023-each-panel-asks-for-the-points-of-its-own-width.md)
records that change.

One function returns the width of a panel, and both the renderer and the runtime
read it. A panel therefore cannot ask for the points of a width that it does not
draw.

Every kind of panel follows this one rule:

- A chart draws a point at each position that the marker can address inside its
  width.
- A stat panel draws a small chart of the most recent points across its width.
- A gauge and a bar gauge read the most recent point. A data source returns
  that point at every point limit.
- The rows of a top panel describe things, and not moments. The sysinfo data
  source and the testdata data source read no point limit for the query
  `processes`.
- A table shows the last rows that fit in its height. A narrow table can
  receive fewer rows than its height holds, and it then shows fewer rows.

A second rule for some kinds of panel would make the behaviour harder to
predict. A person would then need to know which panel follows which rule. One
rule also lets the documentation describe the whole program in one sentence.

## Why the data source reduces the points

A data source can hold more points than a panel can draw. Some part of the
system must reduce them. thunderbox gives that work to the data source, as
Grafana does. Grafana is a dashboard program for the web browser, and thunderbox
has the same shape.
[ADR 0009](../../contributor/adr/0009-the-data-source-reduces-the-points.md)
states the decision.

A data source knows its own data best, and it can reduce the points before they
cross a network:

- The Prometheus data source calculates the step of a range query from the
  point limit. The step is the distance in time between two points. The server
  then does the work, and only the points that the panel can draw reach
  thunderbox.
- The sysinfo data source takes the points from its own store, at an even
  distance apart. It always takes the most recent sample, because a person
  reads the right edge of a chart first.
- The testdata data source makes exactly the number of points that the panel
  asks for.

The Prometheus data source also raises the step in three ways. It raises the
step to the `min_step` option of the data source. It raises the step to a round
value, such as 2 seconds or 5 minutes. It also raises the step until the answer
fits the limit of 11000 points that a Prometheus server holds.

A round step holds the chart still. The data source moves both ends of the range
onto the step. The times of the points therefore stay in place while the range
moves, and a chart that refreshes each second does not shake.

This design has one gap. thunderbox holds no step of its own that reduces the
points. A data source that cannot reduce its points returns every point, and
the panel draws every point. Each data source of this project reduces its own
points today.
[ADR 0062](../../contributor/adr/0062-downsamples-promises-a-step-that-no-code-takes.md)
records the open question.

## Why a new size of the terminal waits a moment

A program reports the size of the area of the panels to the runtime on every
frame. The program `thunderbox` reads its terminal every 100 milliseconds. A
person who
drags the corner of a terminal window therefore reports a new size many times
each second.

A new set of queries for each size would send one request to a server for each
frame. Each new set would also stop the set before it, so most answers would
never arrive. The runtime holds a new size instead. It sends the next set of
queries when no set of queries runs, and when one of these two conditions holds:

- the size stood still for 250 milliseconds;
- the size waited for one second.

A drag across thirty frames therefore costs one set of queries. The second
condition serves a size that never stands still. A scroll bar that appears and
disappears makes such a size.
[ADR 0021](../../contributor/adr/0021-a-new-area-waits.md) states the
decision.

The wait does not grow with the refresh interval. The refresh interval states
how fresh a person wants the numbers. The wait states how long the runtime waits
after a change of the size. The two values answer different questions. A
dashboard that refreshes every ten seconds would otherwise hold a new size for
ten seconds.

Some sizes cost nothing. The width decides the point limit of each panel. The
height decides only which panels the renderer drops. A size that changes no
point limit, and that shows no new panel, therefore starts no set of queries. A
person who drags only the bottom edge of a window sends no query, unless a panel
appears.
[ADR 0022](../../contributor/adr/0022-an-area-that-changes-no-point-limit-starts-no-set.md)
states that rule.

The first size is the one exception to the wait. The runtime sends its first set
of queries before the program draws a frame, at an assumed width of 80 cells.
The first size that the program reports then reaches the data source
immediately, because the first picture must hold the correct number of points. A
terminal of 80 cells that holds every panel needs no second set.

## Where to read more

These records hold the detail:

- [ADR 0009](../../contributor/adr/0009-the-data-source-reduces-the-points.md):
  the data source reduces the points.
- [ADR 0021](../../contributor/adr/0021-a-new-area-waits.md): a new area waits,
  and the next set of queries carries it.
- [ADR 0022](../../contributor/adr/0022-an-area-that-changes-no-point-limit-starts-no-set.md):
  an area that changes no point limit starts no set of queries.
- [ADR 0023](../../contributor/adr/0023-each-panel-asks-for-the-points-of-its-own-width.md):
  each panel asks for the points of its own width.
- [ADR 0062](../../contributor/adr/0062-downsamples-promises-a-step-that-no-code-takes.md):
  no code of thunderbox reduces the points for a data source.

These pages give the facts:

- [The data sources](../reference/data-sources/README.md) gives the options of
  each data source, `min_step` among them.
- [How thunderbox works](how-thunderbox-works.md) explains a set of queries.
- [Panels that the terminal cannot hold](panels-that-the-terminal-cannot-hold.md)
  explains the panels that the renderer drops.
