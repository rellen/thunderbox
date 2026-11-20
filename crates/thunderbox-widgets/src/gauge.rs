//! The panel that draws one value against a range, as a meter.
//!
//! The panel shows how full a thing is. The range therefore matters as much as
//! the value, and the panel draws both ends of the range below the bar. The
//! panel also draws few meters, and it draws each meter large.
//!
//! The bar gauge panel is the other panel that draws a bar. That panel draws
//! one thin bar for each series, and it serves many series.
//!
//! # The range
//!
//! A dashboard should set `min` and `max` for a gauge panel. The two bounds
//! fix the range of the meter. A reader can then compare one refresh against
//! the next refresh.
//!
//! The panel reads the range of the result when the dashboard states neither
//! bound, and when the data source states neither bound. It adds zero to that
//! range. A panel that holds one series alone then draws a full bar, because
//! the value is also the largest value in the result.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Alignment, Rect};
use ratatui_core::style::{Color, Style};
use ratatui_core::symbols::{block, shade};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;
use ratatui_widgets::gauge::Gauge;

use crate::panel::{
    NO_DATA, Reduced, clip, dropped_text, frames_or_message, panel_block, reduce_frames,
    render_message, spread,
};
use crate::{PanelData, PanelOptions, Theme, draws_a_panel};

/// The smallest width of a meter that the panel draws side by side with
/// another meter.
const MIN_COLUMN_WIDTH: usize = 12;

/// The largest number of meters that the panel draws side by side.
const MAX_COLUMNS: usize = 4;

/// A panel that draws one value against a range, as a meter.
///
/// The panel takes frames. It does not take a data source. A program that
/// already holds its own data can therefore draw this panel, and needs no part
/// of thunderbox that runs a query.
///
/// The panel reduces each series to one number. It then draws one meter for
/// each series, from the top of the panel. It draws the meters side by side
/// when the area is much wider than it is tall, and when the series are few.
pub struct GaugePanel<'a> {
    title: &'a str,
    data: PanelData<'a>,
    theme: &'a Theme,
    options: PanelOptions<'a>,
    focused: bool,
    show_range: bool,
}

impl<'a> GaugePanel<'a> {
    /// Returns a panel that uses the default settings.
    pub fn new(title: &'a str, data: PanelData<'a>, theme: &'a Theme) -> Self {
        Self {
            title,
            data,
            theme,
            options: PanelOptions::default(),
            focused: false,
            show_range: true,
        }
    }

    /// Sets the unit, the number of decimal places, the range, and the
    /// thresholds.
    pub fn options(mut self, options: PanelOptions<'a>) -> Self {
        self.options = options;
        self
    }

    /// States whether a person has selected this panel. The border changes
    /// colour when a person selects a panel.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// States whether a meter shows the two ends of its range below the bar.
    ///
    /// The default value is `true`. Every meter of the panel drops that row
    /// when the shortest meter cannot hold it. One meter leaves the row empty
    /// when the two numbers do not fit in the width of that meter.
    pub fn show_range(mut self, show: bool) -> Self {
        self.show_range = show;
        self
    }

    /// Draws one meter inside `area`.
    ///
    /// The `index` argument is the place of the series in the panel, and it
    /// selects the colour of the bar. The `shape` argument states which rows
    /// every meter of the panel holds. The `data` argument holds the smallest
    /// and the largest value across every meter that the panel draws.
    fn draw_meter(
        &self,
        item: &Reduced<'_>,
        index: usize,
        shape: Shape,
        data: Option<(f64, f64)>,
        area: Rect,
        buf: &mut Buffer,
    ) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let width = usize::from(area.width);
        let (min, max) = self.options.bounds_for(item.config, data);
        let mut rest = area.height;
        let mut top = area.y;

        // The name comes first, and the bar keeps at least one row.
        if shape.named && rest >= 2 {
            let name = clip(&item.name, width);
            Line::from(Span::styled(name, self.theme.muted)).render(
                Rect {
                    y: top,
                    height: 1,
                    ..area
                },
                buf,
            );
            top += 1;
            rest -= 1;
        }

        // The range comes last, and the bar again keeps at least one row. The
        // panel holds the row even when the two numbers do not fit in it. Every
        // bar of the panel then keeps the same height.
        let ranged = shape.ranged && rest >= 2;
        if ranged {
            rest -= 1;
        }

        // The bar takes every row that is left, so that it grows with the
        // panel.
        let bar = Rect {
            y: top,
            height: rest,
            ..area
        };
        // The track is the light shade that marks the whole range of the
        // meter. The panel draws the track first, and the gauge then draws the
        // bar over it.
        for position in bar.positions() {
            buf[position].set_symbol(shade::LIGHT);
        }
        let fallback = self.theme.series_color(index);
        let color = self
            .options
            .color_for(item.value, item.config, self.theme, fallback);
        let text = self.options.text(item.value, item.config);
        let label = label_text(&text, bar.width);
        let ratio = ratio_of(item.value, min, max);
        let no_label = label.is_empty();
        Gauge::default()
            .ratio(ratio)
            .label(Span::raw(label))
            .gauge_style(Style::default().fg(color))
            .render(bar, buf);
        if no_label {
            fill_the_label_cell(bar, ratio, color, buf);
        }

