# The architecture of thunderbox

**What thunderbox is:** a dashboard platform for the terminal. It has the shape
of Grafana. A person writes a dashboard as a document. The dashboard then reads
its data from data sources that a person can add.

**What thunderbox is not:** a place to store data. thunderbox stores a dashboard,
and it never stores a measurement. The system that thunderbox queries keeps the
measurements, and that system decides how long to keep them.

This document describes the parts of the program and how they fit together. It
states no decision of its own. Each decision has a record in [the folder of the
records](adr/README.md), and this document links to it.

## 1. The ways to run thunderbox

thunderbox runs in four ways. They are four places for one program, and not
four products.

| way to run | number of processes | where the program draws | who owns the terminal | state |
|---|---|---|---|---|
| alone | one | in the same process | thunderbox | built |
| inside another program | one | in the same process | the other program | built |
| through SSH | two | in the server | thunderbox | [ADR 0013](adr/0013-one-connection-carries-both-ways-over-ssh.md) |
| client and server | two | in the client | thunderbox | [ADR 0059](adr/0059-version-1-may-need-no-server.md) |

The place of the data is not a question of this table. A Prometheus data source
reads another machine, and the data source `sysinfo` reads this machine. Neither
choice changes the shape of thunderbox.

The way inside another program is the hardest one, so the design meets it first.
[ADR 0004](adr/0004-design-for-the-way-inside-another-program-first.md) states
that rule, and [ADR 0015](adr/0015-both-ways-into-another-program-are-in-scope.md)
states the two layers that serve it.

## 2. The main idea: one form of answer

Each data source keeps its own query language. Every data source returns the
same form of answer, which is a list of frames. A frame holds columns, and each
column holds a name, labels, a configuration, and values. A panel reads frames,
and it never names a data source.

[ADR 0003](adr/0003-agree-on-the-answer-and-not-on-the-question.md) records this
decision. `crates/thunderbox-core/src/frame.rs` holds the types `Frame`,
`Field`, `FieldConfig`, and `Values`. `crates/thunderbox-core/src/query.rs`
holds `Query`, which is a name of a data source and a text, and
`QueryRequest`, which adds the window and the point limit.

## 3. The crates

| crate | what it holds | what it depends on |
|---|---|---|
| `thunderbox-core` | the frame, the query, the time range, the dashboard document, the legend pattern, the variables | `serde`, `toml`, `thiserror`. No `ratatui`, no `tokio`. |
| `thunderbox-widgets` | one renderer for each kind of panel | `thunderbox-core`, `ratatui-core`, `ratatui-widgets` alone |
| `thunderbox-source` | the trait `DataSource`, and the registry of data sources | `thunderbox-core`, `async-trait` |
| `thunderbox-source-testdata` | a data source that makes test data | `thunderbox-source` |
| `thunderbox-source-sysinfo` | a data source that reads this machine | `thunderbox-source`, `sysinfo` |
| `thunderbox-source-prometheus` | a data source that reads a Prometheus server | `thunderbox-source`, `prometheus-http-query`, `reqwest` |
| `thunderbox-dashboard` | the runtime that runs the queries, the snapshot, the layout of the grid, and the view that draws a whole dashboard | `thunderbox-widgets`, `thunderbox-source`, `ratatui-core`, `ratatui-widgets`, `tokio` |
| `thunderbox` | the program. It owns the terminal and the event loop, and it binds the keys. | every crate above, `ratatui`, `crossterm`, `tokio` |

The feature `prometheus` of the program adds the Prometheus data source, and it
is on by default.

Two crates do not exist yet. `thunderbox-transform` will hold a pipeline from
frame to frame, with steps such as reduce, filter, rename, join, and group.
`thunderbox_core::Reduce` holds the one step that a panel cannot do without. A
crate for the SSH way will follow [ADR 0013](adr/0013-one-connection-carries-both-ways-over-ssh.md).

## 4. The seam of every data source

One trait covers every data source, and every way to run thunderbox:

