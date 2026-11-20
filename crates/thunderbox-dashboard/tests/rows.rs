//! Tests for the row panels of a dashboard.
//!
//! A row panel holds the panels below it on the grid, and a person collapses it
//! to hide them. The runtime asks a data source for nothing about a hidden
//! panel, and the renderer draws none. These tests hold the two parts together,
//! and they hold the rule that each change of the state reaches the screen.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Rect, Size};
use thunderbox_core::{Dashboard, Error, Field, Frame, QueryRequest, RelativeTime, Result};
use thunderbox_dashboard::{
    AREA_QUIET_TIME, CollapsedRows, DashboardRuntime, DashboardView, Key, Snapshot, row_key,
};
use thunderbox_source::{DataSource, Registry, SourceDescriptor};
use tokio::sync::Semaphore;

mod common;

use common::settle;

/// A dashboard of two row panels. The second one starts collapsed.
///
/// The panels stand at these positions:
///
/// - 0 is the chart above both row panels;
/// - 1 is the first row panel, and 2 is its chart;
/// - 3 is the second row panel, and 4 and 5 are its two panels.
///
/// Each target asks for the title of its own panel, so a test reads
/// which panels reached the data source.
const ROWS: &str = r#"
title = "rows"
[source.s]
kind = "test"

[[panel]]
title = "alpha"
kind = "timeseries"
grid = { x = 0, y = 0, w = 24, h = 6 }
[[panel.target]]
source = "s"
expr = "alpha"

[[panel]]
title = "row one"
kind = "row"
grid = { x = 0, y = 6, w = 24, h = 1 }

[[panel]]
title = "bravo"
kind = "timeseries"
grid = { x = 0, y = 7, w = 24, h = 6 }
[[panel.target]]
source = "s"
expr = "bravo"

[[panel]]
title = "row two"
kind = "row"
collapsed = true
grid = { x = 0, y = 13, w = 24, h = 1 }

[[panel]]
title = "charlie"
kind = "timeseries"
grid = { x = 0, y = 14, w = 12, h = 6 }
[[panel.target]]
source = "s"
expr = "charlie"

[[panel]]
title = "delta"
kind = "table"
grid = { x = 12, y = 14, w = 12, h = 6 }
[[panel.target]]
source = "s"
expr = "delta"
"#;

/// The position of the first row panel.
const ROW_ONE: usize = 1;

/// The position of the second row panel, which starts collapsed.
const ROW_TWO: usize = 3;

/// A data source that records the text of each query that reaches it.
///
/// The source returns an error for the query `bravo` when `broken` holds
/// `true`. It answers no query until the test adds a permit when `gate` holds
/// a semaphore.
#[derive(Default)]
struct Asked {
    seen: Mutex<Vec<String>>,
    broken: bool,
    gate: Option<Arc<Semaphore>>,
}

impl Asked {
    /// Returns the text of each query so far, in order, and forgets them all.
    fn take(&self) -> Vec<String> {
        let mut held = self.seen.lock().expect("No test holds this lock twice.");
        let mut seen = std::mem::take(&mut *held);
        seen.sort();
        seen
    }
}

#[async_trait]
impl DataSource for Asked {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor::new("asked", "Asked")
    }
    async fn query(&self, req: QueryRequest) -> Result<Vec<Frame>> {
        if let Some(gate) = &self.gate {
            gate.acquire()
                .await
                .expect("The test must hold the semaphore open.")
                .forget();
        }
        let expr = req.query.expr.clone();
        self.seen
            .lock()
            .expect("No test holds this lock twice.")
            .push(expr.clone());
        if self.broken && expr == "bravo" {
            return Err(Error::Query("bravo returns an error".into()));
        }
        Ok(vec![Frame::named(
            "f",
            vec![
                Field::time("time", vec![0, 1000]),
                Field::f64(expr, vec![Some(1.0), Some(2.0)]),
            ],
        )])
    }
}

fn dashboard() -> Dashboard {
    Dashboard::from_toml(ROWS).expect("The test dashboard must be correct.")
}

