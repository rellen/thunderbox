//! The [`DataSource`] trait. This trait is the one extension point of thunderbox.
//!
//! Each data source keeps its own query language. The data sources agree only
//! on the form of the result, which is a [`Frame`].
//!
//! The trait makes no assumption that a data source runs inside the same
//! process. It takes an owned request and returns an owned result, and it holds
//! no borrowed value. A later version can therefore add a data source that runs
//! in a separate process, or one that runs as a WebAssembly module. Neither
//! addition changes a panel. See `docs/ARCHITECTURE.md` section 10.
//!
//! # The rule for the author of a data source
//!
//! A data source has five duties, and it applies no pattern:
//!
//! 1. It gives each column a name. That name is different from the name of
//!    every other column of the answer.
//! 2. It attaches one label to the column for each property that separates one
//!    series from another. The host and the job are two examples.
//! 3. It ends every query in a limited time. It returns an error when it
//!    cannot answer. A data source that reads another machine states its
//!    longest time in [`SourceDescriptor::answer_limit`].
//! 4. It returns the moments of the time column in order, from the oldest to
//!    the newest.
//! 5. It gives every column of one [`Frame`] the same number of rows.
//!
//! The fourth duty holds because `thunderbox_core::Reduce` reads a position in
//! the column and not a moment. `Reduce::Last` takes the last value of the
//! column, and `Reduce::First` takes the first one. A stat panel, a gauge panel
//! and a bar gauge panel each show the number that `Reduce` returns. An answer
//! in another order therefore shows a number that is not the newest one. All
//! three data sources of this repository answer in order, so no test of this
//! repository holds this duty.
//!
//! The fifth duty holds because two panels read a frame of two lengths in two
//! ways. `Frame::time_series` returns an error for such a frame, and both
//! callers of that method drop the error. A chart therefore draws nothing, and
//! a table draws the same frame with an empty cell. A person then reads two
//! panels that disagree, and no error and no message names the cause.
//! `Frame::time_series` compares each number column against the time column. It
//! does not compare a column of text, or a column of true and false values. The
//! duty covers those columns too, because the table panel draws every column,
//! and `Frame::len` takes the length of the longest column.
//!
//! The third duty belongs to the data source, because the caller cannot always
//! keep it. A caller stops a query at an await point. A data source that holds
//! a thread of the async runtime, and that awaits nothing, therefore runs to
//! its end whatever the caller does.
//!
//! The second duty asks one more thing of a data source whose column names
//! already separate one series from another. `Field::display_name` writes the
//! labels of a column beside the name of that column. A column named `0` that
//! carries the label `core` therefore draws as `0{core="0"}`. Such a data
//! source states the short name in `FieldConfig::display_name`, and a target
//! that holds no pattern then draws that short name.
//!
//! The sysinfo data source states that name, because it names each column after
//! the position of the core. The Prometheus data source states none. Every
//! column of one metric carries the same name there, and the labels alone
//! separate those columns.
//!
//! A data source cannot always keep the first duty. The Prometheus data source
//! names a column after the text of the query when the answer holds no name of
//! a metric. Every column of that answer then carries the same name. The
//! labels of the second duty separate those columns, and a person writes a
//! pattern that reads a label.
//!
//! A person states a pattern in the `legend` field of a target.
//! `thunderbox-dashboard` applies that pattern after the data source answers. A
//! [`QueryRequest`] therefore holds no pattern, and a data source cannot read
//! one. `thunderbox_core::legend` holds the rule.
//!
//! A data source that keeps the first two duties obeys a pattern, and it needs
//! no code of its own. The reserved word `{{__name__}}` reads the name of the
//! column, so a data source that attaches no label still obeys a pattern.
//!
//! A program that holds `thunderbox-widgets` alone reads no pattern. That
//! program holds no target and no dashboard, and it names each column itself.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use thunderbox_core::{Error, Frame, QueryRequest, Result};