```rust
#[async_trait]
pub trait DataSource: Send + Sync {
    fn descriptor(&self) -> SourceDescriptor;
    async fn query(&self, req: QueryRequest) -> Result<Vec<Frame>>;
    async fn label_values(&self, query: &str) -> Result<Vec<String>>;
}
```

`thunderbox-dashboard` never knows which implementation it holds. Each data
source reads its own options from the table `[source.NAME]` of a dashboard.

These records govern a data source:

- [ADR 0005](adr/0005-one-trait-is-the-seam-of-every-data-source.md): the trait is the one seam;
- [ADR 0011](adr/0011-add-a-data-source-as-code-behind-a-feature.md): a data source is code behind a feature;
- [ADR 0012](adr/0012-the-five-duties-of-a-data-source.md): the five duties of a data source;
- [ADR 0033](adr/0033-no-text-writes-the-password-of-an-address.md): no text writes the password of an address.

## 5. The way of one refresh

The program drives the runtime. The runtime owns no timer and no event loop.
These steps run for each frame that the program draws:

1. The program reads the newest snapshot with `DashboardRuntime::snapshot`. The
   snapshot holds the state of each panel, the window, the values of the
   variables, and the collapsed row panels.
2. The program gives the snapshot to `DashboardView::render`, which draws each
   panel with a renderer of `thunderbox-widgets`.
3. The program reports the area of the panels with
   `DashboardRuntime::set_panel_area`, and the marker with
   `DashboardRuntime::set_marker`.
4. The program calls `DashboardRuntime::tick`. That method starts a set of
   queries in three conditions. The refresh interval passed, an area waited
   long enough, or the choices of a variable arrived.
5. A set of queries runs on the async runtime of the program. `run_all` in
   `crates/thunderbox-dashboard/src/query.rs` runs one task for each panel,
   and it publishes one whole snapshot into a `watch` channel.

A set of queries is the whole group of queries that one refresh sends. These
records govern that way:

| subject | records |
|---|---|
| the rules of a library, and of the program that holds it | [0015](adr/0015-both-ways-into-another-program-are-in-scope.md), [0016](adr/0016-a-program-that-holds-a-dashboard-holds-four-rules.md) |
| the answers that a refresh keeps | [0018](adr/0018-a-refresh-keeps-the-answer-of-each-panel.md), [0027](adr/0027-a-dropped-panel-keeps-its-answer.md) |
| the limit of a query, and a task that ends with no answer | [0019](adr/0019-the-runtime-holds-one-query-to-a-limit.md), [0020](adr/0020-a-task-that-ends-with-no-answer-gives-an-error.md) |
| the area of the panels | [0021](adr/0021-a-new-area-waits.md), [0022](adr/0022-an-area-that-changes-no-point-limit-starts-no-set.md), [0028](adr/0028-the-caller-reports-one-area-as-a-size.md), [0029](adr/0029-the-runtime-asks-about-every-panel-until-an-area-arrives.md) |
| the jobs of one set | [0030](adr/0030-a-set-of-queries-holds-one-job-for-each-panel.md), [0045](adr/0045-the-panels-of-one-set-of-queries-run-at-the-same-time.md) |
| the tests of that way | [0032](adr/0032-each-rule-of-the-path-of-an-area-holds-a-test.md) |

## 6. How a panel draws

`thunderbox-widgets` holds one renderer for each kind of panel:

| kind | renderer | what it draws |
|---|---|---|
| `timeseries` | `TimeSeriesPanel` | a chart of values against time |
| `stat` | `StatPanel` | one large number for each series |
| `gauge` | `GaugePanel` | a meter, with the range written out |
| `bargauge` | `BarGaugePanel` | one named bar for each series |
| `table` | `TablePanel` | the columns of the frames |
| `top` | `TopPanel` | the rows of the frames, in order by one column |
| `row` | `RowPanel` | the line of a row panel, which shows no data |

Each renderer that shows data takes a title, the data, a theme, and a
`PanelOptions` value. `crates/thunderbox-widgets/src/panel.rs` holds the parts
that those renderers share: the border, the three states that a panel reports,
the step that reduces a column, and the step that cuts a name to a width.
`crates/thunderbox-widgets/src/table.rs` holds the layout of a sheet, which the
table panel and the top panel share.

