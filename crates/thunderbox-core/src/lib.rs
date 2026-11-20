//! The core types of thunderbox.
//!
//! This crate depends on no user interface library and on no async runtime. An
//! async runtime is a library that runs concurrent tasks. This crate holds the
//! words that every other crate uses:
//!
//! - [`Frame`] is the data that a data source returns.
//! - [`Query`] is the question that a data source answers.
//! - [`Dashboard`] is the document that states what to draw.
//! - [`legend`] holds the rule that names a series.
//! - [`variable`] holds the rule that puts a selected value into a query.

mod dashboard;
mod error;
mod frame;
pub mod legend;
mod query;
mod reduce;
pub(crate) mod time;
pub mod variable;

pub use dashboard::{
    Align, Dashboard, GRID_COLUMNS, GridPos, Panel, PanelKind, SourceDecl, Target, Variable,
};
pub use error::{Error, Result};
pub use frame::{Field, FieldConfig, Frame, Threshold, Values};
pub use query::{Query, QueryRequest, SourceUid, max_data_points};
pub use reduce::Reduce;
pub use time::{
    MAX_SPAN_MS, MIN_SPAN_MS, RelativeTime, TimeRange, TimeWindow, Timestamp, now_ms,
    parse_duration_ms,
};
