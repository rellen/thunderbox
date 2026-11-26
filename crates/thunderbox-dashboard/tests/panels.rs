//! Tests that each type of panel draws its own picture.
//!
//! Every type of panel drew as a time series before this test existed. The
//! first test here holds the six types apart, so that the fault cannot
//! return.

use std::sync::Arc;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Modifier;
use thunderbox_core::{
    Dashboard, Field, FieldConfig, Frame, Query, QueryRequest, RelativeTime, Threshold, TimeRange,
};
use thunderbox_dashboard::{DashboardView, PanelState, Snapshot};
use thunderbox_source::DataSource;
use thunderbox_source_testdata::TestData;

/// The name that a dashboard writes for each type of panel.
const KINDS: [&str; 6] = ["timeseries", "stat", "gauge", "bargauge", "table", "top"];

/// Returns a frame that holds a time column and two columns of numbers.
fn frames() -> Vec<Frame> {
    let stamps: Vec<i64> = (0..40).map(|i| i * 1000).collect();
    let wave = |phase: f64| -> Vec<Option<f64>> {
        (0..40)
            .map(|i| Some(50.0 + 40.0 * (f64::from(i) / 6.0 + phase).sin()))
            .collect()
    };
    let config = FieldConfig {
        unit: Some("percent".into()),
        min: Some(0.0),
        max: Some(100.0),
        thresholds: vec![Threshold {
            value: 80.0,
            color: "critical".into(),
        }],
        ..Default::default()
    };
    vec![Frame::named(
        "load",
        vec![
            Field::time("time", stamps),
            Field::f64("first", wave(0.0)).with_config(config.clone()),
            Field::f64("second", wave(1.5)).with_config(config),
        ],
    )]
}

/// Returns a dashboard that holds one panel of this type.
fn dashboard_of(kind: &str) -> Dashboard {
    let text = format!(
        r#"
title = "one panel"

[[panel]]
title = "load"
kind = "{kind}"
unit = "percent"
min = 0.0
max = 100.0
grid = {{ x = 0, y = 0, w = 24, h = 10 }}
"#
    );
    Dashboard::from_toml(&text).expect("The test dashboard must be correct.")
}

/// Returns the text that the buffer holds.
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

/// Draws one panel of this type, in this state, and returns what it drew.
fn picture(kind: &str, state: PanelState, w: u16, h: u16) -> String {
    let dashboard = dashboard_of(kind);
    let mut snapshot = Snapshot::loading(1, TimeRange::default());
    snapshot.panels[0] = state;
    let view = DashboardView::new();
    let area = Rect::new(0, 0, w, h);
    let mut buf = Buffer::empty(area);
    view.render(&dashboard, &snapshot, area, &mut buf);
    text(&buf)
}

/// Draws one panel of this type over the data of the test.
fn ready(kind: &str, w: u16, h: u16) -> String {
    picture(kind, PanelState::Ready(Arc::new(frames())), w, h)
}

#[test]
fn each_type_of_panel_draws_its_own_picture() {
    let drawn: Vec<(&str, String)> = KINDS.iter().map(|k| (*k, ready(k, 60, 16))).collect();
    for (i, (kind, picture)) in drawn.iter().enumerate() {
        for (other, another) in drawn.iter().skip(i + 1) {
            assert_ne!(
                picture, another,
                "The {kind} panel and the {other} panel drew the same picture."
            );
        }
    }
}

#[test]
fn each_type_of_panel_draws_the_data_of_the_test() {
    for kind in KINDS {
        let drawn = ready(kind, 60, 16);
        assert!(
            drawn.contains("load"),
            "The {kind} panel must draw the title. It drew this. {drawn}"
        );
        assert!(
            !drawn.contains("no data"),
            "The {kind} panel must draw the data. It drew this. {drawn}"
        );
        assert!(
            !drawn.contains("loading"),
            "The {kind} panel holds an answer. It drew this. {drawn}"
        );
    }
}