/// The name of a data source, and the operations that it supports.
///
/// The user interface and the dashboard checks both read this description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDescriptor {
    /// The type of the data source. This value matches the `kind` field in the
    /// `[source.*]` table of a dashboard.
    pub kind: String,
    /// The name that the user interface shows for this data source.
    pub name: String,
    /// `true` if [`DataSource::label_values`] returns useful values. A variable
    /// can then take its choices from this data source.
    pub supports_label_values: bool,
    /// `true` if the data source reduces the number of points itself, to the
    /// limit in `max_data_points`.
    ///
    /// No code reduces the number of points when this field is `false`. Every
    /// read of this field sits inside a test module today, and `Cargo.lock`
    /// holds no crate that reduces a series. Item 7 of section 13 of
    /// `docs/ARCHITECTURE.md` records that absence.
    pub downsamples: bool,
    /// The longest time that this data source takes to answer one query.
    ///
    /// The third duty of a data source is a limited time. This field states
    /// that limit, and the caller stops a query that passes it. `None` means
    /// that the data source states no limit, and the caller then applies a
    /// limit of its own.
    ///
    /// A data source that keeps a timeout of its own states a value a little
    /// longer than that timeout. The data source then reports its own error,
    /// and that error names the fault. The error of the caller names a wait
    /// alone, and a wait names no fault.
    pub answer_limit: Option<Duration>,
}

impl SourceDescriptor {
    /// Returns a description. Both flags start as `false`, and the data
    /// source states no limit.
    pub fn new(kind: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            name: name.into(),
            supports_label_values: false,
            downsamples: false,
            answer_limit: None,
        }
    }

    /// Records the longest time that this data source takes to answer one
    /// query.
    ///
    /// See [`SourceDescriptor::answer_limit`] for the value that a data
    /// source states here.
    pub fn with_answer_limit(mut self, limit: Duration) -> Self {
        self.answer_limit = Some(limit);
        self
    }

    /// Records that the data source reduces the number of points itself.
    pub fn downsampling(mut self) -> Self {
        self.downsamples = true;
        self
    }

    /// Records that the data source returns useful values for a variable.
    pub fn with_label_values(mut self) -> Self {
        self.supports_label_values = true;
        self
    }
}

/// A backend that answers a query.
#[async_trait]
pub trait DataSource: Send + Sync {
    /// Returns the name of this data source, and the operations that it
    /// supports.
    fn descriptor(&self) -> SourceDescriptor;

    /// Answers one query.
    ///
    /// # Errors
    ///
    /// This method returns an error if the data source cannot answer the query.
    /// A caller must show that error in one panel. A caller must never stop the
    /// dashboard because of it.
    async fn query(&self, req: QueryRequest) -> Result<Vec<Frame>>;

    /// Returns the choices for a variable.
    ///
    /// A data source that does not support this operation returns an empty
    /// list. An empty list is not an error.
    ///
    /// # Errors
    ///
    /// This method returns an error if the data source supports the operation
    /// but cannot complete it.
    async fn label_values(&self, _query: &str) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
}

/// The data sources that a dashboard can query. The key is the name of the
/// data source.
#[derive(Clone, Default)]
pub struct Registry {
    sources: BTreeMap<String, Arc<dyn DataSource>>,
}

impl Registry {
    /// Returns a registry that holds no data source.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one data source. A second call with the same name replaces the
    /// first data source.
    pub fn insert(&mut self, uid: impl Into<String>, source: Arc<dyn DataSource>) -> &mut Self {
        self.sources.insert(uid.into(), source);
        self
    }

    /// Adds one data source and returns the registry.
    pub fn with(mut self, uid: impl Into<String>, source: Arc<dyn DataSource>) -> Self {
        self.insert(uid, source);
        self
    }

    /// Returns the data source that has this name.
    ///
    /// # Errors
    ///
    /// This method returns an error if the registry holds no data source with
    /// this name.
    pub fn get(&self, uid: &str) -> Result<&Arc<dyn DataSource>> {
        self.sources
            .get(uid)
            .ok_or_else(|| Error::UnknownSource(uid.to_string()))
    }

    /// Returns the name of every data source, in alphabetical order.
    pub fn uids(&self) -> impl Iterator<Item = &str> {
        self.sources.keys().map(String::as_str)
    }

    /// Returns `true` if the registry holds no data source.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Returns the number of data sources in the registry.
    pub fn len(&self) -> usize {
        self.sources.len()
    }
}

