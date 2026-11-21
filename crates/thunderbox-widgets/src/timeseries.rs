//! The panel that draws values against time.

use std::ops::Range;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Style;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;
use ratatui_widgets::chart::{Axis, Chart, Dataset, GraphType};
use thunderbox_core::{Frame, TimeWindow, Timestamp};

use crate::panel::{NO_DATA, frames_or_message, panel_block, render_message};
use crate::{Marker, PanelData, PanelOptions, Theme, draws_a_panel, format_from_now, format_value};

/// A panel that draws one or more values against time.
///
/// The panel takes frames. It does not take a data source. A program that
/// already holds its own data can therefore draw this panel, and needs no part
/// of thunderbox that runs a query.
pub struct TimeSeriesPanel<'a> {
    title: &'a str,
    data: PanelData<'a>,
    theme: &'a Theme,
    marker: Marker,
    options: PanelOptions<'a>,
    focused: bool,
    filled: bool,
    show_legend: bool,
    window: Option<TimeWindow>,
    label_width: Option<u16>,
}

impl<'a> TimeSeriesPanel<'a> {
    /// Returns a panel that uses the default settings.
    pub fn new(title: &'a str, data: PanelData<'a>, theme: &'a Theme) -> Self {
        Self {
            title,
            data,
            theme,
            marker: Marker::default(),
            options: PanelOptions::default(),
            focused: false,
            filled: true,
            show_legend: true,
            window: None,
            label_width: None,
        }
    }

    /// States the window that the panel draws.
    ///
    /// A window is the pair of moments that a time range names, and the moment
    /// that named them. The axis of time then covers the whole window, and
    /// each label states the distance from the present moment to that place on
    /// the axis.
    ///
    /// A caller that states no window keeps the rule that the panel followed
    /// before: the newest point of the data is the present moment.
    pub fn window(mut self, window: TimeWindow) -> Self {
        self.window = Some(window);
        self
    }

    /// Sets the glyph that the chart draws each point with.
    pub fn marker(mut self, marker: Marker) -> Self {
        self.marker = marker;
        self
    }

    /// Sets the unit and the number of decimal places.
    ///
    /// The chart reads no threshold of the options. A stat panel, a gauge, a
    /// bar gauge, a table, and a top panel read the thresholds.
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

    /// States whether the chart fills the area below each line.
    /// The default value is `true`.
    pub fn filled(mut self, filled: bool) -> Self {
        self.filled = filled;
        self
    }

    /// States whether the chart shows a legend.
    ///
    /// The chart hides the legend when the chart holds one series only, or when
    /// the panel is too narrow to hold a legend.
    pub fn show_legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }

    /// States the width of the column that holds the labels of the axis of
    /// values.
    ///
    /// The chart keeps a column of cells at its left edge for those labels. It
    /// takes the width of that column from its own widest label. Two panels
    /// that hold labels of different widths therefore draw two charts of
    /// different widths. A person who reads one panel above another then reads
    /// two axes of time that start at different cells.
    ///
    /// A caller that draws a stack of panels measures each panel with
    /// [`TimeSeriesPanel::measure_label_width`]. The caller takes the largest
    /// of the widths that the panels report, and it states that one width here
    /// for every panel of the stack. Every chart of the stack then starts at
    /// the same cell.
    ///
    /// A panel that states no width measures its own labels, as it did before.
    ///
    /// The chart gives no more than a third of its width to the labels. A panel
    /// that asks for more keeps that third. Every panel of a stack holds the
    /// same width, so every panel of the stack also keeps the same third.
    ///
    /// # Examples
    ///
    /// ```
    /// # use thunderbox_widgets::{PanelData, Theme, TimeSeriesPanel};
    /// # let (frames, theme) = (vec![], Theme::default());
    /// let top = TimeSeriesPanel::new("cpu", PanelData::Frames(&frames), &theme);
    /// let bottom = TimeSeriesPanel::new("net", PanelData::Frames(&frames), &theme);
    /// let width = top.measure_label_width().max(bottom.measure_label_width());
    /// let top = top.label_width(width);
    /// let bottom = bottom.label_width(width);
    /// ```
    pub fn label_width(mut self, width: u16) -> Self {
        self.label_width = Some(width);
        self
    }

    /// Returns the width that the labels of this panel need at the left edge.
    ///
    /// The width covers the widest label of the axis of values. It also covers
    /// the first label of the axis of time, because the chart draws that label
    /// in the same cells. The method returns 0 for a panel that draws a message
    /// in place of a chart.
    ///
    /// A caller passes the largest of these widths to
    /// [`TimeSeriesPanel::label_width`]. That method describes the whole rule.
    pub fn measure_label_width(&self) -> u16 {
        let Some((plot, now)) = self.plot() else {
            return 0;
        };
        needed_label_width(
            &plot,
            now,
            self.options.unit.or(plot.unit.as_deref()),
            self.options.decimals.or(plot.decimals),
        )
    }

    /// Returns the points to draw, and the present moment.
    ///
    /// The method returns `None` when the panel draws no chart. A panel draws
    /// no chart in each of these three conditions:
    ///
    /// - the data source has not answered;
    /// - the data source returned an error;
    /// - the result holds no value.
    ///
    /// The present moment comes from the window that the caller states. A
    /// caller that states no window says that its newest point is the present
    /// moment. A program that draws live data holds that promise.
    fn plot(&self) -> Option<(Plot, f64)> {
        let PanelData::Frames(frames) = self.data else {
            return None;
        };
        let mut plot = collect(frames)?;
        // A limit that the panel states wins over the data source, and over the
        // data. The meter panels follow the same order.
        apply_limits(&mut plot, self.options.min, self.options.max);
        let now = match self.window {
            Some(window) => {
                cover_window(&mut plot, window);
                window.now as f64
            }
            None => plot.x_bounds[1],
        };
        Some((plot, now))
    }
}

/// The points to draw, and the limits that the chart scales them against.
struct Plot {
    series: Vec<Series>,
    x_bounds: [f64; 2],
    y_bounds: [f64; 2],
    /// The unit that the first series states, if the data source stated one.
    unit: Option<String>,
    /// The number of decimal places that the first series states.
    decimals: Option<u8>,
}