#[test]
fn each_type_of_panel_reports_that_the_data_source_has_not_answered() {
    for kind in KINDS {
        let drawn = picture(kind, PanelState::Loading, 60, 16);
        assert!(
            drawn.contains("loading"),
            "The {kind} panel must report the wait. It drew this. {drawn}"
        );
    }
}

#[test]
fn each_type_of_panel_reports_the_text_of_an_error() {
    for kind in KINDS {
        let drawn = picture(kind, PanelState::Error("upstream refused".into()), 60, 16);
        assert!(
            drawn.contains("upstream refused"),
            "The {kind} panel must report the error. It drew this. {drawn}"
        );
        assert!(
            drawn.contains("load"),
            "The {kind} panel must still draw the title. It drew this. {drawn}"
        );
    }
}

#[test]
fn each_type_of_panel_reports_a_result_that_holds_no_value() {
    for kind in KINDS {
        let empty = PanelState::Ready(Arc::new(Vec::new()));
        let drawn = picture(kind, empty, 60, 16);
        assert!(
            drawn.contains("no data"),
            "The {kind} panel must report the empty result. It drew this. {drawn}"
        );
    }
}

#[test]
fn each_type_of_panel_draws_a_column_of_holes_as_a_result_that_holds_no_value() {
    let holes = vec![Frame::named(
        "load",
        vec![
            Field::time("time", vec![0, 1000, 2000]),
            Field::f64("first", vec![None, None, None]),
        ],
    )];
    for kind in KINDS {
        // A table and a top panel show the holes themselves, so they hold
        // data that the other panels do not.
        if kind == "table" || kind == "top" {
            continue;
        }
        let drawn = picture(kind, PanelState::Ready(Arc::new(holes.clone())), 60, 16);
        assert!(
            drawn.contains("no data"),
            "The {kind} panel must report the holes. It drew this. {drawn}"
        );
    }
}

/// This test asserts nothing about the picture, and that is the point of it.
/// It draws every type of panel in an area of every shape, and it fails only
/// when a panel panics. A panel that draws into an area of no width, or of one
/// row, meets the hardest case here.
#[test]
fn no_type_of_panel_panics_in_an_area_of_any_size() {
    let sizes = [
        (0, 0),
        (1, 1),
        (3, 3),
        (4, 3),
        (5, 5),
        (8, 4),
        (12, 3),
        (20, 3),
        (30, 8),
        (60, 16),
        (120, 40),
        (200, 3),
        (3, 40),
    ];
    for kind in KINDS {
        for (w, h) in sizes {
            let _ = ready(kind, w, h);
            let _ = picture(kind, PanelState::Loading, w, h);
            let _ = picture(kind, PanelState::Ready(Arc::new(Vec::new())), w, h);
        }
    }
}

/// A panel must never write the text that Rust gives a value which is not
/// finite. `CLAUDE.md` section 1.5 names `—` as the symbol that a panel draws
/// in place of such a value. A table and a top panel show each value, so each
/// of them draws that symbol three times here. The other types of panel reduce
/// the column to one number, and the last value of this column is 7.
#[test]
fn each_type_of_panel_draws_a_value_that_is_not_finite() {
    let odd = vec![Frame::named(
        "load",
        vec![
            Field::time("time", vec![0, 1000, 2000, 3000]),
            Field::f64(
                "first",
                vec![
                    Some(f64::NAN),
                    Some(f64::INFINITY),
                    Some(f64::NEG_INFINITY),
                    Some(7.0),
                ],
            ),
        ],
    )];
    for kind in KINDS {
        let drawn = picture(kind, PanelState::Ready(Arc::new(odd.clone())), 60, 16);
        for text in ["NaN", "inf"] {
            assert!(
                !drawn.contains(text),
                "The {kind} panel must not write `{text}`. It drew this. {drawn}"
            );
        }
    }
    for kind in ["table", "top"] {
        let drawn = picture(kind, PanelState::Ready(Arc::new(odd.clone())), 60, 16);
        assert_eq!(
            drawn.matches('—').count(),
            3,
            "A {kind} panel must draw the symbol for each value that is not finite. It drew \
             this. {drawn}"
        );
    }
}

