# thunderbox architecture

**What thunderbox is:** a dashboard platform for the terminal. It has the shape of
Grafana. A person writes a dashboard as a document. The dashboard then reads its
data from data sources that a person can add.

**What thunderbox is not:** a place to store data. thunderbox owns the layout and the
picture. thunderbox stores a dashboard. thunderbox never stores a measurement. The
system that thunderbox queries keeps the measurements, and that system decides how
long to keep them.

---

## 1. How a person runs thunderbox

thunderbox runs in four ways. These four ways are not four products. Two questions
separate them, and one more question applies to all of them.

| way to run | number of processes | where the program draws | who owns the terminal |
|---|---|---|---|
| alone | one | in the same process | thunderbox |
| client and server | two | in the client | thunderbox |
| through SSH | two | in the server | thunderbox |
| inside another program | one | in the same process | the other program |

Note what does not appear in this table. The place where the data lives is not a
question here. A Prometheus data source is on another machine. A `sysinfo` data
source is on this machine. Neither choice changes the shape of thunderbox. The
choice of data source answers that question, and nothing else does.

The "client and server" way is the one to doubt. thunderbox is a dashboard layer,
not a store. A thunderbox server that only passes a query to Prometheus adds a
step and gives nothing back. Grafana works in that way for one reason only: a
web browser cannot hold a password, and a web browser cannot reach a private
network. Three reasons remain:

- the viewer cannot reach the data source, but the server can;
- one collector serves many viewers, which matters when collection is expensive;
- the server holds a cache, or limits how often it queries the data source.

The SSH way already answers the first reason, and it is simpler. Treat "client
and server" as a later option on the data source layer, and not as a fourth way
to run thunderbox. Build it only when a need appears that the SSH way cannot meet.

### Design for the "inside another program" way first

That way is the hardest one. Meet it first, and the other three follow. Ignore
it, and a later version must separate the async runtime from the code that draws.
An async runtime is a library that runs concurrent tasks. The rule gives two
constraints:

1. **`thunderbox-widgets` depends on `ratatui-core` and `ratatui-widgets` only.**
   It uses no terminal backend, no `crossterm`, no `tokio`, and it reads no
   file. A program that already owns a terminal must be able to call
   `f.render_widget(TimeSeriesPanel::new(&frames, &cfg), area)`, and nothing
   else. The `chandelier` crate does this today.
2. **The code that draws is synchronous, and it reads data that it owns.** No
   `async` appears where the program draws a frame. Another task fills the data
   before the program draws. Only that step changes between the four ways to run
   thunderbox.

---

## 2. The main decision: agree on the answer, not on the question

Grafana has no common query language. Each data source states its own type of
query. One data source reads PromQL, and another reads SQL. The answer is the
part that all of them share. Each data source returns data frames, and a panel
reads a frame without knowledge of its origin.

**Agree on the form of the answer. Do not agree on the form of the question.**
Follow this rule and the other parts fall into place. Break it, and the project
spends its time on a common query language that fits nothing.

```rust
pub struct Frame {
    pub name: Option<String>,
    pub fields: Vec<Field>,
}

pub struct Field {
    pub name: String,
    pub labels: BTreeMap<String, String>,
    pub config: FieldConfig,   // unit, decimals, min, max, thresholds, display name
    pub values: Values,
}

pub enum Values {
    Time(Vec<i64>),
    F64(Vec<Option<f64>>),
    Str(Vec<Option<String>>),
    Bool(Vec<Option<bool>>),
}
```

A panel states the shape of the data that it needs. One time column and one or
more number columns is an example of a shape. A panel never states a data
source.

A query stays opaque, and each data source reads its own form:

```rust
pub struct Query {
    pub source: SourceUid,
    pub expr: String,
}
```

A query holds the name of a data source and a text. It holds nothing else. The
moments of the range, the present moment, and the number of points that a panel
can draw all sit beside the query, in `QueryRequest`. A data source reads both.
`thunderbox-core/src/query.rs` holds the two types.

---

## 3. The crates

```
thunderbox-core        Frame, Field, Values, TimeRange, Query, Dashboard, Panel.
                     serde. No ratatui, no tokio.
thunderbox-widgets     ratatui-core and ratatui-widgets ONLY. The panel
                     renderers: TimeSeries, Stat, Gauge, BarGauge, Table,
                     Heatmap, Logs.  <-- the smaller way in
thunderbox-transform   A pipeline from frame to frame: reduce, filter, rename,
                     join, group.
thunderbox-source      The DataSource trait and the registry.
thunderbox-source-*    prometheus, sysinfo, and testdata today. sql and csv
                     come later.
thunderbox-dashboard   The larger way in: it runs the queries, it refreshes them,
                     and it holds the time range, the variables, and the
                     selection. It holds a `dyn DataSource`. It behaves as a
                     library: it never owns main, the terminal, the async
                     runtime, or the key bindings.
thunderbox             The program. It owns the terminal and the event loop, and
                     it drives thunderbox-dashboard. Each way to run thunderbox is
                     a subcommand behind a feature.
```

### The one seam

One trait covers all four ways to run thunderbox:

```rust
#[async_trait]
pub trait DataSource: Send + Sync {
    fn descriptor(&self) -> SourceDescriptor;
    async fn query(&self, req: QueryRequest) -> Result<Vec<Frame>>;
    async fn label_values(&self, q: &str) -> Result<Vec<String>>;  // for a variable
}
```

`QueryRequest` holds the time range that every panel shares, the point limit of
the panel, and a suggested step. `thunderbox-dashboard` never knows which
implementation it holds. The implementation is a local data source, a client
that reads another machine, or an object that another program supplies.

---

## 4. The libraries

```toml
ratatui   = { version = "0.30.2", features = ["crossterm", "layout-cache", "unstable-backend-writer"] }
crossterm = "0.29"
tokio     = { version = "1", features = ["rt-multi-thread", "macros", "sync", "time"] }
# in thunderbox-widgets only:
ratatui-core    = "0.1"
ratatui-widgets = "0.3"
# for the SSH way:
russh   = "0.63"
termwiz = "0.23"   # to read the keys only. crossterm::event::read() reads
                   # standard input, which an SSH server cannot use.
```

thunderbox uses no other chart library, and no other terminal user interface
library. `docs/RESEARCH.md` gives the reasons.

---

## 5. How a panel draws a chart

The parts that ratatui supplies do most of the work: `Chart` with
`GraphType::Area` and `Dataset::fill_to_y`, and also `Sparkline`, `BarChart`,
`Gauge`, `LineGauge`, and `Canvas`.

**The marker.** A person selects the marker. A terminal cell holds one
character, and some characters divide the cell into smaller parts. This document calls each such part a sub-cell.

| setting | marker | sub-cells in one cell | note |
|---|---|---|---|
| `braille` (default) | `Marker::Braille` | 2 across, 4 down | most fonts hold these characters |
| `octant` | `Marker::Octant` | 2 across, 4 down | no space between cells, but fewer fonts hold these characters |
| `block` | `Marker::Block` | 1 across, 1 down | for a plain terminal, or for a font that lacks the other characters |

**Wrap the chart from the start.** A panel renderer must not build a
`ratatui_widgets::chart::Chart` value. Each renderer must call an internal
`TimeGraph` type. This project has not built that type yet. The
`Chart` type that ratatui supplies will not be enough forever. A chart can need
a time axis that ends at the right, a legend in a chosen place, and a
logarithmic axis. The wrapper keeps that decision open. Do not copy `chart.rs` before a need
appears, because the copy must then follow every change upstream.

**Colour gradients.** A `Dataset` holds one `Style` for a whole series, so a
point cannot have its own colour. A gradient colours each point by its height,
from the bottom of the chart to the top. Two methods give that result:

- draw on a `Canvas`, with one batch of points for each band of colour;
- change the colour of each cell in the `Buffer` after the chart draws.

This work comes later.

### The five panel types

`thunderbox-widgets` holds one renderer for each type of panel. Every renderer
takes the same three arguments, which are a title, the data, and a theme. Every
renderer also takes a `PanelOptions` value. That value carries these settings:

- the name of the unit;
- the number of decimal places;
- the range of a meter, as a smallest value and a largest value;
- the step that reduces a column of numbers to one number;
- the thresholds.

| type | what it draws | the question that it answers |
|---|---|---|
| `TimeSeriesPanel` | a chart of values against time | how did this move |
| `StatPanel` | one large number for each series | what is it now |
| `GaugePanel` | a meter, with the range written out | how full is it |
| `BarGaugePanel` | one named bar for each series | which of these is the largest |
| `TablePanel` | the columns of the frames | what are the numbers |

Three of the five show one number for each series. A `Reduce` step turns a
column of numbers into that one number. The step lives in `thunderbox-core`,
because a dashboard names it. `thunderbox-transform` will hold the rest of the
pipeline that section 3 describes.

**Two sources state how to show a number, and the panel wins.** A data source
fills the `FieldConfig` of each column, and a person writes the panel. A
Prometheus data source therefore states a unit that a dashboard can override.
The `PanelOptions` type holds that rule, so five renderers apply it in one way.

**The shared parts sit in one module.** `thunderbox-widgets/src/panel.rs` holds
the border, the three states that a panel reports, the reduce step, and the
step that cuts a name to a width. A renderer that repeated any of those would
drift away from the others.

---

## 6. How to reduce the number of points

Ask the data source to do it, as Grafana does. A panel that is N cells wide and
that uses a braille marker can address 2N positions. The panel therefore asks
for this number of points:

```
max_data_points = width_in_cells * sub_cells_across
```

The width in that formula is the width of one panel. It is not the width of the
area that holds every panel. thunderbox divides the grid of the dashboard onto the
cells of the terminal, and each panel then holds a width of its own. A panel of
8 grid columns of 24 holds about 37 cells of a terminal of 110 cells, and a
panel of 16 columns holds about 73 cells.

The point limit of each panel therefore differs from the point limit of the
panel beside it. `thunderbox_dashboard::grid::panel_width` returns the width of
one panel, and both the renderer and the runtime read that one function. Section
12 records that decision.

Compute the number again when the panel changes size, then query again. Pass the
number to the data source as the `step` of Prometheus, as `time_bucket` in SQL,
or in whatever form that data source needs.

The Prometheus data source does this work today. It turns the point limit into
the step of a range query. It raises that step in three ways, and it never
lowers it:

