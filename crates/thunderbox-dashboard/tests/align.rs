//! Tests that a dashboard can give one width to the charts of a stack.
//!
//! A stack is the set of panels that hold the same columns of the grid, one
//! above another. A chart takes the width of its column of labels from its own
//! widest label. Two panels of one stack therefore drew two charts of different
//! widths, and the two axes of time started at different cells. A dashboard
//! that states `align = "column"` gives one width to the whole stack.

use std::sync::Arc;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use thunderbox_core::{Dashboard, Field, Frame, TimeRange};
use thunderbox_dashboard::{DashboardView, PanelState, Snapshot};

/// The width of the area that every test in this file draws into.
const WIDTH: u16 = 60;

/// The height of the area that every test in this file draws into.
const HEIGHT: u16 = 20;

/// Returns frames that hold one column of these values.
fn frames(values: Vec<Option<f64>>) -> Vec<Frame> {
    let stamps: Vec<i64> = (0..values.len() as i64).map(|i| i * 1000).collect();
    vec![Frame::named(
        "s",
        vec![Field::time("time", stamps), Field::f64("v", values)],
    )]
}

/// Returns a dashboard that stacks two charts in one column of the grid.
///
/// The first panel shows a share of use, and the second shows a rate. The two
/// panels therefore hold labels of different widths. The `align` argument holds
/// the line that states the rule, or an empty line.
fn stack(align: &str) -> Dashboard {
    let text = format!(
        r#"
title = "stack"
{align}

[[panel]]
title = "cpu"
kind = "timeseries"
unit = "percent"
grid = {{ x = 0, y = 0, w = 24, h = 5 }}

[[panel]]
title = "network"
kind = "timeseries"
unit = "bytes/s"
grid = {{ x = 0, y = 5, w = 24, h = 5 }}
"#
    );
    Dashboard::from_toml(&text).expect("The test dashboard must be correct.")
}

/// Returns a snapshot in which both panels hold data.
fn ready() -> Snapshot {
    let mut snapshot = Snapshot::loading(2, TimeRange::default());
    snapshot.panels[0] = PanelState::Ready(Arc::new(frames(vec![Some(10.0), Some(80.0)])));
    snapshot.panels[1] = PanelState::Ready(Arc::new(frames(vec![Some(1.0e9), Some(8.0e9)])));
    snapshot
}

/// Draws the dashboard, and returns the cell that holds the axis of each panel.
///
/// A chart draws its axis of values as a vertical line, and the border of a
/// panel draws the same character. The function therefore takes the second such
/// cell of a row. It reads one row of each of the two panels.
fn axis_columns(dashboard: &Dashboard) -> Vec<Option<u16>> {
    let view = DashboardView::new();
    let area = Rect::new(0, 0, WIDTH, HEIGHT);
    let mut buf = Buffer::empty(area);
    view.render(dashboard, &ready(), area, &mut buf);
    [1, 11]
        .iter()
        .map(|row| {
            (0..WIDTH)
                .filter(|x| buf[(*x, *row)].symbol() == "\u{2502}")
                .nth(1)
        })
        .collect()
}

#[test]
fn a_dashboard_that_states_no_rule_lets_each_panel_measure_its_own_labels() {
    let columns = axis_columns(&stack(""));
    assert!(
        columns.iter().all(Option::is_some),
        "Both panels must draw an axis. {columns:?}"
    );
    assert_ne!(
        columns[0], columns[1],
        "This dashboard keeps the rule that the project had before. {columns:?}"
    );
}

#[test]
fn a_dashboard_that_states_the_rule_gives_one_chart_width_to_a_stack() {
    let columns = axis_columns(&stack(r#"align = "column""#));
    assert!(
        columns.iter().all(Option::is_some),
        "Both panels must draw an axis. {columns:?}"
    );
    assert_eq!(
        columns[0], columns[1],
        "The two charts of the stack must start at the same cell. {columns:?}"
    );
}

/// Two panels that hold different columns of the grid cannot start and end at
/// the same two cells. The rule therefore leaves such a pair alone.
#[test]
fn the_rule_leaves_two_panels_of_different_widths_alone() {
    let text = r#"
title = "two widths"
align = "column"

[[panel]]
title = "cpu"
kind = "timeseries"
unit = "percent"
grid = { x = 0, y = 0, w = 24, h = 5 }

[[panel]]
title = "network"
kind = "timeseries"
unit = "bytes/s"
grid = { x = 0, y = 5, w = 12, h = 5 }
"#;
    let dashboard = Dashboard::from_toml(text).expect("The test dashboard must be correct.");
    let columns = axis_columns(&dashboard);
    assert_ne!(
        columns[0], columns[1],
        "Each panel measures its own labels. {columns:?}"
    );
}

/// A panel that waits for an answer draws a message, and it holds no label. It
/// must not widen the chart of the panel beside it.
#[test]
fn a_panel_that_holds_no_data_widens_no_other_panel() {
    let dashboard = stack(r#"align = "column""#);
    let view = DashboardView::new();
    let area = Rect::new(0, 0, WIDTH, HEIGHT);

    let mut alone = Snapshot::loading(2, TimeRange::default());
    alone.panels[1] = PanelState::Ready(Arc::new(frames(vec![Some(1.0e9), Some(8.0e9)])));
    let mut buf = Buffer::empty(area);
    view.render(&dashboard, &alone, area, &mut buf);
    let with_message = (0..WIDTH)
        .filter(|x| buf[(*x, 11)].symbol() == "\u{2502}")
        .nth(1);

    let mut only = Snapshot::loading(2, TimeRange::default());
    only.panels[1] = PanelState::Ready(Arc::new(frames(vec![Some(1.0e9), Some(8.0e9)])));
    let mut buf = Buffer::empty(area);
    view.render(&stack(""), &only, area, &mut buf);
    let alone_column = (0..WIDTH)
        .filter(|x| buf[(*x, 11)].symbol() == "\u{2502}")
        .nth(1);

    assert_eq!(
        with_message, alone_column,
        "The panel that waits for an answer must not move the chart beside it."
    );
}
