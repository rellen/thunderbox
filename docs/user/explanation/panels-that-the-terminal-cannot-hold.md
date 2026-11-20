# Panels that the terminal cannot hold

A dashboard places each panel on a grid, and the size of the terminal decides
how many cells each panel receives. A small terminal can give a panel too few
cells to draw anything. This page explains what thunderbox does with such a
panel, and why.

The page gives no steps.
[Read a dashboard on a small terminal](../how-to/read-a-dashboard-on-a-small-terminal.md)
gives the practical advice. An architecture decision record (ADR) states one
decision of the project and the reasons for it. This page summarises each idea
and links to the ADR that holds the detail.

## Why a panel under 3 cells disappears

A panel draws a border on each side, and it needs at least one cell inside the
border for its data. A panel therefore needs 3 cells across and 3 cells down.
The renderer draws no panel under that size. This documentation calls such a
panel a panel that the renderer drops.

A row panel follows a rule of its own. It draws one line, so it needs only one
cell of height.

The place of a panel that the renderer drops stays empty. The renderer does not
give those rows to the other panels. A later version can give those rows to the
panels that the renderer draws.

![A terminal of 19 rows holds a dashboard of three panels. thunderbox drops the first panel, and it leaves those two rows empty. The status row reads `The terminal is too small for 1 panel.`](../../media/too_small.gif)

The picture shows a terminal of 90 cells by 19 rows. The dashboard holds three
panels, one above the other. The first panel holds 4 rows of the grid, and each
of the other two holds 12 rows. The terminal gives the panels 18 rows, so the
first panel receives 2 rows. The renderer drops it.

## Why thunderbox asks for nothing for a panel that the renderer drops

A panel that the renderer drops shows its answer to nobody. A query for that
panel would cost a request to a data source, and nobody would read the answer.
The runtime therefore asks no data source about a panel that the renderer drops.
[ADR 0024](../../contributor/adr/0024-a-panel-that-the-renderer-drops-asks-for-nothing.md)
states the decision.

The runtime and the renderer read the same arithmetic for the place of each
panel. The two parts therefore always agree about which panels the renderer
drops. No query therefore serves a panel that the screen cannot show.

The saving appears only on a terminal that already hides part of the dashboard.
The default dashboard holds 7 panels and 8 targets. It saves nothing on a
terminal of 10 rows or more. On a terminal of 9 rows it drops the table and the
top panel, and it saves 2 queries of 8. A program that reports an area of no
cells saves every query. A program that hides a dashboard behind a tab can
report such an area.

A height that only drops panels starts no set of queries. A set of queries is
the whole group of queries that one refresh sends. Each panel that the renderer
still draws keeps its width, and therefore the answer that it needs.

A height that shows a panel again starts a set of queries, because the runtime
asked nothing for that panel while the renderer dropped it. That set asks about
every panel that the renderer draws, and not only about the panel that appeared.
The project examined five designs that save some of those queries, and a reader
measured a fault in each design.
[ADR 0060](../../contributor/adr/0060-a-height-that-uncovers-a-panel-asks-every-panel.md)
records that open question.

## Why the runtime asks about every panel before the program reports an area

The runtime sends its first set of queries before the program draws its first
frame. At that moment the runtime knows no height. A guess of the height could
be too small. The runtime would then drop panels that the terminal can hold, and
the first picture would show no panel.

The runtime therefore asks about every panel until the program reports the real
area. A program that never reports an area also keeps every panel. A program
that holds a dashboard behind a screen that it does not draw is an example.
[ADR 0029](../../contributor/adr/0029-the-runtime-asks-about-every-panel-until-an-area-arrives.md)
states that rule.

## Why a panel that the renderer drops keeps its answer

A person can make a terminal smaller, and then larger again. Assume that a
panel lost its answer while the terminal was too small. That panel would then
show `loading…` after each change of the size, beside the answers of the other
panels.

The runtime therefore keeps the answer of a panel that the renderer drops. The
panel shows that answer immediately when the terminal grows to hold it again.
[ADR 0027](../../contributor/adr/0027-a-dropped-panel-keeps-its-answer.md)
states the decision.

Two kinds of answer do not stay:

- The answer of another window does not stay. A window is the pair of moments
  that a time range names, and the moment that named them. A new time range, or
  a new value of a variable, returns every panel to the state that has no
  answer. A panel that the renderer drops takes that state as well.
- An error does not stay. No query reaches a panel that the renderer drops, so
  no later set of queries could clear the error. The status row would then
  count an error that nobody can repair. The runtime clears such an error, and
  the panel shows `loading…` when it appears again.

The choice has a cost. A panel that appears again holds an answer of an older
set of queries. Its line can stop a little before the right edge of the chart
until the next set of queries answers. The other choice draws `loading…` in
that panel after each change of the size, and the project preferred the old
answer.

The answer of a panel holds no mark for "too small". One program can draw the
same answers into two areas of different size. Such a mark would be true of one
area and false of the other.
[ADR 0026](../../contributor/adr/0026-panel-state-gains-no-fourth-variant.md)
states that decision.

## The sentence of the status row

An empty screen tells a person nothing. The status row of the program therefore
counts the panels that the renderer drops, in one sentence:

```
The terminal is too small for 3 panels.
```

The program builds that sentence from the rule that the renderer reads, so the
count always describes the screen above it. The count holds no panel that a
collapsed row panel hides, because a person chose to hide that panel.

The status row holds one line, and several sentences share it. When the line is
too narrow, the row drops the list of the keys first, because a person needs
that list least. Only the count of the panels that hold an error stays longer
than the sentence about the small terminal.

A program that holds a dashboard can write the same sentence. The dashboard
layer gives the count, and the program decides where to show it.

One limit remains. The key `Tab` still selects a panel that the renderer drops.
A person then presses a key, and the screen shows no change. The dashboard layer
does not know the size of the last frame when it reads a key.

## Where to read more

These records hold the detail:

- [ADR 0024](../../contributor/adr/0024-a-panel-that-the-renderer-drops-asks-for-nothing.md):
  a panel that the renderer drops asks for nothing.
- [ADR 0026](../../contributor/adr/0026-panel-state-gains-no-fourth-variant.md):
  the state of a panel gains no fourth variant.
- [ADR 0027](../../contributor/adr/0027-a-dropped-panel-keeps-its-answer.md):
  a panel that the renderer drops keeps its answer.
- [ADR 0029](../../contributor/adr/0029-the-runtime-asks-about-every-panel-until-an-area-arrives.md):
  the runtime asks about every panel until an area arrives.
- [ADR 0060](../../contributor/adr/0060-a-height-that-uncovers-a-panel-asks-every-panel.md):
  a height that uncovers a panel asks every panel.

These pages give the facts:

- [The command line](../reference/command-line.md) gives each sentence of the
  status row.
- [Why each panel asks for its own points](why-each-panel-asks-for-its-own-points.md)
  explains how a new size of the terminal reaches a data source.
- [Row panels and the place of a panel](row-panels-and-the-place-of-a-panel.md)
  explains the panels that a row panel hides.
