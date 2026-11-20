//! Tests for the rules that make this crate behave as a library.
//!
//! ADR 0015 and ADR 0016 in `docs/contributor/adr` state those rules. Each test here holds
//! one of them to its word.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};
use thunderbox_core::{
    Dashboard, Error, Field, Frame, QueryRequest, RelativeTime, Result, TimeRange, Timestamp,
};
use thunderbox_dashboard::{
    AREA_MAX_HOLD, AREA_QUIET_TIME, Consumed, DashboardRuntime, DashboardView, Key, PanelState,
    Snapshot, time_key,
};
use thunderbox_source::{DataSource, Registry, SourceDescriptor};
use thunderbox_widgets::Marker;
use tokio::sync::Semaphore;

mod common;

use common::settle;

const TWO_PANELS: &str = r#"
title = "test"
[source.good]
kind = "testdata"
[source.bad]
kind = "testdata"

[[panel]]
title = "first"
kind = "timeseries"
grid = { x = 0, y = 0, w = 12, h = 8 }
[[panel.target]]
source = "good"
expr = "up"

[[panel]]
title = "second"
kind = "timeseries"
grid = { x = 12, y = 0, w = 12, h = 8 }
[[panel.target]]
source = "bad"
expr = "up"
"#;

const ONE_PANEL: &str = r#"
title = "test"
[source.one]
kind = "test"

[[panel]]
title = "first"
kind = "timeseries"
grid = { x = 0, y = 0, w = 12, h = 8 }
[[panel.target]]
source = "one"
expr = "up"
"#;

/// A dashboard whose six panels hold the grid columns that the default
/// dashboard held when the project took the rule of the point limit.
///
/// The default dashboard sits in `crates/thunderbox/src/default.toml`. Its panels
/// held 16, 8, 8, 8, 8 and 24 columns of the grid of 24 columns, and they stood
/// in three rows. A later change gave half of the last row to the panel of the
/// processes. This dashboard keeps the old grid, because the test below records
/// the measurement of that rule.
const SIX_PANELS: &str = r#"
title = "test"
[source.one]
kind = "testdata"

[[panel]]
title = "cpu"
kind = "timeseries"
grid = { x = 0, y = 0, w = 16, h = 7 }
[[panel.target]]
source = "one"
expr = "up"

[[panel]]
title = "cpu now"
kind = "stat"
grid = { x = 16, y = 0, w = 8, h = 7 }
[[panel.target]]
source = "one"
expr = "up"

[[panel]]
title = "memory"
kind = "gauge"
grid = { x = 0, y = 7, w = 8, h = 6 }
[[panel.target]]
source = "one"
expr = "up"

[[panel]]
title = "cores"
kind = "bargauge"
grid = { x = 8, y = 7, w = 8, h = 6 }
[[panel.target]]
source = "one"
expr = "up"

[[panel]]
title = "network"
kind = "timeseries"
grid = { x = 16, y = 7, w = 8, h = 6 }
[[panel.target]]
source = "one"
expr = "up"

[[panel]]
title = "the most recent rows for each core"
kind = "table"
grid = { x = 0, y = 13, w = 24, h = 6 }
[[panel.target]]
source = "one"
expr = "up"
"#;

/// A dashboard of eight panels, one above the other.
///
/// Each panel holds the whole width of the grid and 8 rows of it. The renderer
/// draws seven of the eight panels on a terminal of 24 rows, and it drops the
/// first one. ADR 0024 in `docs/contributor/adr` holds that measurement.
const EIGHT_PANELS: &str = r#"
title = "test"
[source.one]
kind = "testdata"

[[panel]]
title = "panel 0"
kind = "timeseries"
grid = { x = 0, y = 0, w = 24, h = 8 }
[[panel.target]]
source = "one"
expr = "up"

[[panel]]
title = "panel 1"
kind = "timeseries"
grid = { x = 0, y = 8, w = 24, h = 8 }
[[panel.target]]
source = "one"
expr = "up"

[[panel]]
title = "panel 2"
kind = "timeseries"
grid = { x = 0, y = 16, w = 24, h = 8 }
[[panel.target]]
source = "one"
expr = "up"

[[panel]]
title = "panel 3"
kind = "timeseries"
grid = { x = 0, y = 24, w = 24, h = 8 }
[[panel.target]]
source = "one"
expr = "up"

[[panel]]
title = "panel 4"
kind = "timeseries"
grid = { x = 0, y = 32, w = 24, h = 8 }
[[panel.target]]
source = "one"
expr = "up"

[[panel]]
title = "panel 5"
kind = "timeseries"
grid = { x = 0, y = 40, w = 24, h = 8 }
[[panel.target]]
source = "one"
expr = "up"

[[panel]]
title = "panel 6"
kind = "timeseries"
grid = { x = 0, y = 48, w = 24, h = 8 }
[[panel.target]]
source = "one"
expr = "up"

[[panel]]
title = "panel 7"
kind = "timeseries"
grid = { x = 0, y = 56, w = 24, h = 8 }
[[panel.target]]
source = "one"
expr = "up"
"#;

/// A dashboard of three panels, one above the other. The middle panel holds 4
/// rows of the grid, and the other two hold 10 rows each.
///
/// The renderer drops the middle panel on an area of 12 rows. It gives 5 cells
/// to each of the other two panels, and 2 cells to the middle panel.
const MIDDLE_IS_SMALL: &str = r#"
title = "test"
[source.one]
kind = "testdata"

[[panel]]
title = "first"
kind = "timeseries"
grid = { x = 0, y = 0, w = 24, h = 10 }
[[panel.target]]
source = "one"
expr = "up"

[[panel]]
title = "second"
kind = "timeseries"
grid = { x = 0, y = 10, w = 24, h = 4 }
[[panel.target]]
source = "one"
expr = "up"

[[panel]]
title = "third"
kind = "timeseries"
grid = { x = 0, y = 14, w = 24, h = 10 }
[[panel.target]]
source = "one"
expr = "up"
"#;

/// A dashboard of one panel that holds no target.
const NO_TARGET: &str = r#"
title = "test"

[[panel]]
title = "first"
kind = "timeseries"
grid = { x = 0, y = 0, w = 12, h = 8 }
"#;

/// A data source that counts its calls and records the point limit of each
/// one.
///
/// Each panel of a dashboard holds a width of its own, so each panel asks for a
/// number of its own. This data source therefore keeps every number, and not
/// the number of the query that arrived last.
#[derive(Default)]
struct Counter {
    calls: AtomicUsize,
    budgets: Mutex<Vec<u32>>,
}

impl Counter {
    /// Returns the point limit of each query so far, and forgets them all.
    ///
    /// The numbers come back in order, from the smallest to the largest. Two
    /// panels answer in the order that their two tasks end, and a test must not
    /// read that order.
    fn take_budgets(&self) -> Vec<u32> {
        let mut held = self.budgets.lock().expect("No test holds this lock twice.");
        let mut budgets = std::mem::take(&mut *held);
        budgets.sort_unstable();
        budgets
    }
}

#[async_trait]
impl DataSource for Counter {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor::new("counter", "Counter")
    }
    async fn query(&self, req: QueryRequest) -> Result<Vec<Frame>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.budgets
            .lock()
            .expect("No test holds this lock twice.")
            .push(req.max_data_points);
        Ok(vec![Frame::named(
            "f",
            vec![
                Field::time("time", vec![0, 1000]),
                Field::f64("v", vec![Some(1.0), Some(2.0)]),
            ],
        )])
    }
}

/// A data source that always returns an error.
struct Broken;

#[async_trait]
impl DataSource for Broken {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor::new("broken", "Broken")
    }
    async fn query(&self, _req: QueryRequest) -> Result<Vec<Frame>> {
        Err(Error::Query("the machine did not answer".into()))
    }
}

/// A data source that never answers, so a test can watch a query in flight.
struct Slow;

#[async_trait]
impl DataSource for Slow {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor::new("slow", "Slow")
    }
    async fn query(&self, _req: QueryRequest) -> Result<Vec<Frame>> {
        tokio::time::sleep(Duration::from_secs(3600)).await;
        Ok(Vec::new())
    }
}

/// A data source that never answers, and that counts the queries that run now.
///
/// The data source raises `in_flight` when a query starts, and [`InFlight`]
/// lowers that number when the query ends. The field `started` counts every
/// query that this data source ever started. A test therefore learns whether a
/// query of the runtime still runs.
///
/// The runtime stops a task at an await point, and it then drops the future of
/// that query. The drop of [`InFlight`] lowers the number, so a query that the
/// runtime stopped counts as a query that ended.
struct Counted {
    /// The number of the queries of this data source that run now.
    in_flight: Arc<AtomicUsize>,
    /// The number of the queries that this data source ever started.
    started: Arc<AtomicUsize>,
}

/// The value that lowers the count of the queries that run now.
struct InFlight(Arc<AtomicUsize>);

impl Drop for InFlight {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

#[async_trait]
impl DataSource for Counted {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor::new("counted", "Counted")
    }
    async fn query(&self, _req: QueryRequest) -> Result<Vec<Frame>> {
        self.started.fetch_add(1, Ordering::SeqCst);
        self.in_flight.fetch_add(1, Ordering::SeqCst);
        let _count = InFlight(self.in_flight.clone());
        tokio::time::sleep(Duration::from_secs(3600)).await;
        Ok(Vec::new())
    }
}

/// Returns a registry of two data sources that share one pair of counters.
///
/// The dashboard of these tests holds two panels, and each panel names one of
/// the two data sources. The counters therefore count the queries of both
/// panels together.
fn counted(in_flight: &Arc<AtomicUsize>, started: &Arc<AtomicUsize>) -> Registry {
    let one = Arc::new(Counted {
        in_flight: in_flight.clone(),
        started: started.clone(),
    });
    let two = Arc::new(Counted {
        in_flight: in_flight.clone(),
        started: started.clone(),
    });
    Registry::new().with("good", one).with("bad", two)
}

/// Waits until a condition holds, and fails the test when it never holds.
///
/// A test cannot read a task of another thread at a chosen moment. The helper
/// therefore reads the condition again every millisecond, and it fails the test
/// after five seconds. The text `what` names the condition, so the report of a
/// failure states what the test waited for.
async fn until(mut condition: impl FnMut() -> bool, what: &str) {
    for _ in 0..5000 {
        if condition() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    panic!("This condition did not hold in five seconds: {what}");
}

/// A data source that never answers, and that states its own limit.
///
/// The third duty of a data source is a limited time. This data source does
/// not keep that duty, so the runtime stops each of its queries at the limit
/// that the descriptor states. A test that drives this data source therefore
/// waits for the report of the runtime, and not for a period of its own.
struct NeverAnswers;

/// The limit that the data source above states.
const NEVER_ANSWERS_LIMIT: Duration = Duration::from_millis(50);

#[async_trait]
impl DataSource for NeverAnswers {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor::new("never", "Never").with_answer_limit(NEVER_ANSWERS_LIMIT)
    }
    async fn query(&self, _req: QueryRequest) -> Result<Vec<Frame>> {
        tokio::time::sleep(Duration::from_secs(3600)).await;
        Ok(Vec::new())
    }
}

/// A data source that answers after a short wait.
///
/// A test that stops a set of queries needs a task that still runs. This data
/// source parks at an await point, so the runtime stops it there.
struct Brief;