/// One series of the chart.
struct Series {
    name: String,
    /// Each point of the series that holds a value, in the order of the column.
    points: Vec<(f64, f64)>,
    /// The runs of the series. Each run is a range of `points`.
    runs: Vec<Range<usize>>,
}

/// Reads the frames and returns the points to draw.
///
/// A hole is a value of `None`, or a value that is not finite. The function
/// removes each hole, and it does not replace a hole with a zero. It cuts each
/// series at its holes. A run is the stretch of points between two holes. The
/// panel draws each run as a line of its own, so the line stops at each hole.
/// A reader therefore sees no line across a hole, and no fall to the bottom of
/// the chart.
///
/// A gap in time between two values is not a hole. The line joins those two
/// values. ADR 0069 in `docs/contributor/adr` states the reason.
///
/// The function returns `None` if the frames hold no value to draw.
fn collect(frames: &[Frame]) -> Option<Plot> {
    let mut series = Vec::new();
    let mut x: Option<(f64, f64)> = None;
    let mut y: Option<(f64, f64)> = None;
    let mut unit: Option<String> = None;
    let mut decimals: Option<u8> = None;
    // A data source can state the limits of a value. A share of use runs from
    // 0 to 100, whatever the data holds today.
    let mut declared_min: Option<f64> = None;
    let mut declared_max: Option<f64> = None;

    for frame in frames {
        // The chart skips a frame with no time column, and a frame with no column
        // of numbers. It also skips a frame where a column of numbers and the time
        // column hold different numbers of rows. The runtime of a dashboard already
        // returned an error for such a frame of two lengths. The frame therefore
        // reaches this loop only from a program that gives frames to the panel
        // directly.
        let Ok((stamps, numeric)) = frame.time_series() else {
            continue;
        };
        for field in numeric {
            if let Some(low) = field.config.min {
                declared_min = Some(declared_min.map_or(low, |m: f64| m.min(low)));
            }
            if let Some(high) = field.config.max {
                declared_max = Some(declared_max.map_or(high, |m: f64| m.max(high)));
            }
            let values = field.values.as_f64().unwrap_or(&[]);
            let mut points = Vec::with_capacity(values.len());
            let mut runs = Vec::new();
            let mut start = 0;
            for (t, v) in stamps.iter().zip(values) {
                let Some(v) = v.filter(|v| v.is_finite()) else {
                    // A hole ends the run that comes before it.
                    if points.len() > start {
                        runs.push(start..points.len());
                    }
                    start = points.len();
                    continue;
                };
                let (tf, vf) = (*t as f64, v);
                x = Some(match x {
                    None => (tf, tf),
                    Some((lo, hi)) => (lo.min(tf), hi.max(tf)),
                });
                y = Some(match y {
                    None => (vf, vf),
                    Some((lo, hi)) => (lo.min(vf), hi.max(vf)),
                });
                points.push((tf, vf));
            }
            if points.len() > start {
                runs.push(start..points.len());
            }
            if !points.is_empty() {
                if series.is_empty() {
                    unit = field.config.unit.clone();
                    decimals = field.config.decimals;
                }
                series.push(Series {
                    name: field.display_name(),
                    points,
                    runs,
                });
            }
        }
    }

    let (mut x0, mut x1) = x?;
    let (mut y0, mut y1) = y?;
    // A range that has no width would make the chart divide by zero.
    if (x1 - x0).abs() < f64::EPSILON {
        x1 = x0 + 1.0;
    }
    if (y1 - y0).abs() < f64::EPSILON {
        y0 -= 0.5;
        y1 += 0.5;
    }
    // A limit that the data source states wins over the data. An axis of a
    // share of use therefore reaches 100, and it does not stop at the largest
    // value that the machine happened to reach. The data still widens the axis
    // when a value passes the limit.
    match declared_max {
        Some(high) => y1 = y1.max(high),
        None => {
            // Add space above the largest value, so that the peak does not
            // touch the top border.
            y1 += (y1 - y0) * 0.05;
        }
    }
    match declared_min {
        Some(low) => y0 = y0.min(low),
        None => {
            if y0 > 0.0 {
                y0 = (y0 - (y1 - y0) * 0.05).max(0.0);
            }
        }
    }
    let _ = &mut x0;
    Some(Plot {
        series,
        x_bounds: [x0, x1],
        y_bounds: [y0, y1],
        unit,
        decimals,
    })
}

/// Returns the labels of the axis that holds time.
///
/// Each label states the distance from the present moment to that place on the
/// axis. The `now` argument holds the present moment, in the same unit as the
/// two ends of the axis. A label of a place in the past therefore starts with a
/// minus sign, and a label of the present moment reads "now".
///
/// The function passes the distance between two labels to
/// [`format_from_now`], which selects the unit and the precision of the text
/// from that distance. Two labels of one axis therefore differ, unless the
/// axis has no width. An axis of no width names the present moment three
/// times, so each of the three labels then reads "now".
fn x_labels(bounds: [f64; 2], now: f64, style: Style) -> Vec<Line<'static>> {
    let step = (bounds[1] - bounds[0]).max(0.0) / 2.0;
    (0..3)
        .map(|i| {
            let at = bounds[0] + step * f64::from(i);
            let offset = (at - now).round() as i64;
            Line::from(Span::styled(
                format_from_now(offset, step.round() as i64),
                style,
            ))
        })
        .collect()
}

/// Widens the axis of time so that it covers the window and every point.
///
/// The panel draws the window that a person asked for, and not the window that
/// the data happens to cover. A data source can also return a point just
/// outside the window, because a data source aligns its points to a grid of its
/// own. The chart must draw that point.
///
/// The axis of the values follows the same rule. The test
/// `a_value_past_the_limit_still_widens_the_axis` records it, so a reader meets
/// one behaviour on both axes.
fn cover_window(plot: &mut Plot, window: TimeWindow) {
    plot.x_bounds[0] = plot.x_bounds[0].min(window.from as f64);
    plot.x_bounds[1] = plot.x_bounds[1].max(window.to as f64);
}

fn y_labels(
    bounds: [f64; 2],
    unit: Option<&str>,
    decimals: Option<u8>,
    style: Style,
) -> Vec<Line<'static>> {
    (0..3)
        .map(|i| {
            let v = bounds[0] + (bounds[1] - bounds[0]) * i as f64 / 2.0;
            Line::from(Span::styled(format_value(v, unit, decimals), style))
        })
        .collect()
}

