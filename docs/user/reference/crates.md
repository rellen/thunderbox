# The crates

This page lists the library crates of thunderbox. A program uses them to hold a
thunderbox panel or a whole thunderbox dashboard. The page names what each crate
exports for that use.

The page
[Two ways into another program](../explanation/two-ways-into-another-program.md)
explains when a program uses each way.

## The crates of the workspace

| crate | what it holds | the crates of thunderbox that it depends on |
|---|---|---|
| `thunderbox-widgets` | The panels. Each panel draws into a buffer of ratatui. | `thunderbox-core` |
| `thunderbox-dashboard` | The dashboard runtime that runs the queries of a dashboard, and the view that draws it. | `thunderbox-core`, `thunderbox-source`, `thunderbox-widgets` |
| `thunderbox-core` | The types that every other crate uses: the dashboard, the frame, the query, and the time range. | none |
| `thunderbox-source` | The trait of a data source, and the registry of the data sources. | `thunderbox-core` |
| `thunderbox-source-sysinfo` | The data source `sysinfo`. | `thunderbox-core`, `thunderbox-source` |
| `thunderbox-source-testdata` | The data source `testdata`. | `thunderbox-core`, `thunderbox-source` |
| `thunderbox-source-prometheus` | The data source `prometheus`. | `thunderbox-core`, `thunderbox-source` |

- Each crate holds the version `0.0.0`.
- Each crate needs release 1.88 of Rust or a later release.
- Each crate is free software. A person can use it under the licence of the
  file `LICENSE-MIT`, or under the licence of the file `LICENSE-APACHE`.
- The workspace holds one more crate, `thunderbox`. That crate is the program,
  and it exports no library.

## The full interface

Run this command from the root of the workspace. It builds the documentation of
every crate, and it opens that documentation in a browser:

```
cargo doc --no-deps --open
```

The documentation names every item, every field, and every error.

## The first way: `thunderbox-widgets`

A program that holds its own data uses this crate alone. The crate needs no
terminal backend and no async runtime. An async runtime is a library that runs
concurrent tasks. The crate depends on `ratatui-core` and `ratatui-widgets`, and
on `thunderbox-core`.

| item | what it is |
|---|---|
| `TimeSeriesPanel` | The panel of the kind `timeseries`. |
| `StatPanel` | The panel of the kind `stat`. |
| `GaugePanel` | The panel of the kind `gauge`. |
| `BarGaugePanel` | The panel of the kind `bargauge`. |
| `TablePanel` | The panel of the kind `table`. |
| `TopPanel` | The panel of the kind `top`. |
| `RowPanel` | The line of a row panel. |
| `PanelData` | The data that a panel shows: `Loading`, `Frames`, or `Error`. |
| `PanelOptions` | The unit, the decimal places, the range, the step of `reduce`, and the thresholds of a panel. |
| `Theme` | The colours that a panel uses. |
| `Marker` | The glyph that a chart draws a point with. |
| `draws_a_panel` | The function that states whether a panel of one size holds a cell for its data. |
| `format_value`, `format_time`, `format_from_now`, `format_window` | The functions that write a number, a time of day, a label of the axis of time, and the sentence of a window. |
| `last_value` | The function that returns the most recent value of some frames. |
| `LOADING`, `NO_DATA`, `MISSING` | The texts `loading…`, `no data`, and `—`. |
| `ratatui_core` | A second name for the crate `ratatui-core`, in the version that thunderbox uses. |

Each panel implements the trait `Widget` of `ratatui-core`. A program draws a
panel with `Widget::render`.

This command runs a short example of this way:

```
cargo run -p thunderbox-widgets --example panel_embed
```

## The second way: `thunderbox-dashboard`

A program that wants thunderbox to run the queries uses this crate. The program
keeps its terminal and its event loop, and it binds its own keys.

| item | what it is |
|---|---|
| `DashboardRuntime` | The dashboard runtime. It runs the queries of a dashboard, it refreshes them, and it holds the time range and the state of each panel. |
| `DashboardView` | The view. It draws a dashboard from a snapshot, and it reads the keys `Tab`, `Shift+Tab`, and `m`. |
| `Snapshot` | The state of every panel at one moment. |
| `PanelState` | The state of one panel: `Loading`, `Ready`, or `Error`. |
| `Key` | A key that a program passes to thunderbox. |
| `Consumed` | The answer of the view to a key: `Yes` or `No`. |
| `Variables`, `VariableState` | The value and the choices of each variable. |
| `CollapsedRows` | The row panels that are collapsed. |
| `time_key` | The function that maps a key to a new time range. |
| `variable_key` | The function that maps a key to a new choice of a variable. |
| `row_key` | The function that maps a key to the row panel that it collapses or expands. |
| `DEFAULT_ANSWER_LIMIT` | The longest wait for a data source that states no wait of its own: 60 seconds. |
| `AREA_QUIET_TIME`, `AREA_MAX_HOLD` | An area is the part of the terminal that holds the panels. A new area must stand still for 250 milliseconds before the dashboard runtime takes it. The dashboard runtime holds a new area for 1000 milliseconds or less. |
| `widgets` | A second name for the crate `thunderbox-widgets`. |

