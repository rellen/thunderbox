//! The dashboard runtime of thunderbox.
//!
//! This crate is the larger of the two ways to put thunderbox inside another
//! program. It runs the queries of a dashboard, it refreshes them, and it holds
//! the time range and the state of each panel. The program keeps its terminal,
//! its event loop, and its key bindings.
//!
//! The same type serves the program that runs alone, the SSH server, and
//! another program that holds a thunderbox dashboard. Only the owner of the
//! terminal changes. `docs/ARCHITECTURE.md` section 12 states the rules that
//! this crate follows.
//!
//! [`time_key`] holds the rule that maps a key to a new time range.
//! [`variable_key`] holds the rule that maps a key to a choice of a variable.
//! [`row_key`] holds the rule that maps a key to the row panel that it
//! collapses or expands. thunderbox consumes no key of these rules, so the
//! program decides which key reaches them. A program that wants another map
//! ignores these functions.
//!
//! The caller gives [`DashboardRuntime::new`] a `Handle` of its own async
//! runtime. That runtime must hold the timer of tokio, because this crate holds
//! each query to a limit and that limit reads the timer. `#[tokio::main]` and
//! `Runtime::new` each build such a runtime, and a caller that builds its own
//! calls `enable_time` or `enable_all` on the builder. That runtime must also
//! run a task on a thread other than the thread that draws. See
//! [`DashboardRuntime::new`] for the reason.
//!
//! Use [`thunderbox_widgets`] alone when the program already holds its own data.
//! That way needs no async runtime.

mod grid;
mod query;
mod row;
mod runtime;
mod snapshot;
mod time;
mod variable;
mod view;

pub use grid::CollapsedRows;
pub use query::DEFAULT_ANSWER_LIMIT;
pub use row::row_key;
pub use runtime::{AREA_MAX_HOLD, AREA_QUIET_TIME, DashboardRuntime};
pub use snapshot::{PanelState, Snapshot};
pub use time::time_key;
pub use variable::{VariableState, Variables, variable_key};
pub use view::{Consumed, DashboardView, Key};

/// A second name for `thunderbox_widgets`.
///
/// A program that holds a dashboard can read the theme and the marker types
/// from this name.
pub use thunderbox_widgets as widgets;