/// Returns a frame that holds forty columns of numbers.
fn many_series() -> Vec<Frame> {
    let stamps: Vec<i64> = (0..10).map(|i| i * 1000).collect();
    let mut fields = vec![Field::time("time", stamps)];
    for s in 0..40 {
        let values = (0..10).map(|i| Some(f64::from(s + i))).collect();
        fields.push(Field::f64(format!("series {s}"), values));
    }
    vec![Frame::named("load", fields)]
}

/// This test asserts nothing about the picture, and that is the point of it. It
/// fails only when a panel panics over more series than it can hold. The test
/// below reads the note that such a panel writes.
#[test]
fn no_type_of_panel_panics_with_more_series_than_it_can_hold() {
    let many = many_series();
    for kind in KINDS {
        for (w, h) in [(8, 4), (30, 6), (60, 16), (120, 40)] {
            let _ = picture(kind, PanelState::Ready(Arc::new(many.clone())), w, h);
        }
    }
}

/// A panel that drops a series must count the series that it dropped.
/// `CLAUDE.md` states the rule in the section `The rule for a short label
/// inside a panel`. The panel writes the whole sentence where that sentence
/// fits, and it writes the short form where the sentence does not fit.
///
/// A time series panel draws every series, and a wide stat panel holds all
/// forty. Neither of those two drops a series at the size below, so neither
/// writes a note.
#[test]
fn a_panel_that_drops_a_series_counts_the_ones_that_it_dropped() {
    let many = many_series();
    for kind in ["gauge", "bargauge"] {
        let wide = picture(kind, PanelState::Ready(Arc::new(many.clone())), 60, 16);
        assert!(
            wide.contains("series do not fit."),
            "A wide {kind} panel must write the whole sentence. It drew this. {wide}"
        );
    }
    for kind in ["stat", "gauge", "bargauge"] {
        let narrow = picture(kind, PanelState::Ready(Arc::new(many.clone())), 8, 4);
        assert!(
            narrow.contains("+39"),
            "A narrow {kind} panel must write the short form. It drew this. {narrow}"
        );
    }
}

/// Draws a time series panel over the data of the test, for one window.
///
/// The `updated_at` argument holds the moment when the set of queries asked.
/// The panel reads that moment through `Snapshot::window`.
fn picture_of_window(range: TimeRange, updated_at: i64) -> String {
    let dashboard = dashboard_of("timeseries");
    let mut snapshot = Snapshot::loading(1, range);
    snapshot.panels[0] = PanelState::Ready(Arc::new(frames()));
    snapshot.updated_at = Some(updated_at);
    let view = DashboardView::new();
    let area = Rect::new(0, 0, 60, 16);
    let mut buf = Buffer::empty(area);
    view.render(&dashboard, &snapshot, area, &mut buf);
    text(&buf)
}

/// Each label of the axis of time states the distance from the present moment
/// to that place. A window that ended one hour ago therefore holds no place
/// that is the present moment.
///
/// The data of the test covers the first 39 seconds of this window of two
/// minutes. The axis still covers the whole window, so the panel draws the
/// data on the left and it leaves the rest empty.
#[test]
fn a_panel_that_draws_a_window_in_the_past_labels_no_place_as_now() {
    let drawn = picture_of_window(
        TimeRange {
            from: RelativeTime::Absolute(0),
            to: RelativeTime::Absolute(120_000),
        },
        3_720_000,
    );
    assert!(
        !drawn.contains("now"),
        "No place on this axis is the present moment. It drew this. {drawn}"
    );
    assert!(
        drawn.contains("-62m") && drawn.contains("-60m"),
        "The axis must state the distance back in time. It drew this. {drawn}"
    );
}

/// A window that ends at the present moment still labels its right edge as the
/// present moment.
#[test]
fn a_panel_that_draws_a_window_which_ends_now_labels_the_right_edge_as_now() {
    let drawn = picture_of_window(
        TimeRange {
            from: RelativeTime::Offset(-39_000),
            to: RelativeTime::Now,
        },
        39_000,
    );
    assert!(
        drawn.contains("now"),
        "The right edge of this axis is the present moment. It drew this. {drawn}"
    );
}

