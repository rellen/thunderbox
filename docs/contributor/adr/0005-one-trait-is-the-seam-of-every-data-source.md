# ADR 0005: One trait is the seam of every data source

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 3 |
| Code | `crates/thunderbox-source/src/lib.rs`, `crates/thunderbox-core/src/query.rs`, the `Cargo.toml` file of each crate |
| Tests | No test holds this decision alone. The compiler holds the trait. |
| Related | [ADR 0003](0003-agree-on-the-answer-and-not-on-the-question.md), [ADR 0004](0004-design-for-the-way-inside-another-program-first.md), [ADR 0011](0011-add-a-data-source-as-code-behind-a-feature.md), [ADR 0012](0012-the-five-duties-of-a-data-source.md), [ADR 0015](0015-both-ways-into-another-program-are-in-scope.md) |

## Context

[ADR 0004](0004-design-for-the-way-inside-another-program-first.md) names four
ways to run thunderbox. The code holds two of them today: the way alone, and
the way inside another program. The other two ways are plans.

The project divides its code into these crates. The word "Plan" marks a part
that the code does not hold today.

| crate | what it holds |
|---|---|
| `thunderbox-core` | The types `Frame`, `Field`, `Values`, `TimeRange`, `Query`, `Dashboard`, and `Panel`. The crate uses serde, and it holds no ratatui and no tokio. |
| `thunderbox-widgets` | The panel renderers: `TimeSeriesPanel`, `StatPanel`, `GaugePanel`, `BarGaugePanel`, `TablePanel`, `TopPanel`, and `RowPanel`. The crate depends on `thunderbox-core`, `ratatui-core`, and `ratatui-widgets` alone. It is the smaller way into another program. Plan: a heatmap panel and a panel of logs. |
| `thunderbox-transform` | Plan. This crate does not exist. It will hold a pipeline from frame to frame, with the steps reduce, filter, rename, join, and group. |
| `thunderbox-source` | The trait `DataSource`, the type `SourceDescriptor`, and the type `Registry`. |
| `thunderbox-source-prometheus`, `thunderbox-source-sysinfo`, `thunderbox-source-testdata` | One data source in each crate. Plan: a data source for SQL, and a data source for CSV. |
| `thunderbox-dashboard` | The larger way into another program. The crate runs the queries and refreshes them. It holds the time range, the variables, and the selection. It holds each data source as a `dyn DataSource`. It behaves as a library: it never owns `main`, the terminal, the async runtime, or the key bindings. |
| `thunderbox` | The program. It owns the terminal and the event loop, and it drives `thunderbox-dashboard`. It runs in one way today, which is the way alone. Plan: each later way to run will be a subcommand behind a feature. |

The workspace holds eight crates today.
[The architecture](../architecture.md) lists them with their dependencies.

## Decision

One trait serves all four ways to run thunderbox: the two ways that exist
today, and the two ways that the project plans.

```rust
#[async_trait]
pub trait DataSource: Send + Sync {
    fn descriptor(&self) -> SourceDescriptor;
    async fn query(&self, req: QueryRequest) -> Result<Vec<Frame>>;
    async fn label_values(&self, q: &str) -> Result<Vec<String>>;  // for a variable
}
```

`QueryRequest` holds the two moments of the time range that every panel shares,
and the present moment. It also holds the point limit of the panel, and a
suggested step. `thunderbox-dashboard` never knows which implementation it
holds. The implementation is a local data source, a client that reads another
machine, or an object that another program supplies.

## Consequences

`thunderbox-dashboard` holds a `dyn DataSource`, and it behaves as a library.
[ADR 0015](0015-both-ways-into-another-program-are-in-scope.md) states the
rules of that library.

The trait returns the form of answer that
[ADR 0003](0003-agree-on-the-answer-and-not-on-the-question.md) states.
[ADR 0012](0012-the-five-duties-of-a-data-source.md) states the duties of each
implementation. [ADR 0011](0011-add-a-data-source-as-code-behind-a-feature.md)
states how a new data source joins the program.

In the code of today, `label_values` has a default body that returns an empty
list. A data source that offers no choice for a variable therefore writes no
code for it.