        if ranged {
            let low = self.options.text(min, item.config);
            let high = self.options.text(max, item.config);
            if let Some(text) = range_row(&low, &high, width) {
                Line::from(Span::styled(text, self.theme.muted)).render(
                    Rect {
                        y: bar.bottom(),
                        height: 1,
                        ..area
                    },
                    buf,
                );
            }
        }
    }
}

/// The rows that every meter of one panel holds.
///
/// The panel takes this decision once, and every meter follows it. Two meters
/// can hold a different number of rows, because the meters divide the height.
/// A decision for each meter alone would draw a name above one bar, and no
/// name above the next bar. A reader would then not know which name belongs to
/// which bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Shape {
    /// Each meter holds the name of its series above the bar.
    named: bool,
    /// Each meter holds the two ends of its range below the bar.
    ranged: bool,
}

impl Shape {
    /// Returns the rows that every meter holds.
    ///
    /// The `smallest` argument is the height of the shortest meter. The
    /// `several` argument states whether the panel draws more than one meter.
    /// The `show_range` argument states whether the caller asked for the two
    /// ends of the range.
    fn of(smallest: u16, several: bool, show_range: bool) -> Self {
        let named = several && smallest >= 2;
        let left = smallest.saturating_sub(u16::from(named));
        Self {
            named,
            ranged: show_range && left >= 2,
        }
    }
}

/// Returns the place of a value inside a range, from 0.0 to 1.0.
///
/// The method [`Gauge::ratio`] stops the program for an argument outside those
/// two bounds. This function therefore returns 0.0 for a value that is not
/// finite, and for a range that has no width. It also clamps the result.
fn ratio_of(value: f64, min: f64, max: f64) -> f64 {
    let span = max - min;
    if !value.is_finite() || !span.is_finite() || span <= 0.0 {
        return 0.0;
    }
    let place = (value - min) / span;
    // The division cannot return a value that is not a number here. The
    // function still checks, because one such value would stop the program.
    if place.is_nan() {
        0.0
    } else {
        place.clamp(0.0, 1.0)
    }
}

/// Returns the label of a bar, or an empty text.
///
/// The function returns an empty text when the whole number does not fit in the
/// bar. A number that lost its last digits would mislead a reader, so the panel
/// draws no number at all.
fn label_text(text: &str, width: u16) -> String {
    if text.chars().count() <= usize::from(width) {
        text.to_string()
    } else {
        String::new()
    }
}

/// Returns the number of cells of a bar that the gauge fills.
///
/// The function repeats the arithmetic of [`Gauge`], which rounds the length of
/// a bar to the nearest cell.
fn filled_cells(width: u16, ratio: f64) -> u16 {
    let filled = f64::from(width) * ratio.clamp(0.0, 1.0);
    // The product is at most the width of the bar, so the number loses no digit
    // here. A value that is not a number becomes zero.
    filled.round() as u16
}

/// Draws the one cell that a label of no width leaves empty.
///
/// The gauge keeps the middle cell of a bar for the label, and it keeps that
/// cell even for a label that holds no character. A bar that carries no number
/// then shows a hole. This function draws the cell again, when the bar reaches
/// it.
fn fill_the_label_cell(bar: Rect, ratio: f64, color: Color, buf: &mut Buffer) {
    let x = bar.x.saturating_add(bar.width / 2);
    let y = bar.y.saturating_add(bar.height / 2);
    if x >= bar.x.saturating_add(filled_cells(bar.width, ratio)) {
        return;
    }
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol(block::FULL)
            .set_fg(color)
            .set_bg(Color::Reset);
    }
}

/// Returns one row that holds the two ends of the range.
///
/// The row holds the smallest value at the left, and the largest value at the
/// right. The function returns `None` when the two numbers and one space
/// between them do not fit in `width`.
fn range_row(low: &str, high: &str, width: usize) -> Option<String> {
    let needed = low.chars().count() + high.chars().count();
    let gap = width.checked_sub(needed)?;
    if gap == 0 {
        return None;
    }
    Some(format!("{low}{}{high}", " ".repeat(gap)))
}

/// Returns the width of the shortest text that states the dropped series.
fn note_width(dropped: usize) -> u16 {
    // The text holds a plus and the digits of a count, so it stays short.
    u16::try_from(format!("+{dropped}").chars().count()).unwrap_or(u16::MAX)
}