#[async_trait]
impl DataSource for Brief {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor::new("brief", "Brief")
    }
    async fn query(&self, _req: QueryRequest) -> Result<Vec<Frame>> {
        tokio::time::sleep(Duration::from_millis(5)).await;
        Ok(vec![Frame::named(
            "f",
            vec![
                Field::time("time", vec![0, 1000]),
                Field::f64("v", vec![Some(1.0), Some(2.0)]),
            ],
        )])
    }
}

/// A data source that panics on its first query.
///
/// The task of the panel of this data source then ends before it gives an
/// answer, and that panel holds an error. A test that runs this data source
/// writes the report of a panic on the output of the test.
struct PanicsAtOnce;

#[async_trait]
impl DataSource for PanicsAtOnce {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor::new("panics", "Panics")
    }
    async fn query(&self, _req: QueryRequest) -> Result<Vec<Frame>> {
        panic!("This data source panics on every query.");
    }
}

/// A data source that never answers the choices of a variable.
///
/// The data source answers a query at once, so the task of a set of queries
/// ends. The task that asks for the choices of a variable stays, and a test
/// then watches the runtime stop that task.
struct SlowChoices;

#[async_trait]
impl DataSource for SlowChoices {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor::new("slow choices", "Slow choices").with_label_values()
    }
    async fn query(&self, _req: QueryRequest) -> Result<Vec<Frame>> {
        Ok(Vec::new())
    }
    async fn label_values(&self, _query: &str) -> Result<Vec<String>> {
        tokio::time::sleep(Duration::from_secs(3600)).await;
        Ok(Vec::new())
    }
}

/// A dashboard whose one variable takes its choices from a data source.
const A_VARIABLE: &str = r#"
title = "test"
[source.one]
kind = "slow choices"

[[variable]]
name = "job"
query = { source = "one", expr = "job" }

[[panel]]
title = "first"
kind = "timeseries"
grid = { x = 0, y = 0, w = 12, h = 8 }
[[panel.target]]
source = "one"
expr = "up"
"#;

/// A data source that answers one query, and panics after that one.
///
/// A panic inside a data source ends the task of its own panel before that task
/// gives an answer. A test that runs this data source therefore writes the
/// report of a panic on the output of the test.
struct PanicsAfterOne {
    calls: AtomicUsize,
}

#[async_trait]
impl DataSource for PanicsAfterOne {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor::new("panics", "Panics")
    }
    async fn query(&self, _req: QueryRequest) -> Result<Vec<Frame>> {
        assert_eq!(
            self.calls.fetch_add(1, Ordering::SeqCst),
            0,
            "This data source answers one query, and it panics after that one."
        );
        Ok(vec![Frame::named(
            "f",
            vec![
                Field::time("time", vec![0, 1000]),
                Field::f64("v", vec![Some(1.0), Some(2.0)]),
            ],
        )])
    }
}

/// A data source that answers only when a test gives it a permit.
///
/// The data source names its one column after the number of the call. A test
/// therefore reads which set of queries an answer came from. A set of queries
/// is the whole group of queries that one refresh sends.
///
/// A test that holds the permit decides when a set of queries answers. That
/// test therefore needs no period of a fixed length to make an answer late.
struct Gate {
    permits: Arc<Semaphore>,
    runs: AtomicUsize,
}