- it raises the step to the smallest step that the dashboard permits, which is
  the `min_step` option;
- it raises the step to the next round value, such as 2 seconds or 5 minutes.
  It then moves both ends of the range onto that step. The times of the points
  therefore stay still while the range moves, and a chart that refreshes does
  not shake;
- it raises the step again until the number of points fits both the panel and
  the limit of 11000 points that a Prometheus server holds.

The server returns a point for every step, and it returns no point where the
series holds no value. The data source builds the time column from the step,
and it puts each sample in its own place. A place that holds no sample keeps a
hole in the data, so a chart shows a gap and not a straight line.

No code of thunderbox reduces the number of points that a data source returns. The
field `SourceDescriptor::downsamples` promises that reduction for a data source
that cannot reduce the points itself. Every read of that field sits inside a
test module. `Cargo.lock` holds no crate that reduces a series. Item 7 of
section 13 records that absence, and it asks whether this project takes the
reduction.

The design that takes the reduction runs the `minmaxlttb` algorithm in thunderbox
**only when the data source cannot reduce the points itself**. A CSV file and a
plain SQL table are examples. The LTTB algorithm keeps the shape of a signal. A
system monitor instead shows the smallest and the largest value in each column.
That form never hides a short, high peak. Offer both, and use the smallest and
largest value for a signal that behaves like a meter.

---

## 7. The program that runs alone

The program that runs alone is **thunderbox with the built-in `sysinfo` data
source and a default dashboard**. Its look is a theme and a set of panels. It is
not a separate path through the code. The same renderer,
the same frames, and the same panels serve every way to run thunderbox. This
arrangement also gives a demonstration that needs one command.

The look stops at one point. The axis of time covers the window that a person
asked for, and not the data that a data source returned. Section 12
states that rule. The sysinfo data source holds nothing from before the program
started. The first minutes of every run therefore draw a short line at the
right edge. That picture is the truth about the data source, and the truth
comes before the look.

---

## 8. What to build, and when

**Version 1**

- a dashboard as a document on disk;
- a time range that every panel shares, and that a person can move and scale.
  The project has built this one;
- a refresh interval;
- a grid of panels;
- variables that a person selects. The project has built this one;
- these panels: TimeSeries, Stat, Gauge, BarGauge, Table. The project has
  built all five;
- these data sources: testdata, sysinfo, prometheus. The project has built all
  three;
- more than one data source on one dashboard. The project has built this one,
  and section 12 states why it cost no work.

**Later**

- more steps in the transform pipeline than `reduce`;
- a mode in which a person writes a query directly;
- notes that mark an event on a chart;
- the Heatmap and Logs panels.

**Never**

- alarms. That is a different product.
- storage, retention, and summary tables.

---

## 9. The data sources

| data source | crate | note |
|---|---|---|
| testdata | none | it makes test data. **Build this one first.** |
| sysinfo | `sysinfo` | it makes the program that runs alone work |
| prometheus | `prometheus-http-query` 0.9 | 751k downloads in 90 days, and in active development |
| PromQL text | `promql-parser` 0.10 | optional. It checks a query and completes it. |
| influxdb | `influx_db_client` 0.7, `influxdb3-client` | later |

---

## 10. How to add a data source later

Grafana runs a data source as a separate program, and it speaks gRPC to that
program. Do not build that now. Write each data source as an implementation of
the trait, and select it with a feature.

Keep `DataSource` free of any assumption that the data source runs in this
process. The request and the result are owned values, and the trait holds no
borrowed value. A later version can then run a data source in a separate
process, or as a WebAssembly module. Neither change touches a panel.

thunderbox will not load a shared library at run time. The rules that govern the
layout of a Rust value in memory are not stable between two compilers.

### The five duties of a data source

A data source has five duties, and it applies no pattern:

1. It gives each column a name. That name is different from the name of every
   other column of the answer.
2. It attaches one label to the column for each property that separates one
   series from another. The host and the job are two examples.
3. It ends every query in a limited time. It returns an error when it cannot
   answer. A data source that reads another machine states its longest time in
   `SourceDescriptor::answer_limit`.
4. It returns the moments of the time column in order, from the oldest to the
   newest.
5. It gives every column of one `Frame` the same number of rows.

The fourth duty holds because `thunderbox_core::Reduce` reads a position in the
column, and not a moment. `Reduce::Last` takes the last value, so the three
panels that show one number can read an old value from an answer in another
order.

The fifth duty holds because `Frame::time_series` returns an error when a number
column holds a different number of rows from the time column. The chart then
draws no line for that frame. The table draws the same frame, and it fills the
short column with the symbol of a value that is not there.

The third duty belongs to the data source, and not to the caller. A caller
stops a query at an await point. A data source that holds a thread of the
async runtime, and that awaits nothing, therefore runs to its end whatever the
caller does. The Prometheus data source keeps this duty already: it builds its
client with `reqwest::Client::builder().timeout(settings.timeout)`, and it
sends the same limit to the server.

The sysinfo data source and the testdata data source read this machine alone.
Neither awaits anything, so neither states a limit, and the limit of the runtime
cannot stop either of them. That pair is the one condition under which the limit
of the runtime does nothing.

The runtime holds each query to a limit as well, and section 12 states that
limit. The two limits do not compete. A data source states a limit that is a
little longer than its own timeout. The error of that data source therefore
reaches the panel, and that error names the server and the fault. The limit of
the runtime names a wait alone, and it serves a data source that states none.

The second duty asks one more thing of a data source whose column names already
separate one series from another. `Field::display_name` writes the labels of a
column beside the name of that column. A column named `0` that carries the label
`core` therefore draws as `0{core="0"}`. Such a data source states the short
name in `FieldConfig::display_name`, and a target that holds no pattern then
draws that short name.

The sysinfo data source states that name, because it names each column after the
position of the core. The Prometheus data source states none. Every column of
one metric carries the same name there, and the labels alone separate those
columns.

**The sysinfo data source attaches the label `core` to the signal `cpu.cores`,
and it returns the cores in the order of their position.** That signal is the
one signal of that data source which returns more than one series. The store
names each series `cpu.core.0` and so on, and it orders its names by the text.
A machine of ten cores or more therefore gave `cpu.core.0, cpu.core.1,
cpu.core.10, cpu.core.11, cpu.core.2` before this rule, and a panel drew that
order. `Signal::series_names` now reads the number after the last dot and
orders by it, so a person reads `core 0, core 1, core 2`. That change moves the
rows of a bar gauge and of a table, and README records it beside the signal.

A data source cannot always keep the first duty. The Prometheus data source
names a column after the text of the query when the answer holds no name of a
metric. Every column of that answer then carries the same name. The labels of
the second duty separate those columns, and a person writes a pattern that
reads a label.

A person states a pattern in the `legend` field of a target.
`thunderbox-dashboard` applies that pattern after the data source answers, and
`thunderbox_core::legend` holds the rule. A `Query` carries no pattern, so a data
source cannot read one and cannot apply one.

An author of a new data source therefore gets the correct behaviour without a
line of code for it. The reserved word `{{__name__}}` reads the name of the
column, so a data source that attaches no label still obeys a pattern.

This choice also serves the two processes of section 11. A pattern belongs to
the side that draws. It never crosses the boundary between the two processes,
so a data source on another machine needs no change.

---

## 11. How the two processes speak

One connection, two kinds of message. The same SSH channel carries a query and
its answer in the "client and server" way. It carries the characters of the
terminal in the SSH way. Both ways get their password check and their encryption
from SSH. The project therefore writes one connection layer, and not two.

Put a length in front of each message and encode the message with `postcard`.
gRPC through `tonic` is more machinery than a link between two programs needs.

For the SSH way, follow `russh/examples/ratatui_app.rs`. That example writes to
an object that implements `io::Write`, and that object collects the bytes and
sends them with `handle.data()`. The example sets `Viewport::Fixed` from
`pty_request` and from `window_change_request`. Read the keys with
`termwiz::input::InputParser`. Keep one host key in a file, answer
`shell_request`, set `inactivity_timeout`, and limit how many people can connect
at one time.

---

## 12. The decisions that the project has taken

**The file format of a dashboard is TOML. A reader for the Grafana JSON format
comes later.** A native format fits the panels and the data sources of this
project. A reader for the Grafana format is an addition that does not change the
native format. This choice keeps the model in this project, and it does not tie
version 1 to a large format that another project controls and changes.

**Both ways to put thunderbox inside another program are in scope.** They are two
layers, and not two choices.

The smaller way gives the renderers to a program that already holds its own
data. That way is `thunderbox-widgets`, it runs no query, and the program supplies
each `Frame`. The larger way gives a whole dashboard to a program, and thunderbox
runs the queries and refreshes them. That way is `thunderbox-dashboard`, it sits
on top of the first way, and the program supplies a `tokio::runtime::Handle`.

This choice makes one phrase true: one program and one library, both a client
and a server. The same `thunderbox-dashboard` type serves the program that runs
alone, the SSH server, and another program that holds a thunderbox dashboard. Only
the owner of the terminal changes.

`thunderbox-dashboard` must therefore behave as a library:

- **It never makes an async runtime.** It takes a `Handle`. It uses no
  `#[tokio::main]` and no `Runtime::new()`.
- **It never owns the terminal.** It sets no terminal mode, it opens no second
  screen, and it reads no standard input.
- **It never owns the key bindings.** `handle_key` reports whether it used the
  key. The program keeps its own bindings for every key that thunderbox ignores.
  The program also decides which panel a person has selected.
- **It changes nothing outside itself.** It installs no logger, no panic
  handler, and no signal handler.
- **It stops its work when it drops.** It ends each background task. It leaks
  none of them.
- **It never blocks the code that draws.** A query publishes its result to the
  side. The code that draws then reads the most recent result without a wait. A
  slow query or a failed query changes one panel, and never the whole picture.

**A program that holds a thunderbox dashboard must itself hold four rules.** The
list above states what the library never does. This list states what the program
must do, and no other file stated it before.