/// Returns the text of every header cell that carries a line under it.
///
/// A top panel draws that line under the header of the column that puts its
/// rows in order.
fn marked_header(sort: Option<&str>) -> String {
    let field = sort.map_or(String::new(), |name| format!("sort = {name:?}"));
    let text = format!(
        r#"
title = "one panel"

[[panel]]
title = "load"
kind = "top"
{field}
grid = {{ x = 0, y = 0, w = 24, h = 10 }}
"#
    );
    let dashboard = Dashboard::from_toml(&text).expect("The test dashboard must be correct.");
    let mut snapshot = Snapshot::loading(1, TimeRange::default());
    snapshot.panels[0] = PanelState::Ready(Arc::new(frames()));
    let area = Rect::new(0, 0, 60, 16);
    let mut buf = Buffer::empty(area);
    DashboardView::new().render(&dashboard, &snapshot, area, &mut buf);
    buf.content
        .iter()
        .filter(|c| c.modifier.contains(Modifier::UNDERLINED))
        .map(|c| c.symbol().to_string())
        .collect::<String>()
        .trim()
        .to_string()
}

/// The field `sort` of a dashboard must reach the top panel. A view that
/// dropped the field would order every top panel by its first column of
/// numbers, and each test of `thunderbox-widgets` would still pass.
#[test]
fn a_top_panel_of_a_dashboard_reads_the_column_that_the_dashboard_names() {
    assert_eq!(
        marked_header(Some("second")),
        "second",
        "The dashboard names the column second."
    );
    assert_eq!(
        marked_header(None),
        "first",
        "A panel that names no column takes its first column of numbers."
    );
}

/// Draws one panel of this kind over a list of two processes, and returns the
/// header row and the row of the first process.
///
/// The dashboard writes the line `left` as it is. The column `pid` is an
/// identifier, so a panel stands it at the right unless the dashboard names it.
fn pid_rows(kind: &str, left: &str) -> (String, String) {
    let file = format!(
        r#"
title = "one panel"

[[panel]]
title = "processes"
kind = "{kind}"
{left}
grid = {{ x = 0, y = 0, w = 24, h = 10 }}
"#
    );
    let dashboard = Dashboard::from_toml(&file).expect("The test dashboard must be correct.");
    let pid = Field::f64("pid", vec![Some(1234.0), Some(5.0)]).with_config(FieldConfig {
        identifier: true,
        ..Default::default()
    });
    let name = Field {
        name: "name".into(),
        values: thunderbox_core::Values::Str(vec![Some("editor".into()), Some("init".into())]),
        ..Default::default()
    };
    let frames = vec![Frame::new(vec![pid, name])];
    let mut snapshot = Snapshot::loading(1, TimeRange::default());
    snapshot.panels[0] = PanelState::Ready(Arc::new(frames));
    let area = Rect::new(0, 0, 40, 8);
    let mut buf = Buffer::empty(area);
    DashboardView::new().render(&dashboard, &snapshot, area, &mut buf);
    let out = text(&buf);
    let mut lines = out.lines().skip(1);
    let header = lines.next().unwrap_or_default().to_string();
    let first = lines.next().unwrap_or_default().to_string();
    (header, first)
}