impl Gate {
    /// Returns a data source that answers no query until a test adds a permit.
    fn new(permits: Arc<Semaphore>) -> Self {
        Self {
            permits,
            runs: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl DataSource for Gate {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor::new("gate", "Gate")
    }
    async fn query(&self, _req: QueryRequest) -> Result<Vec<Frame>> {
        self.permits
            .acquire()
            .await
            .expect("The test must hold the semaphore open.")
            .forget();
        let run = self.runs.fetch_add(1, Ordering::SeqCst);
        Ok(vec![Frame::named(
            "f",
            vec![
                Field::time("time", vec![0, 1000]),
                Field::f64(format!("run {run}"), vec![Some(1.0), Some(2.0)]),
            ],
        )])
    }
}

/// Returns the name of the first column of numbers that a panel holds.
fn column_name(snapshot: &Snapshot, panel: usize) -> String {
    let PanelState::Ready(frames) = snapshot.panel(panel) else {
        panic!("The panel must hold an answer. {:?}", snapshot.panel(panel));
    };
    frames
        .iter()
        .flat_map(|f| f.numeric_fields())
        .map(Field::display_name)
        .next()
        .expect("The answer must hold a column of numbers.")
}

fn dashboard() -> Dashboard {
    Dashboard::from_toml(TWO_PANELS).expect("The test dashboard must be correct.")
}

/// The height that every test of a point limit reports.
///
/// The renderer drops a panel that holds under 3 cells in either direction, and
/// the runtime then asks the data source of that panel for nothing. A test that
/// measures a point limit must therefore report a height that drops no panel of
/// its own dashboard. This height draws every panel of every dashboard of this
/// file. A test that measures a panel which the renderer drops states a small
/// height of its own.
const TALL_ENOUGH: u16 = 40;

/// Returns an area of this width that draws every panel.
fn area(width: u16) -> Size {
    Size::new(width, TALL_ENOUGH)
}

/// Returns one area of a drag of the corner of a window.
///
/// A person who drags a corner changes the width and the height together. The
/// two numbers rarely stand still on one frame, and they rarely change on the
/// same frame either. Each step of this drag therefore changes both.
fn drag(step: u16) -> Size {
    Size::new(100 + step, TALL_ENOUGH - step % 5)
}

/// Returns one area of a panel area that never stands still.
///
/// The area grows and shrinks by one cell in each direction. A scroll bar that
/// appears and disappears makes such an area.
fn shake(turn: u16) -> Size {
    if turn.is_multiple_of(2) {
        Size::new(120, TALL_ENOUGH)
    } else {
        Size::new(121, TALL_ENOUGH - 1)
    }
}

/// Returns the number of panels that the renderer draws in this area.
///
/// The renderer writes the title of a panel into the top border of that panel.
/// The helper therefore draws the dashboard into a buffer, and it counts the
/// titles that the buffer holds. It reads the screen itself, and it reads no
/// rule of the crate.
fn drawn_panels(dashboard: &Dashboard, area: Rect) -> usize {
    let mut buffer = Buffer::empty(area);
    let snapshot = Snapshot::loading(dashboard.panels.len(), TimeRange::default());
    DashboardView::new().render(dashboard, &snapshot, area, &mut buffer);
    let screen: String = buffer
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    dashboard
        .panels
        .iter()
        .filter(|panel| screen.contains(panel.title.as_str()))
        .count()
}

fn handle() -> tokio::runtime::Handle {
    tokio::runtime::Handle::current()
}

#[tokio::test]
async fn one_data_source_that_returns_an_error_changes_one_panel_only() {
    let registry = Registry::new()
        .with("good", Arc::new(Counter::default()))
        .with("bad", Arc::new(Broken));
    let runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;

    let snap = runtime.snapshot();
    assert!(
        matches!(snap.panel(0), PanelState::Ready(_)),
        "The first panel must hold its data."
    );
    assert_eq!(
        snap.panel(1).error(),
        Some("The data source returned an error. the machine did not answer"),
        "The second panel must hold the text of the error."
    );
    assert_eq!(snap.error_count(), 1);
}

#[tokio::test]
async fn the_point_limit_follows_the_width_of_the_panel_and_the_marker() {
    let counter = Arc::new(Counter::default());
    let registry = Registry::new()
        .with("good", counter.clone())
        .with("bad", counter.clone());
    let mut runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    // Forget the set of queries that ran before a caller reported a width.
    counter.take_budgets();

    runtime.set_panel_area(area(100));
    settle(&runtime).await;
    assert_eq!(
        counter.take_budgets(),
        vec![100, 100],
        "Each of the two panels holds 50 of the 100 cells, and a braille \
         marker addresses two positions in each cell."
    );

    runtime.set_marker(Marker::Block);
    settle(&runtime).await;
    assert_eq!(
        counter.take_budgets(),
        vec![50, 50],
        "A block marker addresses one position in each cell."
    );
}

/// Each panel holds a width of its own, and the point limit of a panel follows
/// that width. The panels of this dashboard hold the grid columns that the
/// default dashboard gave its own panels when the project took this rule.
///
/// The runtime asked every panel for the points of the whole area before this
/// rule. Each of the six panels then asked for 220 points, and the dashboard
/// asked for 1320 points. ADR 0023 in `docs/contributor/adr` holds the same numbers.
#[tokio::test]
async fn each_panel_asks_for_the_points_of_its_own_width() {
    let counter = Arc::new(Counter::default());
    let dashboard = Dashboard::from_toml(SIX_PANELS).expect("The dashboard must be correct.");
    let registry = Registry::new().with("one", counter.clone());
    let mut runtime =
        DashboardRuntime::new(dashboard, registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    counter.take_budgets();

    runtime.set_panel_area(area(110));
    settle(&runtime).await;
    let budgets = counter.take_budgets();
    assert_eq!(
        budgets,
        vec![72, 74, 74, 74, 146, 220],
        "A panel of 36 cells asks for 72 points, and a panel of 110 cells asks \
         for 220 points."
    );
    assert_eq!(
        budgets.iter().sum::<u32>(),
        660,
        "The whole dashboard must ask for half of the 1320 points that it \
         asked for before this rule."
    );
}

/// The renderer and the runtime must never disagree about a panel. The
/// renderer draws no panel under 3 cells wide or under 3 cells high, and the
/// runtime asks the data source of such a panel for nothing. A rule in two
/// places would let the runtime ask about a panel that nobody draws. It would
/// also leave a panel that a person reads with no data at all.
///
/// The test measures the screen and the data source at each height of a
/// terminal. It counts the titles that the renderer wrote, and it counts the
/// queries that reached the data source.
#[tokio::test]
async fn the_runtime_asks_about_exactly_the_panels_that_the_renderer_draws() {
    let dashboard = Dashboard::from_toml(EIGHT_PANELS).expect("The dashboard must be correct.");
    for height in 3..=30_u16 {
        let counter = Arc::new(Counter::default());
        let registry = Registry::new().with("one", counter.clone());
        let mut runtime = DashboardRuntime::new(dashboard.clone(), registry, handle())
            .expect("The runtime must start.");
        settle(&runtime).await;
        counter.calls.store(0, Ordering::SeqCst);

        runtime.set_panel_area(Size::new(110, height));
        settle(&runtime).await;
        assert_eq!(
            counter.calls.load(Ordering::SeqCst),
            drawn_panels(&dashboard, Rect::new(0, 0, 110, height)),
            "The runtime and the renderer must agree on an area of {height} rows."
        );
    }
}

/// A panel that the renderer drops must ask a data source for nothing.
///
/// The dashboard holds eight panels in one column. A terminal of 24 rows gives
/// the panels 23 rows, and the renderer then draws seven of the eight panels.
/// The runtime therefore sends seven queries and not eight.
/// ADR 0024 in `docs/contributor/adr` holds that measurement.
#[tokio::test]
async fn a_panel_that_the_renderer_drops_asks_a_data_source_for_nothing() {
    let counter = Arc::new(Counter::default());
    let dashboard = Dashboard::from_toml(EIGHT_PANELS).expect("The dashboard must be correct.");
    let registry = Registry::new().with("one", counter.clone());
    let mut runtime =
        DashboardRuntime::new(dashboard, registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    counter.calls.store(0, Ordering::SeqCst);

    runtime.set_panel_area(Size::new(110, 23));
    settle(&runtime).await;
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        7,
        "Seven of the eight panels must reach the data source."
    );
    let snapshot = runtime.snapshot();
    assert!(
        matches!(snapshot.panel(0), PanelState::Ready(_)),
        "The panel that the renderer drops must keep the answer that it holds. {:?}",
        snapshot.panel(0)
    );
    for index in 1..8 {
        assert!(
            matches!(snapshot.panel(index), PanelState::Ready(_)),
            "The panel {index} must hold its answer. {:?}",
            snapshot.panel(index)
        );
    }
    assert_eq!(
        snapshot.error_count(),
        0,
        "A panel that the renderer drops is no fault."
    );
}

/// A snapshot holds one state for each panel of the dashboard, and a panel that
/// the renderer drops moves no other panel. A set of queries that took such a
/// panel out of its list would give panel 1 the answer of panel 2.
///
/// The renderer drops the middle panel of this dashboard on an area of 12 rows.
#[tokio::test]
async fn a_panel_that_the_renderer_drops_moves_no_other_panel() {
    let counter = Arc::new(Counter::default());
    let dashboard = Dashboard::from_toml(MIDDLE_IS_SMALL).expect("The dashboard must be correct.");
    let registry = Registry::new().with("one", counter.clone());
    let mut runtime = DashboardRuntime::new(dashboard.clone(), registry, handle())
        .expect("The runtime must start.");
    settle(&runtime).await;
    counter.calls.store(0, Ordering::SeqCst);

    let area = Rect::new(0, 0, 110, 12);
    assert_eq!(
        drawn_panels(&dashboard, area),
        2,
        "The renderer must draw the first panel and the third panel."
    );
    runtime.set_panel_area(area.as_size());
    settle(&runtime).await;
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        2,
        "Two of the three panels must reach the data source."
    );
    let snapshot = runtime.snapshot();
    assert_eq!(
        snapshot.panels.len(),
        3,
        "The snapshot must hold one state for each panel."
    );
    assert!(
        matches!(snapshot.panel(0), PanelState::Ready(_)),
        "The first panel must hold its answer. {:?}",
        snapshot.panel(0)
    );
    assert!(
        matches!(snapshot.panel(1), PanelState::Ready(_)),
        "The middle panel must keep the answer that it holds. {:?}",
        snapshot.panel(1)
    );
    assert!(
        matches!(snapshot.panel(2), PanelState::Ready(_)),
        "The third panel must hold its answer. {:?}",
        snapshot.panel(2)
    );
}

/// Returns a dashboard of `count` panels, one above the other.
///
/// Each panel holds the whole width of the grid and 8 rows of it. A column of
/// 40 panels therefore needs an area of 120 rows, and no terminal is that tall.
fn column_dashboard(count: u16) -> Dashboard {
    let mut text = String::from("title = \"test\"\n[source.one]\nkind = \"testdata\"\n");
    for index in 0..count {
        text.push_str(&format!(
            "[[panel]]\ntitle = \"panel {index}\"\nkind = \"timeseries\"\n\
             grid = {{ x = 0, y = {}, w = 24, h = 8 }}\n\
             [[panel.target]]\nsource = \"one\"\nexpr = \"up\"\n",
            index * 8
        ));
    }
    Dashboard::from_toml(&text).expect("The dashboard must be correct.")
}

/// A caller that reports no area must still receive every panel. The runtime
/// knows no height before the first report, so it can drop no panel. A test of
/// this file that never renders is such a caller. A program that holds a
/// dashboard behind a screen that it does not draw is another one.
///
/// The dashboard of this test needs an area of 120 rows, and no terminal holds
/// that many rows. A runtime that guessed a height would therefore drop every
/// panel of it.
#[tokio::test]
async fn a_runtime_that_receives_no_area_asks_about_every_panel() {
    let counter = Arc::new(Counter::default());
    let registry = Registry::new().with("one", counter.clone());
    let runtime = DashboardRuntime::new(column_dashboard(40), registry, handle())
        .expect("The runtime must start.");
    settle(&runtime).await;
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        40,
        "Every panel must reach the data source while no caller reports an area."
    );
    assert!(
        !runtime.snapshot().is_loading(),
        "Every panel must hold an answer."
    );
}

/// A person makes a terminal window larger, and a panel that the renderer
/// dropped appears again. That panel holds no answer, so the area must reach a
/// data source and the panel must fill.
#[tokio::test]
async fn a_panel_that_the_terminal_grows_to_hold_asks_a_data_source_again() {
    let counter = Arc::new(Counter::default());
    let dashboard = Dashboard::from_toml(EIGHT_PANELS).expect("The dashboard must be correct.");
    let registry = Registry::new().with("one", counter.clone());
    let mut runtime =
        DashboardRuntime::new(dashboard, registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    let first = counter.calls.load(Ordering::SeqCst);

    runtime.set_panel_area(Size::new(110, 23));
    settle(&runtime).await;
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        first + 7,
        "The renderer drops the first panel on an area of 23 rows, so seven \
         panels of the eight reach the data source."
    );

    runtime.set_panel_area(Size::new(110, 40));
    assert!(
        tick_until_it_starts(&mut runtime).await,
        "A larger area must reach a set of queries."
    );
    settle(&runtime).await;
    assert!(
        matches!(runtime.snapshot().panel(0), PanelState::Ready(_)),
        "The panel that the terminal grew to hold must fill. {:?}",
        runtime.snapshot().panel(0)
    );
}

/// Drives `tick` until the runtime has taken the area that waits.
///
/// A tick takes an area that costs no set of queries, and it starts no set for
/// such an area. A test therefore cannot wait for a set of queries there. The
/// loop runs for longer than [`AREA_QUIET_TIME`], which is the time that the
/// runtime holds an area that stands still.
async fn tick_until_it_takes_the_area(runtime: &mut DashboardRuntime) {
    for _ in 0..30 {
        runtime.tick();
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// Drives `tick` until it starts a set of queries, or until the loop has taken
/// one hundred turns.
///
/// The runtime reads the clock inside `tick`, so a test that waits for a width
/// calls `tick` again after the quiet time. This loop waits for the report of
/// the runtime, and a machine that is slower only takes more turns of it.
async fn tick_until_it_starts(runtime: &mut DashboardRuntime) -> bool {
    for _ in 0..100 {
        if runtime.tick() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    false
}

/// Returns a runtime of two panels that share one data source.
fn with_counter(counter: Arc<Counter>, refresh: Option<&str>) -> DashboardRuntime {
    let mut dash = dashboard();
    dash.refresh = refresh.map(str::to_string);
    let registry = Registry::new()
        .with("good", counter.clone())
        .with("bad", counter);
    DashboardRuntime::new(dash, registry, handle()).expect("The runtime must start.")
}

/// The runtime asks for `ASSUMED_WIDTH` before a caller draws a frame, because
/// it knows no area then. The first area of a caller must therefore reach the
/// data source at once. A first picture that holds the wrong number of points
/// would otherwise stand until the next set of queries.
#[tokio::test]
async fn the_first_area_that_a_caller_reports_reaches_the_data_source_at_once() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), None);
    settle(&runtime).await;

    runtime.set_panel_area(area(100));
    assert!(
        runtime.is_refreshing(),
        "The first area of a caller must start a set of queries at once."
    );
    settle(&runtime).await;
    assert_eq!(
        counter.take_budgets(),
        vec![80, 80, 100, 100],
        "The first set of queries carries the assumed width of 80 cells, and \
         the set of the first area of the caller carries 100 cells."
    );
}

/// A person drags the corner of a terminal window, and the caller reports a new
/// area on every frame. Each of those areas cost a set of queries before this
/// rule. A data source on another machine then received one request for each
/// frame, and the runtime stopped every one of them but the last.
///
/// A drag of a corner changes the width and the height together. The runtime
/// holds the two numbers in one field, so that drag costs one set of queries.
/// Two fields, each with a wait of its own, would cost two sets.
///
/// The dashboard of this test refreshes every ten seconds, so no tick here can
/// start a set of queries for the interval.
#[tokio::test]
async fn a_run_of_new_areas_costs_one_set_of_queries() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), Some("10s"));
    settle(&runtime).await;
    // The first area of a caller reaches the data source at once. This report
    // takes that step out of the run below.
    runtime.set_panel_area(area(90));
    settle(&runtime).await;
    let before = counter.calls.load(Ordering::SeqCst);
    counter.take_budgets();

    for step in 0..30 {
        runtime.set_panel_area(drag(step));
        assert!(
            !runtime.is_refreshing(),
            "An area that a caller reported now must start no set of queries. {step}"
        );
        assert!(
            !runtime.tick(),
            "An area that still changes must start no set of queries. {step}"
        );
    }
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        before,
        "An area that still changes must reach no data source."
    );

    assert!(
        tick_until_it_starts(&mut runtime).await,
        "An area that stands still must reach a new set of queries."
    );
    settle(&runtime).await;
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        before + 2,
        "Thirty areas must cost one set of queries, which is one query for each panel."
    );
    assert_eq!(
        counter.take_budgets(),
        vec![128, 130],
        "The last area of the run must reach the data source. That area holds \
         129 cells, so one panel holds 64 cells and the other holds 65."
    );
}

/// A dashboard that states no refresh interval never ticks for the clock. The
/// area must still reach the data source, so the runtime applies an area that
/// waits before it reads the interval.
#[tokio::test]
async fn a_dashboard_that_states_no_refresh_interval_still_receives_a_new_area() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), None);
    settle(&runtime).await;
    runtime.set_panel_area(area(90));
    settle(&runtime).await;
    let before = counter.calls.load(Ordering::SeqCst);
    counter.take_budgets();

    runtime.set_panel_area(area(120));
    assert!(
        !runtime.is_refreshing(),
        "An area that a caller reported now must wait."
    );
    assert!(
        tick_until_it_starts(&mut runtime).await,
        "A tick must give the area to a set of queries, whatever the interval states."
    );
    settle(&runtime).await;
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        before + 2,
        "The area must cost one set of queries."
    );
    assert_eq!(
        counter.take_budgets(),
        vec![120, 120],
        "The new area must reach the data source. Each panel holds 60 of the \
         120 cells."
    );
    assert!(
        runtime.refresh_interval().is_none(),
        "This dashboard states no refresh interval."
    );
}

/// The program calls `set_panel_area`, then `set_marker`, then `tick`, in that
/// order on every frame. A person who changes the size of the window and then
/// presses the key of the marker meets that pair. The one set of queries that
/// follows must carry the new area and the new marker.
#[tokio::test]
async fn a_new_marker_after_a_new_area_sends_one_set_of_queries_that_carries_both() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), None);
    settle(&runtime).await;
    runtime.set_panel_area(area(90));
    settle(&runtime).await;
    let before = counter.calls.load(Ordering::SeqCst);
    counter.take_budgets();

    runtime.set_panel_area(area(120));
    runtime.set_marker(Marker::Block);
    settle(&runtime).await;
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        before + 2,
        "The pair must cost one set of queries."
    );
    assert_eq!(
        counter.take_budgets(),
        vec![60, 60],
        "The set of queries must carry the new area and the new marker. Each \
         panel holds 60 cells, and a block marker addresses one position in \
         each cell."
    );
}

