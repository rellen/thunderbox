//! The panel that draws values against time.

use std::ops::Range;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Alignment, Rect};
use ratatui_core::style::Style;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;
use ratatui_widgets::chart::{Axis, Chart, Dataset, GraphType};
use thunderbox_core::{Frame, LabelAlign, TimeWindow, Timestamp};

use crate::panel::{NO_DATA, frames_or_message, panel_block, render_message};
use crate::{
    Marker, PanelData, PanelOptions, Theme, draws_a_panel, format_clock, format_from_now,
    format_value,
};

/// The text that a panel writes at each label of its axis of time.
///
/// A window that follows the clock names a new moment on each refresh. A
/// window that stands still names the same moments for ever. The two kinds of
/// window therefore need two kinds of label.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimeLabels {
    /// Each label states the distance from the present moment to its place on
    /// the axis, as `-5m` or `now`.
    #[default]
    FromNow,
    /// Each label states the time of its place on the axis, in Coordinated
    /// Universal Time. The text is `HH:MM:SS` for an axis that is narrower than
    /// one day, and `MM-DD HH:MM` for a wider axis. The text does not change
    /// when the present moment changes.
    Clock,
}

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
    value_labels: LabelAlign,
    time_labels: TimeLabels,
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
            value_labels: LabelAlign::default(),
            time_labels: TimeLabels::default(),
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

    /// Sets the kind of label that the axis of time draws.
    ///
    /// The default is [`TimeLabels::FromNow`]. A caller that draws a window
    /// that stands still passes [`TimeLabels::Clock`]. Each label then names
    /// the moment of its place, and it does not count further into the past on
    /// each refresh.
    pub fn time_labels(mut self, labels: TimeLabels) -> Self {
        self.time_labels = labels;
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

    /// States the side of the label column where the labels of the axis of
    /// values stand.
    ///
    /// The labels of one chart differ in width. `100%` and `0%` are an
    /// example. [`LabelAlign::Right`] ends each label at the axis, and
    /// [`LabelAlign::Left`] starts each label at the left edge of the column.
    /// The panel uses [`LabelAlign::Right`] until a caller states another
    /// value. The axis of time does not change.
    pub fn value_labels(mut self, align: LabelAlign) -> Self {
        self.value_labels = align;
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
            self.time_labels,
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

/// Returns the labels of the axis of time in the stated mode.
///
/// The mode [`TimeLabels::FromNow`] gives the labels of [`x_labels`]. The mode
/// [`TimeLabels::Clock`] gives the labels of [`clock_labels`].
fn axis_labels(mode: TimeLabels, bounds: [f64; 2], now: f64, style: Style) -> Vec<Line<'static>> {
    match mode {
        TimeLabels::FromNow => x_labels(bounds, now, style),
        TimeLabels::Clock => clock_labels(bounds, style),
    }
}

/// Returns the labels of the axis that holds time, as times of day.
///
/// Each label states the time of its place on the axis. The function reads no
/// present moment, so the labels stay the same while the axis stays the same.
/// [`format_clock`] selects the form of the text from the width of the axis.
fn clock_labels(bounds: [f64; 2], style: Style) -> Vec<Line<'static>> {
    let span = (bounds[1] - bounds[0]).max(0.0);
    let step = span / 2.0;
    (0..3)
        .map(|i| {
            let at = (bounds[0] + step * f64::from(i)).round() as i64;
            Line::from(Span::styled(format_clock(at, span.round() as i64), style))
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
fn needed_label_width(
    plot: &Plot,
    time_labels: TimeLabels,
    now: f64,
    unit: Option<&str>,
    decimals: Option<u8>,
) -> u16 {
    let values = y_labels(plot.y_bounds, unit, decimals, Style::default())
        .iter()
        .map(Line::width)
        .max()
        .unwrap_or(0) as u16;
    let first_time = axis_labels(time_labels, plot.x_bounds, now, Style::default())
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

/// Adds spaces to each label, until every label holds `width`.
///
/// The chart takes the width of the column of labels from the widest label. A
/// caller that pads the labels of two panels to one width therefore gives the
/// two charts one width. The function leaves a label that is wider than `width`
/// as it is.
///
/// The spaces stand on the side that is away from the alignment. A label that
/// the chart aligns at the right takes its spaces at the left. A label that the
/// chart aligns at the left takes its spaces at the right. Every label of one
/// panel then holds the same width, and the chart keeps its width under both
/// values of `align`.
fn pad_labels(labels: &mut [Line<'static>], width: u16, style: Style, align: LabelAlign) {
    for label in labels {
        let pad = usize::from(width).saturating_sub(label.width());
        if pad > 0 {
            let spaces = Span::styled(" ".repeat(pad), style);
            match align {
                LabelAlign::Right => label.spans.insert(0, spaces),
                LabelAlign::Left => label.spans.push(spaces),
            }
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
        //
        // ratatui draws each dataset as one layer, in order. A later layer
        // replaces the glyph and the colour of each cell that it paints. The
        // fill of a later series would therefore hide an earlier series that
        // lies below it. A filled chart draws every fill first. It then draws
        // the line of each series but the last over the fills (ADR 0082 in
        // docs/contributor/adr).
        //
        // A line layer also replaces the dots of the fill in each cell that
        // the line crosses. The last fill already holds the line of its own
        // series, so no layer of a line covers it. A chart of one series
        // therefore keeps every dot of its fill (ADR 0090).
        let floor = plot.y_bounds[0];
        let runs = |fill: bool, count: usize| {
            plot.series
                .iter()
                .take(count)
                .enumerate()
                .flat_map(move |(i, s)| {
                    let color = self.theme.series_color(i);
                    s.runs.iter().enumerate().map(move |(r, run)| {
                        let ds = Dataset::default()
                            .data(&s.points[run.clone()])
                            .marker(self.marker.into())
                            .style(Style::default().fg(color));
                        // The legend lists each dataset that holds a name. Only
                        // the first run of a series in the first pass takes the
                        // name, so the legend lists each series once.
                        let ds = if r == 0 && (fill || !self.filled) {
                            ds.name(s.name.clone())
                        } else {
                            ds
                        };
                        if fill {
                            // Fill the area down to the bottom of the axis.
                            ds.graph_type(GraphType::Area).fill_to_y(floor)
                        } else {
                            ds.graph_type(GraphType::Line)
                        }
                    })
                })
                .collect::<Vec<Dataset<'_>>>()
        };
        let all = plot.series.len();
        let mut datasets = runs(self.filled, all);
        if self.filled {
            datasets.extend(runs(false, all.saturating_sub(1)));
        }

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
                self.value_labels,
            );
        }
        let alignment = match self.value_labels {
            LabelAlign::Left => Alignment::Left,
            LabelAlign::Right => Alignment::Right,
        };

        let mut chart = Chart::new(datasets)
            .block(block)
            .x_axis(
                Axis::default()
                    .style(self.theme.axis)
                    .bounds(plot.x_bounds)
                    .labels(axis_labels(
                        self.time_labels,
                        plot.x_bounds,
                        now,
                        self.theme.muted,
                    )),
            )
            .y_axis(
                Axis::default()
                    .style(self.theme.axis)
                    .bounds(plot.y_bounds)
                    .labels_alignment(alignment)
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

    /// Returns one frame with two series over the same times. The first
    /// series lies below the second series at each time.
    fn a_lower_series_then_a_higher_series() -> Vec<Frame> {
        let stamps: Vec<i64> = (0..16).map(|i| i * 1000).collect();
        let lower: Vec<Option<f64>> = (0..16).map(|i| Some(1000.0 + i as f64 * 10.0)).collect();
        let higher: Vec<Option<f64>> = (0..16)
            .map(|i| Some(5000.0 + i as f64 * 320.0 / 15.0))
            .collect();
        vec![Frame::named(
            "cpu",
            vec![
                Field::time("time", stamps),
                Field::f64("lower", lower),
                Field::f64("higher", higher),
            ],
        )]
    }

    /// Counts the cells that hold a braille character in the colour of the
    /// first series, and in the colour of the second series. The panel hides
    /// its legend, so the count holds no cell of the legend.
    fn cells_of_two_colours(filled: bool) -> (usize, usize) {
        let f = a_lower_series_then_a_higher_series();
        let theme = Theme::default();
        let p = TimeSeriesPanel::new("CPU", PanelData::Frames(&f), &theme)
            .filled(filled)
            .show_legend(false);
        let buf = render(&p, 60, 12);
        let (first, second) = (theme.series_color(0), theme.series_color(1));
        let (mut a, mut b) = (0, 0);
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                let cell = &buf[(x, y)];
                let braille = cell
                    .symbol()
                    .chars()
                    .next()
                    .is_some_and(|c| ('\u{2801}'..='\u{28FF}').contains(&c));
                if braille && cell.fg == first {
                    a += 1;
                } else if braille && cell.fg == second {
                    b += 1;
                }
            }
        }
        (a, b)
    }

    #[test]
    fn a_filled_chart_shows_a_series_that_lies_below_a_later_series() {
        let (first, second) = cells_of_two_colours(true);
        assert!(
            first > 0,
            "The chart shows no cell in the colour of the first series. Cells of the first colour: {first}. Cells of the second colour: {second}."
        );
    }

    #[test]
    fn a_filled_chart_shows_the_series_that_lies_above_an_earlier_series() {
        let (first, second) = cells_of_two_colours(true);
        assert!(
            second > 0,
            "The chart shows no cell in the colour of the second series. Cells of the first colour: {first}. Cells of the second colour: {second}."
        );
    }

    #[test]
    fn a_line_chart_shows_each_series_that_lies_below_a_later_series() {
        let (first, second) = cells_of_two_colours(false);
        assert!(first > 0, "The chart shows no cell of the first colour.");
        assert!(second > 0, "The chart shows no cell of the second colour.");
    }

    /// Returns 60 values of one series. The values stay low, near 5. A bump of
    /// 40 to 55 sits between the 30th value and the 40th value.
    fn a_low_series_with_a_bump() -> Vec<Frame> {
        let values: Vec<Option<f64>> = (0..60)
            .map(|i| {
                if (30..40).contains(&i) {
                    Some(40.0 + (i - 30) as f64 * 1.5)
                } else {
                    Some(5.0 + (i % 3) as f64)
                }
            })
            .collect();
        frames(values)
    }

    /// Returns the number of dots that one cell draws. A braille character
    /// holds one bit for each dot.
    fn dots_of(cell: &ratatui_core::buffer::Cell) -> u32 {
        cell.symbol()
            .chars()
            .next()
            .filter(|c| ('\u{2801}'..='\u{28FF}').contains(c))
            .map_or(0, |c| (c as u32 - 0x2800).count_ones())
    }

    fn dots_in(buf: &Buffer) -> u32 {
        (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
            .map(|(x, y)| dots_of(&buf[(x, y)]))
            .sum()
    }

    /// Draws the datasets that a filled chart of one series needs and no more.
    /// This function draws the `Area` datasets alone, with the axes of the
    /// panel. It is the reference that ratatui draws without a line pass.
    fn reference_of_the_fill(panel: &TimeSeriesPanel<'_>, w: u16, h: u16) -> Buffer {
        let (plot, now) = panel.plot().expect("The frames hold a value.");
        let floor = plot.y_bounds[0];
        let data: Vec<&[(f64, f64)]> = plot
            .series
            .iter()
            .flat_map(|s| s.runs.iter().map(move |run| &s.points[run.clone()]))
            .collect();
        let datasets: Vec<Dataset<'_>> = data
            .into_iter()
            .map(|points| {
                Dataset::default()
                    .data(points)
                    .marker(panel.marker.into())
                    .style(Style::default().fg(panel.theme.series_color(0)))
                    .graph_type(GraphType::Area)
                    .fill_to_y(floor)
            })
            .collect();
        let chart = Chart::new(datasets)
            .block(panel_block(panel.title, panel.theme, panel.focused))
            .x_axis(
                Axis::default()
                    .style(panel.theme.axis)
                    .bounds(plot.x_bounds)
                    .labels(x_labels(plot.x_bounds, now, panel.theme.muted)),
            )
            .y_axis(
                Axis::default()
                    .style(panel.theme.axis)
                    .bounds(plot.y_bounds)
                    // The panel puts each label at the right by default
                    // (ADR 0089), so the reference does the same.
                    .labels_alignment(Alignment::Right)
                    .labels(y_labels(
                        plot.y_bounds,
                        panel.options.unit.or(plot.unit.as_deref()),
                        panel.options.decimals.or(plot.decimals),
                        panel.theme.muted,
                    )),
            )
            .hidden_legend_constraints((
                ratatui_core::layout::Constraint::Ratio(0, 1),
                ratatui_core::layout::Constraint::Ratio(0, 1),
            ));
        let area = Rect::new(0, 0, w, h);
        let mut buf = Buffer::empty(area);
        chart.render(area, &mut buf);
        buf
    }

    #[test]
    fn a_filled_chart_of_one_series_draws_a_fill_below_its_line_in_the_bottom_row() {
        let f = a_low_series_with_a_bump();
        let theme = Theme::default();
        let p = TimeSeriesPanel::new("CPU", PanelData::Frames(&f), &theme).filled(true);
        let buf = render(&p, 60, 10);
        let (axis_x, axis_y) = corner(&buf);
        let bottom = axis_y - 1;
        let cells: Vec<u32> = (axis_x + 1..buf.area.width - 1)
            .map(|x| dots_of(&buf[(x, bottom)]))
            .filter(|&dots| dots > 0)
            .collect();
        assert!(
            !cells.is_empty(),
            "The bottom row of the plot holds no mark."
        );
        let thin = cells.iter().filter(|&&dots| dots < 4).count();
        assert!(
            thin <= 1,
            "The bottom row holds {thin} cells that hold fewer than 4 dots. The dots of the cells: {cells:?}."
        );
    }

    #[test]
    fn a_filled_chart_of_one_series_draws_the_same_cells_as_its_fill_alone() {
        let f = a_low_series_with_a_bump();
        let theme = Theme::default();
        let p = TimeSeriesPanel::new("CPU", PanelData::Frames(&f), &theme)
            .filled(true)
            .show_legend(false);
        let buf = render(&p, 60, 10);
        let reference = reference_of_the_fill(&p, 60, 10);
        assert_eq!(
            dots_in(&buf),
            dots_in(&reference),
            "The chart draws a different number of dots than its fill alone."
        );
        assert_eq!(
            text(&buf),
            text(&reference),
            "The chart draws different cells than its fill alone."
        );
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
    use std::collections::BTreeSet;

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

    /// Returns frames whose points lie inside a window of two minutes.
    fn frames_at(start: i64, step: i64) -> Vec<Frame> {
        let stamps: Vec<i64> = (0..3).map(|i| start + i * step).collect();
        vec![Frame::named(
            "s",
            vec![
                Field::time("time", stamps),
                Field::f64("v", vec![Some(1.0), Some(2.0), Some(3.0)]),
            ],
        )]
    }

    /// Draws the panel, and returns the text of its last row but one, which
    /// holds the labels of the axis of time.
    fn axis_row(panel: &TimeSeriesPanel<'_>) -> String {
        let area = Rect::new(0, 0, 60, 10);
        let mut buf = Buffer::empty(area);
        panel.render(area, &mut buf);
        (0..area.width)
            .map(|x| buf[(x, 8)].symbol().to_string())
            .collect()
    }

    /// 14:00:00 UTC on 1970-01-02, in milliseconds.
    const TWO_PM: i64 = 86_400_000 + 50_400_000;

    #[test]
    fn a_panel_with_clock_labels_writes_the_time_of_each_place() {
        let theme = Theme::default();
        let f = frames_at(TWO_PM, 60_000);
        let window = TimeWindow {
            from: TWO_PM,
            to: TWO_PM + 120_000,
            now: TWO_PM + 7_200_000,
        };
        let panel = TimeSeriesPanel::new("p", PanelData::Frames(&f), &theme)
            .window(window)
            .time_labels(TimeLabels::Clock);
        let row = axis_row(&panel);
        for label in ["14:00:00", "14:01:00", "14:02:00"] {
            assert!(row.contains(label), "The axis must hold {label}. {row:?}");
        }
        assert!(!row.contains('-') && !row.contains("now"), "{row:?}");
    }

    #[test]
    fn a_panel_with_clock_labels_writes_the_date_for_a_window_of_two_days() {
        let theme = Theme::default();
        let f = frames_at(TWO_PM, 86_400_000);
        let window = TimeWindow {
            from: TWO_PM,
            to: TWO_PM + 172_800_000,
            now: TWO_PM + 200_000_000,
        };
        let panel = TimeSeriesPanel::new("p", PanelData::Frames(&f), &theme)
            .window(window)
            .time_labels(TimeLabels::Clock);
        let row = axis_row(&panel);
        for label in ["01-02 14:00", "01-03 14:00", "01-04 14:00"] {
            assert!(row.contains(label), "The axis must hold {label}. {row:?}");
        }
    }

    #[test]
    fn the_clock_labels_of_a_window_do_not_change_when_only_now_changes() {
        let theme = Theme::default();
        let f = frames_at(TWO_PM, 60_000);
        let axis_at = |now: i64| {
            let window = TimeWindow {
                from: TWO_PM,
                to: TWO_PM + 120_000,
                now,
            };
            let panel = TimeSeriesPanel::new("p", PanelData::Frames(&f), &theme)
                .window(window)
                .time_labels(TimeLabels::Clock);
            axis_row(&panel)
        };
        assert_eq!(axis_at(TWO_PM + 3_600_000), axis_at(TWO_PM + 7_200_000));
    }

    #[test]
    fn the_labels_from_now_change_when_only_now_changes() {
        let theme = Theme::default();
        let f = frames_at(TWO_PM, 60_000);
        let axis_at = |now: i64| {
            let window = TimeWindow {
                from: TWO_PM,
                to: TWO_PM + 120_000,
                now,
            };
            axis_row(&TimeSeriesPanel::new("p", PanelData::Frames(&f), &theme).window(window))
        };
        assert_ne!(axis_at(TWO_PM + 3_600_000), axis_at(TWO_PM + 7_200_000));
    }

    #[test]
    fn the_measurement_covers_the_first_clock_label() {
        let theme = Theme::default();
        let f = frames_at(TWO_PM, 60_000);
        let window = TimeWindow {
            from: TWO_PM,
            to: TWO_PM + 120_000,
            now: TWO_PM + 120_000,
        };
        let p = TimeSeriesPanel::new("p", PanelData::Frames(&f), &theme).window(window);
        assert_eq!(p.measure_label_width(), 2, "The label from now is -2m.");
        assert_eq!(
            p.time_labels(TimeLabels::Clock).measure_label_width(),
            7,
            "The label 14:00:00 needs one cell less than its own width."
        );
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
    fn pad_labels_puts_the_spaces_on_the_side_away_from_the_alignment() {
        let mut right = vec![Line::from("1%")];
        pad_labels(&mut right, 6, Style::default(), LabelAlign::Right);
        let text: String = right[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(
            text, "    1%",
            "A right label takes its spaces at the left."
        );
        let mut left = vec![Line::from("1%")];
        pad_labels(&mut left, 6, Style::default(), LabelAlign::Left);
        let text: String = left[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(
            text, "1%    ",
            "A left label takes its spaces at the right."
        );
    }

    /// Returns a panel whose axis of values holds `0%` and `100%`.
    fn percent_frames() -> Vec<Frame> {
        frames(vec![Some(0.0), Some(100.0)])
    }

    /// Returns the rows of the label column that hold a label of the axis of
    /// values. The last row inside the border holds the first label of the axis
    /// of time, and the function leaves that row out.
    fn label_rows(panel: &TimeSeriesPanel<'_>, w: u16, h: u16) -> Vec<String> {
        let mut rows = drawn_labels(panel, w, h);
        rows.pop();
        let rows: Vec<String> = rows
            .into_iter()
            .filter(|row| !row.trim().is_empty())
            .collect();
        assert!(rows.len() >= 2, "The chart must draw two labels. {rows:?}");
        assert!(
            rows.iter()
                .map(|r| r.trim().len())
                .collect::<BTreeSet<_>>()
                .len()
                > 1,
            "The labels must differ in width. {rows:?}"
        );
        rows
    }

    #[test]
    fn a_chart_ends_each_label_of_the_axis_of_values_in_the_same_cell_by_default() {
        let theme = Theme::default();
        let data = percent_frames();
        let rows = label_rows(&panel(&data, "percent", &theme), 40, 8);
        for row in &rows {
            assert!(
                !row.ends_with(' '),
                "Each label must end at the axis. {rows:?}"
            );
        }
    }

    #[test]
    fn a_chart_starts_each_label_of_the_axis_of_values_in_the_same_cell_for_the_value_left() {
        let theme = Theme::default();
        let data = percent_frames();
        let p = panel(&data, "percent", &theme).value_labels(LabelAlign::Left);
        let rows = label_rows(&p, 40, 8);
        for row in &rows {
            assert!(
                !row.starts_with(' '),
                "Each label must start at the left edge. {rows:?}"
            );
        }
    }

    #[test]
    fn the_value_right_draws_the_same_chart_as_the_default() {
        let theme = Theme::default();
        let data = percent_frames();
        let plain = panel(&data, "percent", &theme);
        let right = panel(&data, "percent", &theme).value_labels(LabelAlign::Right);
        assert_eq!(
            drawn_labels(&plain, 40, 8),
            drawn_labels(&right, 40, 8),
            "The default must be the value right."
        );
    }

    #[test]
    fn a_stack_keeps_the_chart_width_under_both_values_and_the_labels_follow_the_value() {
        let theme = Theme::default();
        let data = percent_frames();
        let wide = panel(&data, "percent", &theme).measure_label_width() + 4;
        let left = panel(&data, "percent", &theme)
            .label_width(wide)
            .value_labels(LabelAlign::Left);
        let right = panel(&data, "percent", &theme).label_width(wide);
        let axis = axis_column(&right, 40, 8);
        assert!(axis.is_some(), "The panel must draw an axis.");
        assert_eq!(
            axis_column(&left, 40, 8),
            axis,
            "Both values must keep the chart at one width."
        );
        for row in label_rows(&right, 40, 8) {
            assert!(
                !row.ends_with(' '),
                "The labels must end at the axis. {row:?}"
            );
        }
        for row in label_rows(&left, 40, 8) {
            assert!(
                !row.starts_with(' '),
                "The labels must start at the left edge. {row:?}"
            );
        }
    }

    #[test]
    fn pad_labels_gives_every_label_one_width() {
        let mut labels = vec![Line::from("1%"), Line::from("100%")];
        pad_labels(&mut labels, 6, Style::default(), LabelAlign::Right);
        let widths: Vec<usize> = labels.iter().map(Line::width).collect();
        assert_eq!(widths, vec![6, 6]);
    }

    #[test]
    fn pad_labels_leaves_a_label_that_is_wider_than_the_width() {
        let mut labels = vec![Line::from("100000%")];
        pad_labels(&mut labels, 3, Style::default(), LabelAlign::Right);
        assert_eq!(labels[0].width(), 7, "The function removes nothing.");
    }

    #[test]
    fn the_limit_gives_no_more_than_a_third_of_the_chart_to_the_labels() {
        assert_eq!(label_width_limit(Rect::new(0, 0, 32, 8)), 10);
        assert_eq!(label_width_limit(Rect::new(0, 0, 3, 8)), 0);
        assert_eq!(label_width_limit(Rect::new(0, 0, 0, 8)), 0);
    }
}