/// Returns a runtime over the dashboard `ROWS` and this data source.
fn runtime(source: &Arc<Asked>) -> DashboardRuntime {
    let registry = Registry::new().with("s", source.clone());
    DashboardRuntime::new(dashboard(), registry, tokio::runtime::Handle::current())
        .expect("The runtime must start.")
}

/// Returns the text of every line of the buffer.
fn text(buf: &Buffer) -> String {
    (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Draws the dashboard `ROWS` into a buffer, and returns the text.
fn picture(snapshot: &Snapshot, view: &DashboardView, width: u16, height: u16) -> String {
    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    view.render(&dashboard(), snapshot, area, &mut buf);
    text(&buf)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_collapsed_row_panel_asks_about_none_of_its_panels() {
    let source = Arc::new(Asked::default());
    let mut dash = runtime(&source);
    settle(&dash).await;
    assert_eq!(
        source.take(),
        vec!["alpha", "bravo"],
        "The second row panel starts collapsed, so it hides two panels."
    );
    dash.set_panel_area(Size::new(110, 40));
    settle(&dash).await;
    assert_eq!(
        source.take(),
        vec!["alpha", "bravo"],
        "An area must not change which panels reach the data source."
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn expanding_a_row_panel_asks_about_the_panels_that_it_holds() {
    let source = Arc::new(Asked::default());
    let mut dash = runtime(&source);
    settle(&dash).await;
    source.take();
    dash.toggle_row(ROW_TWO);
    assert!(
        !dash.snapshot().collapsed.contains(ROW_TWO),
        "The snapshot must carry the new state at once."
    );
    settle(&dash).await;
    let asked = source.take();
    assert!(
        asked.contains(&"charlie".to_string()) && asked.contains(&"delta".to_string()),
        "The panels that the row panel shows now need an answer. {asked:?}"
    );
}

/// A collapse changes no width, and a width decides the point limit of a
/// panel. A collapse that only hides panels therefore asks for nothing.
#[tokio::test(flavor = "multi_thread")]
async fn collapsing_a_row_panel_asks_about_nothing() {
    let source = Arc::new(Asked::default());
    let mut dash = runtime(&source);
    dash.set_panel_area(Size::new(110, 40));
    settle(&dash).await;
    source.take();
    dash.toggle_row(ROW_ONE);
    assert!(
        !dash.is_refreshing(),
        "A collapse that only hides panels starts no set of queries."
    );
    assert!(dash.snapshot().collapsed.contains(ROW_ONE));
    settle(&dash).await;
    assert!(
        source.take().is_empty(),
        "No query may reach the data source."
    );
}

/// A collapse gives the height of the hidden panels to every other panel. A
/// panel that was too small to draw can then draw, and it needs an answer.
#[tokio::test(flavor = "multi_thread")]
async fn a_collapse_that_lets_a_small_panel_draw_asks_about_it() {
    let source = Arc::new(Asked::default());
    let mut dash = runtime(&source);
    // Six lines hold two lines of the row panels and four lines for the
    // panels. The panels `alpha` and `bravo` then hold two lines each, and the
    // renderer draws neither of them.
    dash.set_panel_area(Size::new(110, 6));
    settle(&dash).await;
    source.take();
    dash.toggle_row(ROW_ONE);
    settle(&dash).await;
    assert_eq!(
        source.take(),
        vec!["alpha"],
        "The panel alpha now holds four lines, so it draws."
    );
}

/// A set of queries that started before a collapse ends after it. The answer
/// of that set must not restore the state from before the collapse.
#[tokio::test(flavor = "multi_thread")]
async fn a_set_of_queries_that_started_before_a_collapse_keeps_the_collapse() {
    let gate = Arc::new(Semaphore::new(0));
    let source = Arc::new(Asked {
        gate: Some(gate.clone()),
        ..Default::default()
    });
    let mut dash = runtime(&source);
    assert!(dash.is_refreshing(), "The first set waits for the gate.");
    dash.toggle_row(ROW_ONE);
    assert!(dash.snapshot().collapsed.contains(ROW_ONE));
    gate.add_permits(10);
    settle(&dash).await;
    assert!(
        dash.snapshot().collapsed.contains(ROW_ONE),
        "The answer of the set before the collapse expanded the row panel again."
    );
    assert!(
        dash.snapshot().collapsed.contains(ROW_TWO),
        "The answer must keep the state that the dashboard stated as well."
    );
}

/// No set of queries reaches a hidden panel, so no set could clear its error.
/// The status row would then count an error that nobody can see.
#[tokio::test(flavor = "multi_thread")]
async fn a_panel_that_a_row_panel_hides_loses_its_error() {
    let source = Arc::new(Asked {
        broken: true,
        ..Default::default()
    });
    let mut dash = runtime(&source);
    settle(&dash).await;
    assert_eq!(
        dash.snapshot().error_count(),
        1,
        "The panel bravo holds an error."
    );
    dash.toggle_row(ROW_ONE);
    assert_eq!(
        dash.snapshot().error_count(),
        0,
        "The collapse hides bravo, so its error must go."
    );
}

/// A new time range builds a new snapshot in which no panel holds an answer.
/// That snapshot must keep the row panels that a person collapsed.
#[tokio::test(flavor = "multi_thread")]
async fn a_new_time_range_keeps_the_row_panels_that_a_person_collapsed() {
    let source = Arc::new(Asked::default());
    let mut dash = runtime(&source);
    settle(&dash).await;
    dash.toggle_row(ROW_ONE);
    let mut range = dash.time_range().clone();
    range.from = RelativeTime::Offset(-600_000);
    dash.set_time_range(range);
    assert!(
        dash.snapshot().collapsed.contains(ROW_ONE),
        "The snapshot of the new window lost the collapse."
    );
    settle(&dash).await;
    assert!(dash.snapshot().collapsed.contains(ROW_ONE));
}

/// A row panel asks a data source for nothing, so it never holds an answer.
///
/// The runtime gives each panel the width `ASSUMED_WIDTH` before a caller
/// reports an area. A row panel would then take a job of its own. Once an area
/// arrives, the line of a row panel is too small for `draws_a_panel`.
/// This test therefore reads the state before any area.
#[tokio::test(flavor = "multi_thread")]
async fn a_row_panel_holds_no_answer_before_an_area_arrives() {
    let source = Arc::new(Asked::default());
    let dash = runtime(&source);
    settle(&dash).await;
    for row in [ROW_ONE, ROW_TWO] {
        assert!(
            dash.snapshot().panel(row).is_loading(),
            "The row panel {row} took a job. {:?}",
            dash.snapshot().panel(row)
        );
    }
}

/// A row panel shows no data, so it asks a data source for nothing. A height
/// that only uncovers its line must therefore start no set of queries.
#[tokio::test(flavor = "multi_thread")]
async fn a_height_that_only_uncovers_a_row_panel_starts_no_set_of_queries() {
    let source = Arc::new(Asked::default());
    let mut dash = runtime(&source);
    // One line holds the first row panel alone. No panel that shows data
    // draws, and the second row panel takes no line.
    dash.set_panel_area(Size::new(110, 1));
    settle(&dash).await;
    // Two lines hold both row panels, and still no panel that shows data.
    dash.set_panel_area(Size::new(110, 2));
    tokio::time::sleep(AREA_QUIET_TIME + std::time::Duration::from_millis(50)).await;
    assert!(
        !dash.tick(),
        "The line of a row panel needs no answer, so it costs no set of queries."
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn toggle_row_does_nothing_for_a_panel_that_is_not_a_row_panel() {
    let source = Arc::new(Asked::default());
    let mut dash = runtime(&source);
    settle(&dash).await;
    source.take();
    let before = dash.collapsed_rows().clone();
    dash.toggle_row(0);
    dash.toggle_row(99);
    assert_eq!(dash.collapsed_rows(), &before);
    assert!(!dash.is_refreshing());
}

/// Returns the number of panels that show data and whose title the renderer
/// wrote into a buffer of this height.
fn drawn_data_panels(snapshot: &Snapshot, height: u16) -> usize {
    let drawn = picture(snapshot, &DashboardView::new(), 110, height);
    ["alpha", "bravo", "charlie", "delta"]
        .iter()
        .filter(|title| drawn.contains(*title))
        .count()
}

/// The renderer and the runtime must hide the same panels. They must also
/// drop the same panels, in each state of the row panels and at each height.
#[tokio::test(flavor = "multi_thread")]
async fn the_runtime_asks_about_exactly_the_panels_that_the_renderer_draws() {
    for toggles in [vec![], vec![ROW_ONE], vec![ROW_TWO], vec![ROW_ONE, ROW_TWO]] {
        for height in 2..=30_u16 {
            let source = Arc::new(Asked::default());
            let mut dash = runtime(&source);
            settle(&dash).await;
            for row in &toggles {
                dash.toggle_row(*row);
                settle(&dash).await;
            }
            source.take();
            dash.set_panel_area(Size::new(110, height));
            settle(&dash).await;
            let asked = source.take().len();
            assert_eq!(
                asked,
                drawn_data_panels(&dash.snapshot(), height),
                "A terminal of {height} lines, after the toggles {toggles:?}."
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_renderer_draws_no_panel_that_a_collapsed_row_panel_hides() {
    let source = Arc::new(Asked::default());
    let dash = runtime(&source);
    settle(&dash).await;
    let drawn = picture(&dash.snapshot(), &DashboardView::new(), 110, 30);
    assert!(
        drawn.contains("alpha") && drawn.contains("bravo"),
        "{drawn}"
    );
    assert!(
        !drawn.contains("charlie") && !drawn.contains("delta"),
        "A collapsed row panel hides these two panels. {drawn}"
    );
    assert!(drawn.contains("▾ row one ─"), "{drawn}");
    assert!(drawn.contains("▸ row two ─"), "{drawn}");
    assert!(
        drawn.contains("The row hides 2 panels."),
        "The line counts the panels that it hides. {drawn}"
    );
}

/// A person cannot see a hidden panel, so the key `Tab` must not select it. A
/// key that selected it would seem to do nothing.
#[test]
fn tab_skips_the_panels_that_a_collapsed_row_panel_hides() {
    let dash = dashboard();
    let snapshot = Snapshot {
        collapsed: CollapsedRows::of(&dash),
        ..Snapshot::loading(dash.panels.len(), dash.time.clone())
    };
    let mut view = DashboardView::new();
    let mut seen = Vec::new();
    for _ in 0..5 {
        view.handle_key(Key::Tab, &dash, &snapshot);
        seen.push(view.selected());
    }
    assert_eq!(seen, vec![1, 2, 3, 0, 1], "The panels 4 and 5 are hidden.");
    view.handle_key(Key::BackTab, &dash, &snapshot);
    view.handle_key(Key::BackTab, &dash, &snapshot);
    assert_eq!(view.selected(), 3, "The key BackTab skips them as well.");
}

/// A hidden panel is not a panel that the terminal is too small for. The
/// status row must not name it.
#[test]
fn dropped_panels_counts_no_panel_that_a_collapsed_row_panel_hides() {
    let dash = dashboard();
    let snapshot = Snapshot {
        collapsed: CollapsedRows::of(&dash),
        ..Snapshot::loading(dash.panels.len(), dash.time.clone())
    };
    let view = DashboardView::new();
    assert_eq!(
        view.dropped_panels(&dash, &snapshot, Rect::new(0, 0, 110, 40)),
        0
    );
    let open = Snapshot::loading(dash.panels.len(), dash.time.clone());
    assert_eq!(
        view.dropped_panels(&dash, &open, Rect::new(0, 0, 110, 2)),
        4,
        "Two lines hold both row panels, and no panel that shows data."
    );
}

/// The program passes the selection and the key to `row_key`, and it passes
/// the answer to the runtime. That path must collapse the row panel.
#[tokio::test(flavor = "multi_thread")]
async fn enter_on_a_selected_row_panel_reaches_the_runtime() {
    let source = Arc::new(Asked::default());
    let mut dash = runtime(&source);
    settle(&dash).await;
    let mut view = DashboardView::new();
    view.handle_key(Key::Tab, dash.dashboard(), &dash.snapshot());
    assert_eq!(view.selected(), ROW_ONE);
    let row = row_key(Key::Enter, view.selected(), dash.dashboard())
        .expect("The selection holds a row panel.");
    dash.toggle_row(row);
    assert!(dash.snapshot().collapsed.contains(ROW_ONE));
}