[ADR 0007](adr/0007-the-settings-of-a-panel-win-over-the-column.md) states how
the settings of a panel and the configuration of a column combine.
[ADR 0008](adr/0008-draw-each-chart-through-a-wrapper-of-the-project.md) states
the wrapper that each chart must use, and that the code does not use yet.

A chart draws each point with a marker. A terminal cell holds one character, and
some characters divide the cell into smaller parts. This document calls each
such part a sub-cell.

| marker | sub-cells in one cell | note |
|---|---|---|
| `Braille` (default) | 2 across, 4 down | Most fonts hold these characters. |
| `Octant` | 2 across, 4 down | No space separates two cells, but fewer fonts hold these characters. |
| `HalfBlock` | 1 across, 2 down | Every font holds these characters. |
| `Block` | 1 across, 1 down | For a plain terminal. |

A gradient of colour along a chart is work for later. A `Dataset` of ratatui
holds one style for all of its points, so a point cannot take a colour of its own.
Two methods give that result: a `Canvas` with one batch of points for each band
of colour, or a change of the colour of each cell after the chart draws.

## 7. The layout of the grid

A dashboard places each panel on a grid of 24 columns. The rows of the grid
belong to the dashboard, and the renderer scales them onto the height of the
area. `crates/thunderbox-dashboard/src/grid.rs` holds that arithmetic, and both
the renderer and the runtime read the function `places`. The two parts therefore
agree about the size of each panel.

| subject | records |
|---|---|
| the width of each panel, and its point limit | [0023](adr/0023-each-panel-asks-for-the-points-of-its-own-width.md) |
| the rows of the grid | [0025](adr/0025-the-rows-of-the-grid-belong-to-the-dashboard.md) |
| a panel that the renderer drops | [0024](adr/0024-a-panel-that-the-renderer-drops-asks-for-nothing.md), [0026](adr/0026-panel-state-gains-no-fourth-variant.md), [0031](adr/0031-a-snapshot-waits-in-a-third-case.md) |
| a stack of charts | [0039](adr/0039-a-stack-of-charts-can-share-one-width.md) |
| row panels | [0050](adr/0050-a-row-panel-holds-panels-by-their-place.md), [0051](adr/0051-a-row-panel-takes-one-line.md), [0052](adr/0052-the-runtime-owns-the-collapse.md), [0053](adr/0053-a-panel-that-appears-costs-a-set-of-queries.md) |

## 8. The number of points

A panel states how many points it can draw, and the data source reduces the
points to that number. The formula is the width of the panel in cells, times
the sub-cells of the marker across one cell. `thunderbox_core::max_data_points`
holds it. [ADR 0009](adr/0009-the-data-source-reduces-the-points.md) records
the decision, and [ADR 0062](adr/0062-downsamples-promises-a-step-that-no-code-takes.md)
records the step that thunderbox does not take.

## 9. The data sources

| data source | crate that it reads | state |
|---|---|---|
| testdata | none | built. It makes test data, and the project built it first. |
| sysinfo | `sysinfo` | built. It makes the program that runs alone work. |
| prometheus | `prometheus-http-query` 0.9 | built |
| PromQL text | `promql-parser` 0.10 | later, and optional. It would check a query and complete it. |
| influxdb | `influx_db_client` 0.7, `influxdb3-client` | later |

[The research](research.md) records the evidence for each library. These
records govern the data sources that exist:

- sysinfo: [0038](adr/0038-a-sysinfo-sample-carries-the-moment-of-the-machine.md), [0047](adr/0047-the-process-list-of-sysinfo-keeps-no-past.md), [0048](adr/0048-the-process-list-reads-the-machine-through-its-own-system.md), [0049](adr/0049-the-number-of-a-process-is-text.md);
- prometheus: [0033](adr/0033-no-text-writes-the-password-of-an-address.md), [0064](adr/0064-plain-http-needs-no-ca-file.md);
- more than one data source on one dashboard: [0034](adr/0034-a-dashboard-reads-more-than-one-data-source.md).