impl std::fmt::Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registry")
            .field("uids", &self.sources.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thunderbox_core::{Field, Query, RelativeTime, TimeRange};

    struct Stub;

    #[async_trait]
    impl DataSource for Stub {
        fn descriptor(&self) -> SourceDescriptor {
            SourceDescriptor::new("stub", "Stub")
        }
        async fn query(&self, _req: QueryRequest) -> Result<Vec<Frame>> {
            Ok(vec![Frame::new(vec![Field::time("time", vec![0])])])
        }
    }

    fn req() -> QueryRequest {
        QueryRequest::new(
            Query {
                source: "a".into(),
                expr: "x".into(),
            },
            &TimeRange {
                from: RelativeTime::Offset(-1000),
                to: RelativeTime::Now,
            },
            0,
            10,
        )
    }

    #[test]
    fn get_returns_an_error_for_an_unknown_name() {
        let reg = Registry::new();
        assert!(reg.get("nope").is_err());
        assert!(reg.is_empty());
    }

    #[test]
    fn get_returns_a_data_source_that_the_registry_holds() {
        let reg = Registry::new().with("a", Arc::new(Stub));
        assert!(reg.get("a").is_ok());
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.uids().collect::<Vec<_>>(), vec!["a"]);
    }

    #[test]
    fn label_values_returns_an_empty_list_by_default() {
        let out = futures_lite_block(Stub.label_values("anything"));
        assert_eq!(out.unwrap(), Vec::<String>::new());
    }

    #[test]
    fn descriptor_starts_with_both_flags_false_and_with_no_limit() {
        let d = Stub.descriptor();
        assert!(
            !d.downsamples,
            "A data source must state that it reduces the points itself."
        );
        assert!(!d.supports_label_values);
        assert_eq!(
            d.answer_limit, None,
            "A new description states no limit. The caller then applies a limit of its own."
        );
    }

    #[test]
    fn a_data_source_can_state_the_longest_time_that_it_takes() {
        let d = SourceDescriptor::new("stub", "Stub").with_answer_limit(Duration::from_secs(35));
        assert_eq!(d.answer_limit, Some(Duration::from_secs(35)));
    }

    #[test]
    fn query_returns_a_frame() {
        let frames = futures_lite_block(Stub.query(req())).unwrap();
        assert_eq!(frames.len(), 1);
    }

    /// Runs a future to completion. This function lets the tests of this crate
    /// run without a dependency on an async runtime.
    ///
    /// The loop polls without a pause and without a limit. The waker that it
    /// builds wakes nothing. The function therefore holds one rule: the future
    /// must reach its end after a limited number of polls, and it must never
    /// wait for a wake. Every data source of this crate awaits nothing, so each
    /// future here reaches its end on the first poll.
    ///
    /// A future that returns `Poll::Pending` even once turns this loop into a
    /// spin that never ends. Such a test reports no failure. It holds a
    /// processor at full speed until the job reaches its limit of time, and the
    /// report then names a timeout. Use an async runtime for a future that
    /// awaits.
    ///
    /// `crates/thunderbox-source-sysinfo/src/lib.rs` and
    /// `crates/thunderbox-source-testdata/src/lib.rs` each hold a copy of this
    /// function, under the name `block_on`.
    ///
    /// # Safety
    ///
    /// The vtable of the waker holds no state, and the pointer of the waker is
    /// null. No function of that vtable reads the pointer, so no code ever
    /// dereferences it. `Pin::new_unchecked` is sound because `fut` stays on
    /// this stack frame, and this function never moves it after the pin.
    fn futures_lite_block<F: std::future::Future>(mut fut: F) -> F::Output {
        use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
        fn vtable() -> &'static RawWakerVTable {
            &RawWakerVTable::new(
                |_| RawWaker::new(std::ptr::null(), vtable()),
                |_| {},
                |_| {},
                |_| {},
            )
        }
        let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), vtable())) };
        let mut cx = Context::from_waker(&waker);
        let mut fut = unsafe { std::pin::Pin::new_unchecked(&mut fut) };
        loop {
            if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }
}