- **The async runtime must run a task on a thread other than the thread that
  draws.** `DashboardRuntime::new` asks for a `Handle` that holds the timer of
  tokio, and a runtime of the flavour `current_thread` holds that timer. It is
  still not enough. The runtime starts a task for each panel, and the loop that
  draws never awaits. `crates/thunderbox/src/main.rs` waits for a key inside
  `event::poll`, which parks the thread. A runtime of one thread therefore polls
  no query task, and every panel reads `loading…` for ever. No error reaches the
  screen, and no limit ends the wait.

  The measurement: the same loop against the testdata data source answers under
  `Builder::new_multi_thread`, and it never leaves `PanelState::Loading` under
  `Builder::new_current_thread`.

  Both programs of this repository call `Builder::new_multi_thread`. They are
  `crates/thunderbox/src/main.rs` and
  `crates/thunderbox-dashboard/examples/full_embed.rs`. Neither one states the
  reason, so a reader of those two files learns nothing about this rule.

- **A method that starts a new set of queries needs a guard for the value that
  did not change.** Both programs call `set_marker` on every frame.
  `DashboardRuntime::set_marker` compares the new marker against the marker that
  it holds, and it starts nothing when the two are equal. That comparison is the
  reason the call is safe on every frame. A reader who deletes it sees a simpler
  method, and every test stays green.

  The measurement: without the comparison, ten frames over 200 milliseconds
  start ten sets of queries, and no panel leaves `PanelState::Loading`. With the
  comparison the same ten frames start one set, and the panel answers.

- **`Dashboard::validate` checks a source name only for a dashboard that
  declares a `[source]` table.** Both checks of the name stand behind
  `!self.sources.is_empty()`. A dashboard that declares no such table therefore
  accepts any name at all. That shape is the ordinary shape of an embedded
  dashboard. The program builds its own `Registry`, and the dashboard then
  names no source of its own. The mistake then reaches the screen as a sentence
  in every panel, and not as one line before the program opens the terminal.
  `full_embed.rs` holds the name twice, once in its dashboard and once in its
  registry, and it keeps the two the same by hand.

- **A `watch::Receiver` that outlives the runtime reports `refreshing` for
  ever.** `Drop for DashboardRuntime` raises the generation and ends the task,
  and it writes nothing to the channel. `ClearOnEnd` then finds a generation
  that does not match, and it writes nothing either. A program that drops one
  dashboard and makes another one is the ordinary way to read a dashboard file
  again. A program that reads `is_refreshing()` on a receiver of the old
  dashboard reads `true` for the rest of the process. A program that awaits
  `changed()` meets `Err` at once, because the sender drops with the runtime.

**A data source never reads the pattern of the `legend` field. The dashboard
applies that pattern after the data source answers.** The pattern names a
series, and a name is a matter for the side that draws.

The project took this decision because only one data source of three obeyed the
pattern. The other two ignored the `legend` field. `thunderbox_core::Query` held
the pattern then, so every data source received one, and each author had to
remember to apply it. The pattern now stays on `thunderbox_core::Target`, and
`Target::to_query` drops it. The compiler therefore rejects a data source that
reads a pattern, and a comment no longer has to ask for the correct behaviour.

`thunderbox_core::legend` holds the one rule, and `run_panel` is the one place
that applies it. That place sees both a target and the answer of that target.

**A refresh keeps the answer that each panel holds. The runtime puts a panel
back to the state that has no answer only when that answer describes another
window.**

The dashboard put every panel back to the state that has no answer on each
refresh. The default dashboard refreshes each second, so a reader met the word
`loading…` in every panel many times a minute. The comment beside the code said
that this step keeps an old answer away from a new one.

That reason does not hold. `run_all` builds one whole `Snapshot`, and the
runtime publishes it in one call. Two panels can therefore never come from two
different sets of queries. A set of queries is the whole group of queries that
one refresh sends.

A window is the pair of moments that a time range names, and the moment that
named them. The range `the last five minutes` names a new window on each
refresh. `thunderbox_core::TimeWindow` holds all three values, and a panel needs
all three. A person who moves the range makes the end of the window and the
present moment two different moments.

The runtime now writes `true` into `Snapshot::refreshing` before it starts a set
of queries, and the end of that set writes `false`. The field is not a count of
the queries. A dashboard whose panels hold no target sends a set that holds no
query, and the field reports that set as it reports any other.
`Snapshot::is_refreshing` reads the field. The program draws a status row from
it, and the row reports a set of queries that waits for longer than one second.
A data source that answers at once therefore changes no text.

The task of a set of queries writes that `false` even when it publishes no
answer. A data source that panics ends its task, and the runtime must not wait
for a set of queries that no longer runs.

`DashboardRuntime::refresh_age` reports the time that the set which runs now has
waited, and the program reads that time. The row therefore reports a data source
that answers slowly, and it never reports the age of the frames of the program.

`DashboardRuntime::set_time_range` puts every panel back to the state that has
no answer when the new range differs from the range before. Those answers
describe another window, and a reader would take them for the new window. The
runtime compares the two expressions of the range, and not the two windows that
they name. A range that says `the last five minutes` therefore keeps its
answers.

`DashboardRuntime::tick` starts nothing while a set of queries waits for an
answer. A data source that is slower than the interval now finishes one set of
queries. A tick no longer stops a set of queries before its answer arrives. A
call to `refresh`, to `set_marker` or to `set_time_range` still stops the set
that runs, and so does the first call to `set_panel_area`.

**The runtime holds one query to a limit, and it holds no set of queries to
one.** `run_panel` wraps each call to `DataSource::query` in
`tokio::time::timeout`. A query that passes the limit gives its own panel an
error, and that error names the data source and the wait. Every other panel
keeps its answer.

That rule removes a cost that this project carried before. A data source that
never answered stopped every later tick, because `Snapshot::refreshing` never
returned to `false`. Each query now ends, so the task of the set ends as well,
and that field returns to `false`. A program that drives `tick` alone needs no
way out of its own.

The limit sits on one query for three reasons:

- A limit on the whole set would throw away every panel that had already
  answered. This section promises that a slow query changes one panel alone.
- A person reads the name of the data source that did not answer. A set of
  queries that the runtime stopped names none.
- A dashboard that states no refresh interval never ticks for the clock. A
  limit inside `tick` would never reach such a dashboard.

That choice has a cost, and this section states it. One set of queries takes the
longest of the limits of its panels, because `run_all` runs the panels at the
same time. A dashboard of four panels, where each panel names a data source that
never answers, holds one set of queries for that one limit. `tick` starts
nothing in that time, and it gives no width to a data source in that time
either. A limit on the whole set would cut almost none of that time. It would
also cut a dashboard whose four panels each answer slowly and correctly. The
project therefore keeps the limit on one query.

The value of the limit comes from the data source, and never from the refresh
interval. The two answer different questions. The refresh interval states how
fresh a person wants the numbers, and a dashboard can state `1ms`. The limit
states how long a data source may take. `SourceDescriptor::answer_limit` holds
it, and `thunderbox_dashboard::DEFAULT_ANSWER_LIMIT` of 60 seconds serves a data
source that states none.

The runtime therefore needs the timer of tokio. The caller must supply a
`Handle` of an async runtime that holds that timer. `#[tokio::main]` and
`Runtime::new` each build such an async runtime, and a caller that builds its
own calls `enable_time` or `enable_all` on the builder. The task of a set of
queries panics on an async runtime that holds no timer. The timer is not the
only need. The async runtime must also run a task on a thread other than the
thread that draws. A runtime of the flavour `current_thread` holds the timer,
and every panel on it still reads `loading…` for ever. An earlier item of this
section states that rule in full. Its title reads "A program that holds a
thunderbox dashboard must itself hold four rules." The rule is the first of the
four. `#[tokio::main]` and `Runtime::new` each call `Builder::new_multi_thread`,
so an async runtime from either one holds that rule as well. A `#[tokio::main]`
that names `flavor = "current_thread"` calls `Builder::new_current_thread`
instead, and it breaks the rule.

**A task that ends with no answer gives an error to the panels that it asked
about.** A task ends that way when a data source panics, and also on an async
runtime that holds no timer.
The runtime wrote `false` into `Snapshot::refreshing` for such a task before
this rule, and it wrote nothing else. Every panel then kept `loading…` for ever
when the first set of queries ended that way. The status row reported no error
and no wait. `ClearOnEnd` now writes the error as well, so a person reads the
fault. The runtime raises the generation number before it stops a task, so a
task that the runtime stopped writes nothing.

A set of queries asks about the panels that the renderer draws, and it asks
about no other panel. `ClearOnEnd` therefore writes the error into those panels
alone, and it leaves a panel that the renderer drops as it is. A later item of
this section states that rule in full. Its title reads "A set of queries holds
one job for each panel, and it names the skip."

The limit stops a query at an await point alone. A data source that holds a
thread of the async runtime, and that awaits nothing, runs to its end. The
third duty of a data source covers that condition, and section 10 states it.
The runtime cannot keep a promise for such a data source, and this document
does not make one.

**A new area waits, and the next set of queries carries it.**
`DashboardRuntime::set_panel_area` started a set of queries at once before this
rule. A caller reports the area on every frame. A person who dragged the corner
of a terminal window therefore started and stopped a set on every frame. A data
source on another machine received one request for each frame, and the runtime
stopped every one of them but the last.

The runtime now holds a new area. `tick` gives that area to a new set of
queries when no set of queries runs, and when one of these two conditions
holds:

- the area has stood still for `AREA_QUIET_TIME`, which is 250 milliseconds;
- the area has waited for `AREA_MAX_HOLD`, which is one second.

This document calls that first time the quiet time. The step comes before the
check of the refresh interval. A dashboard that states no refresh interval
therefore receives a new area as well.

The second condition serves a caller whose area never stands still. A panel
area that grows and shrinks by one cell is one example, and a scroll bar that
appears and disappears makes one. The quiet time alone would hold such an area
for as long as it moved. The point limit would then stay wrong for that whole
time. The runtime records the moment when the wait of an area began, and a later
change does not start that wait again.

`area_wait_since` holds that moment. The runtime writes the moment when it arms
an area that costs a set of queries and no wait is open. An area that changes no
point limit makes no point limit wrong, so it opens no wait. A person can move
the bottom edge of a window for longer than `AREA_MAX_HOLD`. Such a caller would
otherwise leave a wait open. The next area that changed a point limit would then
reach a data source at once.

The runtime writes `None` at two steps. A set of queries that starts writes it,
because that set carries the area which waited.
`end_the_wait_of_a_settled_area` writes it when no area waits and the area of the
caller has stood still for `AREA_QUIET_TIME`.

Two steps leave no area in the wait, and neither one ends the wait by itself.
`tick` takes an area that costs no set of queries. `set_panel_area` drops the
area that waits when a caller reports the area in use again. A caller that still
moves would otherwise start the wait again on its next report.