/// The field `left` of a dashboard must reach the table panel and the top
/// panel. A view that dropped the field would stand every column of numbers at
/// the right, and each test of `thunderbox-widgets` would still pass.
#[test]
fn a_table_panel_and_a_top_panel_of_a_dashboard_keep_the_columns_that_the_dashboard_names_at_the_left()
 {
    for kind in ["table", "top"] {
        let (header, row) = pid_rows(kind, "");
        assert!(
            header.contains(" pid") && !header.starts_with("\u{2502}pid"),
            "No field left. The header must stand at the right. {kind} {header}"
        );
        assert!(
            row.contains("1234") && !row.starts_with("\u{2502}1234"),
            "No field left. The cell must stand at the right. {kind} {row}"
        );
        let (header, row) = pid_rows(kind, r#"left = ["pid"]"#);
        assert!(
            header.starts_with("\u{2502}pid "),
            "The dashboard names the column pid. {kind} {header}"
        );
        assert!(
            row.starts_with("\u{2502}1234 "),
            "The cell must stand at the left. {kind} {row}"
        );
    }
}

/// The field `widths` of a dashboard must reach the table panel and the top
/// panel. A view that dropped the field would give each column an equal share,
/// and each test of `thunderbox-widgets` would still pass.
#[test]
fn a_table_panel_and_a_top_panel_of_a_dashboard_give_the_columns_the_widths_that_the_dashboard_names()
 {
    let start_of_name = |header: &str| {
        header
            .chars()
            .position(|c| c == 'n')
            .expect("The header holds `name`.")
    };
    for kind in ["table", "top"] {
        let (header, _) = pid_rows(kind, "");
        // The 38 inner cells hold two columns of 19 cells and one gap, so the
        // name starts at the cell 21.
        assert_eq!(
            start_of_name(&header),
            21,
            "No field widths. {kind} {header}"
        );
        let (header, _) = pid_rows(kind, "widths = { pid = 8 }");
        // The pid takes 8 cells, so the name starts at the cell 10.
        assert_eq!(start_of_name(&header), 10, "{kind} {header}");
    }
}

/// Returns the process list of the data source testdata for one moment.
async fn process_list() -> Vec<Frame> {
    let at = 1_000_000;
    let request = QueryRequest::new(
        Query {
            source: "t".into(),
            expr: "processes".into(),
        },
        &TimeRange {
            from: RelativeTime::Absolute(at - 60_000),
            to: RelativeTime::Absolute(at),
        },
        at,
        10,
    );
    TestData::new()
        .query(request)
        .await
        .expect("The query must answer.")
}

/// A top panel that draws the process list of the data source testdata gives
/// `pid` 6 cells, `cpu` 4 cells, and `MemB` 4 cells. The name takes the rest.
#[tokio::test]
async fn a_top_panel_gives_the_columns_of_the_process_list_the_widths_that_the_data_source_states()
{
    let file = r#"
title = "one panel"

[[panel]]
title = "processes"
kind = "top"
grid = { x = 0, y = 0, w = 24, h = 10 }
"#;
    let dashboard = Dashboard::from_toml(file).expect("The test dashboard must be correct.");
    let frames = process_list().await;
    let draw = |frames: Vec<Frame>| {
        let mut snapshot = Snapshot::loading(1, TimeRange::default());
        snapshot.panels[0] = PanelState::Ready(Arc::new(frames));
        let area = Rect::new(0, 0, 40, 8);
        let mut buf = Buffer::empty(area);
        DashboardView::new().render(&dashboard, &snapshot, area, &mut buf);
        text(&buf)
    };
    let out = draw(frames.clone());
    let header: Vec<char> = out.lines().nth(1).unwrap_or_default().chars().collect();
    let at = |start: usize, count: usize| header[start..start + count].iter().collect::<String>();
    // The 38 inner cells hold 6, 4, and 4 cells, three gaps, and 21 cells of
    // the name. The border stands in the cell 0.
    assert_eq!(at(4, 3), "pid", "{out}");
    assert_eq!(at(8, 4), "name", "{out}");
    assert_eq!(at(31, 3), "cpu", "{out}");
    assert_eq!(at(35, 4), "MemB", "{out}");
    // A list with other rows keeps every column at the same cells.
    let mut other = frames;
    let rows = other[0].len();
    other[0].fields[1].values =
        thunderbox_core::Values::Str(vec![Some("a process with a very long name".into()); rows]);
    let moved = draw(other);
    assert_eq!(
        moved.lines().nth(1),
        out.lines().nth(1),
        "The columns must not move when the rows change."
    );
}