/// A person drags the corner of a window and then drags it back. The area of
/// the set of queries that runs is then correct again, and the runtime must ask
/// no data source.
#[tokio::test]
async fn an_area_that_returns_to_the_area_in_use_starts_no_set_of_queries() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), None);
    settle(&runtime).await;
    runtime.set_panel_area(area(100));
    settle(&runtime).await;
    let before = counter.calls.load(Ordering::SeqCst);

    runtime.set_panel_area(area(120));
    runtime.set_panel_area(area(100));
    tokio::time::sleep(AREA_QUIET_TIME * 2).await;
    for _ in 0..10 {
        assert!(
            !runtime.tick(),
            "An area that returns to the area in use must start no set of queries."
        );
    }
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        before,
        "The runtime must ask no data source again."
    );
}

#[tokio::test]
async fn the_same_area_does_not_start_the_queries_again() {
    let counter = Arc::new(Counter::default());
    let registry = Registry::new()
        .with("good", counter.clone())
        .with("bad", counter.clone());
    let mut runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    let before = counter.calls.load(Ordering::SeqCst);

    for _ in 0..10 {
        runtime.set_panel_area(area(80));
    }
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        before,
        "A caller reports the area on every frame. That must cost nothing."
    );
}

#[tokio::test]
async fn the_runtime_stops_its_task_when_it_drops() {
    let registry = Registry::new()
        .with("good", Arc::new(Slow))
        .with("bad", Arc::new(Slow));
    let runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");
    tokio::time::sleep(Duration::from_millis(20)).await;
    let snap = runtime.snapshot();
    assert!(
        snap.is_loading(),
        "A data source that never answers must leave every panel without one."
    );
    assert!(
        snap.is_refreshing(),
        "The snapshot must report the set of queries that waits for an answer."
    );

    let before = handle().metrics().num_alive_tasks();
    drop(runtime);
    tokio::time::sleep(Duration::from_millis(20)).await;
    let after = handle().metrics().num_alive_tasks();
    assert!(
        after < before,
        "The runtime must stop its task. Before: {before}. After: {after}."
    );
}

/// The runtime runs the panels of one set at the same time, and it holds one
/// task for each panel. Every one of those tasks must end with the dashboard.
/// A task that outlived the dashboard would ask a data source for a screen that
/// no person reads.
///
/// This test runs on several threads, because the task of a panel ends on
/// another thread than the one that drops the runtime.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_runtime_stops_the_task_of_each_panel_when_it_drops() {
    let in_flight = Arc::new(AtomicUsize::new(0));
    let started = Arc::new(AtomicUsize::new(0));
    let runtime = DashboardRuntime::new(dashboard(), counted(&in_flight, &started), handle())
        .expect("The runtime must start.");
    until(
        || in_flight.load(Ordering::SeqCst) == 2,
        "both panels ask their data source",
    )
    .await;

    drop(runtime);
    until(
        || in_flight.load(Ordering::SeqCst) == 0,
        "no query of the runtime still runs",
    )
    .await;
}

/// A refresh stops the set of queries that runs now. The runtime holds one task
/// for each panel of that set, and every one of those tasks must end. The
/// queries of two sets would otherwise reach a data source at the same time.
///
/// This test runs on several threads. The task of a panel of the set before
/// ends on another thread than the one that started the new set.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_refresh_stops_the_task_of_each_panel_of_the_set_before() {
    let in_flight = Arc::new(AtomicUsize::new(0));
    let started = Arc::new(AtomicUsize::new(0));
    let mut runtime = DashboardRuntime::new(dashboard(), counted(&in_flight, &started), handle())
        .expect("The runtime must start.");
    until(
        || started.load(Ordering::SeqCst) == 2,
        "the first set of queries reaches both data sources",
    )
    .await;

    runtime.refresh();
    until(
        || started.load(Ordering::SeqCst) == 4,
        "the second set of queries reaches both data sources",
    )
    .await;
    until(
        || in_flight.load(Ordering::SeqCst) == 2,
        "the second set of queries waits alone",
    )
    .await;
}

/// The runtime asks a data source for the choices of a variable in a task of
/// its own. That task must end with the dashboard, as the task of a set of
/// queries does.
#[tokio::test]
async fn the_runtime_stops_the_task_of_a_variable_when_it_drops() {
    let dashboard = Dashboard::from_toml(A_VARIABLE).expect("The test dashboard must be correct.");
    let registry = Registry::new().with("one", Arc::new(SlowChoices));
    let runtime =
        DashboardRuntime::new(dashboard, registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    let before = handle().metrics().num_alive_tasks();
    assert!(
        before > 0,
        "The task that asks for the choices must still wait."
    );

    drop(runtime);
    tokio::time::sleep(Duration::from_millis(20)).await;
    let after = handle().metrics().num_alive_tasks();
    assert!(
        after < before,
        "The runtime must stop the task of the variable. Before: {before}. After: {after}."
    );
}

#[tokio::test]
async fn a_read_of_a_snapshot_never_waits_for_a_query() {
    let registry = Registry::new()
        .with("good", Arc::new(Slow))
        .with("bad", Arc::new(Slow));
    let runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");
    // The data source never answers. A read must still return at once.
    for _ in 0..1000 {
        let snap = runtime.snapshot();
        assert_eq!(snap.panels.len(), 2);
    }
}

#[test]
fn the_view_leaves_a_key_that_it_does_not_use_to_the_program() {
    let dash = dashboard();
    let mut view = DashboardView::new();
    assert_eq!(
        view.handle_key(Key::Char('q'), &dash, &Snapshot::default()),
        Consumed::No
    );
    assert_eq!(
        view.handle_key(Key::Other, &dash, &Snapshot::default()),
        Consumed::No
    );
    assert_eq!(
        view.handle_key(Key::Tab, &dash, &Snapshot::default()),
        Consumed::Yes
    );
}

/// thunderbox binds no key of a variable. The view must report that it used none
/// of them, so a program that wants those keys keeps them.
#[test]
fn the_view_uses_no_key_of_a_variable() {
    let dash = dashboard();
    let mut view = DashboardView::new();
    for key in [Key::Char('v'), Key::Char('['), Key::Char(']')] {
        assert_eq!(
            view.handle_key(key, &dash, &Snapshot::default()),
            Consumed::No,
            "The program binds this key. thunderbox must leave it alone. {key:?}"
        );
    }
}

/// thunderbox binds no key of the time range. The view must report that it used
/// none of them, so a program that wants those keys keeps them.
#[test]
fn the_view_uses_no_key_of_the_time_range() {
    let dash = dashboard();
    let mut view = DashboardView::new();
    for key in [
        Key::Left,
        Key::Right,
        Key::Char('-'),
        Key::Char('+'),
        Key::Char('0'),
        Key::Char('='),
    ] {
        assert_eq!(
            view.handle_key(key, &dash, &Snapshot::default()),
            Consumed::No,
            "The program binds this key. thunderbox must leave it alone. {key:?}"
        );
    }
}

/// The program binds every key of the time range, and it passes the key to
/// `time_key`. This test presses one of those keys, and it then reads the
/// range that the runtime holds.
#[tokio::test]
async fn a_key_that_the_program_binds_moves_the_time_range() {
    let counter = Arc::new(Counter::default());
    let registry = Registry::new()
        .with("good", counter.clone())
        .with("bad", counter.clone());
    let mut runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;

    let now = thunderbox_core::now_ms();
    let before = runtime.time_range().clone();
    let asked = time_key(
        Key::Left,
        runtime.time_range(),
        &runtime.dashboard().time,
        now,
    )
    .expect("The left key must move the range.");
    runtime.set_time_range(asked);

    let after = runtime.time_range().clone();
    assert_ne!(after, before, "The key must change the range.");
    assert!(
        !after.moves_with_the_clock(),
        "The window must stand still while a person studies it."
    );
    assert_eq!(
        after.span_ms(now),
        before.span_ms(now),
        "A move must keep the width that a person chose."
    );
    assert!(
        runtime.snapshot().is_loading(),
        "A new window must put every panel back to the state that has no answer."
    );

    let back = time_key(
        Key::Char('0'),
        runtime.time_range(),
        &runtime.dashboard().time,
        now,
    )
    .expect("The zero key must return the range of the dashboard.");
    assert_eq!(back, before, "One key must bring a person back.");
}

/// A person holds a key down, and the terminal repeats it many times each
/// second. A key that asks for the range that the runtime already holds must
/// therefore ask for nothing. `set_time_range` stops the set of queries that
/// runs, so such a key would leave a dashboard that never completes a refresh.
#[tokio::test]
async fn a_key_that_changes_nothing_starts_no_set_of_queries() {
    let counter = Arc::new(Counter::default());
    let registry = Registry::new()
        .with("good", counter.clone())
        .with("bad", counter.clone());
    let runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    let before = counter.calls.load(Ordering::SeqCst);

    for _ in 0..30 {
        let asked = time_key(
            Key::Right,
            runtime.time_range(),
            &runtime.dashboard().time,
            thunderbox_core::now_ms(),
        );
        // The program passes a range to `set_time_range` here, and this key
        // gives it none.
        assert_eq!(
            asked, None,
            "The range already ends at the present moment, so the key asks for nothing."
        );
    }

    assert!(
        !runtime.snapshot().is_loading(),
        "Every panel must keep the answer that it holds."
    );
    assert!(
        !runtime.snapshot().is_refreshing(),
        "The runtime must run no new set of queries."
    );
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        before,
        "The runtime must ask no data source again."
    );
}

#[test]
fn the_view_uses_no_key_while_the_program_holds_the_attention() {
    let dash = dashboard();
    let mut view = DashboardView::new();
    view.set_focused(false);
    assert_eq!(
        view.handle_key(Key::Tab, &dash, &Snapshot::default()),
        Consumed::No,
        "A program that owns the attention must keep every key."
    );
    assert_eq!(view.selected(), 0, "The selection must not move.");
}

#[test]
fn the_tab_key_moves_the_selection_and_returns_to_the_start() {
    let dash = dashboard();
    let mut view = DashboardView::new();
    assert_eq!(view.selected(), 0);
    view.handle_key(Key::Tab, &dash, &Snapshot::default());
    assert_eq!(view.selected(), 1);
    view.handle_key(Key::Tab, &dash, &Snapshot::default());
    assert_eq!(
        view.selected(),
        0,
        "The selection must return to the start."
    );
    view.handle_key(Key::BackTab, &dash, &Snapshot::default());
    assert_eq!(view.selected(), 1, "The shift key must move the other way.");
}

#[tokio::test]
async fn a_dashboard_that_names_no_data_source_returns_an_error_in_its_panel() {
    let registry = Registry::new();
    let runtime = DashboardRuntime::new(dashboard(), registry, handle())
        .expect("The runtime must start even with an empty registry.");
    settle(&runtime).await;
    let snap = runtime.snapshot();
    assert_eq!(snap.error_count(), 2, "Both panels must report the fault.");
    assert!(
        snap.panel(0).error().unwrap().contains("good"),
        "The message must name the data source."
    );
}