That rule matters, because a caller can report a size that needs no set of
queries. The item "An area that changes no point limit starts no set of queries"
below states which sizes those are. A caller can report one such size on one
frame, and a size that needs a set on the next. Such a caller would otherwise arm
and clear the area on alternate frames. Each arm would start the wait again, so
neither the quiet time nor `AREA_MAX_HOLD` could pass. That area would then never
reach a data source.
The runtime held that fault before this rule, for the one size that equals the
area in use. The tests `an_area_that_moves_back_to_the_area_in_use_still_arrives`
and `an_area_that_moves_in_and_out_of_the_point_limit_in_use_still_arrives` hold
both shapes to the longest hold.

The quiet time ends the wait, and a step that leaves no area in the wait does
not. A caller that reports the area in use again would otherwise leave the wait
open for ever. Every later area would then reach a data source on its first
frame. A person who drags a corner would pay a set of queries at a width that
they move past. The tests
`a_new_area_waits_after_a_caller_dropped_the_area_that_waited`,
`a_new_area_waits_after_an_area_that_cost_no_set_of_queries` and
`a_set_of_queries_ends_the_wait_of_an_area` hold the three steps that end a
wait.

The width and the height wait together, in one field of the runtime. A person
who drags the corner of a window changes both numbers, and the two numbers
rarely stand still on one frame. Two fields, each with a wait of its own, would
therefore cost two sets of queries for one drag. `area_is_due` also asks that no
set of queries runs, so the second of those two sets would wait for the first to
answer. The test `a_run_of_new_areas_costs_one_set_of_queries` drags a corner
across thirty frames and holds the cost at one set.

The quiet time does not grow with the refresh interval, because the two answer
different questions. The interval states how fresh a person wants the numbers.
The quiet time states how long a person waits after a change of the size of the
window. A dashboard of `refresh = "10s"` would hold a new area for ten seconds
under any rule that read the interval.

The first area that a caller reports reaches the data source at once. The
runtime runs its first set of queries before any caller draws a frame. That set
carries `ASSUMED_WIDTH`, which is a panel area of 80 cells, and it asks about
every panel. That first report is not a change of the size of the window, and
the first picture must hold the correct number of points. `ASSUMED_WIDTH`
therefore serves one set of queries alone, which is the set that runs before a
caller knows the area.

That first report costs nothing when it changes no point limit. The runtime
compares the point limit of each panel, and it starts a set only when one of
those limits changed. A caller whose terminal holds 80 cells therefore starts no
second set of queries.

**An area that changes no point limit starts no set of queries.** The width of
an area decides the point limit of each panel. The height decides which panels
the renderer drops, and it decides nothing else that a query carries. An area
that changes neither would therefore ask each data source for exactly what the
set of queries before asked.

`DashboardRuntime::limits_for` returns the point limit that each panel would take
on any area, and `point_limits` reads it for the area in use. `tick` compares the
two lists when the area that waits falls due. It starts a set of queries when the
two differ, and it takes the area without a set when they agree.

The comparison reads the panels that the new area draws, and it reads no other
panel. A panel that the new area drops asks a data source for nothing. A point
limit that such a panel loses therefore costs no set of queries. A height that hides
one panel of a column therefore leaves every other panel of that column with the
answer that it holds. A dashboard of eight panels in one column sent 7 queries
for that height before this rule, and it now sends none.

A panel that the new area draws and that the area before dropped costs a set of
queries. The runtime holds no point limit for such a panel, so it cannot know
whether the answer that the panel holds has the right shape. The test
`a_panel_that_the_terminal_grows_to_hold_asks_a_data_source_again` holds that
case, and `a_height_that_only_drops_panels_starts_no_set_of_queries` holds the
other one.

A set of queries takes the error of each panel that it drops, and
`kept_state` states that rule. A height that only drops panels starts no such
set, so `tick` must take that error itself.
`clear_the_error_of_each_panel_that_the_area_drops` does so. It writes into the
channel only when it changes a panel. It takes the error of a panel that the
renderer draws from no panel at all. `Snapshot::error_count` would otherwise
count an error that no later set of queries can clear.

`tick` holds the comparison, and `set_panel_area` does not. The wait of an area
therefore keeps every rule that the item above states. A comparison inside
`set_panel_area` would clear the area that waits on each frame that needs
nothing. The wait of a caller that moves between the two sizes would then never
end.

The runtime writes an area that costs no set of queries into the field `area`
all the same. That field carries the point limits of the set of queries that
runs now, and an area that changes no limit carries the same ones. A runtime
that dropped such an area would compare every later area against an area that no
caller reports.

The measurement: a dashboard of two panels on a terminal of 80 cells by 40
rows sent 2 queries. A change to 80 cells by 41 rows sent 2 more queries, and
each one carried the same point limit of 80. It now sends none.

One consequence belongs to this rule. A height that changes nothing no longer
asks a data source again, so it no longer clears the error of a panel that
returned one. A dashboard that states a refresh interval clears such an error at
that interval. A dashboard that states none clears it on one of five steps: the
key `r`, a new marker, a new time range, a new choice of a variable, or an area
that changes a point limit.

Each start of a set of queries takes the area that waits, whatever started that
set. `refresh`, `set_marker`, `set_time_range`, the key `r`, and `tick` all
carry the newest area. A change of the marker after a change of the area sends
one set of queries, and that set carries both.

Each task carries the generation of the set of queries that started it, and it
publishes its answer only while that generation still runs. The runtime raises
the generation, and each task compares it, inside one call to
`send_if_modified`. The lock of the channel therefore holds the check and the
publish together.

**Each panel asks its data source for the points of its own width.** A dashboard
holds a grid of 24 columns, and the renderer divides that grid onto the cells of
the terminal. A panel of 8 columns therefore holds about a third of the screen.

Every panel asked for the points of the whole panel area before this change. A
panel of 8 columns asked for about three times the points that it draws. The
runtime held one number, it read that number one time, and `run_all` gave that
one number to every panel.

`thunderbox_dashboard::grid::panel_width` now returns the width of one panel. The
runtime reads that function for the point limit of each panel. The renderer
reads the same function inside `place_panel`, for the place of that same panel.
One function returns both numbers, so a panel cannot ask for the points of a
width that it does not draw. The test
`the_place_of_a_panel_holds_the_width_that_the_point_limit_reads` holds the two
numbers together at every width from 0 cells to 300 cells.

`DashboardRuntime::set_panel_area` carried the name `set_panel_width` before
this change, and then the name `set_panel_area_width`. Both callers of that
method always reported the width of the whole area, so the first name named the
wrong thing. A reader of the newest name asks no question about which panel the
number describes. The passage below states why the third name is the last one.

`run_all` took one point limit for the whole set of queries. It now takes one
`PanelQueries` value for each panel. Each value holds the targets of a panel
beside the point limit of that same panel. The two fields travel together, so no
panel can take the limit of another panel.

This table holds the default dashboard on a terminal of 110 cells. The column
`before` holds the points that a panel asked for before this change. The column
`after` holds the points that it asks for now.

| panel | kind | grid columns | cells | before | after |
|---|---|---|---|---|---|
| cpu | timeseries | 16 | 73 | 220 | 146 |
| cpu now | stat | 8 | 37 | 220 | 74 |
| memory | gauge | 8 | 36 | 220 | 72 |
| cores | bargauge | 8 | 37 | 220 | 74 |
| network | timeseries | 8 | 37 | 220 | 74 |
| the most recent rows for each core | table | 24 | 110 | 220 | 220 |
| the whole dashboard | | | | 1320 | 660 |

Two numbers of that table need a word. The panel `memory` holds 36 cells and the
panel `cores` holds 37 cells, and both of them hold 8 grid columns. `scale`
multiplies before it divides, so the width of a panel follows `x` as well as `w`.
The panel on the right takes the remainder of the division. The panel of the
table holds the number that it held before, because that panel holds the whole
width of the area.

The dashboard asks for half of the points that it asked for before. It holds six
panels in three rows, and the sum of the widths of one row is the width of the
area.

Every kind of panel follows this one rule:

- a chart draws a point at each position that the marker can address inside its
  own width;
- a stat panel draws a small chart of the most recent points, across its own
  width;
- a gauge and a bar gauge read the most recent point of the answer. A data
  source returns that point at every limit. `History::range` of the sysinfo data
  source always takes the newest sample, and a Prometheus server returns the
  last step before the end of the range. A coarser step moves that point by less
  than the width of one step;
- a table shows the last rows that fit in its height. The point limit of a
  narrow table can fall under that number of rows, and such a table then shows
  fewer rows. The renderer draws no panel under 3 cells wide, and a table of 3
  cells asks for 6 points with a braille marker.

A rule that gave the last three kinds a limit of their own would be a second
rule. A reader of the code would then need to know which panel follows which
rule. `README.md` could also no longer state one sentence about the whole
program.

The width of a panel counts the border of that panel. It also counts the column
of cells that a chart keeps for the labels of the axis of values. A panel
therefore asks for a few points more than it draws, and never for fewer.
thunderbox keeps that margin. A panel that asked for fewer points than it draws
would lose a shape that the data holds.

**A panel that the renderer drops asks a data source for nothing.** The
renderer draws no panel under 3 cells wide or under 3 cells high. This document
calls such a panel a panel that the renderer drops. The runtime knew the width
of the panel area alone before this change, and it knew no height. It therefore
asked a data source about every panel, and a person paid for a query whose
answer nobody read.

`thunderbox_widgets::draws_a_panel` now holds the smallest size. That function
returns `true` when a panel of a size holds a cell for its data. The rule
belongs to that crate, because every panel of that crate draws nothing at a
smaller size. The number 3 lived in seven places before this change: five panels
of `thunderbox-widgets`, and two lines of `view.rs`. It now lives in one constant
of `thunderbox-widgets/src/panel.rs`, and every one of those seven places reads
the function.

`thunderbox_dashboard::grid` holds the arithmetic that gives a panel its size:

- `rows_of_grid` returns the rows of the grid of a dashboard;
- `panel_size` returns the size of one panel, in terminal cells.

`place_panel` reads both for the place of a panel.
`DashboardRuntime::point_limits` reads both for the point limit of that same
panel. Each of the two then gives that size to `draws_a_panel`.