/// Returns the start and the size of one part of a length.
///
/// The function divides `total` into `parts` of nearly equal size. It gives one
/// cell more to each of the first parts, so that the parts fill the whole
/// length. It returns a size of zero when `parts` is zero, and when `index`
/// names no part.
fn share(total: u16, parts: usize, index: usize) -> (u16, u16) {
    if parts == 0 || index >= parts {
        return (0, 0);
    }
    let total = usize::from(total);
    let base = total / parts;
    let extra = total % parts;
    let start = base * index + extra.min(index);
    let size = base + usize::from(index < extra);
    // Both numbers are at most `total`, so neither one loses a digit here.
    (start as u16, size as u16)
}

/// Returns `true` when the panel draws the meters side by side.
///
/// The panel stacks the meters in most areas. It draws them side by side when
/// the series are few, and when the area is much wider than it is tall. Each
/// meter then keeps the whole height of the panel.
fn side_by_side(inner: Rect, count: usize) -> bool {
    let width = usize::from(inner.width);
    let height = usize::from(inner.height);
    (2..=MAX_COLUMNS).contains(&count) && width >= MIN_COLUMN_WIDTH * count && width >= height * 3
}

/// The place of each meter, and the number of series that the panel drops.
struct Plan {
    /// The rectangle of each meter that the panel draws.
    meters: Vec<Rect>,
    /// The place of the text that states the number of dropped series.
    note: Option<Rect>,
    /// The number of series that the panel does not draw.
    dropped: usize,
}

/// Returns one rectangle for each meter.
///
/// The function puts the rectangles one below the other, inside `inner`.
fn stack(inner: Rect, height: u16, count: usize) -> Vec<Rect> {
    (0..count)
        .map(|index| {
            let (start, size) = share(height, count, index);
            Rect {
                y: inner.y + start,
                height: size,
                ..inner
            }
        })
        .collect()
}

/// Returns the place of each meter inside `inner`.
///
/// The function gives every meter an equal share of the area. It puts the
/// meters side by side in a wide area, and it stacks them in every other area.
/// It keeps the meters that fit at one row each, and it drops the other series.
/// It also returns the place where the panel states the number of the dropped
/// series. The function keeps a place for that number every time it drops a
/// series.
fn plan(inner: Rect, count: usize) -> Plan {
    if count == 0 || inner.width == 0 || inner.height == 0 {
        return Plan {
            meters: Vec::new(),
            note: None,
            dropped: count,
        };
    }
    if side_by_side(inner, count) {
        let meters = (0..count)
            .map(|index| {
                let (start, size) = share(inner.width, count, index);
                // A gap of one cell keeps two bars apart.
                let width = if index + 1 < count {
                    size.saturating_sub(1)
                } else {
                    size
                };
                Rect {
                    x: inner.x + start,
                    width,
                    ..inner
                }
            })
            .collect();
        return Plan {
            meters,
            note: None,
            dropped: 0,
        };
    }
    let rows = usize::from(inner.height);
    if count <= rows {
        return Plan {
            meters: stack(inner, inner.height, count),
            note: None,
            dropped: 0,
        };
    }
    if rows >= 2 {
        // The last row states the number of series that the panel drops.
        let shown = rows - 1;
        let height = inner.height.saturating_sub(1);
        return Plan {
            meters: stack(inner, height, shown),
            note: Some(Rect {
                y: inner.y + height,
                height: 1,
                ..inner
            }),
            dropped: count - shown,
        };
    }
    // The area holds one row only. The meter and the note share that row.
    let dropped = count - 1;
    let note = note_width(dropped);
    if inner.width <= note {
        // The note fills the whole row. The panel draws no meter here, because
        // it must always state the number of the series that it drops.
        return Plan {
            meters: Vec::new(),
            note: Some(inner),
            dropped: count,
        };
    }
    // The note takes its own width at the right of the row. One cell keeps the
    // note apart from the bar, when the row holds that cell.
    let gap = u16::from(inner.width > note.saturating_add(1));
    Plan {
        meters: vec![Rect {
            width: inner.width.saturating_sub(note).saturating_sub(gap),
            ..inner
        }],
        note: Some(Rect {
            x: inner.right().saturating_sub(note),
            width: note,
            ..inner
        }),
        dropped,
    }
}

impl Widget for GaugePanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self).render(area, buf);
    }
}