/// Returns the width that the labels need at the left edge of the chart.
///
/// The chart keeps one column of cells at the left edge, and it draws two kinds
/// of label there. The first kind is every label of the axis of values. The
/// second kind is the first label of the axis of time, which the chart draws
/// below that column. The chart puts the last character of that time label
/// below the axis of values. That label therefore needs one cell less than its
/// own width.
fn needed_label_width(plot: &Plot, now: f64, unit: Option<&str>, decimals: Option<u8>) -> u16 {
    let values = y_labels(plot.y_bounds, unit, decimals, Style::default())
        .iter()
        .map(Line::width)
        .max()
        .unwrap_or(0) as u16;
    let first_time = x_labels(plot.x_bounds, now, Style::default())
        .first()
        .map_or(0, Line::width) as u16;
    values.max(first_time.saturating_sub(1))
}

/// Returns the widest column of labels that a chart of this area accepts.
///
/// The chart gives no more than a third of its width to the labels at the left
/// edge. It cuts a label that asks for more. The border of the panel takes two
/// cells of the area, and the chart measures the space inside that border.
fn label_width_limit(area: Rect) -> u16 {
    area.width.saturating_sub(2) / 3
}

/// Adds spaces at the left of each label, until every label holds `width`.
///
/// The chart takes the width of the column of labels from the widest label. A
/// caller that pads the labels of two panels to one width therefore gives the
/// two charts one width. The function leaves a label that is wider than `width`
/// as it is.
///
/// The chart draws each label from the left of that column. Every label of one
/// panel then ends at the axis, because every label holds the same width.
fn pad_labels(labels: &mut [Line<'static>], width: u16, style: Style) {
    for label in labels {
        let pad = usize::from(width).saturating_sub(label.width());
        if pad > 0 {
            label.spans.insert(0, Span::styled(" ".repeat(pad), style));
        }
    }
}

/// Moves the ends of the axis to the limits that the panel states.
///
/// The function ignores a limit that is not finite. It also ignores a pair of
/// limits that would leave the axis with no height, because the chart would
/// then divide by zero.
fn apply_limits(plot: &mut Plot, min: Option<f64>, max: Option<f64>) {
    let low = min.filter(|v| v.is_finite()).unwrap_or(plot.y_bounds[0]);
    let high = max.filter(|v| v.is_finite()).unwrap_or(plot.y_bounds[1]);
    if high > low {
        plot.y_bounds = [low, high];
    }
}

impl Widget for TimeSeriesPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self).render(area, buf);
    }
}

impl Widget for &TimeSeriesPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // The panel draws inside the buffer only. ADR 0070 in docs/contributor/adr
        // states this rule for each panel.
        let area = area.intersection(buf.area);
        if !draws_a_panel(area.as_size()) {
            return;
        }

        let block = panel_block(self.title, self.theme, self.focused);
        if let Err((message, style)) = frames_or_message(self.data, self.theme) {
            return render_message(block, message, style, area, buf);
        }

        let Some((plot, now)) = self.plot() else {
            return render_message(block, NO_DATA, self.theme.muted, area, buf);
        };

        // ratatui joins each point of a dataset to the next point. Each run of a
        // series therefore takes a dataset of its own, and no line crosses a
        // hole. A dataset of one point still draws that point.
        let floor = plot.y_bounds[0];
        let datasets: Vec<Dataset<'_>> = plot
            .series
            .iter()
            .enumerate()
            .flat_map(|(i, s)| {
                let color = self.theme.series_color(i);
                s.runs.iter().enumerate().map(move |(r, run)| {
                    let ds = Dataset::default()
                        .data(&s.points[run.clone()])
                        .marker(self.marker.into())
                        .style(Style::default().fg(color));
                    // The legend lists each dataset that holds a name. The
                    // first run alone takes the name, so the legend lists each
                    // series once.
                    let ds = if r == 0 { ds.name(s.name.clone()) } else { ds };
                    if self.filled {
                        // Fill the area down to the bottom of the axis.
                        ds.graph_type(GraphType::Area).fill_to_y(floor)
                    } else {
                        ds.graph_type(GraphType::Line)
                    }
                })
            })
            .collect();

        let mut value_labels = y_labels(
            plot.y_bounds,
            self.options.unit.or(plot.unit.as_deref()),
            self.options.decimals.or(plot.decimals),
            self.theme.muted,
        );
        // A caller that draws a stack of panels states one width for the whole
        // stack. Every chart of the stack then starts at the same cell.
        if let Some(width) = self.label_width {
            pad_labels(
                &mut value_labels,
                width.min(label_width_limit(area)),
                self.theme.muted,
            );
        }

        let mut chart = Chart::new(datasets)
            .block(block)
            .x_axis(
                Axis::default()
                    .style(self.theme.axis)
                    .bounds(plot.x_bounds)
                    .labels(x_labels(plot.x_bounds, now, self.theme.muted)),
            )
            .y_axis(
                Axis::default()
                    .style(self.theme.axis)
                    .bounds(plot.y_bounds)
                    .labels(value_labels),
            );

        // A legend takes width from the chart. Hide the legend when the chart
        // holds one series only, or when the panel is too narrow.
        if !self.show_legend || plot.series.len() < 2 || area.width < 30 {
            chart = chart.hidden_legend_constraints((
                ratatui_core::layout::Constraint::Ratio(0, 1),
                ratatui_core::layout::Constraint::Ratio(0, 1),
            ));
        }

        chart.render(area, buf);
    }
}