The test `the_runtime_asks_about_exactly_the_panels_that_the_renderer_draws`
holds the two parts together. It draws the dashboard into a buffer and counts
the titles that the renderer wrote. It then counts the queries that reached a
data source. The two counts must agree at each height from 3 rows to 30 rows.

**The rows of the grid belong to the dashboard, and not to the area.**
`place_panel` reads `dashboard.panels` for that number, and it reads nothing of
the area. The area supplies its height alone, and `scale` divides the rows of
the dashboard onto that height. The runtime therefore needs the height of the
area and nothing else.

`rows_of_grid` adds `y` and `h` with `u16::saturating_add`. `place_panel` added
them with a plain addition before this change. A dashboard of `y = 65535` then
ended the program, and `Dashboard::validate` accepts such a dashboard. That
fault reached the renderer alone before this change. The runtime now reads the
same arithmetic, so that fault would reach `DashboardRuntime::new` and the task
of a set of queries. This branch therefore removes it.
`Dashboard::validate` now adds `x` and `w` with
`u16::saturating_add` as well. That check ended the program on a panel of
`x = 65535`.

**`PanelState` gains no fourth variant for a panel that the renderer drops.**
Such a panel holds `PanelState::Loading` until a data source answers it once,
and it holds the answer of that data source after. `thunderbox_widgets::PanelData`
gains no fourth variant either.

A fourth variant would carry a fact about one view inside the data of a
dashboard. `run_all` builds a whole snapshot, and it knows nothing of a layout.
A view holds no data, so one program can draw one snapshot into two areas of
different size. A snapshot that said `panel 3 is too small` would then be true
of one area and false of the other. The variant would also reach every `match`
of every program that holds a thunderbox dashboard. `error_count` would then count
it as no error, exactly as it counts `Loading` as no error.

**A panel that the renderer drops keeps the answer that it held.** The set of
queries that drops the panel writes that answer back, and `PanelJob::Skip`
carries it. The runtime asks no data source about such a panel. A set that
wrote `Loading` into it would draw `loading…` there until the terminal grows
again. A person who makes a terminal smaller and then larger would meet that
text in every panel. The answers of the panels that the renderer still draws
would stand beside it.

`runtime::kept_state` states the two answers that a panel must not keep, and it
returns `PanelState::Loading` in their place.

The first is an answer of another window. `DashboardRuntime::set_time_range`
reports `Kept::Nothing` for a window that differs, and the answers of the set
before then describe the window before. This section already promises that the
fields `panels`, `range`, `variables` and `updated_at` of one snapshot come from
one set of queries. An answer of an older window would break that promise, and
it would also mislead a person. The snapshot holds one window for every panel. A
chart of older frames would draw a line that stops before the right edge of that
window. A person reads that shape as a data source that died.

The second is an error. A panel that the renderer drops reaches no data source,
so no later set of queries can clear an error that it holds.
`Snapshot::error_count` would count that error for as long as the terminal stays
small, and the status row would name a fault that nobody can repair.

This rule has a cost, and the doc comment of `Snapshot` names it. That comment
promises that the fields `panels`, `range`, `variables` and `updated_at` come
from one set of queries. A panel that the renderer drops now breaks that
promise, because its answer comes from the set of queries that last asked about
it. The comment states that one exception.

The project examined the other choice, which writes `PanelState::Loading` into a
dropped panel. That choice keeps that promise. It also draws `loading…` in
every panel that the terminal grows to hold again, and it draws that text until
one set of queries answers. A person who makes a terminal smaller and then
larger meets that text on every change of the size. The project took the answer.
The window of that answer is the window of the snapshot, and `kept_state` holds
the two cases in which it is not.

**The caller reports one area, in one method and one value.**
`DashboardRuntime::set_panel_area` takes a `ratatui_core::layout::Size`. The
caller must report the area that it gives to `DashboardView::render`. Both
programs of this repository write one line, and that line names the same value
that they draw into.

The project rejected three other shapes:

- A method of two `u16` values lets a caller pass the height first, and the
  compiler reports nothing. A body of 110 cells by 31 cells would then reach the
  runtime as 31 cells wide by 110 cells high, and every panel would drop.
- A second method beside `set_panel_area_width` lets a caller report the width
  alone. That caller keeps the old behaviour, and the compiler reports nothing.
- A `Rect` carries an `x` and a `y` that the runtime must ignore, and a caller
  cannot see that. A pane that moves without a change of size would also start a
  set of queries.

`Rect` already stands in the public interface of this crate, because
`DashboardView::render` takes one. `Size` therefore adds no new dependency for a
caller. The name of the method states the thing that a caller reports, which is
the area that the caller gives to `DashboardView::render`. Every future number
about that area still fits that name, so this is the last name of this method.

The runtime never measures, and the caller always reports. That rule keeps the
promise that this crate never owns the terminal. The runtime also holds no rule
of a layout of its own. It reads `thunderbox_dashboard::grid`, and the renderer
reads the same module.

**The runtime asks about every panel until a caller reports an area.** It holds
`None` for the area before that report, and it holds no assumed height. A height
that this project guessed could be too small, and the first picture of a caller
would then hold no panel. A caller that never reports an area also keeps every
panel. The tests of `crates/thunderbox-dashboard/tests/` are such a caller. A
program that holds a dashboard behind a screen that it does not draw is another
one. The test `a_runtime_that_receives_no_area_asks_about_every_panel` holds
that rule.

**A set of queries holds one job for each panel, and it names the skip.**
`run_all` takes a `PanelJob` for each panel, and that value is either
`PanelJob::Ask` or `PanelJob::Skip`. The snapshot then holds one state for each
panel, in the order of the dashboard. `Snapshot::panel` reads a panel at the
position that the dashboard gives it. A list that held no entry for a dropped
panel would give panel 1 the answer of panel 2. The last panel would then hold
nothing at all. No test would report that fault, because `Snapshot::panel`
returns a state that waits for an index past the end of the list.

A skipped panel is not a panel of an empty list of targets. `run_panel` gives a
panel of an empty list an answer that holds no frame, and that panel draws
`no data`. A dashboard can declare a panel that holds no target, and the test
dashboard `ONE_PANEL` is such a panel.

`ClearOnEnd` reports the end of a set of queries that ended with no answer. It
now writes the error into the panels that the set asked about, and it leaves a
panel that the renderer drops as it is. No query of such a panel ran, so the
text of that error would name a fault that the panel never met.
`Snapshot::error_count` would also count a panel that nobody draws. The panel
keeps the state that `DashboardRuntime::start` gave it, which is the answer that
it held, or `PanelState::Loading` when the window changed.

**`Snapshot::is_loading` gains a third case, and that case has no end of its
own.** The method returns `true` while every panel holds `PanelState::Loading`.
Its doc comment names three cases, and the third is the whole time in which the
renderer drops every panel. The runtime asks no data source about a panel that
the renderer drops. No answer can therefore arrive for such a panel while the
renderer drops it. When every panel holds `PanelState::Loading`, the method
returns `true` for as long as the renderer drops every panel.

The third case reaches a terminal that was too small before the first answers
arrived. It also reaches a terminal that is too small when the runtime puts
every panel back to the state that has no answer.
`DashboardRuntime::set_time_range` does so for a new time range, and
`DashboardRuntime::set_variables` does so for a new value of a variable.
`kept_state` of `crates/thunderbox-dashboard/src/runtime.rs` then returns
`PanelState::Loading` for each panel that the renderer drops, whatever state
that panel held. The test
`a_new_window_puts_a_panel_that_the_renderer_drops_back` holds that rule.

The third case does not reach a terminal that shrinks after the panels
answered. The runtime keeps the answer of a panel that the renderer drops, so
each panel stays at `PanelState::Ready` and the method returns `false`. The test
`a_panel_that_the_renderer_drops_keeps_its_answer` holds that rule. A panel
that held an error is the one exception. The item `A panel that the renderer
drops keeps the answer that it held` above states why such a panel loses its
error.

`Snapshot::is_loading` alone therefore cannot tell a program that the terminal
is too small. A program that draws a message about a small terminal reads
`DashboardView::dropped_panels`. It reads that method whatever
`Snapshot::is_loading` returns. `crates/thunderbox/src/main.rs` does so, and the
doc comment of `Snapshot::is_loading` states this rule. Such a program must
hold four other rules as well. The list under the item `A program that holds a
thunderbox dashboard must itself hold four rules` states them. That item stands
earlier in this section.

`DashboardView::dropped_panels` returns the number of panels that the renderer
drops in an area. It reads the rule that `DashboardView::render` reads, so the
count describes the screen beside it. The status row of the program that runs
alone now writes one sentence from that count. That sentence reads
`The terminal is too small for 3 panels.` A person who read an empty screen
learned nothing before this change.

Two tests hold that count against the renderer. The test
`the_count_of_the_dropped_panels_holds_the_panels_that_the_renderer_leaves_empty`
draws a dashboard into a buffer at many sizes of a terminal. It then counts the
panels that hold no cell, and that number must equal the count. A rule that
reaches `DashboardView::render` alone therefore fails this test. The test
`the_count_of_the_dropped_panels_holds_the_panels_that_are_under_three_cells`
states the smallest size a second time, and it reads an area that starts away
from the first cell.

**What the change saves.** This table holds a dashboard of eight panels in one
column, on a terminal of 110 cells. The program keeps one row of that terminal
for the status row, so the panels hold one row fewer. The test
`a_column_of_eight_panels_drops_these_panels_on_each_terminal` holds the same
numbers.

| rows of the terminal | panels drawn | queries saved |
|---|---|---|
| 25 and above | 8 | 0 of 8 |
| 24 | 7 | 1 of 8 |
| 23 | 6 | 2 of 8 |
| 22 | 5 | 3 of 8 |
| 21 | 4 | 4 of 8 |
| 20 | 3 | 5 of 8 |
| 19 | 2 | 6 of 8 |
| 18 | 1 | 7 of 8 |
| 17 and below | 0 | 8 of 8 |

The default dashboard of six panels saves nothing on a terminal of 10 rows or
more. It saves 2 queries of 6 on a terminal of exactly 9 rows, 5 queries on a
terminal of 8 rows, and 6 queries on a terminal of 7 rows or fewer. The test
`the_default_dashboard_drops_a_panel_only_under_a_terminal_of_ten_rows` holds
those numbers.

The change therefore saves a query only on a terminal that already hides part of
the dashboard. The `too_small` scene of `scripts/record-demo.py` records such a
terminal. That scene runs a dashboard of three panels on a terminal of 90 cells
by 19 rows. The renderer drops the first panel, so the runtime saves 1 query of
3. Every other scene of this project records a terminal that is large enough for
every panel, and each one saves no query.