#[tokio::test]
async fn a_dashboard_that_is_not_correct_returns_an_error() {
    let mut broken = dashboard();
    broken.title = "  ".into();
    let result = DashboardRuntime::new(broken, Registry::new(), handle());
    assert!(result.is_err(), "The runtime must check the dashboard.");
}

/// Returns a window of one minute. The default window is five minutes, so this
/// window really differs from it.
fn other_window() -> TimeRange {
    TimeRange {
        from: RelativeTime::Offset(-60_000),
        to: RelativeTime::Now,
    }
}

#[tokio::test]
async fn a_new_time_range_starts_the_queries_again() {
    let counter = Arc::new(Counter::default());
    let registry = Registry::new()
        .with("good", counter.clone())
        .with("bad", counter.clone());
    let mut runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    let before = counter.calls.load(Ordering::SeqCst);

    runtime.set_time_range(other_window());
    settle(&runtime).await;
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        before + 2,
        "A new range must reach the data source of each panel once."
    );
}

/// A panel with no answer is correct here, because the answers describe another
/// window, and a reader would take them for the new one.
///
/// `#[tokio::test]` builds a runtime of one thread. The task of the new set of
/// queries therefore cannot run before the read below.
#[tokio::test]
async fn a_new_time_range_puts_every_panel_back_to_the_state_that_has_no_answer() {
    let counter = Arc::new(Counter::default());
    let registry = Registry::new()
        .with("good", counter.clone())
        .with("bad", counter.clone());
    let mut runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    assert!(
        !runtime.snapshot().is_loading(),
        "The first set of queries must reach every panel."
    );

    runtime.set_time_range(other_window());
    // The test reads the snapshot with no await in between. The runtime must
    // therefore write the state before it returns.
    let during = runtime.snapshot();
    assert!(
        during.is_loading(),
        "A new window must put every panel back to the state that has no answer."
    );
    assert_eq!(
        during.range,
        other_window(),
        "The snapshot must hold the new window beside the panels."
    );
    assert!(
        during.is_refreshing(),
        "The snapshot must report the set of queries that runs now."
    );
}

/// The range `the last five minutes` names a later window on each refresh. The
/// expression stays the same, so the panels keep their answers.
#[tokio::test]
async fn a_time_range_that_does_not_change_keeps_the_answer_that_each_panel_holds() {
    let counter = Arc::new(Counter::default());
    let registry = Registry::new()
        .with("good", counter.clone())
        .with("bad", counter.clone());
    let mut runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    let before = counter.calls.load(Ordering::SeqCst);

    // The dashboard states no time range, so it already holds the default one.
    runtime.set_time_range(TimeRange::default());
    assert!(
        !runtime.snapshot().is_loading(),
        "A range that does not change must keep the answer of each panel."
    );
    settle(&runtime).await;
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        before + 2,
        "The runtime must still ask each data source again."
    );
}

/// `#[tokio::test]` builds a runtime of one thread. The task of the new set of
/// queries therefore cannot run before the read below.
#[tokio::test]
async fn a_new_area_and_a_new_marker_keep_the_answer_that_each_panel_holds() {
    let counter = Arc::new(Counter::default());
    let registry = Registry::new()
        .with("good", counter.clone())
        .with("bad", counter.clone());
    let mut runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;

    runtime.set_panel_area(area(100));
    let during = runtime.snapshot();
    assert!(
        !during.is_loading(),
        "A new area changes the number of points, and not the window."
    );
    assert!(
        during.is_refreshing(),
        "The snapshot must report the set of queries that runs now."
    );
    settle(&runtime).await;

    runtime.set_marker(Marker::Block);
    assert!(
        !runtime.snapshot().is_loading(),
        "A new marker changes the number of points, and not the window."
    );
}

/// This test guards the fault that the dashboard showed. Every panel read
/// `loading…` for a moment on each refresh, and a reader saw a page with no
/// answer each second.
///
/// The permit of the semaphore holds the second set of queries, so no scheduler
/// can let the second answer arrive too early. The one period of a fixed length
/// below is a lower bound: it moves the clock, and a machine that is slower
/// only moves the clock further.
#[tokio::test]
async fn a_refresh_keeps_the_answer_that_each_panel_holds() {
    let permits = Arc::new(Semaphore::new(0));
    let registry = Registry::new().with("one", Arc::new(Gate::new(permits.clone())));
    let dashboard = Dashboard::from_toml(ONE_PANEL).expect("The test dashboard must be correct.");
    let mut runtime =
        DashboardRuntime::new(dashboard, registry, handle()).expect("The runtime must start.");

    permits.add_permits(1);
    settle(&runtime).await;
    let first = runtime.snapshot();
    assert_eq!(
        column_name(&first, 0),
        "run 0",
        "The first set of queries must reach the panel."
    );

    // The clock counts milliseconds. The two sets of queries must therefore
    // start in two different milliseconds. The test otherwise reads one time
    // for both answers.
    tokio::time::sleep(Duration::from_millis(5)).await;
    runtime.refresh();
    let during = runtime.snapshot();
    assert!(
        matches!(during.panel(0), PanelState::Ready(_)),
        "A refresh must keep the answer that the panel holds. {:?}",
        during.panel(0)
    );
    assert_eq!(
        column_name(&during, 0),
        "run 0",
        "The panel must still hold the answer of the set of queries before."
    );
    assert!(
        during.is_refreshing(),
        "The snapshot must report the set of queries that runs now."
    );
    assert!(
        runtime.is_refreshing(),
        "The runtime must report the set of queries that runs now."
    );
    assert_eq!(
        during.updated_at, first.updated_at,
        "A refresh must not move the time of the answer that the panel holds."
    );

    permits.add_permits(1);
    settle(&runtime).await;
    let after = runtime.snapshot();
    assert_eq!(
        column_name(&after, 0),
        "run 1",
        "The new answer must reach the panel."
    );
    assert!(
        !after.is_refreshing(),
        "The answer must report the end of the set of queries."
    );
}

#[tokio::test]
async fn a_refresh_moves_the_time_that_the_snapshot_records() {
    let registry = Registry::new()
        .with("good", Arc::new(Counter::default()))
        .with("bad", Arc::new(Counter::default()));
    let mut runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    let first: Option<Timestamp> = runtime.snapshot().updated_at;
    assert!(first.is_some(), "The snapshot must record the time.");

    // The clock counts milliseconds, so two sets of queries that start inside
    // one millisecond record one time.
    tokio::time::sleep(Duration::from_millis(2)).await;
    runtime.refresh();
    settle(&runtime).await;
    let after: Option<Timestamp> = runtime.snapshot().updated_at;
    assert!(
        after > first,
        "A new answer must move the time. Before: {first:?}. After: {after:?}."
    );
}

#[tokio::test]
async fn tick_starts_the_queries_again_after_the_interval() {
    let counter = Arc::new(Counter::default());
    let mut dashboard = dashboard();
    dashboard.refresh = Some("100ms".into());
    let registry = Registry::new()
        .with("good", counter.clone())
        .with("bad", counter.clone());
    let mut runtime =
        DashboardRuntime::new(dashboard, registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    let before = counter.calls.load(Ordering::SeqCst);

    // A call before the interval has passed must do nothing.
    assert!(
        !runtime.tick(),
        "A tick must start nothing before the interval has passed."
    );
    assert_eq!(counter.calls.load(Ordering::SeqCst), before);

    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(runtime.tick(), "The interval has passed.");
    settle(&runtime).await;
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        before + 2,
        "The tick must reach the data source of each panel once."
    );
}

/// A data source that answers slower than the interval must still finish one
/// set of queries. A tick that stopped each set before its answer would leave
/// the dashboard without a number for ever.
#[tokio::test]
async fn tick_starts_nothing_while_a_set_of_queries_waits_for_an_answer() {
    let permits = Arc::new(Semaphore::new(0));
    let registry = Registry::new().with("one", Arc::new(Gate::new(permits.clone())));
    let mut dashboard = Dashboard::from_toml(ONE_PANEL).expect("The dashboard must be correct.");
    dashboard.refresh = Some("10ms".into());
    let mut runtime =
        DashboardRuntime::new(dashboard, registry, handle()).expect("The runtime must start.");

    tokio::time::sleep(Duration::from_millis(30)).await;
    for _ in 0..10 {
        assert!(
            !runtime.tick(),
            "A tick must start nothing while a set of queries waits for an answer."
        );
    }
    permits.add_permits(1);
    settle(&runtime).await;
    assert_eq!(
        column_name(&runtime.snapshot(), 0),
        "run 0",
        "The first set of queries must reach the panel."
    );
}

/// A data source that never answers must not stop every later tick. The
/// runtime holds each query to a limit, so the task of that set of queries
/// ends and the next tick starts a new set.
///
/// The test waits for the report of the runtime, and it waits for no period of
/// a fixed length. `settle` returns when the set of queries ends. A runtime
/// that waited for ever would never end that set, and `settle` would then fail
/// the test.
#[tokio::test]
async fn a_data_source_that_never_answers_does_not_stop_the_next_tick() {
    let mut dashboard = Dashboard::from_toml(ONE_PANEL).expect("The dashboard must be correct.");
    dashboard.refresh = Some("10ms".into());
    let registry = Registry::new().with("one", Arc::new(NeverAnswers));
    let mut runtime =
        DashboardRuntime::new(dashboard, registry, handle()).expect("The runtime must start.");

    settle(&runtime).await;
    let snapshot = runtime.snapshot();
    let error = snapshot
        .panel(0)
        .error()
        .expect("The panel must hold the wait as its error.");
    assert!(
        error.contains("one"),
        "The message must name the data source. {error}"
    );
    assert!(
        error.contains("50 milliseconds"),
        "The message must state the wait. {error}"
    );

    // The limit of the query is longer than the interval, so the interval has
    // passed by now.
    assert!(
        runtime.tick(),
        "A data source that never answers must not stop the next tick."
    );
    assert!(
        runtime.is_refreshing(),
        "The tick must start a new set of queries."
    );

    settle(&runtime).await;
    assert!(
        runtime.tick(),
        "Every later tick must start a set of queries as well."
    );
}

/// A person reads the panels of the dashboard while one data source never
/// answers. ADR 0015 in `docs/contributor/adr` states that rule: a slow query
/// changes one panel, and never the whole picture.
#[tokio::test]
async fn a_data_source_that_never_answers_stops_no_other_panel() {
    let counter = Arc::new(Counter::default());
    let registry = Registry::new()
        .with("good", Arc::new(NeverAnswers))
        .with("bad", counter.clone());
    let runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");

    settle(&runtime).await;
    let snapshot = runtime.snapshot();
    assert!(
        snapshot.panel(0).error().is_some(),
        "The panel of the data source that never answers must hold an error."
    );
    assert!(
        matches!(snapshot.panel(1), PanelState::Ready(_)),
        "The other panel must hold its answer. {:?}",
        snapshot.panel(1)
    );
}