## 10. The program, and the keys

The program that runs alone is a dashboard over the data source `sysinfo`.
[ADR 0010](adr/0010-the-program-that-runs-alone-is-a-dashboard-over-sysinfo.md)
records that decision. `crates/thunderbox/src/default.toml` holds its
dashboard, and `crates/thunderbox/src/main.rs` holds the loop.

thunderbox consumes few keys. The program binds every key that changes what the
runtime asks, and a function of `thunderbox-dashboard` holds the default map of
each group:

| group | function | record |
|---|---|---|
| the time range | `time_key` | [0035](adr/0035-the-program-binds-each-key-of-the-time-range.md), [0076](adr/0076-a-key-that-would-pass-the-limit-leaves-the-range-as-it-was.md) |
| a variable | `variable_key` | [0043](adr/0043-the-program-binds-each-key-of-a-variable.md) |
| a row panel | `row_key` | [0054](adr/0054-the-program-binds-enter-and-space.md) |

`DashboardView::handle_key` consumes `Tab`, `BackTab`, and `m`, because those
keys change the view alone.

## 11. The libraries, the build, and the licence

| subject | record |
|---|---|
| ratatui, crossterm, and tokio | [ADR 0006](adr/0006-use-ratatui-crossterm-and-tokio.md) |
| the dashboard file in TOML | [ADR 0014](adr/0014-the-file-format-of-a-dashboard-is-toml.md) |
| the Nix flake | [ADR 0055](adr/0055-build-the-package-with-a-nix-flake.md), [ADR 0056](adr/0056-the-check-phase-of-the-package-reads-a-ca-file.md) |
| the licence | [ADR 0057](adr/0057-license-under-mit-or-apache.md) |
| the style of every document | [ADR 0002](adr/0002-write-every-document-in-simplified-technical-english.md) |

## 12. What to build, and when

**Version 1** holds these parts, and the project has built each of them:

- a dashboard as a document on disk;
- a time range that every panel shares, and that a person can move and scale;
- a refresh interval;
- a grid of panels;
- variables that a person selects;
- the panels TimeSeries, Stat, Gauge, BarGauge, and Table;
- the data sources testdata, sysinfo, and prometheus;
- more than one data source on one dashboard.

The project then added the top panel and the row panels.

**Later** holds these parts:

- more steps in the transform pipeline than `reduce`;
- a mode in which a person writes a query directly;
- notes that mark an event on a chart;
- the Heatmap panel and the Logs panel;
- the way through SSH.

**Never** holds these parts:

- alarms, because they belong to a different product;
- storage, retention, and summary tables.

## 13. The questions that remain open

Each record of the status Proposed holds one question:
[0013](adr/0013-one-connection-carries-both-ways-over-ssh.md),
[0058](adr/0058-the-public-interface-holds-ratatui-core.md),
[0059](adr/0059-version-1-may-need-no-server.md),
[0060](adr/0060-a-height-that-uncovers-a-panel-asks-every-panel.md),
[0062](adr/0062-downsamples-promises-a-step-that-no-code-takes.md),
[0063](adr/0063-a-narrow-top-panel-drops-the-sort-column.md), and
[0064](adr/0064-plain-http-needs-no-ca-file.md).
[The open findings](open-findings.md) hold the faults that a sweep of the
repository found, and that no record holds yet.

## 14. What other projects did

- **`grafatui`** is a terminal program for Prometheus in the shape of Grafana. It
  reads Grafana dashboard files in the JSON format for several kinds of panel,
  and it supports variables, thresholds, and a grid. It is small, and it reads
  Prometheus alone. It took the decision of
  [ADR 0014](adr/0014-the-file-format-of-a-dashboard-is-toml.md) in the other
  direction. Read it first.
- **`rebels-in-the-sky`** and `frittura-ssh-core` serve ratatui through SSH to
  people today.