The case that saves every query is a caller that reports an area of no cells. A
dashboard behind a tab that a person hid is such a caller, and it paid for every
query of every refresh before this change.

**Three things that this change does not do.** Each one is work for a later
version:

1. The renderer leaves the rows of a dropped panel empty. A person on a terminal
   of 24 rows loses 2 of the 23 rows of the body to a panel that nobody sees. A
   later version can give those rows to the panels that the renderer draws.
2. The key `Tab` still selects a panel that the renderer drops. A person then
   presses a key and the screen does not change. `DashboardView::handle_key`
   holds no area today, so a rule that skips such a panel needs the area of the
   last frame.
3. `Dashboard::validate` bounds `x` and `w` against the columns of the grid, and
   it bounds neither `y` nor `h`. The grid states no number of rows, so this
   project has no bound to check against. `rows_of_grid` saturates, so no such
   dashboard ends the program. That rule holds on the path of the grid alone.
   `parse_duration_ms` of `crates/thunderbox-core/src/time.rs` ends with
   `Ok(n * mult)`, and that multiplication does not saturate.
   `Dashboard::validate` calls that function for the `refresh` field. A
   `refresh` of `20000000000w` therefore ends the program with an overflow of
   an integer before the program opens the terminal. An overflow is a result
   that does not fit in its type. A build for release wraps the number instead,
   and the dashboard then refreshes at a period that nobody asked for. Item 6
   of section 13 records that fault, with its measurement.

**A change to this path breaks a rule of an earlier change, and the suite stays
green.** Three pull requests in a row altered the way from
`DashboardRuntime::set_panel_area` to `run_all`. Each one broke a rule that an
earlier change had written, and no test reported the fault. A reader found each fault by a measurement of the program
that runs, and no reader found one when they read the code.

| the rule that broke | the change that broke it |
|---|---|
| a panel that the renderer drops keeps the answer that it held | the first design of the skip wrote `PanelState::Loading` in its place |
| a panel that the renderer drops holds no answer of another window, and no error | the skip wrote back every state that a panel held |
| `AREA_MAX_HOLD` ends the wait of an area | the comparison of the point limits dropped the area that waits |
| `AREA_MAX_HOLD` ends the wait of an area | a tick that took a free area ended the wait of a costly one |
| a new area waits for `AREA_QUIET_TIME` | a caller that dropped the area which waits left the wait open |
| a panel that the renderer drops holds no error | a height that only drops panels started no set of queries |

Two facts follow, and a writer who alters this path must hold both.

The rules of this path live in more than one step. A rule of `panel_jobs` also
binds `tick`, and a rule of `kept_state` also binds
`clear_the_error_of_each_panel_that_the_area_drops`. A change that moves work
from one step to another therefore carries every rule of the step that it left.

No reader finds these faults in the code alone. Take each rule out of the code,
run the whole suite, and read which tests fail. A rule that no test holds is a rule that
the next change will break. Each of the six rows above now holds at least two
tests.

**No text of this project writes the password of an address.** A person can
write a user name and a password into the `url` field of a Prometheus data
source. The client reads that pair and builds a header of the kind `Basic` from
it, so the address is one place that a password lives.

Two texts wrote that address in full before this rule. The `Debug`
implementation of `Prometheus` wrote `http://alice:s3cret@127.0.0.1:9090/`, and
a log message, a panic message and the report of a test each hold that text.
`Prometheus::new` wrote the same address in the error of an address that no
parser accepts. A person reads that error on the screen when the dashboard
loads.

`without_password` in `crates/thunderbox-source-prometheus/src/lib.rs` takes the
password out of an address, and both texts call it. It keeps the user name,
because a user name is no secret and it names which server the message means.

A writer who adds another text that names an address must call that function.
The rule binds every `Debug` implementation, every `Display` implementation,
every log message, every panic message and every error. No test can find such a
text by itself, because a test reads the texts that the project already wrote.

**A dashboard reads more than one data source, and that cost no work.** Section
8 listed this item under Later. The project then found that it holds the item
already.

`Dashboard::sources` is a map from a name to a declaration.
`thunderbox_source::Registry` is a map from that same name to a data source.
`run_panel` reads `sources.get(&target.source)` for each target, and not once
for each panel and not once for each dashboard. Each target therefore names its
own data source.

The rule of section 2 gave this away for nothing. That rule states: agree on the
form of the answer, and do not agree on the form of the question. Every data
source returns a `Frame`, so a panel needs no knowledge of the data source that
answered. Two data sources on one dashboard are then two rows of one map.

A data source that returns an error changes the panels of that data source
alone. Section 12 already states that rule for one data source, and it holds for
several.

**The program binds every key of the time range. thunderbox consumes none of
them.** `DashboardView::handle_key` keeps the three keys that it had, and it
reports `Consumed::No` for every key of the time range. A program that holds a
thunderbox dashboard therefore keeps the arrow keys, `-`, `+`, `0`, and `=` for
itself.

The arithmetic still has one home, and the policy has one home.
`thunderbox_core::TimeRange` holds every operation on a range: `window`,
`to_absolute`, `moved`, `panned`, `with_span_ms`, and `moves_with_the_clock`.
`thunderbox_dashboard::time_key` offers a default map from one key to one new
range, and it states the policy: a move is half of the width, a scale is a
factor of two, and `0` returns the range that the dashboard states. The
function consumes nothing, and a program that wants another map ignores it. The
program reads the key of its terminal, it turns that key into a `Key`, and it
decides whether to call the function. The program that runs alone and
`full_embed.rs` each do that in one short block. No program holds a copy of the
policy.

A move fixes a moment at each end of the range, so the window stands still. A
person who studies the past needs a picture that stands still. A move forward
that would carry the window past the present moment stops at the present
moment. It then returns a range of the same width that follows the clock, and
the panels follow the clock again. No data source of this project answers a
question about the future.

A move forward that stops less than one step before the present moment also
returns to the clock. The program reads the clock again for each key press. A
move back and then a move forward would otherwise leave a window a few
milliseconds behind the present moment. That window would look live, and it
would never move again.

A move back holds the window still in every condition. A dashboard can state a
range that ends after the present moment, in the form `now+5m`. The move back
fixes both ends of such a range as well, so the two keys never return one
range.

`time_key` returns `None` for a key that asks for the range that the runtime
already holds. `DashboardRuntime::set_time_range` starts a new set of queries on
every call, and a terminal repeats a key that a person holds down. A key that
changes nothing would otherwise stop each set of queries before it answers.

**Each label of the axis of time states the distance from the present moment to
that place.** The panel measured every label from the newest point of the data
before this change, so the right label always read `now`. A person who moved the
range one hour into the past then read `now` at the right edge. That label was
not correct.

`TimeSeriesPanel::window` takes the window, which holds the present moment.
`thunderbox_widgets::format_from_now` writes each label. The unit of a label comes
from the distance between two labels, and not from the value of one label. Two
labels of one axis therefore differ, unless the axis has no width.

The function then corrects that unit against the value, in two conditions. It
takes a shorter unit while the value rounds to zero, because the text `-0m`
states both a distance and no distance. It takes a longer unit while the value
needs four digits, because the text `-61000ms` does not fit a narrow panel. A
unit that is longer than the distance between two labels keeps a decimal place,
so the two labels still differ.

The axis covers the window and every point. A window that a data source covers
in part therefore fills one part of the panel, and the rest of the panel stays
empty. That picture is the truth about the data source. The axis of the values
follows the same rule, and the test `a_value_past_the_limit_still_widens_the_axis`
records it.

**The refresh continues while a person holds the window still.** A window that
stands still names the same two moments on each refresh, so a data source
returns the same answer. Three reasons keep the refresh:

- the store of the sysinfo data source fills only while a query runs. A refresh
  that stopped would leave a hole in that store, and that hole would be as long
  as the visit to the past;
- a refresh is the way that a panel which holds an error tries again;
- a data source can change its answer for a window that stands still.
  Prometheus takes a late sample, and a rule of that server can write a value
  after the fact.

The first reason settles the question by itself. A person can read the last hour
for five minutes and then press `0`. That person must not meet a hole of five
minutes at the present moment.

The cost is real. A dashboard that refreshes each second asks the data source
for the same window once a second, and it learns nothing new. A person who does
not want that load states no `refresh` in the dashboard, and the key `r` then
runs the queries on demand.

**A sample of the sysinfo data source carries the moment when the machine
produced it.** `QueryRequest` holds `now` beside `from` and `to`. The data
source stamped each new sample with `to` before this change. It then stamped
every later sample with a moment one hour old, while a person studied a window
one hour in the past. The store then held its samples out of order, and a chart
drew a line that jumped backwards.

The status row of the program names the window while that window stands still
in the past. It writes this sentence:

```
The panels show 5 minutes that ended 1 hour ago.
```

The row reads `Snapshot::window`, which is the window that the panels draw. The
row and the axis of time therefore name one window, and the row names no window
before the first set of queries answers.

The sysinfo data source holds nothing from before the program started, so a
window far in the past leaves every panel with `no data`. The row is the one
place that can tell a person why. `thunderbox_widgets::format_window` writes that
sentence, so the SSH server and another program that holds a dashboard write
the same words.

That report has one limit. No part of thunderbox states the earliest moment that
a data source holds. Only the data source knows that moment, and
`SourceDescriptor` has no field for it. A person therefore joins two facts: the
row names the window, and each panel reports that it has no data. The full
report needs a new field on `SourceDescriptor`, and that change is a matter for
section 3.

**A dashboard can give one width to the charts of a stack, and it does not do
so by default.** A stack is the set of panels that hold the same columns of the
grid, one above another.

A chart keeps a column of cells at its left edge for the labels of the axis of
values. `ratatui` takes the width of that column from the widest label of that
one chart. A panel that shows a share of use writes `100%`, and a panel that
shows a rate writes `8.3GB/s`. The second chart therefore starts four cells
further to the right. A person who reads one panel above the other reads two
axes of time that start at different cells. One moment then stands in two
places. Grafana had the same fault.

The `align` field of a dashboard states the rule:

| value | what the renderer does |
|---|---|
| `none` | Each panel measures its own labels. This value is the default value. |
| `column` | Every chart of a stack starts at the same cell. |