/// A dashboard that states no refresh interval starts no set of queries for
/// the clock. A tick of such a dashboard still gives a width that waits to a
/// new set, and
/// `a_dashboard_that_states_no_refresh_interval_still_receives_a_new_width`
/// covers that case. This test reports no width.
#[tokio::test]
async fn a_dashboard_with_no_refresh_interval_starts_no_set_of_queries_for_the_clock() {
    let counter = Arc::new(Counter::default());
    let mut dashboard = dashboard();
    dashboard.refresh = None;
    let registry = Registry::new()
        .with("good", counter.clone())
        .with("bad", counter.clone());
    let mut runtime =
        DashboardRuntime::new(dashboard, registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    let before = counter.calls.load(Ordering::SeqCst);

    tokio::time::sleep(Duration::from_millis(120)).await;
    for _ in 0..50 {
        assert!(
            !runtime.tick(),
            "A dashboard with no interval must not refresh."
        );
    }
    assert_eq!(counter.calls.load(Ordering::SeqCst), before);
    assert!(runtime.refresh_interval().is_none());
}

/// Each panel runs in a task of its own. A data source that panics therefore
/// ends the task of its own panel, and every other panel keeps its answer.
///
/// This test writes the report of a panic on the output of the test. That
/// report belongs to the test, and it is not a fault.
#[tokio::test]
async fn a_data_source_that_panics_leaves_every_other_panel_with_its_answer() {
    let registry = Registry::new()
        .with("good", Arc::new(Counter::default()))
        .with("bad", Arc::new(PanicsAtOnce));
    let runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    let snapshot = runtime.snapshot();
    assert_eq!(
        snapshot.error_count(),
        1,
        "A data source that panics must change one panel alone. {snapshot:?}"
    );
    assert!(
        matches!(snapshot.panel(0), PanelState::Ready(_)),
        "The other panel must keep its answer. {:?}",
        snapshot.panel(0)
    );
}

/// The task of a panel that panics gives no answer. The runtime must still
/// report the end of that set of queries, or every later tick starts nothing.
///
/// This test writes the report of a panic on the output of the test. That
/// report belongs to the test, and it is not a fault.
#[tokio::test]
async fn a_data_source_that_panics_does_not_stop_the_next_tick() {
    let source = Arc::new(PanicsAfterOne {
        calls: AtomicUsize::new(0),
    });
    let mut dashboard = Dashboard::from_toml(ONE_PANEL).expect("The dashboard must be correct.");
    dashboard.refresh = Some("10ms".into());
    let registry = Registry::new().with("one", source);
    let mut runtime =
        DashboardRuntime::new(dashboard, registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    assert!(
        !runtime.snapshot().is_loading(),
        "The first set of queries must reach the panel."
    );

    // The second query panics, so the task of that panel ends with no answer.
    runtime.refresh();
    settle(&runtime).await;
    assert!(
        !runtime.is_refreshing(),
        "A set that holds a task which panicked must still report its end."
    );
    assert!(
        runtime.snapshot().panel(0).error().is_some(),
        "A task that ended with no answer must give the panel an error."
    );

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(
        runtime.tick(),
        "A data source that panicked must not stop the next tick."
    );
}

/// A task that ends with no answer must reach the person who reads the panels.
/// The runtime reported such a task in one field alone before this rule, and a
/// panel that held no answer then drew `loading…` for ever.
///
/// Each panel now runs in a task of its own, and the panel of a task that ends
/// with no answer holds the error.
///
/// This test writes the report of a panic on the output of the test. That
/// report belongs to the test, and it is not a fault.
#[tokio::test]
async fn a_task_that_ends_with_no_answer_gives_its_panel_an_error() {
    let dashboard = Dashboard::from_toml(ONE_PANEL).expect("The dashboard must be correct.");
    let registry = Registry::new().with("one", Arc::new(PanicsAtOnce));
    let runtime =
        DashboardRuntime::new(dashboard, registry, handle()).expect("The runtime must start.");

    settle(&runtime).await;
    let snapshot = runtime.snapshot();
    assert!(
        !snapshot.is_loading(),
        "A panel that never receives an answer must not wait for ever."
    );
    let error = snapshot
        .panel(0)
        .error()
        .expect("The panel must hold an error.");
    assert!(
        error.contains("no answer"),
        "The error must state that the query ended with no answer. {error}"
    );
}

/// The runtime holds each query to a limit, and that limit reads the timer of
/// tokio. A `Handle` of a runtime that holds no timer therefore makes the task
/// of every panel panic. The panels must report that fault, and they must not
/// draw `loading…` for ever.
///
/// This test builds its own async runtime, so it takes no async form of its
/// own. It writes the report of a panic on the output of the test.
#[test]
fn a_handle_of_a_runtime_with_no_timer_gives_every_panel_an_error() {
    let async_runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .build()
        .expect("The test must build an async runtime.");
    let registry = Registry::new()
        .with("good", Arc::new(Counter::default()))
        .with("bad", Arc::new(Counter::default()));
    let runtime = DashboardRuntime::new(dashboard(), registry, async_runtime.handle().clone())
        .expect("The runtime must start.");

    for _ in 0..100 {
        if !runtime.is_refreshing() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        !runtime.is_refreshing(),
        "The task must end, whether it answers or not."
    );
    let snapshot = runtime.snapshot();
    assert_eq!(
        snapshot.error_count(),
        2,
        "Every panel must report the fault. {snapshot:?}"
    );
}

/// A caller can report an area that never stands still. A panel area that
/// grows and shrinks by one cell is one example. The quiet time alone would
/// hold such an area for as long as it moved, so the runtime also ends that
/// wait after `AREA_MAX_HOLD`.
#[tokio::test]
async fn an_area_that_never_stands_still_still_reaches_the_data_source() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), None);
    settle(&runtime).await;
    // The first area of a caller reaches the data source at once. This report
    // takes that step out of the run below.
    runtime.set_panel_area(area(100));
    settle(&runtime).await;
    let before = counter.calls.load(Ordering::SeqCst);
    counter.take_budgets();

    let mut started = false;
    for turn in 0..200 {
        runtime.set_panel_area(shake(turn));
        if runtime.tick() {
            started = true;
            break;
        }
        tokio::time::sleep(AREA_QUIET_TIME / 5).await;
    }
    assert!(
        started,
        "An area that never stands still must still reach the data source."
    );
    settle(&runtime).await;
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        before + 2,
        "The run of areas must cost one set of queries."
    );
    let budgets = counter.take_budgets();
    assert!(
        budgets == vec![120, 120] || budgets == vec![120, 122],
        "The set of queries must carry one of the two areas of the run. An \
         area of 120 cells holds two panels of 60 cells. An area of 121 cells \
         holds a panel of 60 cells and a panel of 61 cells. {budgets:?}"
    );
}

/// The wait of an area has a known length. An area that changes on every frame
/// must therefore reach a data source inside `AREA_MAX_HOLD`. The test above
/// must not depend on a long run of turns.
#[tokio::test]
async fn the_wait_of_an_area_ends_inside_the_longest_hold() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), None);
    settle(&runtime).await;
    runtime.set_panel_area(area(100));
    settle(&runtime).await;

    let start = std::time::Instant::now();
    let mut turn = 0;
    while !runtime.tick() {
        runtime.set_panel_area(shake(turn));
        turn += 1;
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(
            start.elapsed() < AREA_MAX_HOLD * 3,
            "The runtime must end the wait of an area inside the longest hold."
        );
    }
}

/// The runtime raises the generation number before it stops a task. A task that
/// the runtime stopped therefore writes nothing into the channel. It leaves no
/// error of a set of queries that ended with no answer.
///
/// This test runs on several threads, because the task of a stopped set ends on
/// another thread than the one that stopped it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_set_of_queries_that_the_runtime_stopped_leaves_no_error() {
    let registry = Registry::new()
        .with("good", Arc::new(Brief))
        .with("bad", Arc::new(Brief));
    let mut runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");
    // Each turn stops the task of the turn before, and that task still waits at
    // an await point.
    for turn in 0..200 {
        runtime.refresh();
        tokio::time::sleep(Duration::from_millis(1)).await;
        assert_eq!(
            runtime.snapshot().error_count(),
            0,
            "A set of queries that the runtime stopped must write no error. Turn {turn}."
        );
    }
    settle(&runtime).await;
    assert_eq!(
        runtime.snapshot().error_count(),
        0,
        "The last set of queries must answer every panel."
    );
}

/// A dashboard whose panel holds no target still sends a set of queries. That
/// set holds no query, and the runtime must report it as it reports any other.
/// The helper `settle` reads that report, and it would otherwise return on the
/// state of the set before.
#[tokio::test]
async fn a_dashboard_whose_panel_holds_no_target_reports_its_set_of_queries() {
    let dashboard = Dashboard::from_toml(NO_TARGET).expect("The dashboard must be correct.");
    let runtime = DashboardRuntime::new(dashboard, Registry::new(), handle())
        .expect("The runtime must start.");
    // `#[tokio::test]` builds a runtime of one thread. The task of the set
    // therefore cannot run before this read.
    assert!(
        runtime.is_refreshing(),
        "A set of queries that holds no query must still report itself."
    );

    settle(&runtime).await;
    assert!(
        !runtime.is_refreshing(),
        "The end of the set of queries must report itself."
    );
    assert!(
        !runtime.snapshot().is_loading(),
        "A panel that holds no target must hold an answer."
    );
}

/// The runtime times each set of queries from the moment when it started that
/// set. The program draws a status row from that time. A person who presses the
/// key `r` starts a new set, and the row must then report no slow data source.
///
/// The second read of the clock happens one statement after the first. The test
/// therefore fails only on a machine that stops for 40 milliseconds between two
/// statements.
#[tokio::test]
async fn a_new_set_of_queries_starts_the_clock_of_the_wait_again() {
    let registry = Registry::new()
        .with("good", Arc::new(Slow))
        .with("bad", Arc::new(Slow));
    let mut runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");

    tokio::time::sleep(Duration::from_millis(40)).await;
    let before = runtime
        .refresh_age()
        .expect("A set of queries that waits must report the time of its wait.");
    assert!(
        before >= Duration::from_millis(40),
        "The wait must hold the whole time of the set. Time: {before:?}."
    );

    runtime.refresh();
    let after = runtime
        .refresh_age()
        .expect("The new set of queries must report the time of its wait.");
    assert!(
        after < before,
        "A new set of queries must start the clock again. Before: {before:?}. After: {after:?}."
    );
}

#[tokio::test]
async fn a_set_of_queries_that_answered_reports_no_wait() {
    let registry = Registry::new()
        .with("good", Arc::new(Counter::default()))
        .with("bad", Arc::new(Counter::default()));
    let runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    assert_eq!(
        runtime.refresh_age(),
        None,
        "A set of queries that answered must report no wait."
    );
}

#[tokio::test]
async fn a_refresh_interval_that_is_not_correct_returns_an_error() {
    let mut dashboard = dashboard();
    dashboard.refresh = Some("soon".into());
    let result = DashboardRuntime::new(dashboard, Registry::new(), handle());
    assert!(result.is_err(), "The runtime must check the interval.");
}