/// Returns the most recent value in the frames, and its time.
///
/// The function ignores a hole in the data. It also ignores a value that is not
/// finite. It returns `None` if the frames hold no value. A panel that shows a
/// single number uses this function.
pub fn last_value(frames: &[Frame]) -> Option<(Timestamp, f64)> {
    let mut best: Option<(Timestamp, f64)> = None;
    for frame in frames {
        // The function skips a frame with no time column, and a frame with no
        // column of numbers. It also skips a frame where a column of numbers and
        // the time column hold different numbers of rows. The runtime of a
        // dashboard already returned an error for such a frame of two lengths.
        let Ok((stamps, numeric)) = frame.time_series() else {
            continue;
        };
        for field in numeric {
            let values = field.values.as_f64().unwrap_or(&[]);
            for (t, v) in stamps.iter().zip(values).rev() {
                if let Some(v) = v.filter(|v| v.is_finite()) {
                    if best.is_none_or(|(bt, _)| *t > bt) {
                        best = Some((*t, v));
                    }
                    break;
                }
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use thunderbox_core::{Field, Frame};

    fn frames(values: Vec<Option<f64>>) -> Vec<Frame> {
        let stamps: Vec<i64> = (0..values.len() as i64).map(|i| i * 1000).collect();
        vec![Frame::named(
            "cpu",
            vec![Field::time("time", stamps), Field::f64("usage", values)],
        )]
    }

    /// Draws the panel into a plain buffer. This test needs no terminal
    /// backend and no terminal. That fact is the promise of this crate.
    fn render(panel: &TimeSeriesPanel<'_>, w: u16, h: u16) -> Buffer {
        let area = Rect::new(0, 0, w, h);
        let mut buf = Buffer::empty(area);
        panel.render(area, &mut buf);
        buf
    }

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

    #[test]
    fn the_panel_shows_that_the_data_source_has_not_answered() {
        let theme = Theme::default();
        let p = TimeSeriesPanel::new("CPU", PanelData::Loading, &theme);
        assert!(text(&render(&p, 30, 8)).contains("loading"));
    }

    #[test]
    fn the_panel_shows_the_text_of_an_error() {
        let theme = Theme::default();
        let p = TimeSeriesPanel::new("CPU", PanelData::Error("upstream refused"), &theme);
        let out = text(&render(&p, 40, 8));
        assert!(
            out.contains("upstream refused"),
            "The panel drew this. {out}"
        );
        assert!(out.contains("CPU"), "The panel must still draw the title.");
    }

    #[test]
    fn the_panel_reports_that_it_has_no_data() {
        let theme = Theme::default();
        let empty: Vec<Frame> = vec![];
        let p = TimeSeriesPanel::new("CPU", PanelData::Frames(&empty), &theme);
        assert!(text(&render(&p, 30, 8)).contains("no data"));
    }

    #[test]
    fn the_panel_reports_no_data_for_a_column_of_holes() {
        let f = frames(vec![None, None, None]);
        let theme = Theme::default();
        let p = TimeSeriesPanel::new("CPU", PanelData::Frames(&f), &theme);
        assert!(text(&render(&p, 30, 8)).contains("no data"));
    }

    #[test]
    fn the_panel_draws_braille_characters_for_a_series() {
        let f = frames(vec![Some(10.0), Some(50.0), Some(20.0), Some(80.0)]);
        let theme = Theme::default();
        let p = TimeSeriesPanel::new("CPU", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 40, 10));
        assert!(
            !out.contains("no data"),
            "The panel must draw the series. {out}"
        );
        // Unicode holds the braille characters from U+2800 to U+28FF.
        assert!(
            out.chars().any(|c| ('\u{2800}'..='\u{28FF}').contains(&c)),
            "The panel must draw braille characters. {out}"
        );
    }

    /// Returns 10 points at 80, then 10 holes, then 10 points at 20.
    fn series_with_a_hole() -> Vec<Option<f64>> {
        let mut values = vec![Some(80.0); 10];
        values.extend([None; 10]);
        values.extend([Some(20.0); 10]);
        values
    }

    /// Returns the place of the corner where the two axes meet.
    fn corner(buf: &Buffer) -> (u16, u16) {
        (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
            .find(|&(x, y)| buf[(x, y)].symbol() == "└")
            .expect("The panel must draw the two axes.")
    }

    /// Returns the number of columns inside the axes that hold no mark.
    ///
    /// The columns run from the axis of values to the right border. The rows
    /// run from the top border to the axis of time.
    fn empty_columns(buf: &Buffer) -> usize {
        let (axis_x, axis_y) = corner(buf);
        let right = buf.area.width - 1;
        (axis_x + 1..right)
            .filter(|&x| (1..axis_y).all(|y| buf[(x, y)].symbol() == " "))
            .count()
    }

    /// Returns the number of cells of one row inside the axes that hold a mark.
    fn marks_in_row(buf: &Buffer, y: u16) -> usize {
        let (axis_x, _) = corner(buf);
        let right = buf.area.width - 1;
        (axis_x + 1..right)
            .filter(|&x| buf[(x, y)].symbol() != " ")
            .count()
    }

    #[test]
    fn a_line_chart_leaves_the_columns_over_a_hole_empty() {
        let f = frames(series_with_a_hole());
        let theme = Theme::default();
        let p = TimeSeriesPanel::new("CPU", PanelData::Frames(&f), &theme).filled(false);
        let buf = render(&p, 62, 12);
        let empty = empty_columns(&buf);
        assert!(
            empty >= 12,
            "The line must stop at the hole. {empty} columns are empty.\n{}",
            text(&buf)
        );
    }

    #[test]
    fn a_filled_chart_leaves_the_columns_over_a_hole_empty() {
        let f = frames(series_with_a_hole());
        let theme = Theme::default();
        let p = TimeSeriesPanel::new("CPU", PanelData::Frames(&f), &theme).filled(true);
        let buf = render(&p, 62, 12);
        let empty = empty_columns(&buf);
        assert!(
            empty >= 12,
            "The fill must stop at the hole. {empty} columns are empty.\n{}",
            text(&buf)
        );
    }

    /// The single point sits at 80, and every other point sits at 20. The
    /// point at 80 is therefore the one mark in the top row of the chart.
    #[test]
    fn a_run_of_one_point_between_two_holes_draws_a_cell() {
        let mut values = vec![Some(20.0); 5];
        values.extend([None; 5]);
        values.push(Some(80.0));
        values.extend([None; 5]);
        values.extend([Some(20.0); 5]);
        let f = frames(values);
        let theme = Theme::default();
        for filled in [false, true] {
            let p = TimeSeriesPanel::new("CPU", PanelData::Frames(&f), &theme).filled(filled);
            let buf = render(&p, 62, 12);
            assert!(
                marks_in_row(&buf, 1) >= 1,
                "A run of one point must stay on the screen.\n{}",
                text(&buf)
            );
        }
    }

    #[test]
    fn the_legend_lists_each_series_once_when_each_series_holds_a_hole() {
        let stamps: Vec<i64> = (0..30).map(|i| i * 1000).collect();
        let high = series_with_a_hole();
        let low: Vec<Option<f64>> = high.iter().map(|v| v.map(|v| v / 2.0)).collect();
        let f = vec![Frame::named(
            "cpu",
            vec![
                Field::time("time", stamps),
                Field::f64("alpha", high),
                Field::f64("beta", low),
            ],
        )];
        let theme = Theme::default();
        let p = TimeSeriesPanel::new("CPU", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 80, 24));
        for name in ["alpha", "beta"] {
            assert_eq!(
                out.matches(name).count(),
                1,
                "The legend must list {name} once.\n{out}"
            );
        }
    }

    /// A gap in time between two values is not a hole. The line therefore
    /// joins the values at 2 seconds and at 10 seconds.
    #[test]
    fn a_line_chart_joins_two_values_with_a_gap_in_time_between_them() {
        let stamps = vec![0, 1_000, 2_000, 10_000, 11_000, 12_000];
        let values = [30.0, 40.0, 50.0, 60.0, 70.0, 80.0].map(Some).to_vec();
        let f = vec![Frame::named(
            "cpu",
            vec![Field::time("time", stamps), Field::f64("usage", values)],
        )];
        let theme = Theme::default();
        let p = TimeSeriesPanel::new("CPU", PanelData::Frames(&f), &theme).filled(false);
        let buf = render(&p, 62, 12);
        assert_eq!(
            empty_columns(&buf),
            0,
            "The line must cross the gap in time.\n{}",
            text(&buf)
        );
    }

    #[test]
    fn collect_removes_a_hole_and_keeps_the_floor_above_zero() {
        let f = frames(vec![Some(80.0), None, Some(80.0)]);
        let plot = collect(&f).expect("The frames must hold data.");
        assert_eq!(
            plot.series[0].points.len(),
            2,
            "The function must remove the hole."
        );
        assert!(
            plot.y_bounds[0] > 0.0,
            "The bottom of the range must stay above zero."
        );
    }

    #[test]
    fn collect_widens_a_range_that_has_no_height() {
        let f = frames(vec![Some(5.0), Some(5.0), Some(5.0)]);
        let plot = collect(&f).expect("The frames must hold data.");
        assert!(
            plot.y_bounds[1] > plot.y_bounds[0],
            "A range that has no height would divide by zero. {:?}",
            plot.y_bounds
        );
    }

    #[test]
    fn collect_widens_a_range_that_has_no_width() {
        let f = frames(vec![Some(5.0)]);
        let plot = collect(&f).expect("The frames must hold data.");
        assert!(plot.x_bounds[1] > plot.x_bounds[0], "{:?}", plot.x_bounds);
    }

    #[test]
    fn collect_ignores_a_value_that_is_not_finite() {
        let f = frames(vec![Some(1.0), Some(f64::NAN), Some(2.0)]);
        let plot = collect(&f).expect("The frames must hold data.");
        assert_eq!(plot.series[0].points.len(), 2);
    }

    /// Returns the time of each point of each run of the first series, in
    /// seconds. The helper `frames` puts one value at each second.
    ///
    /// The function returns `None` when `collect` finds no value to draw.
    fn run_seconds(values: Vec<Option<f64>>) -> Option<Vec<Vec<i64>>> {
        let plot = collect(&frames(values))?;
        let series = &plot.series[0];
        let runs = series
            .runs
            .iter()
            .map(|run| {
                series.points[run.clone()]
                    .iter()
                    .map(|(t, _)| (*t / 1000.0) as i64)
                    .collect()
            })
            .collect();
        Some(runs)
    }

    #[test]
    fn collect_cuts_a_series_at_a_hole_at_the_start_in_the_middle_and_at_the_end() {
        assert_eq!(
            run_seconds(vec![None, Some(1.0), Some(2.0)]),
            Some(vec![vec![1, 2]])
        );
        assert_eq!(
            run_seconds(vec![Some(0.0), None, Some(2.0)]),
            Some(vec![vec![0], vec![2]])
        );
        assert_eq!(
            run_seconds(vec![Some(0.0), Some(1.0), None]),
            Some(vec![vec![0, 1]])
        );
    }

    #[test]
    fn collect_makes_one_cut_for_two_holes_next_to_each_other() {
        assert_eq!(
            run_seconds(vec![Some(0.0), None, None, Some(3.0)]),
            Some(vec![vec![0], vec![3]])
        );
    }

    #[test]
    fn collect_keeps_a_run_of_one_point_between_two_holes() {
        assert_eq!(
            run_seconds(vec![Some(0.0), None, Some(2.0), None, Some(4.0)]),
            Some(vec![vec![0], vec![2], vec![4]])
        );
    }

    #[test]
    fn collect_finds_no_value_in_a_series_of_holes_alone() {
        assert_eq!(run_seconds(vec![None, None, None]), None);
        assert_eq!(run_seconds(vec![Some(f64::NAN), None]), None);
    }

    #[test]
    fn collect_cuts_a_series_at_each_value_that_is_not_finite() {
        let values = vec![
            Some(0.0),
            Some(f64::NAN),
            Some(2.0),
            Some(f64::INFINITY),
            Some(4.0),
            Some(f64::NEG_INFINITY),
            Some(6.0),
        ];
        assert_eq!(
            run_seconds(values),
            Some(vec![vec![0], vec![2], vec![4], vec![6]])
        );
    }

    /// A gap in time between two values is not a hole, so the series holds one
    /// run.
    #[test]
    fn collect_keeps_a_gap_in_time_inside_one_run() {
        let stamps = vec![0, 1_000, 10_000];
        let values = vec![Some(1.0), Some(2.0), Some(3.0)];
        let f = vec![Frame::named(
            "cpu",
            vec![Field::time("time", stamps), Field::f64("usage", values)],
        )];
        let plot = collect(&f).expect("The frames must hold data.");
        assert_eq!(plot.series[0].runs, vec![0..3]);
    }

    #[test]
    fn the_panel_draws_nothing_in_a_small_area() {
        let f = frames(vec![Some(1.0), Some(2.0)]);
        let theme = Theme::default();
        let p = TimeSeriesPanel::new("CPU", PanelData::Frames(&f), &theme);
        for (w, h) in [(0, 0), (1, 1), (2, 2), (2, 20), (20, 2)] {
            let _ = render(&p, w, h);
        }
    }

    #[test]
    fn a_limit_of_the_panel_moves_the_ends_of_the_axis() {
        let frames = frames(vec![Some(10.0), Some(20.0)]);
        let mut plot = collect(&frames).expect("The frames hold data.");
        apply_limits(&mut plot, Some(0.0), Some(100.0));
        assert_eq!(plot.y_bounds, [0.0, 100.0]);
    }

    #[test]
    fn a_limit_of_the_panel_that_is_not_correct_leaves_the_axis_alone() {
        let frames = frames(vec![Some(10.0), Some(20.0)]);
        let mut plot = collect(&frames).expect("The frames hold data.");
        let before = plot.y_bounds;
        // The chart would divide by zero for an axis of no height.
        apply_limits(&mut plot, Some(50.0), Some(50.0));
        assert_eq!(plot.y_bounds, before);
        apply_limits(&mut plot, Some(f64::NAN), Some(f64::INFINITY));
        assert_eq!(plot.y_bounds, before);
    }

    #[test]
    fn the_panel_draws_the_axis_that_its_limits_state() {
        let f = frames(vec![Some(10.0), Some(20.0)]);
        let theme = Theme::default();
        let p = TimeSeriesPanel::new("cpu", PanelData::Frames(&f), &theme).options(PanelOptions {
            unit: Some("percent"),
            min: Some(0.0),
            max: Some(100.0),
            ..Default::default()
        });
        let out = text(&render(&p, 40, 10));
        assert!(out.contains("100%"), "The panel drew this. {out}");
    }

    /// A person who moves the range one hour into the past must not read
    /// "now" at the right edge of a panel. The panel measured every label from
    /// the newest point before it took a window, so the right label was always
    /// "now".
    #[test]
    fn a_panel_that_draws_a_window_in_the_past_writes_no_label_that_reads_now() {
        let f = frames(vec![Some(10.0), Some(20.0), Some(30.0), Some(40.0)]);
        let theme = Theme::default();
        let window = TimeWindow {
            from: 0,
            to: 3_000,
            now: 3_603_000,
        };
        let p = TimeSeriesPanel::new("cpu", PanelData::Frames(&f), &theme).window(window);
        let out = text(&render(&p, 40, 10));
        assert!(
            !out.contains("now"),
            "No place on this axis is the present moment. {out}"
        );
    }

    /// A panel draws the window that a person asked for. A data source that
    /// covers one part of that window therefore fills one part of the panel.
    /// A person then sees how much of the window that data source holds.
    #[test]
    fn a_window_that_the_data_covers_in_part_leaves_the_rest_of_the_axis_empty() {
        let f = frames(vec![Some(10.0), Some(20.0)]);
        let mut plot = collect(&f).expect("The frames must hold data.");
        assert_eq!(plot.x_bounds, [0.0, 1_000.0], "The data covers one second.");
        cover_window(
            &mut plot,
            TimeWindow {
                from: -300_000,
                to: 1_000,
                now: 1_000,
            },
        );
        assert_eq!(
            plot.x_bounds,
            [-300_000.0, 1_000.0],
            "The axis must cover the whole window."
        );
    }

    /// A data source aligns its points to a grid of its own, so it can return
    /// a point just outside the window. The axis of the values keeps such a
    /// value, and the axis of time keeps such a point.
    #[test]
    fn a_point_outside_the_window_still_widens_the_axis_of_time() {
        let f = frames(vec![Some(10.0), Some(20.0), Some(30.0)]);
        let mut plot = collect(&f).expect("The frames must hold data.");
        cover_window(
            &mut plot,
            TimeWindow {
                from: 500,
                to: 1_500,
                now: 1_500,
            },
        );
        assert_eq!(
            plot.x_bounds,
            [0.0, 2_000.0],
            "The chart must draw every point."
        );
    }

    #[test]
    fn last_value_returns_the_most_recent_value() {
        let f = frames(vec![Some(1.0), Some(9.0), None]);
        assert_eq!(last_value(&f), Some((1000, 9.0)));
    }

    #[test]
    fn last_value_returns_none_for_frames_that_hold_no_value() {
        assert_eq!(last_value(&[]), None);
        assert_eq!(last_value(&frames(vec![None])), None);
    }

    /// A caller can state any number in a window. A panel must still draw.
    #[test]
    fn a_window_at_the_limits_of_the_number_draws_without_a_panic() {
        let f = frames(vec![Some(10.0), Some(50.0), Some(20.0)]);
        let theme = Theme::default();
        let ends = [i64::MIN, 0, i64::MAX];
        for from in ends {
            for to in ends {
                for now in ends {
                    let window = TimeWindow { from, to, now };
                    let p = TimeSeriesPanel::new("p", PanelData::Frames(&f), &theme).window(window);
                    let out = text(&render(&p, 40, 10));
                    assert!(out.contains('p'), "The panel drew nothing. {out}");
                }
            }
        }
    }
}

#[cfg(test)]
mod axis_tests {
    use super::*;

    /// Returns the labels of an axis that ends at the present moment.
    fn labels(span_ms: f64) -> Vec<String> {
        labels_of([0.0, span_ms], span_ms)
    }

    /// Returns the labels of an axis, for a stated present moment.
    fn labels_of(bounds: [f64; 2], now: f64) -> Vec<String> {
        x_labels(bounds, now, Style::default())
            .iter()
            .map(|line| line.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect()
    }

    #[test]
    fn a_short_range_does_not_report_the_same_time_three_times() {
        let out = labels(900.0);
        assert_eq!(out[2], "now");
        assert_ne!(out[0], out[1], "The labels must differ. {out:?}");
        assert_ne!(out[1], out[2], "The labels must differ. {out:?}");
    }

    #[test]
    fn a_range_of_seconds_reports_seconds() {
        assert_eq!(labels(60_000.0), vec!["-60s", "-30s", "now"]);
    }

    #[test]
    fn a_range_of_minutes_reports_minutes() {
        assert_eq!(labels(600_000.0), vec!["-10m", "-5m", "now"]);
    }

    #[test]
    fn a_range_of_hours_reports_hours() {
        let out = labels(21_600_000.0);
        assert_eq!(out[0], "-6h", "{out:?}");
        assert_eq!(out[2], "now");
    }

    #[test]
    fn a_range_of_no_width_reports_now_only() {
        assert_eq!(labels(0.0), vec!["now", "now", "now"]);
    }

    /// A window that ended in the past holds no place that is the present
    /// moment. The axis lied about this before the panel took a window.
    #[test]
    fn a_window_that_ended_in_the_past_labels_no_place_as_now() {
        let now = 3_600_000.0 + 120_000.0;
        let out = labels_of([0.0, 120_000.0], now);
        assert_eq!(out, vec!["-62m", "-61m", "-60m"], "{out:?}");
        assert!(
            !out.iter().any(|label| label == "now"),
            "No place on this axis is the present moment. {out:?}"
        );
    }

    /// A window that ends one second before the present moment must not read
    /// `-0m` at its right edge. That text states both a distance and no
    /// distance, and the axis keeps the word `now` for a distance of nothing.
    #[test]
    fn a_window_that_ended_a_moment_ago_writes_no_count_of_zero() {
        let out = labels_of([0.0, 120_000.0], 121_000.0);
        assert_eq!(out, vec!["-2m", "-1m", "-1s"], "{out:?}");
    }

    /// An axis of one second that stands one minute in the past must not read
    /// a count of milliseconds. Three labels of eight characters do not fit a
    /// panel that a person can read.
    #[test]
    fn a_narrow_axis_far_from_the_present_moment_reports_seconds() {
        let out = labels_of([0.0, 1_000.0], 61_000.0);
        assert_eq!(out, vec!["-61.0s", "-60.5s", "-60.0s"], "{out:?}");
    }

    /// A dashboard can ask for a moment in the future. The parser reads the
    /// form `now+5m`, so the axis must mark such a place with a plus sign.
    #[test]
    fn a_place_in_the_future_carries_a_plus_sign() {
        let out = labels_of([0.0, 600_000.0], 300_000.0);
        assert_eq!(out, vec!["-5m", "now", "+5m"], "{out:?}");
    }
}

#[cfg(test)]
mod bounds_tests {
    use super::*;
    use thunderbox_core::{Field, FieldConfig, Frame};

    fn percent_frame(values: Vec<Option<f64>>) -> Vec<Frame> {
        let stamps: Vec<i64> = (0..values.len() as i64).map(|i| i * 1000).collect();
        vec![Frame::named(
            "m",
            vec![
                Field::time("time", stamps),
                Field::f64("share", values).with_config(FieldConfig {
                    unit: Some("percent".into()),
                    min: Some(0.0),
                    max: Some(100.0),
                    ..Default::default()
                }),
            ],
        )]
    }

    #[test]
    fn a_declared_limit_sets_the_axis_rather_than_the_data() {
        let frames = percent_frame(vec![Some(4.1), Some(4.2), Some(4.3)]);
        let plot = collect(&frames).expect("The frames hold data.");
        assert_eq!(
            plot.y_bounds,
            [0.0, 100.0],
            "A share of use runs from 0 to 100, whatever the data holds."
        );
    }

    #[test]
    fn a_full_value_does_not_push_the_axis_past_its_limit() {
        let frames = percent_frame(vec![Some(100.0), Some(99.0)]);
        let plot = collect(&frames).expect("The frames hold data.");
        assert_eq!(
            plot.y_bounds[1], 100.0,
            "An axis of a share of use must not read 105."
        );
    }

    #[test]
    fn a_value_past_the_limit_still_widens_the_axis() {
        let frames = percent_frame(vec![Some(110.0)]);
        let plot = collect(&frames).expect("The frames hold data.");
        assert!(
            plot.y_bounds[1] >= 110.0,
            "The chart must draw every value. {:?}",
            plot.y_bounds
        );
    }

    #[test]
    fn a_field_that_declares_no_limit_keeps_the_space_above_the_peak() {
        let stamps = vec![0_i64, 1000];
        let frames = vec![Frame::named(
            "m",
            vec![
                Field::time("time", stamps),
                Field::f64("bytes", vec![Some(10.0), Some(20.0)]),
            ],
        )];
        let plot = collect(&frames).expect("The frames hold data.");
        assert!(
            plot.y_bounds[1] > 20.0,
            "The peak must not touch the border. {:?}",
            plot.y_bounds
        );
    }

    #[test]
    fn a_declared_limit_keeps_the_labels_of_the_axis_apart() {
        let frames = percent_frame(vec![Some(4.1), Some(4.2), Some(4.3)]);
        let plot = collect(&frames).expect("The frames hold data.");
        let labels: Vec<String> = y_labels(plot.y_bounds, Some("percent"), None, Style::default())
            .iter()
            .map(|line| line.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect();
        assert_eq!(labels, vec!["0%", "50%", "100%"], "{labels:?}");
    }
}

#[cfg(test)]
mod align_tests {
    use super::*;
    use thunderbox_core::{Field, Frame};

    /// Returns frames that hold one column of these values.
    fn frames(values: Vec<Option<f64>>) -> Vec<Frame> {
        let stamps: Vec<i64> = (0..values.len() as i64).map(|i| i * 1000).collect();
        vec![Frame::named(
            "s",
            vec![Field::time("time", stamps), Field::f64("v", values)],
        )]
    }

    /// Returns a panel that draws these values in this unit.
    fn panel<'a>(frames: &'a [Frame], unit: &'a str, theme: &'a Theme) -> TimeSeriesPanel<'a> {
        TimeSeriesPanel::new("p", PanelData::Frames(frames), theme).options(PanelOptions {
            unit: Some(unit),
            ..Default::default()
        })
    }

    /// Draws the panel, and returns the cell that holds the axis of values.
    ///
    /// The chart draws that axis as a vertical line. The border of the panel
    /// draws the same character, so the function takes the second one of the
    /// row. The function returns `None` for a panel that draws no axis.
    fn axis_column(panel: &TimeSeriesPanel<'_>, w: u16, h: u16) -> Option<u16> {
        let area = Rect::new(0, 0, w, h);
        let mut buf = Buffer::empty(area);
        panel.render(area, &mut buf);
        (0..w)
            .filter(|x| buf[(*x, 1)].symbol() == "\u{2502}")
            .nth(1)
    }

    /// Returns the labels of the axis of values that the panel draws.
    fn drawn_labels(panel: &TimeSeriesPanel<'_>, w: u16, h: u16) -> Vec<String> {
        let area = Rect::new(0, 0, w, h);
        let mut buf = Buffer::empty(area);
        panel.render(area, &mut buf);
        let axis = axis_column(panel, w, h).expect("The panel must draw an axis.");
        (1..h - 1)
            .map(|y| {
                (1..axis)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect()
    }

    /// A share of use and a rate hold labels of different widths. This test
    /// records the fault that [`TimeSeriesPanel::label_width`] answers.
    #[test]
    fn two_panels_that_hold_labels_of_different_widths_draw_charts_of_different_widths() {
        let theme = Theme::default();
        let share = frames(vec![Some(10.0), Some(80.0)]);
        let rate = frames(vec![Some(1.0e9), Some(8.0e9)]);
        let top = axis_column(&panel(&share, "percent", &theme), 40, 8);
        let bottom = axis_column(&panel(&rate, "bytes/s", &theme), 40, 8);
        assert_ne!(
            top, bottom,
            "A panel that states no width measures its own labels."
        );
    }

    #[test]
    fn a_width_that_the_caller_states_gives_two_panels_one_chart_width() {
        let theme = Theme::default();
        let share = frames(vec![Some(10.0), Some(80.0)]);
        let rate = frames(vec![Some(1.0e9), Some(8.0e9)]);
        let width = panel(&share, "percent", &theme)
            .measure_label_width()
            .max(panel(&rate, "bytes/s", &theme).measure_label_width());
        let top = axis_column(&panel(&share, "percent", &theme).label_width(width), 40, 8);
        let bottom = axis_column(&panel(&rate, "bytes/s", &theme).label_width(width), 40, 8);
        assert_eq!(
            top, bottom,
            "The two charts must start at the same cell. {top:?} {bottom:?}"
        );
        assert!(top.is_some(), "The panels must draw an axis.");
    }

    /// The chart gives no more than a third of its width to the labels. A
    /// caller that asks for more must still receive two charts of one width,
    /// and each label must still hold a number.
    #[test]
    fn a_width_that_is_too_large_still_gives_two_panels_one_chart_width() {
        let theme = Theme::default();
        let share = frames(vec![Some(10.0), Some(80.0)]);
        let rate = frames(vec![Some(1.0e9), Some(8.0e9)]);
        let top = panel(&share, "percent", &theme).label_width(200);
        let bottom = panel(&rate, "bytes/s", &theme).label_width(200);
        assert_eq!(
            axis_column(&top, 30, 8),
            axis_column(&bottom, 30, 8),
            "The two charts must start at the same cell."
        );
        let labels = drawn_labels(&top, 30, 8);
        assert!(
            labels.iter().any(|l| l.contains('%')),
            "The labels must still hold a number. {labels:?}"
        );
    }

    /// A panel that shows a stack draws the same width at every size of
    /// terminal. This test walks the widths that a person can reach.
    #[test]
    fn two_panels_hold_one_chart_width_at_every_width_of_the_area() {
        let theme = Theme::default();
        let share = frames(vec![Some(10.0), Some(80.0)]);
        let rate = frames(vec![Some(1.0e9), Some(8.0e9)]);
        let width = panel(&share, "percent", &theme)
            .measure_label_width()
            .max(panel(&rate, "bytes/s", &theme).measure_label_width());
        for w in 4..80 {
            let top = axis_column(&panel(&share, "percent", &theme).label_width(width), w, 8);
            let bottom = axis_column(&panel(&rate, "bytes/s", &theme).label_width(width), w, 8);
            assert_eq!(
                top, bottom,
                "The two charts differ in an area of {w} cells."
            );
        }
    }

    #[test]
    fn a_panel_that_draws_a_message_measures_no_labels() {
        let theme = Theme::default();
        let empty: Vec<Frame> = vec![];
        for data in [
            PanelData::Loading,
            PanelData::Error("upstream refused"),
            PanelData::Frames(&empty),
        ] {
            let p = TimeSeriesPanel::new("p", data, &theme);
            assert_eq!(
                p.measure_label_width(),
                0,
                "A panel that draws no chart widens no other panel."
            );
        }
    }

    /// The chart draws the first label of the axis of time below the labels of
    /// the axis of values. The measurement covers that label.
    #[test]
    fn the_measurement_covers_the_first_label_of_the_axis_of_time() {
        let theme = Theme::default();
        let f = frames(vec![Some(1.0), Some(2.0)]);
        let window = TimeWindow {
            from: 0,
            to: 1_000,
            now: 61_000,
        };
        let p = TimeSeriesPanel::new("p", PanelData::Frames(&f), &theme).window(window);
        assert_eq!(
            x_labels(
                p.plot().expect("The frames hold data.").0.x_bounds,
                61_000.0,
                Style::default()
            )
            .first()
            .map_or(0, Line::width),
            6,
            "The axis starts at -61.0s."
        );
        assert_eq!(
            p.measure_label_width(),
            5,
            "That label needs one cell less than its own width."
        );
    }

    #[test]
    fn pad_labels_gives_every_label_one_width() {
        let mut labels = vec![Line::from("1%"), Line::from("100%")];
        pad_labels(&mut labels, 6, Style::default());
        let widths: Vec<usize> = labels.iter().map(Line::width).collect();
        assert_eq!(widths, vec![6, 6]);
    }

    #[test]
    fn pad_labels_leaves_a_label_that_is_wider_than_the_width() {
        let mut labels = vec![Line::from("100000%")];
        pad_labels(&mut labels, 3, Style::default());
        assert_eq!(labels[0].width(), 7, "The function removes nothing.");
    }

    #[test]
    fn the_limit_gives_no_more_than_a_third_of_the_chart_to_the_labels() {
        assert_eq!(label_width_limit(Rect::new(0, 0, 32, 8)), 10);
        assert_eq!(label_width_limit(Rect::new(0, 0, 3, 8)), 0);
        assert_eq!(label_width_limit(Rect::new(0, 0, 0, 8)), 0);
    }
}