The rule is a choice, and not a fix that the project forces. A dashboard that
holds one chart in a column gains nothing from the rule. A person who wants
those cells for the chart, and not for the labels, keeps the default rule.

`DashboardView::render` measures every chart panel of the dashboard before it
draws any of them. It groups the panels by the pair `(x, w)` of the grid,
because two panels of that pair hold the same cells across the screen. It then
passes the widest measurement of each group to `TimeSeriesPanel::label_width`.
That method adds spaces at the left of each label of the axis of values, until
every label holds the stated width. `ratatui` then measures the same width in
every chart of the stack.

Two limits belong to this rule:

- `ratatui` gives no more than a third of the width of a chart to those labels.
  A panel that asks for more keeps that third. Every panel of a stack holds the
  same width and the same third, so the charts still start at the same cell.
- Two panels of different widths keep their own measurements. Two such charts
  cannot start and end at the same two cells, so the rule leaves them alone.

The measurement reads the data a second time. It does so only for a dashboard
that states `align = "column"`.

**A reference to a variable reaches the text of a query, and it reaches
nothing else.** A person writes `$job` in the `expr` field of a target. thunderbox
then puts the value that the person selected in the place of that reference.
`thunderbox_core::variable` holds the rule, and `Target::to_query` applies it.

That method takes the values as a parameter. A caller therefore cannot build
the query of a target without them. No caller can send the name of a variable
to a data source by mistake. The `legend` field holds no reference. A pattern
names a series, and `thunderbox_core::legend` holds that rule alone.

The rule reads the text one time. A value that itself holds a reference reaches
the data source as it is, so the rule cannot loop. `Dashboard::validate`
rejects a variable whose own query refers to a variable, because that dashboard
asks for a rule that this version does not hold. A chained variable needs an
order between the variables, and that order is a matter for a later version.

The text `$$` writes one `$`, and every other `$` stays as it is. A name starts
with a letter or an underscore, and it then holds letters, digits, and
underscores. The text `$1` of a SQL data source therefore reaches that data
source without a change.

**`Dashboard::validate` reports the fault of a variable, and it reports it
before the program draws.** The method rejects each of these:

- a name that no query can refer to;
- two variables that carry one name;
- a variable that states no choice;
- a variable that states its choices in two places;
- a `current` value that is not one of the options;
- a data source that the dashboard does not declare;
- a target that refers to a variable which the dashboard does not declare.

A person who writes `$jbo` for `$job` would otherwise read the parse error of a
data source, and that error names no variable. The panel error and the load
error therefore answer two different questions. The load error says that the
dashboard is wrong. The panel error says that the answer did not arrive.

A variable that holds no value gives its panels an error that names the
variable. Only a variable that takes its choices from a data source reaches
that state. It reaches it when the data source holds no choice. The panel would
otherwise ask the data source about the name itself, and the answer of that
question means nothing.

**The runtime asks a data source for the choices of a variable one time, when
it starts.** The runtime holds a task of its own for that question, so
`DashboardRuntime::new` returns at once and blocks nothing.
`DashboardRuntime::tick` takes the answer, and a caller that drives `tick`
needs no other call. The choices of a variable change rarely, and a question on
each refresh would reach the data source as often as the panels do.

The first set of queries runs before that answer arrives. A variable that
states a `current` value therefore holds that value from the first moment. The
runtime starts a second set of queries only when the answer changes a value.
`SourceDescriptor::supports_label_values` reports whether a data source holds
choices at all. A data source that holds none gives the variable a sentence
that a program shows.

**The program binds every key of a variable. thunderbox consumes none of them.**
`thunderbox_dashboard::variable_key` offers a default map from one key to a new
selection, and it states the policy: `v` moves the cursor to the variable that
follows, `]` takes the choice that follows, and `[` takes the choice before.
`DashboardView::handle_key` reports `Consumed::No` for all three, so a program
that wants those keys keeps them. This rule is the rule of the time range, and
the two functions have the same shape.

The cursor sits in `Variables` beside the choices. A dashboard that declares
several variables therefore needs no key for each one. The row of the variables
marks the one that a key changes.

`DashboardRuntime::set_variables` starts a new set of queries only when a value
differs from the value before. A key that moves the cursor asks a data source
for nothing. The runtime puts every panel back to the state that has no answer
when a value changed. The answers of the set before describe another question.

**A row of variables holds short labels, and the status row holds the
sentence.** `Variables::row_text` writes `[job: web]  region: eu`, so every
program that draws a thunderbox dashboard writes the same words. That row is one
line, as the row of the keys is. `Variables::error` returns the sentence for a
variable that has no choice, and the program writes that sentence in the status
row. The status row already drops the sentence that a person needs least, so
the two rules meet in one place.

**The runtime runs the panels of one set of queries at the same time.**
`run_all` builds one job for each panel, and `run_together` spawns one task for
each job. A dashboard of four panels, where each data source takes one second,
now answers in about one second. It answered in about four seconds before this
change. A dashboard that reads a server on another machine gains the most from this
change. The wait of such a panel is the wait of a network. A dashboard that
reads this machine gains nothing. The sysinfo data source takes a lock over its
whole answer, so the panels of such a dashboard cannot overlap. That dashboard
pays the cost of one task for each panel. That cost is a fraction of a
millisecond.

The crate takes no new dependency for this work. `tokio::task::JoinSet` comes
from the tokio that the workspace already holds, and no crate of this workspace
holds `futures` or `futures-util`. A spawned task holds no borrow. Each job
therefore takes a copy of four things:

- the registry;
- the time range;
- the targets of its own panel;
- the value of each variable.

`Registry` holds each data source behind an `Arc`. A copy of the registry
therefore copies no data source.

The answers keep the order of the panels. Each task carries the position of its
own job, and `run_together` writes the answer of that task at that position.
`Snapshot::panel` reads a panel at the position that the dashboard gives it. A
function that returned the answers in the order of their arrival would draw one
panel in the place of another.

The drop of a `JoinSet` stops every task that the set holds. The task of a set
of queries holds that `JoinSet` inside the future of `run_all`. The stop of that
one task therefore reaches the task of each panel. `DashboardRuntime` stops the
task of a set when a person refreshes the dashboard, and when the dashboard
drops. A `Vec` of `JoinHandle` would stop the task of no panel. The queries of
the set before would then reach a data source beside the queries of the new set.
Two tests hold that rule, and each one counts the queries that run now.

That stop reaches a task at an await point alone. A data source that holds a
thread of the async runtime, and that awaits nothing, runs to its end. Section
10 states that condition as the third duty of a data source. This change widens
the cost of a data source that does not keep that duty. One panel of such a data
source ran at a time before this change, and every panel of the set runs now. A
test drove six panels on such a data source. Three queries ended after the stop
before this change, and six ended after it. The sysinfo data source and the
testdata data source both read this machine. Each one answers in about one
millisecond, so a person waits for no such query.

A data source that panics ends the task of its own panel. That panel takes an
error of its own, and every other panel keeps its answer. The whole set ended
with no answer before this change, and every panel then held one sentence about
the set. The limit of one query already followed this rule, and a panic now
follows it as well.

One data source can now receive several queries at once. The trait already asks
for `Send` and for `Sync`, and it takes `&self`, so a data source already
permitted that call. The Prometheus data source holds one `reqwest::Client`,
which sends several requests at once over one pool of connections. The sysinfo
data source and the testdata data source read this machine and hold no
connection.

`load_choices` follows the same rule for the variables. The runtime asks the
data source of every variable at the same time, and it returns the answers in
the order of the variables.

---

## 13. The decisions that remain open

1. **`ratatui-core` version 0.1.x appears in the public interface of
   `thunderbox-widgets`.** Every release of ratatui that breaks its interface
   therefore breaks the interface of thunderbox for a program that holds a
   thunderbox panel. To limit that effect, keep the set of public types small,
   export `ratatui_core` under a second name, and write the version down.
2. **Does version 1 need `thunderbox-server`?** The SSH way, together with data
   sources that this process runs, may cover every need.
3. **The wrapper that section 5 asks for does not exist.**
   `thunderbox-widgets/src/timeseries.rs` builds a `ratatui_widgets::chart::Chart`
   value, and it calls no internal `TimeGraph` type. The rule that gives one
   width to a stack of charts shows the cost of that choice. The function
   `label_width_limit` holds a rule of `ratatui-widgets` version 0.3.2. That
   rule gives no more than a third of the width of a chart to the labels at the
   left edge. It measures the space inside the border of the panel. The
   documentation of `ratatui` does not state that rule. A comment inside a
   private method of `chart.rs` holds it, and a release that changes the third
   would cut a label of this project. The wrapper would hold such a rule in one
   place. Build the wrapper when a second rule of that kind appears.