These rules bind a program that uses this crate:

- The program gives `DashboardRuntime::new` a `Handle` of its own async runtime
  of tokio.
- That async runtime must hold the timer of tokio. `#[tokio::main]` and
  `Runtime::new` each build such a runtime.
- That async runtime must run a task on a thread other than the thread that
  draws.
  `Builder::new_multi_thread` builds such a runtime.
- The program drives the refresh. It calls `DashboardRuntime::tick` from its own
  event loop, because thunderbox starts no timer of its own.
- The program reports the area of the panels with
  `DashboardRuntime::set_panel_area`.
- thunderbox consumes no key of the time range, no key of a variable, and no
  key of a row panel. The program passes such a key to `time_key`,
  `variable_key`, or `row_key`. A program that wants another map ignores those
  functions.

This command runs a short example of this way:

```
cargo run -p thunderbox-dashboard --example full_embed
```

## The crate `thunderbox-core`

| item | what it is |
|---|---|
| `Dashboard` | The dashboard document. `Dashboard::from_toml` reads it, and `Dashboard::validate` checks it. |
| `Panel`, `PanelKind`, `GridPos`, `Target`, `Variable`, `SourceDecl`, `Align` | The parts of a dashboard. |
| `GRID_COLUMNS` | The width of the grid: 24 columns. |
| `Frame`, `Field`, `FieldConfig`, `Values`, `Threshold` | The answer of a data source, and the configuration of a column. |
| `Query`, `QueryRequest`, `SourceUid` | A query, and the context that a data source needs to answer it. |
| `max_data_points` | The function that returns the point limit of a panel. |
| `Reduce` | The step that reduces a column of numbers to one number. |
| `TimeRange`, `RelativeTime`, `TimeWindow`, `Timestamp` | The time range, a moment, a window, and a count of milliseconds after the Unix epoch. |
| `MIN_SPAN_MS`, `MAX_SPAN_MS` | The narrowest and the widest time range that the keys give. |
| `now_ms`, `parse_duration_ms` | The functions that read the clock, and that read a duration. |
| `legend` | The module that applies a pattern. |
| `variable` | The module that puts the value of a variable into a query. |
| `Error`, `Result` | The error of this crate. |

## The crate `thunderbox-source`

| item | what it is |
|---|---|
| `DataSource` | The trait that every data source implements. |
| `SourceDescriptor` | The name of a data source, and the operations that it supports. |
| `Registry` | The data sources of a dashboard, by name. |

A program that holds a dashboard can supply a data source of its own. It
implements `DataSource`, and it adds the data source to a `Registry`.

## The crates of the data sources

| crate | items |
|---|---|
| `thunderbox-source-sysinfo` | `SysinfoSource`, and `KIND`, which holds `"sysinfo"`. |
| `thunderbox-source-testdata` | `TestData`, and `KIND`, which holds `"testdata"`. |
| `thunderbox-source-prometheus` | `Prometheus`, `Settings`, `KIND`, which holds `"prometheus"`, and `DEFAULT_URL`, which holds `"http://127.0.0.1:9090"`. |

Each data source offers the function `from_declaration`. That function makes the
data source from a `[source.NAME]` table of a dashboard. The page
[The data sources](data-sources/README.md) lists the options of each kind.

## The pages that give the reason

- [Put a panel inside your program](../tutorials/put-a-panel-inside-your-program.md)
  uses the first way.
- [Put a dashboard inside your program](../tutorials/put-a-dashboard-inside-your-program.md)
  uses the second way.
- [Architecture decision record 0004](../../contributor/adr/0004-design-for-the-way-inside-another-program-first.md)
  states why the design starts from the way inside another program.
- [Architecture decision record 0016](../../contributor/adr/0016-a-program-that-holds-a-dashboard-holds-four-rules.md)
  states the four rules of a program that holds a dashboard.