- **`timeless`** is a crate that holds measurements in memory. Its `ChunkedData`
  type holds a series that has holes, and it stores no value for a hole. That
  idea suits this project. Do not depend on the crate, because its own
  documentation states that it changes often.

## 15. The features, and their documents

[The contributor guide](guide.md) states the rule: each feature holds
user documentation and contributor documentation. This table holds the
documents of each feature that exists.

| feature | user documents | records |
|---|---|---|
| the dashboard file | [reference](../user/reference/dashboard-file.md), [tutorial](../user/tutorials/write-your-first-dashboard.md) | 0014, 0041, 0073, 0074 |
| the program that runs alone | [tutorial](../user/tutorials/watch-this-machine.md), [reference](../user/reference/command-line.md) | 0010, 0071, 0072 |
| the time series panel | [reference](../user/reference/panels/timeseries.md) | 0007, 0008, 0036, 0069 |
| the stat panel, the gauge panel, the bar gauge panel | [stat](../user/reference/panels/stat.md), [gauge](../user/reference/panels/gauge.md), [bar gauge](../user/reference/panels/bargauge.md) | 0007, 0065 |
| the table panel | [reference](../user/reference/panels/table.md) | 0007, 0065 |
| the top panel | [how-to](../user/how-to/show-the-busiest-processes.md), [reference](../user/reference/panels/top.md) | 0046, 0047, 0048, 0049, 0063, 0065 |
| the row panels | [how-to](../user/how-to/group-panels-under-row-panels.md), [reference](../user/reference/panels/row.md), [explanation](../user/explanation/row-panels-and-the-place-of-a-panel.md) | 0050, 0051, 0052, 0053, 0054 |
| the time range | [how-to](../user/how-to/show-an-earlier-window.md), [reference](../user/reference/time-ranges.md), [explanation](../user/explanation/the-time-range-and-the-window.md) | 0035, 0036, 0037, 0038, 0073, 0074, 0075, 0076, 0077 |
| the variables | [how-to](../user/how-to/let-a-person-select-a-value.md), [reference](../user/reference/variables.md) | 0040, 0041, 0042, 0043, 0044 |
| the legend pattern | [how-to](../user/how-to/name-each-series.md), [reference](../user/reference/legend-patterns.md) | 0017 |
| the thresholds and the units | [how-to](../user/how-to/colour-a-value-that-reaches-a-bound.md), [reference](../user/reference/numbers-and-units.md) | 0007 |
| one width for a stack of charts | [how-to](../user/how-to/start-each-chart-of-a-stack-at-the-same-cell.md) | 0039 |
| the marker | [how-to](../user/how-to/change-the-glyph-of-a-chart.md) | 0009 |
| a terminal that is too small | [how-to](../user/how-to/read-a-dashboard-on-a-small-terminal.md), [explanation](../user/explanation/panels-that-the-terminal-cannot-hold.md) | 0024, 0027, 0031, 0060 |
| the data source sysinfo | [reference](../user/reference/data-sources/sysinfo.md) | 0038, 0047, 0048 |
| the data source prometheus | [tutorial](../user/tutorials/read-a-prometheus-server.md), [how-to](../user/how-to/give-the-prometheus-source-a-ca-file.md), [reference](../user/reference/data-sources/prometheus.md) | 0009, 0033, 0064, 0073 |
| the data source testdata | [reference](../user/reference/data-sources/testdata.md) | 0010 |
| more than one data source | [how-to](../user/how-to/show-two-data-sources-on-one-dashboard.md) | 0034 |
| one panel inside another program | [tutorial](../user/tutorials/put-a-panel-inside-your-program.md), [reference](../user/reference/crates.md) | 0004, 0015, 0070 |
| a dashboard inside another program | [tutorial](../user/tutorials/put-a-dashboard-inside-your-program.md), [explanation](../user/explanation/two-ways-into-another-program.md) | 0015, 0016 |
| the Nix flake | [how-to](../user/how-to/install-thunderbox.md), [reference](../user/reference/nix-flake.md) | 0055, 0056 |