/// A panel that the renderer drops keeps the answer that it holds.
///
/// The runtime asks no data source about such a panel. A set of queries that
/// wrote the state which holds no answer would draw `loading…` in that panel.
/// That text would stand for as long as the next set of queries takes, and it
/// would stand beside the panels that hold their answers.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_panel_that_the_renderer_drops_keeps_its_answer() {
    let counter = Arc::new(Counter::default());
    let registry = Registry::new()
        .with("good", counter.clone())
        .with("bad", counter);
    let mut runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");

    // An area that holds every panel. Each panel then holds an answer.
    runtime.set_panel_area(Size::new(80, 40));
    settle(&runtime).await;
    let before = runtime.snapshot();
    assert!(
        (0..2).all(|index| matches!(before.panel(index), PanelState::Ready(_))),
        "Each panel must hold an answer before the area becomes small."
    );

    // An area of two rows. The renderer drops both panels, because each one then
    // holds two cells.
    runtime.set_panel_area(Size::new(80, 2));
    tick_until_it_takes_the_area(&mut runtime).await;
    settle(&runtime).await;
    let after = runtime.snapshot();
    for index in 0..2 {
        assert!(
            matches!(after.panel(index), PanelState::Ready(_)),
            "A panel that the renderer drops must keep its answer. The panel at \
             {index} holds {:?}.",
            after.panel(index)
        );
    }
}

/// A new window puts a panel that the renderer drops back to the state that
/// has no answer.
///
/// Such a panel keeps its answer across a refresh, because the window of that
/// refresh is the window of the answer. A new window is a different matter. The
/// answer then describes the window before. The runtime asks no data source
/// about the panel, so that answer would stand until the terminal grows. One
/// snapshot would hold the answers of two windows.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_new_window_puts_a_panel_that_the_renderer_drops_back() {
    let counter = Arc::new(Counter::default());
    let registry = Registry::new()
        .with("good", counter.clone())
        .with("bad", counter);
    let mut runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");

    // An area that holds every panel, and then an area of two rows that drops
    // both of them.
    runtime.set_panel_area(Size::new(80, 40));
    settle(&runtime).await;
    runtime.set_panel_area(Size::new(80, 2));
    tick_until_it_takes_the_area(&mut runtime).await;
    settle(&runtime).await;

    runtime.set_time_range(other_window());
    settle(&runtime).await;
    let after = runtime.snapshot();
    for index in 0..2 {
        assert!(
            after.panel(index).is_loading(),
            "A new window must put the panel at {index} back to the state that \
             has no answer. It holds {:?}.",
            after.panel(index)
        );
    }
}

/// A panel that the renderer drops holds no error.
///
/// The runtime asks no data source about such a panel, so no later set of
/// queries can clear an error that the panel holds. [`Snapshot::error_count`]
/// counts that error, and the status row of a program names a fault that
/// nobody can repair.
///
/// A height that only drops panels starts no set of queries, so this test
/// drives the tick that takes the area. That tick clears the error itself.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_panel_that_the_renderer_drops_holds_no_error() {
    let registry = Registry::new()
        .with("good", Arc::new(Counter::default()))
        .with("bad", Arc::new(Broken));
    let mut runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");

    runtime.set_panel_area(Size::new(80, 40));
    settle(&runtime).await;
    assert_eq!(
        runtime.snapshot().error_count(),
        1,
        "The panel of the data source that returns an error must hold that \
         error while the renderer draws it."
    );

    // An area of two rows. The renderer drops both panels, because each one
    // then holds two cells.
    runtime.set_panel_area(Size::new(80, 2));
    tick_until_it_takes_the_area(&mut runtime).await;
    settle(&runtime).await;
    let after = runtime.snapshot();
    assert_eq!(
        after.error_count(),
        0,
        "A panel that the renderer drops must hold no error. The panel at 1 \
         holds {:?}.",
        after.panel(1)
    );
}

/// An area that changes the height alone starts no set of queries.
///
/// The height of an area decides which panels the renderer drops, and the width
/// decides the point limit of each panel. An area that changes neither would
/// ask a data source for exactly what the set of queries before asked. A person
/// who drags the bottom edge of a terminal window therefore pays nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_area_that_changes_no_point_limit_starts_no_set_of_queries() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), None);
    settle(&runtime).await;
    runtime.set_panel_area(area(100));
    settle(&runtime).await;
    let before = counter.calls.load(Ordering::SeqCst);

    // A caller reports the area on every frame, so this loop reports the new
    // height on every turn. The loop runs for longer than `AREA_MAX_HOLD`.
    for _ in 0..60 {
        runtime.set_panel_area(Size::new(100, TALL_ENOUGH + 1));
        runtime.tick();
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    settle(&runtime).await;
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        before,
        "An area that changes no point limit must reach no data source."
    );
}

/// The runtime takes an area that costs no set of queries, and a later area
/// carries the point limit of the newest area.
///
/// The runtime compares a new area against the area that the set of queries in
/// use carries. An area that costs nothing must therefore still reach that
/// field. A runtime that dropped such an area would compare every later area
/// against an area that no caller reports.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_area_that_costs_no_set_of_queries_still_reaches_the_next_comparison() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), None);
    settle(&runtime).await;
    runtime.set_panel_area(area(100));
    settle(&runtime).await;
    counter.take_budgets();

    for _ in 0..30 {
        runtime.set_panel_area(Size::new(100, TALL_ENOUGH + 1));
        runtime.tick();
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    // The width now changes, and the height stays at the height above.
    runtime.set_panel_area(Size::new(120, TALL_ENOUGH + 1));
    assert!(
        tick_until_it_starts(&mut runtime).await,
        "A new width must reach a set of queries."
    );
    settle(&runtime).await;
    assert_eq!(
        counter.take_budgets(),
        vec![120, 120],
        "The set of queries must carry the point limit of the newest area."
    );
}

/// An area that moves in and out of the point limit in use still reaches a data
/// source.
///
/// A scroll bar that appears and disappears makes such an area. One of the two
/// sizes costs a set of queries, and the other costs none. The runtime must not
/// start the wait of an area again for the size that costs none, because
/// neither [`AREA_QUIET_TIME`] nor [`AREA_MAX_HOLD`] could then pass. The panels
/// would hold the point limit of the area before. They would hold it for as
/// long as the caller moved.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_area_that_moves_in_and_out_of_the_point_limit_in_use_still_arrives() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), None);
    settle(&runtime).await;
    runtime.set_panel_area(area(100));
    settle(&runtime).await;
    counter.take_budgets();

    let start = Instant::now();
    let mut started = false;
    for turn in 0..80 {
        // An even turn changes the height alone, and it costs no set of
        // queries. An odd turn changes the width, and it costs one.
        let next = if turn % 2 == 0 {
            Size::new(100, TALL_ENOUGH + 1)
        } else {
            Size::new(120, TALL_ENOUGH)
        };
        runtime.set_panel_area(next);
        if runtime.tick() {
            started = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(
        started,
        "An area that moves in and out of the point limit in use must reach a \
         data source."
    );
    assert!(
        start.elapsed() < AREA_MAX_HOLD + AREA_QUIET_TIME * 2,
        "The wait must end inside the longest hold and a few frames. A wait \
         that a free area ended would take twice the longest hold. It took \
         {:?}.",
        start.elapsed()
    );
    settle(&runtime).await;
    assert_eq!(
        counter.take_budgets(),
        vec![120, 120],
        "The set of queries must carry the point limit of the area that needs \
         it."
    );
}

/// An area that moves back to the area in use still reaches a data source.
///
/// A caller that reports the area in use clears the area that waits. The
/// runtime must not start the wait of an area again on the next report, because
/// neither [`AREA_QUIET_TIME`] nor [`AREA_MAX_HOLD`] could then pass. The panels
/// would keep the point limit of the area in use. A person would then read a
/// chart of the wrong number of points for as long as the caller moved.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_area_that_moves_back_to_the_area_in_use_still_arrives() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), None);
    settle(&runtime).await;
    runtime.set_panel_area(area(100));
    settle(&runtime).await;
    counter.take_budgets();

    let start = Instant::now();
    let mut started = false;
    for turn in 0..80 {
        let next = if turn % 2 == 0 { area(100) } else { area(120) };
        runtime.set_panel_area(next);
        if runtime.tick() {
            started = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(
        started,
        "An area that moves back to the area in use must reach a data source."
    );
    assert!(
        start.elapsed() < AREA_MAX_HOLD * 3,
        "The wait must end inside three times the longest hold. It took {:?}.",
        start.elapsed()
    );
    settle(&runtime).await;
    assert_eq!(
        counter.take_budgets(),
        vec![120, 120],
        "The set of queries must carry the point limit of the area that needs \
         it."
    );
}

/// A new area waits, even when the area before it cost no set of queries.
///
/// The runtime ends the wait of an area when it takes that area. It takes an
/// area that costs no set of queries as well. A runtime that left the wait open
/// would hold the next area for no time at all, because [`AREA_MAX_HOLD`] had
/// already passed. A person who then dragged the corner of a terminal window
/// would pay one set of queries for the first frame of that drag. That person
/// would pay more sets for the frames after it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_new_area_waits_after_an_area_that_cost_no_set_of_queries() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), None);
    settle(&runtime).await;
    runtime.set_panel_area(area(100));
    settle(&runtime).await;

    // The loop runs for longer than `AREA_MAX_HOLD`, so a wait that stayed
    // open would hold no later area.
    for _ in 0..60 {
        runtime.set_panel_area(Size::new(100, TALL_ENOUGH + 1));
        runtime.tick();
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    runtime.set_panel_area(Size::new(120, TALL_ENOUGH + 1));
    assert!(
        !runtime.tick(),
        "A new area must wait for the quiet time, as it waits after a set of \
         queries."
    );
}

/// A caller that drops the area which waits still holds the next area for the
/// quiet time.
///
/// A person who makes a terminal window larger and then puts it back reports
/// the area in use again. The runtime then drops the area that waits, and no
/// area waits. A runtime that left the wait open would hold no later area at
/// all, because [`AREA_MAX_HOLD`] had already passed. The first frame of the
/// next drag would start a set of queries, at a width that the person still
/// moves past.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_new_area_waits_after_a_caller_dropped_the_area_that_waited() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), None);
    settle(&runtime).await;
    runtime.set_panel_area(area(100));
    settle(&runtime).await;
    let before = counter.calls.load(Ordering::SeqCst);

    // A new area waits, and the caller then reports the area in use again.
    runtime.set_panel_area(area(120));
    runtime.tick();
    runtime.set_panel_area(area(100));

    // The caller now stands still for longer than `AREA_MAX_HOLD`.
    for _ in 0..60 {
        runtime.set_panel_area(area(100));
        runtime.tick();
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        before,
        "An area that the caller dropped must reach no data source."
    );

    runtime.set_panel_area(area(140));
    assert!(
        !runtime.tick(),
        "A new area must wait for the quiet time, and the area that the caller \
         dropped must not end that wait."
    );
}

/// A set of queries ends the wait of an area.
///
/// The runtime measures the longest hold from the moment when the wait of an
/// area began. A set of queries carries the area that waits, so that wait ends.
/// A runtime that kept the moment would measure the next area against a wait
/// that another area began. That next area would then reach a data source on
/// its first frame. It would carry a width that a person still moves past.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_set_of_queries_ends_the_wait_of_an_area() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), None);
    settle(&runtime).await;
    runtime.set_panel_area(area(100));
    settle(&runtime).await;

    // An area waits, and the wait then grows past the longest hold. The test
    // drives no tick, so no tick can take that area.
    runtime.set_panel_area(area(120));
    tokio::time::sleep(AREA_MAX_HOLD + Duration::from_millis(100)).await;

    // The key `r` starts a set of queries, and that set carries the area.
    runtime.refresh();
    settle(&runtime).await;

    runtime.set_panel_area(area(140));
    assert!(
        !runtime.tick(),
        "A new area must wait for the quiet time. The set of queries before \
         must have ended the wait of the area that it carried."
    );
}