impl Widget for &GaugePanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // The panel draws inside the buffer only. A cell outside the buffer
        // would stop the program.
        let area = area.intersection(buf.area);
        if !draws_a_panel(area.as_size()) {
            return;
        }

        let block = panel_block(self.title, self.theme, self.focused);
        let frames = match frames_or_message(self.data, self.theme) {
            Ok(frames) => frames,
            Err((message, style)) => return render_message(block, message, style, area, buf),
        };

        let mut series = reduce_frames(frames, self.options.reduce);
        // The reduce step drops each value that is not finite. The sum of two
        // large numbers is still not finite, so the panel checks again. It
        // drops that series, because a bar of no length would read as a zero.
        series.retain(|item| item.value.is_finite());
        if series.is_empty() {
            return render_message(block, NO_DATA, self.theme.muted, area, buf);
        }

        let inner = block.inner(area);
        block.render(area, buf);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let plan = plan(inner, series.len());
        let shown = series.get(..plan.meters.len()).unwrap_or(&series);
        // The range covers every series, and not only the series that the
        // panel draws. A series that the panel drops would otherwise change
        // the length of every bar.
        let data = spread(&series);
        let smallest = plan.meters.iter().map(|m| m.height).min().unwrap_or(0);
        let shape = Shape::of(smallest, shown.len() > 1, self.show_range);
        for (index, (item, meter)) in shown.iter().zip(&plan.meters).enumerate() {
            self.draw_meter(item, index, shape, data, *meter, buf);
        }
        if let Some(note) = plan.note {
            let text = dropped_text(plan.dropped, usize::from(note.width));
            Line::from(Span::styled(text, self.theme.muted))
                .alignment(Alignment::Right)
                .render(note, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::style::Color;
    use thunderbox_core::{Field, Frame, Reduce, Threshold};

    /// The symbol of one cell that the bar fills.
    const FULL: char = '█';

    /// The symbol of one cell of the track that carries the bar.
    const TRACK: char = '░';

    fn frames(series: &[(&str, f64)]) -> Vec<Frame> {
        let fields = series
            .iter()
            .map(|(name, value)| Field::f64(*name, vec![Some(*value)]))
            .collect();
        vec![Frame::named("host", fields)]
    }

    fn percent() -> PanelOptions<'static> {
        PanelOptions {
            unit: Some("percent"),
            min: Some(0.0),
            max: Some(100.0),
            ..Default::default()
        }
    }

    /// Draws the panel into a plain buffer. This test needs no terminal
    /// backend and no terminal. That fact is the promise of this crate.
    fn render(panel: &GaugePanel<'_>, w: u16, h: u16) -> Buffer {
        let area = Rect::new(0, 0, w, h);
        let mut buf = Buffer::empty(area);
        panel.render(area, &mut buf);
        buf
    }

    fn text(buf: &Buffer) -> String {
        (0..buf.area.height)
            .map(|y| row(buf, y))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn row(buf: &Buffer, y: u16) -> String {
        (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect()
    }

    #[test]
    fn the_panel_shows_that_the_data_source_has_not_answered() {
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Loading, &theme);
        assert!(text(&render(&p, 30, 8)).contains("loading"));
    }

    #[test]
    fn the_panel_shows_the_text_of_an_error() {
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Error("upstream refused"), &theme);
        let out = text(&render(&p, 40, 8));
        assert!(
            out.contains("upstream refused"),
            "The panel drew this. {out}"
        );
        assert!(out.contains("Load"), "The panel must still draw the title.");
    }

    #[test]
    fn the_panel_reports_that_it_has_no_data() {
        let theme = Theme::default();
        let empty: Vec<Frame> = vec![];
        let p = GaugePanel::new("Load", PanelData::Frames(&empty), &theme);
        assert!(text(&render(&p, 30, 8)).contains("no data"));
    }

    #[test]
    fn the_panel_reports_no_data_for_a_column_of_holes() {
        let f = vec![Frame::named(
            "host",
            vec![Field::f64("usage", vec![None, None])],
        )];
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme);
        assert!(text(&render(&p, 30, 8)).contains("no data"));
    }

    #[test]
    fn the_panel_reports_no_data_for_a_value_that_is_not_finite() {
        // The sum of two large numbers is not finite. The panel must drop that
        // series, and it must not draw a bar of no length.
        let f = vec![Frame::named(
            "host",
            vec![Field::f64("usage", vec![Some(f64::MAX), Some(f64::MAX)])],
        )];
        let theme = Theme::default();
        let options = PanelOptions {
            reduce: Reduce::Sum,
            ..percent()
        };
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme).options(options);
        let out = text(&render(&p, 30, 8));
        assert!(out.contains("no data"), "The panel drew this. {out}");
    }

    #[test]
    fn the_panel_draws_a_bar_and_the_text_of_the_value() {
        let f = frames(&[("usage", 75.0)]);
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme).options(percent());
        let buf = render(&p, 40, 8);
        let out = text(&buf);
        assert!(out.contains(FULL), "The panel must draw a bar. {out}");
        assert!(out.contains(TRACK), "The panel must draw a track. {out}");
        assert!(
            row(&buf, 3).contains("75%"),
            "The bar must carry the value. {out}"
        );
    }

    #[test]
    fn the_panel_draws_the_two_ends_of_the_range() {
        let f = frames(&[("usage", 75.0)]);
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme).options(percent());
        let buf = render(&p, 40, 8);
        let line = row(&buf, 6);
        let body = line.trim_matches(|c| c == '│' || c == ' ');
        assert!(body.starts_with("0%"), "The row drew this. {line}");
        assert!(body.ends_with("100%"), "The row drew this. {line}");
    }

    #[test]
    fn a_caller_can_hide_the_two_ends_of_the_range() {
        let f = frames(&[("usage", 75.0)]);
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme)
            .options(percent())
            .show_range(false);
        let buf = render(&p, 40, 8);
        let line = row(&buf, 6);
        assert!(!line.contains("100%"), "The row drew this. {line}");
        assert!(
            line.contains(TRACK) || line.contains(FULL),
            "The bar must take the row. {line}"
        );
    }

    #[test]
    fn the_panel_draws_one_meter_for_each_series_with_its_name() {
        let f = frames(&[("aa", 10.0), ("bb", 20.0), ("cc", 30.0)]);
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme).options(percent());
        let out = text(&render(&p, 20, 9));
        for part in ["aa", "bb", "cc", "10%", "20%", "30%"] {
            assert!(out.contains(part), "The panel must draw {part}. {out}");
        }
    }

    #[test]
    fn the_panel_draws_no_name_for_one_series() {
        let f = frames(&[("usage", 50.0)]);
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme).options(percent());
        let out = text(&render(&p, 30, 8));
        assert!(!out.contains("usage"), "One meter needs no name. {out}");
    }

    #[test]
    fn the_panel_cuts_a_name_that_is_too_long() {
        let f = frames(&[("a_very_long_series_name", 10.0), ("bb", 20.0)]);
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme).options(percent());
        let out = text(&render(&p, 12, 8));
        assert!(
            out.contains("a_very_lo."),
            "The panel must cut the name. {out}"
        );
    }

    #[test]
    fn the_panel_states_the_number_of_series_that_it_dropped() {
        let f = frames(&[
            ("a", 1.0),
            ("b", 2.0),
            ("c", 3.0),
            ("d", 4.0),
            ("e", 5.0),
            ("f", 6.0),
        ]);
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme).options(percent());
        let out = text(&render(&p, 20, 5));
        // A panel of 20 cells holds 18 cells inside its border, and the whole
        // sentence needs 20. The panel therefore draws the short form.
        assert!(out.contains("+4"), "The panel drew this. {out}");
    }

    #[test]
    fn the_panel_states_the_dropped_series_beside_a_meter_in_one_row() {
        let f = frames(&[("a", 1.0), ("b", 2.0), ("c", 3.0)]);
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme).options(percent());
        let buf = render(&p, 20, 3);
        let line = row(&buf, 1);
        assert!(line.contains("+2"), "The row drew this. {line}");
        assert!(line.contains(TRACK), "The meter must keep the row. {line}");
    }

    #[test]
    fn the_panel_reads_the_range_of_every_series_that_the_result_holds() {
        let f = frames(&[
            ("a", 1.0),
            ("b", 2.0),
            ("c", 3.0),
            ("d", 4.0),
            ("e", 5.0),
            ("f", 6.0),
        ]);
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme);
        // The panel drops four series at this height, and it keeps every
        // series at the other height. The series `b` holds 2.0, and the range
        // of the result is 0.0 to 6.0. The bar must therefore stay short in
        // both panels.
        let short = row(&render(&p, 24, 5), 2);
        let tall = row(&render(&p, 24, 9), 3);
        assert!(
            short.contains(TRACK),
            "A value in the middle of the range must not fill the bar. {short}"
        );
        assert_eq!(
            short.matches(FULL).count(),
            tall.matches(FULL).count(),
            "The height of the panel must not change the length of a bar."
        );
    }

    #[test]
    fn the_panel_states_the_dropped_series_in_a_narrow_row() {
        let f = frames(&[("aa", 10.0), ("bb", 20.0), ("cc", 30.0)]);
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme).options(percent());
        let line = row(&render(&p, 12, 3), 1);
        assert!(
            line.contains("+2"),
            "The panel must state the number of the series that it dropped. {line}"
        );
    }

    #[test]
    fn the_panel_draws_the_meters_side_by_side_in_a_wide_area() {
        let f = frames(&[("aa", 10.0), ("bb", 20.0), ("cc", 30.0)]);
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme).options(percent());
        let buf = render(&p, 60, 8);
        let line = row(&buf, 4);
        for part in ["10%", "20%", "30%"] {
            assert!(line.contains(part), "One row holds every meter. {line}");
        }
    }

    #[test]
    fn the_panel_draws_an_empty_bar_for_a_value_below_the_smallest_bound() {
        let f = frames(&[("usage", -50.0)]);
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme).options(percent());
        let out = text(&render(&p, 40, 8));
        assert!(!out.contains(FULL), "The bar must stay empty. {out}");
        assert!(out.contains(TRACK), "The track must stay. {out}");
    }

    #[test]
    fn the_panel_draws_a_full_bar_for_a_value_above_the_largest_bound() {
        let f = frames(&[("usage", 500.0)]);
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme).options(percent());
        let buf = render(&p, 40, 8);
        let line = row(&buf, 2);
        assert!(line.contains(FULL), "The bar must fill the row. {line}");
        assert!(!line.contains(TRACK), "The track must not stay. {line}");
    }

    #[test]
    fn the_panel_draws_the_colour_of_a_threshold() {
        let f = frames(&[("usage", 95.0)]);
        let theme = Theme::default();
        let list = [Threshold {
            value: 90.0,
            color: "critical".into(),
        }];
        let options = PanelOptions {
            thresholds: &list,
            ..percent()
        };
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme).options(options);
        let buf = render(&p, 40, 8);
        let colors: Vec<Color> = buf.content.iter().map(|cell| cell.fg).collect();
        assert!(
            colors.contains(&theme.critical),
            "The bar must take the colour of the threshold."
        );
        assert!(
            !colors.contains(&theme.series_color(0)),
            "The colour of the threshold must win over the colour of the series."
        );
    }

    #[test]
    fn the_panel_draws_no_number_that_does_not_fit() {
        let f = frames(&[("usage", 1234.5678)]);
        let theme = Theme::default();
        let options = PanelOptions {
            decimals: Some(4),
            ..percent()
        };
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme).options(options);
        let buf = render(&p, 8, 3);
        let line = row(&buf, 1);
        assert!(
            !line.chars().any(|c| c.is_ascii_digit()),
            "A number that lost its last digits would mislead a reader. {line}"
        );
        let bar = line.trim_matches('\u{2502}');
        assert!(
            bar.chars().all(|c| c == FULL),
            "A full bar that carries no number must hold no hole. {line}"
        );
        // The gauge keeps the middle cell of the bar for the label, and that
        // cell stands at x = 4 here. A terminal that shows no colour must draw
        // a full bar.
        let middle = &buf[(4, 1)];
        assert_eq!(middle.fg, theme.series_color(0));
        assert_eq!(middle.bg, Color::Reset);
    }

    #[test]
    fn the_panel_draws_nothing_in_a_small_area() {
        let f = frames(&[("usage", 1.0)]);
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme);
        for (w, h) in [(0, 0), (1, 1), (2, 2), (2, 20), (20, 2)] {
            let buf = render(&p, w, h);
            assert!(!text(&buf).contains(FULL), "The panel must draw nothing.");
        }
    }

    #[test]
    fn the_panel_draws_every_area_size_without_a_panic() {
        let theme = Theme::default();
        let many: Vec<(&str, f64)> = vec![
            ("a", -50.0),
            ("b", 0.0),
            ("c", 12.5),
            ("d", 99.0),
            ("e", 500.0),
            ("f", 1e12),
            ("g", 3.0),
        ];
        let empty: Vec<Frame> = vec![];
        let one = frames(&many[..1]);
        let three = frames(&many[..3]);
        let seven = frames(&many);
        let sets = [
            PanelData::Loading,
            PanelData::Error("upstream refused"),
            PanelData::Frames(&empty),
            PanelData::Frames(&one),
            PanelData::Frames(&three),
            PanelData::Frames(&seven),
        ];
        let choices = [PanelOptions::default(), percent()];
        for data in sets {
            for options in choices {
                for range in [true, false] {
                    let p = GaugePanel::new("Load", data, &theme)
                        .options(options)
                        .show_range(range);
                    for (w, h) in [
                        (0, 0),
                        (1, 1),
                        (3, 3),
                        (4, 3),
                        (8, 4),
                        (20, 3),
                        (40, 12),
                        (120, 40),
                        (3, 40),
                        (200, 3),
                    ] {
                        let _ = render(&p, w, h);
                    }
                }
            }
        }
    }

    #[test]
    fn the_panel_draws_inside_a_buffer_that_is_smaller_than_the_area() {
        let f = frames(&[("usage", 50.0)]);
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme).options(percent());
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 5));
        p.render(Rect::new(0, 0, 40, 20), &mut buf);
        assert!(text(&buf).contains(TRACK), "The panel must draw a meter.");
    }

    #[test]
    fn the_panel_draws_the_name_of_every_meter_or_of_none() {
        let f = frames(&[("aa", 10.0), ("bb", 20.0), ("cc", 30.0)]);
        let theme = Theme::default();
        let p = GaugePanel::new("Load", PanelData::Frames(&f), &theme).options(percent());
        for (w, h) in [(20, 5), (20, 9), (30, 12), (60, 8), (40, 6), (12, 20)] {
            let out = text(&render(&p, w, h));
            let names = ["aa", "bb", "cc"]
                .iter()
                .filter(|name| out.contains(*name))
                .count();
            assert!(
                names == 0 || names == 3,
                "A name above one bar and no name above the next bar would \
                 confuse a reader. {w} {h} {out}"
            );
        }
    }

    #[test]
    fn shape_of_drops_the_name_and_then_the_range_as_the_meter_shrinks() {
        assert_eq!(
            Shape::of(4, true, true),
            Shape {
                named: true,
                ranged: true
            }
        );
        assert_eq!(
            Shape::of(2, true, true),
            Shape {
                named: true,
                ranged: false
            }
        );
        assert_eq!(
            Shape::of(1, true, true),
            Shape {
                named: false,
                ranged: false
            }
        );
        assert_eq!(
            Shape::of(0, true, true),
            Shape {
                named: false,
                ranged: false
            }
        );
    }

    #[test]
    fn shape_of_drops_the_name_of_one_meter_that_stands_alone() {
        let out = Shape::of(9, false, true);
        assert!(!out.named, "One meter needs no name.");
        assert!(out.ranged, "The meter still shows its range.");
        assert!(
            !Shape::of(9, true, false).ranged,
            "The caller hid the range."
        );
    }

    #[test]
    fn ratio_of_returns_the_place_of_a_value_inside_the_range() {
        assert_eq!(ratio_of(50.0, 0.0, 100.0), 0.5);
        assert_eq!(ratio_of(0.0, 0.0, 100.0), 0.0);
        assert_eq!(ratio_of(100.0, 0.0, 100.0), 1.0);
        assert_eq!(ratio_of(-5.0, -10.0, 10.0), 0.25);
    }

    #[test]
    fn ratio_of_clamps_a_value_outside_the_range() {
        assert_eq!(ratio_of(-50.0, 0.0, 100.0), 0.0);
        assert_eq!(ratio_of(500.0, 0.0, 100.0), 1.0);
    }

    #[test]
    fn ratio_of_returns_zero_for_a_value_that_is_not_finite() {
        assert_eq!(ratio_of(f64::NAN, 0.0, 100.0), 0.0);
        assert_eq!(ratio_of(f64::INFINITY, 0.0, 100.0), 0.0);
        assert_eq!(ratio_of(f64::NEG_INFINITY, 0.0, 100.0), 0.0);
    }

    #[test]
    fn ratio_of_returns_zero_for_a_range_that_it_cannot_use() {
        assert_eq!(ratio_of(5.0, 7.0, 7.0), 0.0);
        assert_eq!(ratio_of(5.0, 9.0, 1.0), 0.0);
        assert_eq!(ratio_of(5.0, f64::NAN, 1.0), 0.0);
        assert_eq!(ratio_of(5.0, 0.0, f64::INFINITY), 0.0);
    }

    #[test]
    fn ratio_of_stays_inside_the_bounds_that_the_gauge_asks_for() {
        let numbers = [-1e308, -1.0, 0.0, 1e-9, 7.5, 1e308, f64::MAX, f64::MIN];
        for value in numbers {
            for min in numbers {
                for max in numbers {
                    let r = ratio_of(value, min, max);
                    assert!((0.0..=1.0).contains(&r), "{value} {min} {max} {r}");
                }
            }
        }
    }

    #[test]
    fn filled_cells_rounds_the_length_of_a_bar_to_the_nearest_cell() {
        assert_eq!(filled_cells(10, 0.0), 0);
        assert_eq!(filled_cells(10, 0.5), 5);
        assert_eq!(filled_cells(10, 1.0), 10);
        assert_eq!(filled_cells(7, 0.5), 4);
    }

    #[test]
    fn filled_cells_returns_no_cell_for_a_ratio_that_it_cannot_use() {
        assert_eq!(filled_cells(10, f64::NAN), 0);
        assert_eq!(filled_cells(10, -1.0), 0);
        assert_eq!(filled_cells(10, 5.0), 10);
        assert_eq!(filled_cells(0, 1.0), 0);
    }

    #[test]
    fn label_text_returns_a_number_that_fits() {
        assert_eq!(label_text("75%", 3), "75%");
        assert_eq!(label_text("75%", 9), "75%");
    }

    #[test]
    fn label_text_returns_an_empty_text_for_a_number_that_does_not_fit() {
        assert_eq!(label_text("1234.5%", 4), "");
        assert_eq!(label_text("7", 0), "");
    }

    #[test]
    fn range_row_holds_the_two_ends_and_fills_the_width() {
        let out = range_row("0%", "100%", 10).expect("The two numbers fit.");
        assert_eq!(out, "0%    100%");
        assert_eq!(out.chars().count(), 10);
    }

    #[test]
    fn range_row_returns_none_for_a_width_that_is_too_small() {
        assert_eq!(range_row("0%", "100%", 6), None);
        assert_eq!(range_row("0%", "100%", 5), None);
        assert_eq!(range_row("0%", "100%", 0), None);
    }

    #[test]
    fn dropped_text_states_the_count() {
        assert_eq!(dropped_text(4, 20), "4 series do not fit.");
        assert_eq!(dropped_text(4, 19), "+4");
    }

    #[test]
    fn dropped_text_returns_a_short_form_for_a_small_width() {
        assert_eq!(dropped_text(4, 6), "+4");
        assert_eq!(dropped_text(12, 2), "+.");
        assert_eq!(dropped_text(4, 0), "");
    }

    #[test]
    fn note_width_returns_the_width_of_the_short_form() {
        assert_eq!(note_width(4), 2);
        assert_eq!(note_width(120), 4);
    }

    #[test]
    fn share_divides_a_length_into_equal_parts() {
        assert_eq!(share(9, 3, 0), (0, 3));
        assert_eq!(share(9, 3, 1), (3, 3));
        assert_eq!(share(9, 3, 2), (6, 3));
    }

    #[test]
    fn share_gives_the_remainder_to_the_first_parts() {
        assert_eq!(share(10, 3, 0), (0, 4));
        assert_eq!(share(10, 3, 1), (4, 3));
        assert_eq!(share(10, 3, 2), (7, 3));
    }

    #[test]
    fn share_fills_the_whole_length_and_leaves_no_gap() {
        for total in [0_u16, 1, 2, 5, 13, 64, 1000] {
            for parts in 1..=9_usize {
                let mut next = 0;
                for index in 0..parts {
                    let (start, size) = share(total, parts, index);
                    assert_eq!(start, next, "{total} {parts} {index}");
                    next = start + size;
                }
                assert_eq!(next, total, "The parts must fill {total}.");
            }
        }
    }

    #[test]
    fn share_returns_no_size_for_a_part_that_does_not_exist() {
        assert_eq!(share(10, 0, 0), (0, 0));
        assert_eq!(share(10, 2, 5), (0, 0));
    }

    #[test]
    fn side_by_side_returns_true_for_few_meters_in_a_wide_area() {
        assert!(side_by_side(Rect::new(0, 0, 58, 6), 3));
        assert!(side_by_side(Rect::new(0, 0, 24, 8), 2));
    }

    #[test]
    fn side_by_side_returns_false_for_every_other_area() {
        assert!(
            !side_by_side(Rect::new(0, 0, 58, 6), 1),
            "The panel stacks a single series."
        );
        assert!(
            !side_by_side(Rect::new(0, 0, 58, 6), 5),
            "The panel stacks many series."
        );
        assert!(
            !side_by_side(Rect::new(0, 0, 20, 6), 3),
            "The area is too narrow for three meters."
        );
        assert!(
            !side_by_side(Rect::new(0, 0, 58, 30), 3),
            "The area is too tall for three meters."
        );
    }

    /// Returns the fault in a plan, or `None` when the plan holds no fault.
    fn fault(inner: Rect, count: usize) -> Option<String> {
        let plan = plan(inner, count);
        if plan.meters.len() + plan.dropped != count {
            return Some(format!("The plan lost a series. {count}"));
        }
        if plan.dropped > 0 && plan.note.is_none() {
            return Some(format!(
                "The plan gives the dropped series no note. {count}"
            ));
        }
        let mut places: Vec<Rect> = plan.meters.clone();
        places.extend(plan.note);
        for (i, a) in places.iter().enumerate() {
            if a.union(inner) != inner {
                return Some(format!("A part left the area. {a:?} {inner:?}"));
            }
            for b in places.iter().skip(i + 1) {
                if a.intersects(*b) {
                    return Some(format!("Two parts cover one cell. {a:?} {b:?}"));
                }
            }
        }
        None
    }

    #[test]
    fn plan_keeps_every_part_inside_the_area_and_apart() {
        for width in [1_u16, 2, 6, 12, 18, 40, 58, 120] {
            for height in [1_u16, 2, 3, 4, 8, 38] {
                for count in [1_usize, 2, 3, 4, 7, 40] {
                    let inner = Rect::new(1, 1, width, height);
                    assert_eq!(fault(inner, count), None, "{width} {height} {count}");
                }
            }
        }
    }

    #[test]
    fn plan_states_the_number_of_series_that_it_drops() {
        let out = plan(Rect::new(1, 1, 18, 3), 6);
        assert_eq!(out.meters.len(), 2);
        assert_eq!(out.dropped, 4);
        assert_eq!(out.note.map(|r| r.height), Some(1));
    }

    #[test]
    fn plan_keeps_every_series_that_fits() {
        let out = plan(Rect::new(1, 1, 18, 6), 6);
        assert_eq!(out.meters.len(), 6);
        assert_eq!(out.dropped, 0);
        assert_eq!(out.note, None);
    }

    #[test]
    fn plan_keeps_a_place_for_the_note_in_a_narrow_row() {
        let out = plan(Rect::new(1, 1, 5, 1), 3);
        assert_eq!(out.meters.len(), 1);
        assert_eq!(out.dropped, 2);
        assert_eq!(out.meters[0].width, 2);
        assert_eq!(out.note, Some(Rect::new(4, 1, 2, 1)));
    }

    #[test]
    fn plan_gives_the_whole_row_to_the_note_when_no_meter_fits() {
        let out = plan(Rect::new(1, 1, 2, 1), 3);
        assert!(out.meters.is_empty(), "No meter fits beside the note.");
        assert_eq!(out.dropped, 3);
        assert_eq!(out.note, Some(Rect::new(1, 1, 2, 1)));
    }

    #[test]
    fn plan_returns_no_meter_for_an_area_that_holds_nothing() {
        let out = plan(Rect::new(0, 0, 0, 0), 3);
        assert!(out.meters.is_empty());
        assert_eq!(out.dropped, 3);
    }
}
