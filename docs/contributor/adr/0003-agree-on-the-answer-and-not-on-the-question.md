# ADR 0003: Agree on the form of the answer, and not on the form of the question

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 2 |
| Code | `crates/thunderbox-core/src/frame.rs`, `crates/thunderbox-core/src/query.rs` |
| Tests | No test holds this decision alone. The types of `thunderbox-core` hold it. |
| Related | [ADR 0005](0005-one-trait-is-the-seam-of-every-data-source.md), [ADR 0012](0012-the-five-duties-of-a-data-source.md), [ADR 0017](0017-the-dashboard-applies-the-pattern-of-a-legend.md), [ADR 0034](0034-a-dashboard-reads-more-than-one-data-source.md) |

## Context

This record holds the main decision of the project.

Grafana is a dashboard program that runs in a web browser. Grafana has no
common query language. Each data source states its own type of query. One data
source reads PromQL, and another reads SQL. The answer is the part that all of
them share. A data frame is a set of columns. Each data source returns data
frames, and a panel reads a frame without knowledge of its origin.

## Decision

The project agrees on the form of the answer. It does not agree on the form of
the question.

Every data source returns this form of answer:

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

A panel states the shape of the data that it needs. A time column beside one or
more number columns is an example of a shape. A panel never states a data
source.

thunderbox does not interpret the text of a query. Each data source reads its
own form:

```rust
pub struct Query {
    pub source: SourceUid,
    pub expr: String,
}
```

A query holds the name of a data source and a text. It holds nothing else. Four
values sit beside the query, in `QueryRequest`:

- the two moments of the range;
- the present moment;
- the number of points that a panel can draw;
- a suggested distance between two points.

A data source receives one `QueryRequest`, and that value holds the query as
well. `crates/thunderbox-core/src/query.rs` holds the two types.

## Consequences

When the project follows this rule, the other parts of the design follow from
it. When the project breaks it, the project spends its time on a common query
language that fits nothing.

The trait of [ADR 0005](0005-one-trait-is-the-seam-of-every-data-source.md)
returns this form of answer. [ADR 0012](0012-the-five-duties-of-a-data-source.md)
states what a data source must write into it.
