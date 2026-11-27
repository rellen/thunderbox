//! Tests that a dashboard labels the axis of time by the kind of time range.
//!
//! A range that holds a fixed moment at each end is held. It names one window
//! for ever, so the labels state clock times. A range that follows the clock
//! keeps labels that state a distance from the present moment. ADR 0091 in
//! `docs/contributor/adr` records the rule.

use std::sync::Arc;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use thunderbox_core::{Dashboard, Field, Frame, RelativeTime, TimeRange};
use thunderbox_dashboard::{DashboardView, PanelState, Snapshot};

/// The width of the area that each test draws into.
const WIDTH: u16 = 60;

/// The height of the area that each test draws into.
const HEIGHT: u16 = 12;

/// The moment 14:00:00 UTC on 1970-01-02, in milliseconds.
const TWO_PM: i64 = 86_400_000 + 50_400_000;

/// Returns a dashboard of one chart that holds the width of the area.
fn dashboard() -> Dashboard {
    Dashboard::from_toml(
        r#"
title = "clock"

[[panel]]
title = "cpu"
kind = "timeseries"
unit = "percent"
grid = { x = 0, y = 0, w = 24, h = 12 }
"#,
    )
    .expect("The test dashboard must be correct.")
}

/// Returns frames whose three points lie inside the window of the tests.
fn frames() -> Vec<Frame> {
    let stamps: Vec<i64> = (0..3).map(|i| TWO_PM + i * 60_000).collect();
    vec![Frame::named(
        "s",
        vec![
            Field::time("time", stamps),
            Field::f64("v", vec![Some(10.0), Some(50.0), Some(30.0)]),
        ],
    )]
}

/// Draws the dashboard for a range and a moment of the last refresh.
fn picture(range: TimeRange, updated_at: i64) -> String {
    let mut snapshot = Snapshot::loading(1, range);
    snapshot.panels[0] = PanelState::Ready(Arc::new(frames()));
    snapshot.updated_at = Some(updated_at);
    let area = Rect::new(0, 0, WIDTH, HEIGHT);
    let mut buf = Buffer::empty(area);
    DashboardView::new().render(&dashboard(), &snapshot, area, &mut buf);
    (0..HEIGHT)
        .map(|y| {
            (0..WIDTH)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Returns a range that holds a fixed moment at each end.
fn held() -> TimeRange {
    TimeRange {
        from: RelativeTime::Absolute(TWO_PM),
        to: RelativeTime::Absolute(TWO_PM + 120_000),
    }
}

#[test]
fn a_held_range_draws_clock_labels() {
    let drawn = picture(held(), TWO_PM + 3_600_000);
    for label in ["14:00:00", "14:01:00", "14:02:00"] {
        assert!(
            drawn.contains(label),
            "The axis must hold {label}. It drew this. {drawn}"
        );
    }
}

#[test]
fn a_held_range_draws_the_same_axis_after_a_refresh() {
    assert_eq!(
        picture(held(), TWO_PM + 3_600_000),
        picture(held(), TWO_PM + 7_200_000),
        "A refresh must not change a window that stands still."
    );
}

#[test]
fn a_live_range_draws_labels_from_now() {
    let range = TimeRange {
        from: RelativeTime::Offset(-120_000),
        to: RelativeTime::Now,
    };
    let drawn = picture(range, TWO_PM + 120_000);
    assert!(
        drawn.contains("now") && drawn.contains("-2m"),
        "A live range must keep the labels from now. It drew this. {drawn}"
    );
    assert!(
        !drawn.contains("14:0"),
        "A live range must write no clock time. It drew this. {drawn}"
    );
}

#[test]
fn a_following_range_draws_labels_from_now() {
    let range = TimeRange {
        from: RelativeTime::Offset(-3_720_000),
        to: RelativeTime::Offset(-3_600_000),
    };
    let drawn = picture(range, TWO_PM + 3_720_000);
    assert!(
        drawn.contains("-62m") && !drawn.contains("14:0"),
        "A following range must keep the labels from now. It drew this. {drawn}"
    );
}