/// An area that changes no point limit starts no wait of an area.
///
/// A person can make a terminal window taller over several seconds, and no
/// point limit changes while they do. A runtime that started a wait for such an
/// area would hold that wait open for longer than [`AREA_MAX_HOLD`]. The next
/// area that changed a point limit would then reach a data source at once. It
/// would carry a width that the person still moves past.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_area_that_changes_no_point_limit_starts_no_wait() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), None);
    settle(&runtime).await;
    runtime.set_panel_area(area(100));
    settle(&runtime).await;
    let before = counter.calls.load(Ordering::SeqCst);

    // The caller changes the height alone on every frame, for longer than the
    // longest hold. No point limit changes, so no wait may start.
    for turn in 0..60_u16 {
        runtime.set_panel_area(Size::new(100, TALL_ENOUGH + 1 + turn % 3));
        runtime.tick();
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        before,
        "An area that changes no point limit must reach no data source."
    );

    // A width now changes. That area must wait, as the first area of any drag
    // waits.
    runtime.set_panel_area(Size::new(120, TALL_ENOUGH + 1));
    assert!(
        !runtime.tick(),
        "The first area that changes a point limit must wait for the quiet \
         time."
    );
}

/// A tick that takes an area which costs no set of queries still reads the
/// refresh interval.
///
/// The runtime takes such an area and starts no set of queries for it. A tick
/// that returned there would hold the set of queries of the interval until the
/// next tick. The numbers of a dashboard would then stand still for one frame
/// more than the interval states.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_tick_that_takes_a_free_area_still_reads_the_refresh_interval() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), Some("400ms"));
    settle(&runtime).await;
    runtime.set_panel_area(area(100));
    settle(&runtime).await;

    // An area that changes no point limit waits. The sleep passes both the
    // quiet time and the refresh interval, so one tick must take both steps.
    runtime.set_panel_area(Size::new(100, TALL_ENOUGH + 1));
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert!(
        runtime.tick(),
        "A tick that takes an area which costs no set of queries must still \
         start the set of queries of the interval."
    );
}

/// An area that costs no set of queries does not end the wait of an area that
/// costs one.
///
/// A caller can report an area that changes a point limit, then move through
/// areas that change none, and then change a point limit again. The runtime
/// takes the areas that cost nothing, and the wait that the first area opened
/// must stay open. A runtime that ended it would measure the longest hold from
/// the second area. A point limit would then stay wrong for two of those
/// holds.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_area_that_costs_nothing_does_not_end_the_wait_of_one_that_costs_a_set() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), None);
    settle(&runtime).await;
    runtime.set_panel_area(area(100));
    settle(&runtime).await;
    counter.take_budgets();
    let before = counter.calls.load(Ordering::SeqCst);

    // A width that costs a set of queries opens the wait. The test drives no
    // tick here, so no tick can take that area.
    runtime.set_panel_area(area(120));

    // The caller now moves through heights alone for longer than the longest
    // hold. One of those ticks takes the area that costs nothing.
    for turn in 0..30_u16 {
        runtime.set_panel_area(Size::new(100, TALL_ENOUGH + 1 + turn % 3));
        runtime.tick();
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        before,
        "The areas that cost nothing must reach no data source."
    );

    // A width changes again. The wait that the first width opened has passed
    // the longest hold, so this area must reach a data source at once.
    runtime.set_panel_area(area(140));
    assert!(
        runtime.tick(),
        "The wait that the first width opened must still hold, so this area \
         must reach a data source at once."
    );
    settle(&runtime).await;
    assert_eq!(
        counter.take_budgets(),
        vec![140, 140],
        "The set of queries must carry the point limit of the newest area."
    );
}

/// The runtime ends the wait of a settled area in the tick that takes that
/// area, and not in the tick after it.
///
/// `tick` reads the area that waits and then ends the wait of a settled area.
/// A runtime that read the two in the other order would hold the wait for one
/// more tick. An area still waits at that first read. A new area that arrived
/// inside that one tick would then find the wait of an older area still open.
/// That new area would reach a data source at once.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_tick_that_takes_a_settled_area_also_ends_the_wait_of_that_area() {
    let counter = Arc::new(Counter::default());
    let mut runtime = with_counter(counter.clone(), None);
    settle(&runtime).await;
    runtime.set_panel_area(area(100));
    settle(&runtime).await;

    // A width opens the wait. The test drives no tick, so the wait grows past
    // the longest hold.
    runtime.set_panel_area(area(120));
    tokio::time::sleep(AREA_MAX_HOLD + Duration::from_millis(100)).await;

    // A height alone now replaces that width, and the caller stands still.
    runtime.set_panel_area(Size::new(100, TALL_ENOUGH + 1));
    tokio::time::sleep(AREA_QUIET_TIME + Duration::from_millis(100)).await;

    // One tick takes that area, and it must end the wait in the same tick.
    assert!(
        !runtime.tick(),
        "The area that this tick takes costs no set of queries."
    );

    // A width changes again. The wait of the width above must be gone, so this
    // area must wait for the quiet time.
    runtime.set_panel_area(area(140));
    assert!(
        !runtime.tick(),
        "A new area must wait for the quiet time. The tick that took the area \
         before must have ended the wait of that area."
    );
}

/// A height that only drops panels starts no set of queries.
///
/// A panel that the renderer drops asks a data source for nothing. A point
/// limit that such a panel loses therefore costs no set of queries. Every panel
/// that the new area still draws holds the width that it held, and therefore
/// the answer that it needs. A person who makes a terminal window shorter until a panel
/// goes therefore reaches no data source.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_height_that_only_drops_panels_starts_no_set_of_queries() {
    let counter = Arc::new(Counter::default());
    let dashboard = Dashboard::from_toml(EIGHT_PANELS).expect("The dashboard must be correct.");
    let registry = Registry::new().with("one", counter.clone());
    let mut runtime =
        DashboardRuntime::new(dashboard, registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    // Every panel of the eight holds an answer on an area of 40 rows.
    runtime.set_panel_area(Size::new(110, 40));
    settle(&runtime).await;
    let before = counter.calls.load(Ordering::SeqCst);

    // An area of 23 rows drops the first panel of the eight. The other seven
    // hold the width that they held.
    runtime.set_panel_area(Size::new(110, 23));
    tick_until_it_takes_the_area(&mut runtime).await;
    settle(&runtime).await;
    assert_eq!(
        counter.calls.load(Ordering::SeqCst),
        before,
        "A height that only drops panels must reach no data source."
    );
    let snapshot = runtime.snapshot();
    for index in 1..8 {
        assert!(
            matches!(snapshot.panel(index), PanelState::Ready(_)),
            "The panel {index} must keep its answer. {:?}",
            snapshot.panel(index)
        );
    }
}

/// A height that uncovers a panel starts a set of queries.
///
/// The runtime holds no point limit for a panel that the area before dropped.
/// It cannot know whether the answer that such a panel holds has the right
/// shape. Such a panel must therefore reach a data source.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_height_that_uncovers_a_panel_starts_a_set_of_queries() {
    let counter = Arc::new(Counter::default());
    let dashboard = Dashboard::from_toml(EIGHT_PANELS).expect("The dashboard must be correct.");
    let registry = Registry::new().with("one", counter.clone());
    let mut runtime =
        DashboardRuntime::new(dashboard, registry, handle()).expect("The runtime must start.");
    settle(&runtime).await;
    runtime.set_panel_area(Size::new(110, 23));
    settle(&runtime).await;
    let before = counter.calls.load(Ordering::SeqCst);

    // An area of 40 rows draws the panel that 23 rows dropped.
    runtime.set_panel_area(Size::new(110, 40));
    assert!(
        tick_until_it_starts(&mut runtime).await,
        "An area that uncovers a panel must reach a set of queries."
    );
    settle(&runtime).await;
    assert!(
        counter.calls.load(Ordering::SeqCst) > before,
        "The panel that the terminal grew to hold must reach the data source."
    );
}

/// A tick that takes an area takes the error of each panel that the renderer
/// drops, and it takes no other error.
///
/// A person can read a panel that the renderer draws. Such a panel would
/// otherwise change from its error to `loading…` on a resize of the height. The
/// status row would then report one error fewer than the screen holds.
///
/// The dashboard of this test holds three panels in one column, and its data
/// source returns an error for each one. An area of 12 rows drops the middle
/// panel alone, and it changes the point limit of no other panel.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_tick_that_takes_an_area_takes_the_error_of_a_dropped_panel_alone() {
    let dashboard = Dashboard::from_toml(MIDDLE_IS_SMALL).expect("The dashboard must be correct.");
    let registry = Registry::new().with("one", Arc::new(Broken));
    let mut runtime =
        DashboardRuntime::new(dashboard, registry, handle()).expect("The runtime must start.");
    runtime.set_panel_area(Size::new(100, 40));
    settle(&runtime).await;
    assert_eq!(
        runtime.snapshot().error_count(),
        3,
        "Each of the three panels must hold an error while the renderer draws \
         it."
    );

    // An area of 12 rows drops the middle panel, and it changes the point limit
    // of no other panel. The tick that takes it starts no set of queries.
    runtime.set_panel_area(Size::new(100, 12));
    tick_until_it_takes_the_area(&mut runtime).await;
    let after = runtime.snapshot();
    assert_eq!(
        after.error_count(),
        2,
        "The middle panel must lose its error, and the other two must keep \
         theirs. The three panels hold {:?}, {:?} and {:?}.",
        after.panel(0),
        after.panel(1),
        after.panel(2)
    );
    assert!(
        after.panel(1).is_loading(),
        "The panel that the renderer drops must hold no error. {:?}",
        after.panel(1)
    );
}

/// A tick that takes an area and changes no panel wakes no reader of the
/// channel.
///
/// A program that draws only after a change reads that channel. A tick that
/// wrote an unchanged snapshot would make such a program draw the same screen
/// again. It would do so on every tick, while a person moves the edge of a
/// terminal window.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_tick_that_changes_no_panel_wakes_no_reader() {
    let registry = Registry::new()
        .with("good", Arc::new(Counter::default()))
        .with("bad", Arc::new(Broken));
    let mut runtime =
        DashboardRuntime::new(dashboard(), registry, handle()).expect("The runtime must start.");
    runtime.set_panel_area(area(100));
    settle(&runtime).await;
    assert_eq!(
        runtime.snapshot().error_count(),
        1,
        "One panel must hold an error, so the runtime has a panel to read."
    );

    let mut rx = runtime.subscribe();
    rx.borrow_and_update();

    // A height that changes no point limit and drops no panel. The renderer
    // draws both panels, so the tick that takes this area changes nothing.
    runtime.set_panel_area(Size::new(100, TALL_ENOUGH + 1));
    tick_until_it_takes_the_area(&mut runtime).await;
    assert!(
        !rx.has_changed()
            .expect("The runtime must hold the channel."),
        "A tick that changes no panel must write no snapshot."
    );
}
