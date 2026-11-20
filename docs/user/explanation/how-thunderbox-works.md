# How thunderbox works

This page explains how the parts of thunderbox work as one program. It answers
one question: how do a dashboard, a data source, a frame, and a panel work
together?

The page gives no steps. [Write your first dashboard](../tutorials/write-your-first-dashboard.md)
teaches the steps, and [The dashboard file](../reference/dashboard-file.md)
gives every field. An architecture decision record (ADR) states one decision of
the project and the reasons for it. This page summarises each idea and links to
the ADR that holds the detail.

## A dashboard is a document

A person writes a dashboard as a file in the TOML format. The file names the
data sources, the time range, and the refresh interval. It also gives each
panel a place on a grid of 24 columns. thunderbox reads the file, and it never
writes it.

The file states what a person wants to see. It holds no measurement, and
thunderbox stores no measurement either. The system that a data source reads
keeps the measurements, and that system decides how long to keep them. A
Prometheus server is such a system. thunderbox owns the layout and the picture,
and nothing else.

The sysinfo data source is the one special case. The machine reports its state
at one moment, and it keeps no past. That data source therefore keeps a store
of samples in memory while the program runs. The store writes nothing to a
disk, and it disappears when the program stops.
[The time range and the window](the-time-range-and-the-window.md) explains what
that store means for a chart.

The program `thunderbox` holds a default dashboard over the sysinfo data source,
and it draws that dashboard when a person names no file. That dashboard is an
ordinary dashboard, and the same code draws every dashboard.
[ADR 0010](../../contributor/adr/0010-the-program-that-runs-alone-is-a-dashboard-over-sysinfo.md)
states that decision.

## Four parts and one form of answer

Four parts do the work:

- the dashboard, which is the document above;
- the data sources, which answer the queries;
- the runtime, which runs the queries and holds the answers;
- the panels, which draw the answers.

A panel holds one target or more. A target is one query, and it names the data
source that answers that query.

Grafana is a dashboard program for the web browser, and thunderbox has the same
shape. Grafana has no common query language. One data source reads PromQL, and
another data source reads SQL. thunderbox follows the same rule. Each data
source reads its own form of query. thunderbox puts the value of a variable in
the place of each reference to that variable, and it changes nothing else in the
text. Only the data source knows what the text asks for.

The parts agree on the form of the answer instead. Every data source returns
its answer as frames. A frame is a table of named columns. One column usually
holds moments in time. The other columns hold numbers, text, or values of true
and false. Each column can carry labels, such as the name of a host, and hints,
such as a unit.

A panel reads frames, and it has no knowledge of the data source that made
them. A chart draws a column of numbers against a column of time. It draws a
series of a Prometheus server and a series of this machine in the same way.

That rule is the main decision of the project: agree on the form of the answer,
and not on the form of the question. A common query language would need to fit
every data source, and no language fits all of them. One form of answer fits
every panel. [ADR 0003](../../contributor/adr/0003-agree-on-the-answer-and-not-on-the-question.md)
states the decision.

## What the rule gives

The rule gives three results, and none of them costs extra work.

A new data source needs no change to a panel. A trait is a set of functions
that a Rust type must supply. The author of a new data source writes one
implementation of one trait. Every panel can then draw the answers of the new
data source. That trait is the one place where thunderbox meets a data source.
[ADR 0005](../../contributor/adr/0005-one-trait-is-the-seam-of-every-data-source.md)
states that decision.
[ADR 0012](../../contributor/adr/0012-the-five-duties-of-a-data-source.md)
states the five duties that a data source keeps.

One dashboard can read several data sources. Each target names its own data
source, and the runtime finds that data source for each target. Two data
sources on one dashboard are therefore two entries in one list.
[ADR 0034](../../contributor/adr/0034-a-dashboard-reads-more-than-one-data-source.md)
states this result, and
[Show two data sources on one dashboard](../how-to/show-two-data-sources-on-one-dashboard.md)
gives the steps.

The name of a series follows one rule for every data source. A person writes a
pattern in the `legend` field of a target. thunderbox applies that pattern after
the data source answers, so no data source needs a rule of its own for a name.
[Name each series](../how-to/name-each-series.md) gives the steps.

## The runtime runs the queries

The runtime of the dashboard stands between the data sources and the panels. It
sends each query, it collects each answer, and it holds the state of each
panel. A panel never runs a query, and it never waits. The code that draws
reads the most recent answers, and it returns immediately.

A set of queries is the whole group of queries that one refresh sends. The
runtime sends a set of queries in these conditions:

- the program starts;
- the refresh interval passes, for a dashboard that states a `refresh` field;
- a person presses the key `r`;
- a person changes the time range, the marker, or the value of a variable;
- the choices of a variable arrive from a data source, and they change a value;
- a new size of the terminal changes the number of points that a panel asks
  for;
- a panel appears, because a row panel expands or the terminal grows.

The program that holds the dashboard calls the runtime on each frame, and the
runtime then starts a refresh when the refresh interval passes. thunderbox
starts no timer of its own.
[Two ways into another program](two-ways-into-another-program.md) explains why.

The runtime runs the panels of one set of queries at the same time. Assume a
dashboard of four panels, where each data source takes one second. That
dashboard answers in about one second, and not in four seconds. The queries of
one panel run one after another, in the order of its targets.
[ADR 0045](../../contributor/adr/0045-the-panels-of-one-set-of-queries-run-at-the-same-time.md)
states that decision.

The runtime publishes the answers of one set of queries together, when the
whole set ends. Two panels that the screen shows side by side therefore hold
answers of the same set. A panel that appears again is the one exception.
[Panels that the terminal cannot hold](panels-that-the-terminal-cannot-hold.md)
explains it.

## A refresh keeps each answer

A refresh does not clear the panels. Each panel keeps the answer that it holds
until the new answer arrives. The default dashboard refreshes each second. A
panel that cleared on each refresh would show `loading…` many times a minute.
[ADR 0018](../../contributor/adr/0018-a-refresh-keeps-the-answer-of-each-panel.md)
states that decision.

The runtime clears every panel in two conditions alone: a new time range, and a
new value of a variable. The old answers then describe a different question,
and a person would read them as answers to the new question. Each panel
therefore shows `loading…` until its new answer arrives.

A window is the pair of moments that a time range names, and the moment that
named them. The range "the last five minutes" names a new window on each
refresh. The runtime still keeps the answers of such a range. It compares the
range itself, and not the window that the range names.

The status row tells a person what the runtime does. It writes
`The dashboard waits for an answer.` when a set of queries waits for longer than
one second. It also states the age of the numbers when that age reaches five
seconds. A data source that answers immediately therefore changes no text.

## A slow query or a query that returns an error changes one panel

Each panel runs in a task of its own. A task is a piece of work that runs beside
the other pieces of work. A fault stays inside the task of its panel:

- A data source that returns an error gives that error to its own panel. Every
  other panel keeps its answer.
- A data source that does not answer in time gives its panel an error. That
  error names the data source and the wait.
- A panic is a stop of the code on a fault that the code did not expect. A data
  source that panics gives its own panel an error as well.

A data source can state the longest time that it can take. The runtime takes 60
seconds for a data source that states no time. The status row counts the panels
that hold an error. Each panel writes its own error, so a person can read which
data source returned it.

The limit applies to one query, and not to the whole set of queries. A limit on
the whole set would discard every answer that had already arrived. It would also
name no data source.
[ADR 0019](../../contributor/adr/0019-the-runtime-holds-one-query-to-a-limit.md)
states the reasons.

This design has one cost. The runtime publishes a set of queries when its
slowest panel ends. While one data source waits, the other panels keep the
answers that they hold, but they take no new answer. A data source that never
answers therefore holds the whole dashboard still for the length of its limit.
The status row then reports the wait and the age of the numbers.

## Where to read more

These records hold the detail:

- [ADR 0003](../../contributor/adr/0003-agree-on-the-answer-and-not-on-the-question.md):
  agree on the form of the answer, and not on the form of the question.
- [ADR 0005](../../contributor/adr/0005-one-trait-is-the-seam-of-every-data-source.md):
  one trait is the seam of every data source.
- [ADR 0012](../../contributor/adr/0012-the-five-duties-of-a-data-source.md):
  the five duties of a data source.
- [ADR 0018](../../contributor/adr/0018-a-refresh-keeps-the-answer-of-each-panel.md):
  a refresh keeps the answer of each panel.
- [ADR 0019](../../contributor/adr/0019-the-runtime-holds-one-query-to-a-limit.md):
  the runtime holds one query to a limit.
- [ADR 0034](../../contributor/adr/0034-a-dashboard-reads-more-than-one-data-source.md):
  a dashboard reads more than one data source.
- [ADR 0045](../../contributor/adr/0045-the-panels-of-one-set-of-queries-run-at-the-same-time.md):
  the panels of one set of queries run at the same time.

These pages give the facts:

- [The data sources](../reference/data-sources/README.md) gives each data source
  and its queries.
- [The panels](../reference/panels/README.md) gives each kind of panel.
- [The command line](../reference/command-line.md) gives each sentence of the
  status row.
