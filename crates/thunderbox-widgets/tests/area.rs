//! The tests of the rule for an area that reaches past the buffer.
//!
//! A program that embeds a panel gives the panel an area and a buffer. The area
//! can reach past the buffer, and the buffer can start away from the origin.
//! Each panel must then draw exactly what it draws for the intersection of the
//! area and the buffer. No panel may panic for any area.
//!
//! Each test draws a panel two times. The first draw takes the area of the
//! caller. The second draw takes the intersection. The two buffers must be
//! equal. Two draws that are wrong in the same way are also equal. Each test
//! therefore also reads the title on the top row of the intersection.

use std::panic::{AssertUnwindSafe, catch_unwind};

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::Widget;
use thunderbox_core::{Field, Frame};
use thunderbox_widgets::{
    BarGaugePanel, GaugePanel, PanelData, RowPanel, StatPanel, TablePanel, Theme, TimeSeriesPanel,
    TopPanel, draws_a_panel,
};

/// The title of each panel. A test reads it on the top row.
const TITLE: &str = "CPU";

/// One case, as the area of the buffer and then the area of the panel.
struct Case {
    /// The words that name the case in a failure.
    name: &'static str,
    /// The area of the buffer.
    buffer: Rect,
    /// The area that the caller gives to the panel.
    area: Rect,
}

/// The four cases that each panel must draw.
const CASES: [Case; 4] = [
    Case {
        name: "the area reaches past the right edge and the lower edge",
        buffer: Rect::new(0, 0, 40, 10),
        area: Rect::new(20, 5, 40, 10),
    },
    Case {
        name: "the buffer starts away from the origin",
        buffer: Rect::new(10, 4, 40, 10),
        area: Rect::new(0, 0, 40, 10),
    },
    Case {
        name: "the area is much larger than the buffer",
        buffer: Rect::new(0, 0, 20, 6),
        area: Rect::new(0, 0, 80, 30),
    },
    Case {
        name: "the area and the buffer share no cell",
        buffer: Rect::new(0, 0, 20, 6),
        area: Rect::new(30, 10, 40, 10),
    },
];

/// The panels that show data.
#[derive(Debug, Clone, Copy)]
enum Kind {
    TimeSeries,
    Stat,
    Gauge,
    BarGauge,
    Table,
    Top,
}

/// Each panel that shows data.
const KINDS: [Kind; 6] = [
    Kind::TimeSeries,
    Kind::Stat,
    Kind::Gauge,
    Kind::BarGauge,
    Kind::Table,
    Kind::Top,
];

/// Returns one frame that holds two series.
fn two_series() -> Vec<Frame> {
    let stamps: Vec<i64> = (0..20).map(|i| i * 1000).collect();
    let user: Vec<Option<f64>> = (0..20).map(|i| Some(f64::from(i) * 4.0)).collect();
    let system: Vec<Option<f64>> = (0..20).map(|i| Some(80.0 - f64::from(i) * 3.0)).collect();
    vec![Frame::named(
        "cpu",
        vec![
            Field::time("time", stamps),
            Field::f64("user", user),
            Field::f64("system", system),
        ],
    )]
}

/// Returns the four states of the data, each with the words that name it.
fn states(frames: &[Frame]) -> [(&'static str, PanelData<'_>); 4] {
    [
        ("a frame of two series", PanelData::Frames(frames)),
        ("no frame", PanelData::Frames(&[])),
        ("no answer yet", PanelData::Loading),
        (
            "an error",
            PanelData::Error("The data source refused the query."),
        ),
    ]
}

/// Draws one panel of one kind into the buffer.
fn draw(kind: Kind, data: PanelData<'_>, theme: &Theme, area: Rect, buf: &mut Buffer) {
    match kind {
        Kind::TimeSeries => TimeSeriesPanel::new(TITLE, data, theme).render(area, buf),
        Kind::Stat => StatPanel::new(TITLE, data, theme).render(area, buf),
        Kind::Gauge => GaugePanel::new(TITLE, data, theme).render(area, buf),
        Kind::BarGauge => BarGaugePanel::new(TITLE, data, theme).render(area, buf),
        Kind::Table => TablePanel::new(TITLE, data, theme).render(area, buf),
        Kind::Top => TopPanel::new(TITLE, data, theme)
            .sort_by("user")
            .render(area, buf),
    }
}

/// Returns the text of one row of the buffer, from the left edge to the right
/// edge of the area.
fn row_text(buf: &Buffer, area: Rect, y: u16) -> String {
    (area.left()..area.right())
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect()
}

/// Draws a panel at the area and at the intersection, and compares the two.
///
/// The function returns the text of a failure, or `None` when the draw keeps
/// the rule. `title_row` returns the row where the title must show, or `None`
/// when the panel draws nothing for the intersection.
fn check(
    name: &str,
    case: &Case,
    title_row: impl Fn(Rect) -> Option<u16>,
    paint: impl Fn(Rect, &mut Buffer),
) -> Option<String> {
    let clipped = case.area.intersection(case.buffer);
    let mut expected = Buffer::empty(case.buffer);
    paint(clipped, &mut expected);

    let mut actual = Buffer::empty(case.buffer);
    let drawn = catch_unwind(AssertUnwindSafe(|| paint(case.area, &mut actual)));
    if drawn.is_err() {
        return Some(format!("{name}, {}: the panel panics.", case.name));
    }
    if actual != expected {
        return Some(format!(
            "{name}, {}: the panel draws a buffer that differs from the intersection.",
            case.name
        ));
    }
    if let Some(y) = title_row(clipped) {
        let row = row_text(&actual, clipped, y);
        if !row.contains(TITLE) {
            return Some(format!(
                "{name}, {}: the top row of the intersection holds no title. It holds {row:?}.",
                case.name
            ));
        }
    }
    None
}

/// Returns the top row of a panel that `draws_a_panel` accepts.
fn top_row_of_a_panel(clipped: Rect) -> Option<u16> {
    draws_a_panel(clipped.as_size()).then_some(clipped.y)
}

#[test]
fn each_panel_draws_the_intersection_of_the_area_and_the_buffer() {
    let theme = Theme::default();
    let frames = two_series();
    let mut failures = Vec::new();
    for kind in KINDS {
        for (state, data) in states(&frames) {
            for case in &CASES {
                let name = format!("{kind:?} with {state}");
                let failure = check(&name, case, top_row_of_a_panel, |area, buf| {
                    draw(kind, data, &theme, area, buf);
                });
                failures.extend(failure);
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} draws break the rule of the area:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn the_row_panel_draws_the_intersection_of_the_area_and_the_buffer() {
    let theme = Theme::default();
    let mut failures = Vec::new();
    for collapsed in [false, true] {
        for case in &CASES {
            let name = format!("RowPanel with collapsed {collapsed}");
            let title_row = |clipped: Rect| (!clipped.is_empty()).then_some(clipped.y);
            let failure = check(&name, case, title_row, |area, buf| {
                RowPanel::new(TITLE, &theme)
                    .collapsed(collapsed)
                    .hidden(3)
                    .render(area, buf);
            });
            failures.extend(failure);
        }
    }
    assert!(
        failures.is_empty(),
        "{} draws break the rule of the area:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