4. **A height that uncovers a panel asks every panel of the dashboard.** Only
   the panel that appeared needs a new answer. The runtime holds no point limit
   for a panel that the area before dropped. It cannot know whether the answer
   that such a panel holds has the right shape. Every other panel holds the
   width that it held, and therefore the answer that it needs.

   The saving follows the number of panels that stand one above the other. A
   dashboard of P equal panels in one column loses its first panel at about 3P
   rows. The last column counts the queries of the step that costs the most.

   | panels in one column | rows where a panel goes | queries today |
   |---|---|---|
   | 4 | 8 to 11 | 4 |
   | 8 | 16 to 23 | 8 |
   | 12 | 24 to 35 | 12 |

   The height of a step is not 3P in general. `panel_size` scales each panel by
   its own `h` against `rows_of_grid`. The step of a dashboard therefore sits at
   about 3 times `rows_of_grid`, divided by the smallest `h` of that
   dashboard. A
   person writes both numbers. `TOO_SMALL_DASHBOARD` of `scripts/record-demo.py`
   is a dashboard of this repository, and the `too_small` scene draws it. Its
   first panel holds 4 grid rows of 28, and that panel appears at 21 rows of the
   body, which is a terminal of 22 rows. A dashboard that holds one short strip
   above two tall charts therefore uncovers a panel at a height that a person
   uses every day.

   A drag costs much more than one step. `AREA_QUIET_TIME` is 250 milliseconds.
   A drag that rests longer than that on each row settles on each row. It
   therefore starts a whole set of queries on each row. I measured a drag against the
   runtime, in the order that the loop of `crates/thunderbox/src/main.rs` uses:

   | the drag | 100 ms a row | 300 ms a row | 500 ms a row | a runtime that asks only what it needs |
   |---|---|---|---|---|
   | 8 panels, 16 rows to 24 rows | 8 | 12 | 36 | 8 |
   | 12 panels, 24 rows to 36 rows | 23 | 24 | 78 | 12 |

   A drag of 500 milliseconds a row costs the sum 1 + 2 + ... + P, and a person
   who drags an edge deliberately rests that long on a row. The debounce
   collapses a fast drag alone.

   The project designed two ways to take that saving, and it took neither. A
   reader measured a fault in each one.

   The first sends a set of queries that asks about the panel which appeared,
   and that keeps the answer of every other panel. `DashboardView::render` gives
   one window to every chart. The item "A panel that the renderer drops keeps
   the answer that it held" of section 12 states why. A panel that kept
   its frames would draw a line that stops before the right edge of that window.
   A person reads that shape as a data source that died. A stat, a gauge and a
   table give a person no sign at all.

   The second holds `Snapshot::updated_at` still for such a set, so that every
   panel of the snapshot describes one window. That moment then never moves
   again on a dashboard that states no refresh interval, because each later area
   holds it still once more. It also gives a data source a moment in the past as
   the present moment, and `QueryRequest::now` names the present moment.

   The project then examined three more ways, and it took none of them. A reader
   measured a fault in each one.

   The third gives each answer its own moment. `PanelState::Ready` carries the
   moment that its set of queries asked about. `Snapshot::panel_window` builds
   the window of one panel from that moment, and `view.rs` hands each chart its
   own window. A chart that holds an older answer then draws an axis that ends
   before the present moment. Its line reaches the right edge of that axis.

   The fault is that the cue does not exist where the saving does. A chart of 3
   cells draws a border and a line, and it draws no axis of time at all. Every
   height at which a panel appears gives the panels around it 3 cells or 4. The
   axis that carries the whole argument is therefore absent at each of those
   heights. A stat, a gauge, a bar gauge and a table draw no axis at any
   height.

   The fourth keeps one window and sends two sets of queries. A set that an area
   starts asks about the window that the snapshot already names, as two fixed
   moments. It gives the true present moment to each data source.
   `TimeRange::to_absolute` and `QueryRequest::new` already hold that shape,
   because a person who moves the time range into the past sends it today.

   Two faults stand. The set still asks about every panel that the renderer
   draws. It therefore saves nothing at any step of a drag, which is where the
   cost lives. The chart also writes `now` at the right edge of a window that is
   arbitrarily old.

   The fifth remembers the point limit that each panel was last asked at, so a
   panel that appears at the width it held needs nothing. The fault is that two
   other steps write a `PanelState` without that memory.
   `clear_the_error_of_each_panel_that_the_area_drops` and `ClearOnEnd` each
   write `PanelState::Loading` and leave the memory as it was. Take a panel that
   held an error, that a height then dropped, and that a height then uncovered.
   That panel draws `loading…` until the refresh interval, and for ever on a
   dashboard that states none.

   A sixth way must therefore answer two questions, and the first one is new.

   Where does a person read that one panel of a screen is older than the rest?
   The answer must hold for a panel of 3 cells, which draws no axis. It must
   also hold for a stat, a gauge, a bar gauge and a table, which draw no axis at
   any size.

   How does a set of queries that an area starts save a query at each step of a
   drag? A saving at the last step alone leaves the cost of a drag as it is.
5. **The repository states its licence. This item is closed.** The workspace
   section of `Cargo.toml` sets `license = "MIT OR Apache-2.0"`, and every crate
   takes that field. `LICENSE-MIT` and `LICENSE-APACHE` hold the two texts, and
   each names The Thunderbox contributors as the holder of the copyright. The
   section "Licence" of `README.md` states the choice and the terms of a
   contribution.
6. **A dashboard file can end the program with an overflow of an integer.**
   `parse_duration_ms` of `crates/thunderbox-core/src/time.rs` ends with
   `Ok(n * mult)`. That multiplication does not saturate, and it returns no
   error. `Dashboard::validate` calls that function for the `refresh` field.

   Ten lines below that call, a comment states the opposite rule for the grid.
   The addition there saturates, because a panel must not end the program. The
   rule holds on the path of the grid alone.

   The measurement: a dashboard of two lines, which sets
   `refresh = "20000000000w"`, gives `attempt to multiply with overflow` and the
   code 101. The program writes that message before it opens the terminal, where
   `crates/thunderbox/src/main.rs` promises a message and a stop. A build for
   release wraps the number instead, and the dashboard then refreshes at a
   period that nobody asked for.

   `validate` never reads the `[time]` table, so the `from` field reaches the
   same fault through a different path. `clippy::missing_panics_doc` reads
   `panic!`, `unwrap` and `expect`, and it reads no arithmetic, so the workspace
   reports zero faults.

   The next design answers one question. Does `parse_duration_ms` return an
   error for a number that does not fit, or does it saturate as the grid does?
   The two answers differ for a person who writes a large number by mistake.
7. **`SourceDescriptor::downsamples` promises a step that no code takes.** The
   comment on that field stated that the caller reduces the number of points
   when a data source cannot. Every read of the field sits inside a test module,
   and no code outside a test reads it. `Cargo.lock` holds no crate that reduces
   a series.

   Section 6 of this document and `docs/RESEARCH.md` each promised the same
   step. Those two texts and the comment on the field now state that no such
   code exists. Each of the three texts names this item. The row `LTTB` of
   section 1.4 of `CLAUDE.md` defines an abbreviation, and it promises no
   step, so it stays. A data source that returns 50000 points for a budget of
   160 draws all 50000 today, and this item says so.

   The next design answers one question. Does this project take the reduction,
   or does the field leave the interface until it does?

---

## 14. What other projects did

- **`grafatui`** is a Grafana-like terminal program for Prometheus. It reads
  Grafana dashboard files in the JSON format for several types of panel, and it
  supports variables, thresholds, and a grid. It is small, and it reads
  Prometheus only. It has already taken decision 1 in section 12. Read it first.
- **`rebels-in-the-sky`** and `frittura-ssh-core` serve ratatui through SSH to
  people today.
- **`timeless`** is a crate that holds measurements in memory. Its
  `ChunkedData` type holds a series that has holes, and it stores no value for a
  hole. That idea suits this project. Do not depend on the crate: its own
  documentation states that it changes often.

---

## 15. The first goal, which the project has reached

The project has built `thunderbox-core`, `thunderbox-widgets`,
`thunderbox-dashboard`, the testdata data source, the sysinfo data source, and
the `thunderbox` program that runs alone. Two examples hold the two ways in:

- `thunderbox-widgets/examples/panel_embed.rs` puts one panel inside a program
  that owns its data, its terminal, and its event loop;
- `thunderbox-dashboard/examples/full_embed.rs` puts a whole dashboard inside a
  program that owns the terminal and the async runtime.

Both examples were the test of the design. Each one was easy to write, so the
layers hold.

### The Prometheus data source

The project has built the prometheus data source. It is the first data source
that reads a server on another machine. It is therefore the first test of two
rules that the design states.

The first rule is section 6: the panel decides the number of points, and the
data source reduces them. A test starts a server that answers over HTTP, and it
then states which step the data source must ask for. That test holds the rule
in place.

The second rule is section 10: a data source reads its own options. The
`[source.*]` table of a dashboard now reaches the data source that it declares.
Each data source reads that declaration itself, and it returns an error for an
option that it does not know. The `thunderbox` program holds the list of types
that it can make, and a feature selects each type.

### The time range that a person moves and scales

A person can now move the time range while the program runs, and can also scale
it. Every panel shares that one range. The five keys are `Left`, `Right`, `-`,
`+`, and `0`. The key `=` does what `0` does.

Section 12 records the four decisions that this work took. The program binds
every key of the time range, and thunderbox consumes none of them. Each label of
the axis of time states its distance from the present moment. The refresh
continues while the window stands still. A sample of the sysinfo data source
carries the moment when the machine produced it.

### What comes next

- an option for a password on the prometheus data source. A person can write a
  user name and a password into the `url` field today. The client reads that
  pair, and it builds a header of the kind `Basic`. No option carries a bearer
  token, and no option carries another header;
- thunderbox asks a data source for the choices of a variable one time, when the
  program starts. A dashboard that runs for a long time never reads a choice
  that appeared after that moment;
- one variable cannot reach another. A dashboard that asks for that reports a
  fault, and section 12 states the reason;
- more steps in the transform pipeline than the step that reduces a column.

The program now acts on the refresh interval of a dashboard. The caller drives
that work with `DashboardRuntime::tick`, because thunderbox owns no event loop.

Every type of panel now draws its own picture. Section 5 lists the five
renderers. The default dashboard inside the program shows all five, so one
command demonstrates each of them.

### The queries that run at the same time

The panels of one set of queries now run at the same time. `run_all` spawns one
task for each panel, and it holds those tasks in a `tokio::task::JoinSet`. A
dashboard that reads a server on another machine therefore waits for its
slowest panel. It no longer waits for the sum of its panels. The variables
follow the same rule.

Section 12 records the decisions that this work took. The answers keep the order
of the panels. The stop of the task of a set reaches the task of each panel. A
data source that panics changes the panel of that data source alone.

### The pattern that names a series

The `legend` field of a target now covers every data source. One rule sits in
`thunderbox_core::legend`, and `thunderbox-dashboard` applies it to the answer of
each target. The prometheus data source held the only copy of that rule before
this change, and the other two data sources ignored the field.

Section 10 states the five duties that a data source keeps, and section 12
records the decision. A data source that keeps the first duty and the second
duty needs no rule of its own for the name of a series.

### The variables that a person selects

A person can now select a value while the program runs, and every target that
refers to that variable reads the new value. This item was the last item of the
version 1 scope in section 8 that the project had not built.

The three keys are `v`, `]`, and `[`. The program binds all three, and thunderbox
consumes none of them. `thunderbox_dashboard::variable_key` holds the rule, as
`time_key` holds the rule of the time range.

A variable takes its choices from an `options` list, or from a data source.
`DataSource::label_values` holds that second way, and this work is the first
caller of that method outside the data sources themselves.

`dashboards/variables.toml` demonstrates the first way over the test data, so
it needs no server. `dashboards/prometheus.toml` demonstrates the second way
over a Prometheus server. The default dashboard inside the program declares no
variable, so `cargo run -p thunderbox` draws exactly the picture that it drew
before.
