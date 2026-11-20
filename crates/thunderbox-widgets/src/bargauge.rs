//! The panel that draws one bar for each series.

use ratatui_core::buffer::{Buffer, CellWidth};
use ratatui_core::layout::Rect;
use ratatui_core::style::Style;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;

use crate::panel::{
    NO_DATA, Reduced, dropped_text, frames_or_message, panel_block, reduce_frames, render_message,
    spread,
};
use crate::{PanelData, PanelOptions, Theme, draws_a_panel};

/// The characters that fill a cell from the left, in eighths of a cell.
///
/// Unicode holds these characters from U+258F to U+2588. The last character
/// fills a whole cell. A bar of 4.6 cells therefore ends in a part of a cell,
/// and two values that lie close together look different.
const EIGHTHS: [char; 8] = ['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

/// The character that draws the part of the range that a bar does not reach.
///
/// A reader sees the whole range, and not the bar alone. This character also
/// keeps the two parts apart on a terminal that shows no colour.
const TRACK: char = '░';

/// The narrowest bar that the panel draws.
///
/// The panel drops the name of a series first, and then the text of a value,
/// to keep a bar of this width.
const MIN_BAR: usize = 3;

/// A panel that draws one bar for each series.
///
/// The panel reduces each series to one number, and it draws one row for each
/// series. A reader compares the rows and sees which series is the largest.
/// The gauge panel serves a small number of series, and this panel serves a
/// large number.
///
/// Each row holds the name of the series, the bar, and the text of the value.
/// The panel drops a part of a row when the panel is too narrow to hold that
/// part. It keeps the order of the series that the frames give, so that a row
/// stays in place when the data source answers again.
///
/// The panel takes frames. It does not take a data source. A program that
/// already holds its own data can therefore draw this panel, and needs no part
/// of thunderbox that runs a query.
///
/// # Examples
///
/// ```
/// use thunderbox_widgets::{BarGaugePanel, PanelData, Theme};
/// use ratatui_core::buffer::Buffer;
/// use ratatui_core::layout::Rect;
/// use ratatui_core::widgets::Widget;
///
/// let theme = Theme::default();
/// let area = Rect::new(0, 0, 20, 5);
/// let mut buf = Buffer::empty(area);
/// BarGaugePanel::new("load", PanelData::Frames(&[]), &theme).render(area, &mut buf);
/// ```
pub struct BarGaugePanel<'a> {
    title: &'a str,
    data: PanelData<'a>,
    theme: &'a Theme,
    options: PanelOptions<'a>,
    focused: bool,
    show_values: bool,
}

impl<'a> BarGaugePanel<'a> {
    /// Returns a panel that uses the default settings.
    pub fn new(title: &'a str, data: PanelData<'a>, theme: &'a Theme) -> Self {
        Self {
            title,
            data,
            theme,
            options: PanelOptions::default(),
            focused: false,
            show_values: true,
        }
    }

    /// Sets the unit, the number of decimal places, and the thresholds.
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

    /// States whether each row shows the text of its value.
    ///
    /// The panel gives the room of that text to the bar when the caller passes
    /// `false`. The default value is `true`.
    pub fn show_values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }
}

/// The width in cells of each part of one row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Parts {
    /// The number of cells that the name of the series takes.
    name: usize,
    /// The number of cells that the bar takes.
    bar: usize,
    /// The number of cells that the text of the value takes.
    value: usize,
}

/// Returns the width of each part of one row.
///
/// The `name` argument and the `value` argument state the width of the longest
/// name and of the longest text of a value. The name takes no more than a
/// third of the row.
///
/// The panel then holds a bar of [`MIN_BAR`] cells. It drops the name first,
/// and it drops the text of the value second. The panel never cuts the text of
/// a value, because a reader reads a cut number as a different number.
///
/// One cell of space follows the name, and one cell of space comes before the
/// text of the value. A part that has no width takes no space with it.
fn row_parts(width: usize, name: usize, value: usize) -> Parts {
    let name = name.min(width / 3);
    let fits = |name: usize, value: usize| {
        let used = name + value + usize::from(name > 0) + usize::from(value > 0);
        (used.saturating_add(MIN_BAR) <= width).then(|| Parts {
            name,
            bar: width - used,
            value,
        })
    };
    fits(name, value)
        .or_else(|| fits(0, value))
        .unwrap_or(Parts {
            name: 0,
            bar: width,
            value: 0,
        })
}

/// Returns the place of a value inside a range, as a ratio.
///
/// The ratio is a number from 0.0 to 1.0. It is 0.0 at the smallest bound of
/// the range, and 1.0 at the largest bound. A value below the smallest bound
/// gives 0.0, and a value above the largest bound gives 1.0.
///
/// The function returns 0.0 for a value that is not finite, and for a range
/// that has no width. A bar therefore stays empty where the panel knows no
/// place for the value.
///
/// The width of a range of two very large bounds does not fit in a number. The
/// function halves the value and each bound where that happens. The ratio of
/// the three halves is the ratio of the three numbers.
fn ratio_of(value: f64, min: f64, max: f64) -> f64 {
    let scale = if (max - min).is_finite() { 1.0 } else { 2.0 };
    let span = max / scale - min / scale;
    if !value.is_finite() || !span.is_finite() || span <= 0.0 {
        return 0.0;
    }
    let ratio = (value / scale - min / scale) / span;
    if ratio.is_nan() {
        return 0.0;
    }
    ratio.clamp(0.0, 1.0)
}

/// Returns the text of a bar.
///
/// The `ratio` argument states how much of `width` the bar fills. The function
/// draws a full cell for each whole cell of the bar. It then draws one
/// character from [`EIGHTHS`] for the part of a cell that follows. It returns
/// an empty text for a width of zero, and for a ratio that is not finite.
fn bar_text(ratio: f64, width: usize) -> String {
    if width == 0 || !ratio.is_finite() {
        return String::new();
    }
    let eighths = (ratio.clamp(0.0, 1.0) * (width * 8) as f64).round() as usize;
    let full = (eighths / 8).min(width);
    let rest = eighths % 8;
    let mut out = EIGHTHS[7].to_string().repeat(full);
    if rest > 0 && full < width {
        out.push(EIGHTHS[rest - 1]);
    }
    out
}

/// Returns the number of terminal cells that one character takes.
///
/// A terminal draws most characters in one cell. It draws a Japanese character
/// in two cells. The function asks ratatui for the width of each character
/// that Unicode holds above U+007F. It counts no cell for a control character,
/// and one cell for every other character.
fn cells_of(c: char) -> usize {
    if c.is_ascii() {
        // ratatui removes a control character before it draws a row.
        return usize::from(!c.is_ascii_control());
    }
    let mut buf = [0_u8; 4];
    usize::from(c.encode_utf8(&mut buf).cell_width())
}

/// Returns the number of terminal cells that a text takes.
///
/// A row of a terminal holds cells, and it does not hold characters. The panel
/// therefore measures each name and each value in cells. It measures each
/// character on its own, and it adds the widths.
fn cells(text: &str) -> usize {
    text.chars().map(cells_of).sum()
}

/// Returns the text, cut to `width` terminal cells.
///
/// The function counts cells, where the `clip` function of the panel module
/// counts characters. A name that holds a Japanese character therefore keeps
/// its row inside the panel. A row that is too wide loses its right end, and
/// that end carries the text of the value.
///
/// The function marks a text that it cut with one full stop, as `clip` does. A
/// reader meets the same mark in every panel.
fn clip_cells(text: &str, width: usize) -> String {
    if cells(text) <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    // The full stop takes the last cell. The text takes the cells before it.
    let mut out = String::new();
    let mut used = 1;
    for c in text.chars() {
        let next = used + cells_of(c);
        if next > width {
            break;
        }
        out.push(c);
        used = next;
    }
    out.push('.');
    out
}

/// Returns the text, and the spaces that follow it, in `width` cells.
///
/// The function adds no space to a text that is wider than `width`. A panel
/// draws the name of a series with this function, so that every bar starts in
/// the same column.
fn pad_after(text: &str, width: usize) -> String {
    let gap = " ".repeat(width.saturating_sub(cells(text)));
    format!("{text}{gap}")
}

/// Returns the spaces that come before a text, and the text, in `width` cells.
///
/// The function adds no space to a text that is wider than `width`. A panel
/// draws the text of a value with this function, so that every value ends in
/// the same column.
fn pad_before(text: &str, width: usize) -> String {
    let gap = " ".repeat(width.saturating_sub(cells(text)));
    format!("{gap}{text}")
}

impl Widget for BarGaugePanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self).render(area, buf);
    }
}

impl Widget for &BarGaugePanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // The panel measures each row against the width of the area. An area
        // wider than the buffer would push the end of a row past the border.
        let area = area.intersection(buf.area);
        if !draws_a_panel(area.as_size()) {
            return;
        }

        let block = panel_block(self.title, self.theme, self.focused);
        let frames = match frames_or_message(self.data, self.theme) {
            Ok(frames) => frames,
            Err((message, style)) => return render_message(block, message, style, area, buf),
        };

        // A value that is not finite has no place inside a range. The panel
        // drops that series, and it never draws a zero in place of it.
        let series: Vec<Reduced<'_>> = reduce_frames(frames, self.options.reduce)
            .into_iter()
            .filter(|s| s.value.is_finite())
            .collect();
        if series.is_empty() {
            return render_message(block, NO_DATA, self.theme.muted, area, buf);
        }

        let inner = block.inner(area);
        block.render(area, buf);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        // The last row reports the number of series that the panel dropped. A
        // panel of one row keeps that row for a bar, because a report on its
        // own holds no data.
        let rows = usize::from(inner.height);
        let shown = if series.len() > rows {
            rows.saturating_sub(1).max(1)
        } else {
            series.len()
        };
        let width = usize::from(inner.width);
        let range = spread(&series);

        let values: Vec<String> = if self.show_values {
            series[..shown]
                .iter()
                .map(|s| self.options.text(s.value, s.config))
                .collect()
        } else {
            Vec::new()
        };
        let parts = row_parts(
            width,
            series[..shown]
                .iter()
                .map(|s| cells(&s.name))
                .max()
                .unwrap_or(0),
            values.iter().map(|v| cells(v)).max().unwrap_or(0),
        );

        for (i, s) in series[..shown].iter().enumerate() {
            let (min, max) = self.options.bounds_for(s.config, range);
            let filled = bar_text(ratio_of(s.value, min, max), parts.bar);
            let track = TRACK
                .to_string()
                .repeat(parts.bar.saturating_sub(filled.chars().count()));
            let color =
                self.options
                    .color_for(s.value, s.config, self.theme, self.theme.series_color(i));

            let mut spans = Vec::with_capacity(5);
            if parts.name > 0 {
                let name = clip_cells(&s.name, parts.name);
                spans.push(Span::styled(pad_after(&name, parts.name), self.theme.text));
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(filled, Style::default().fg(color)));
            spans.push(Span::styled(track, self.theme.muted));
            if parts.value > 0 {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    pad_before(&values[i], parts.value),
                    self.theme.text,
                ));
            }
            let y = inner.y.saturating_add(i as u16);
            Line::from(spans).render(
                Rect {
                    y,
                    height: 1,
                    ..inner
                },
                buf,
            );
        }

        if shown < series.len() && shown < rows {
            let text = dropped_text(series.len() - shown, width);
            let y = inner.y.saturating_add(shown as u16);
            Line::from(Span::styled(text, self.theme.muted)).render(
                Rect {
                    y,
                    height: 1,
                    ..inner
                },
                buf,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::style::Color;
    use thunderbox_core::{Field, FieldConfig, Frame, Reduce, Threshold};

    /// Returns one frame that holds one column of numbers for each pair.
    fn frames(series: &[(&str, f64)]) -> Vec<Frame> {
        let mut fields = vec![Field::time("time", vec![0, 1000])];
        for (name, value) in series {
            fields.push(Field::f64(*name, vec![Some(*value), Some(*value)]));
        }
        vec![Frame::named("load", fields)]
    }

    /// Draws the panel into a plain buffer. This test needs no terminal
    /// backend and no terminal. That fact is the promise of this crate.
    fn render(panel: &BarGaugePanel<'_>, w: u16, h: u16) -> Buffer {
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

    /// Returns one row of the buffer, as text.
    fn row(buf: &Buffer, y: u16) -> String {
        (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect()
    }

    /// Returns the number of cells that the bar covers in one row.
    fn bar_cells(row: &str) -> usize {
        row.chars().filter(|c| EIGHTHS.contains(c)).count()
    }

    /// Returns the colour of the first full cell of the bar in one row.
    fn bar_color(buf: &Buffer, y: u16) -> Option<Color> {
        (0..buf.area.width)
            .map(|x| &buf[(x, y)])
            .find(|cell| cell.symbol() == "█")
            .map(|cell| cell.fg)
    }

    #[test]
    fn the_panel_shows_that_the_data_source_has_not_answered() {
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Loading, &theme);
        assert!(text(&render(&p, 30, 8)).contains("loading"));
    }

    #[test]
    fn the_panel_shows_the_text_of_an_error() {
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Error("upstream refused"), &theme);
        let out = text(&render(&p, 40, 8));
        assert!(
            out.contains("upstream refused"),
            "The panel drew this. {out}"
        );
        assert!(out.contains("load"), "The panel must still draw the title.");
    }

    #[test]
    fn the_panel_reports_that_it_has_no_data() {
        let theme = Theme::default();
        let empty: Vec<Frame> = vec![];
        let p = BarGaugePanel::new("load", PanelData::Frames(&empty), &theme);
        assert!(text(&render(&p, 30, 8)).contains("no data"));
    }

    #[test]
    fn the_panel_reports_no_data_for_a_column_of_holes() {
        let f = vec![Frame::named(
            "load",
            vec![
                Field::time("time", vec![0, 1000]),
                Field::f64("a", vec![None, None]),
            ],
        )];
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        assert!(text(&render(&p, 30, 8)).contains("no data"));
    }

    #[test]
    fn the_panel_reports_no_data_for_a_value_that_is_not_finite() {
        let f = frames(&[("a", f64::NAN), ("b", f64::INFINITY)]);
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 30, 8));
        assert!(out.contains("no data"), "The panel drew this. {out}");
    }

    #[test]
    fn the_panel_keeps_the_series_that_hold_a_number() {
        let f = frames(&[("a", f64::NAN), ("b", 50.0)]);
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let buf = render(&p, 30, 8);
        let out = text(&buf);
        assert!(!out.contains("no data"), "The panel drew this. {out}");
        assert!(out.contains('b'), "The panel drew this. {out}");
        assert_eq!(
            bar_cells(&row(&buf, 2)),
            0,
            "The panel must draw one row only. {out}"
        );
    }

    #[test]
    fn the_panel_draws_one_bar_for_each_series() {
        let f = frames(&[("a", 10.0), ("b", 50.0), ("c", 100.0)]);
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let buf = render(&p, 40, 6);
        let out = text(&buf);
        assert!(!out.contains("no data"), "The panel drew this. {out}");
        for name in ["a", "b", "c"] {
            assert!(out.contains(name), "The panel drew this. {out}");
        }
        assert!(
            bar_cells(&row(&buf, 1)) < bar_cells(&row(&buf, 2)),
            "A larger value must draw a longer bar. {out}"
        );
        assert!(
            bar_cells(&row(&buf, 2)) < bar_cells(&row(&buf, 3)),
            "A larger value must draw a longer bar. {out}"
        );
    }

    #[test]
    fn the_panel_draws_the_text_of_each_value() {
        let f = frames(&[("a", 10.0), ("c", 100.0)]);
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 40, 6));
        assert!(out.contains(" 10"), "The panel drew this. {out}");
        assert!(out.contains("100"), "The panel drew this. {out}");
    }

    #[test]
    fn the_panel_draws_the_range_that_a_bar_does_not_reach() {
        let f = frames(&[("a", 10.0), ("c", 100.0)]);
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let buf = render(&p, 40, 6);
        assert!(
            row(&buf, 1).contains(TRACK),
            "The small value must show the rest of the range. {}",
            row(&buf, 1)
        );
        assert!(
            !row(&buf, 2).contains(TRACK),
            "The largest value must fill the whole range. {}",
            row(&buf, 2)
        );
    }

    #[test]
    fn the_panel_ends_a_bar_inside_a_cell() {
        let f = frames(&[("a", 10.0), ("c", 100.0)]);
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let buf = render(&p, 40, 6);
        let first = row(&buf, 1);
        assert!(
            first.chars().any(|c| EIGHTHS[..7].contains(&c)),
            "The bar must end in a part of a cell. {first}"
        );
    }

    #[test]
    fn a_value_above_the_largest_bound_draws_a_full_bar() {
        let f = frames(&[("a", 400.0)]);
        let theme = Theme::default();
        let options = PanelOptions {
            min: Some(0.0),
            max: Some(100.0),
            ..Default::default()
        };
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme).options(options);
        let buf = render(&p, 40, 5);
        assert!(
            !row(&buf, 1).contains(TRACK),
            "The bar must fill the whole range. {}",
            row(&buf, 1)
        );
    }

    #[test]
    fn a_value_below_the_smallest_bound_draws_an_empty_bar() {
        let f = frames(&[("a", 10.0)]);
        let theme = Theme::default();
        let options = PanelOptions {
            min: Some(50.0),
            max: Some(100.0),
            ..Default::default()
        };
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme).options(options);
        let buf = render(&p, 40, 5);
        assert_eq!(
            bar_cells(&row(&buf, 1)),
            0,
            "The bar must stay empty. {}",
            row(&buf, 1)
        );
        assert!(
            row(&buf, 1).contains(TRACK),
            "The panel must still show the range. {}",
            row(&buf, 1)
        );
    }

    #[test]
    fn the_panel_reports_the_series_that_do_not_fit() {
        let many: Vec<(&str, f64)> = (0..10).map(|i| ("cpu", f64::from(i) + 1.0)).collect();
        let f = frames(&many);
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let buf = render(&p, 40, 6);
        let out = text(&buf);
        assert!(
            out.contains("7 series do not fit."),
            "The panel drew this. {out}"
        );
        assert_eq!(
            bar_cells(&row(&buf, 4)),
            0,
            "The last row holds the report, and no bar. {out}"
        );
    }

    #[test]
    fn a_narrow_panel_reports_the_series_that_do_not_fit_in_a_short_form() {
        let many: Vec<(&str, f64)> = (0..10).map(|i| ("cpu", f64::from(i) + 1.0)).collect();
        let f = frames(&many);
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 12, 6));
        assert!(out.contains("+7"), "The panel drew this. {out}");
    }

    #[test]
    fn show_values_gives_the_room_of_the_text_to_the_bar() {
        let f = frames(&[("a", 100.0)]);
        let theme = Theme::default();
        let with = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let without = BarGaugePanel::new("load", PanelData::Frames(&f), &theme).show_values(false);
        let wide = render(&without, 40, 5);
        let narrow = render(&with, 40, 5);
        assert!(
            !text(&wide).contains("100"),
            "The panel must hide the text of the value. {}",
            text(&wide)
        );
        assert!(
            bar_cells(&row(&wide, 1)) > bar_cells(&row(&narrow, 1)),
            "The bar must take the room of the text. {} {}",
            row(&wide, 1),
            row(&narrow, 1)
        );
    }

    #[test]
    fn the_panel_reads_the_thresholds_that_the_caller_states() {
        let f = frames(&[("a", 90.0)]);
        let theme = Theme::default();
        let list = [Threshold {
            value: 80.0,
            color: "critical".into(),
        }];
        let options = PanelOptions {
            thresholds: &list,
            ..Default::default()
        };
        let plain = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let hot = BarGaugePanel::new("load", PanelData::Frames(&f), &theme).options(options);
        assert_eq!(
            bar_color(&render(&plain, 40, 5), 1),
            Some(theme.series_color(0))
        );
        assert_eq!(bar_color(&render(&hot, 40, 5), 1), Some(theme.critical));
    }

    #[test]
    fn the_panel_reads_the_unit_that_the_caller_states() {
        let f = frames(&[("a", 42.0)]);
        let theme = Theme::default();
        let options = PanelOptions {
            unit: Some("percent"),
            ..Default::default()
        };
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme).options(options);
        let out = text(&render(&p, 40, 5));
        assert!(out.contains("42%"), "The panel drew this. {out}");
    }

    #[test]
    fn the_panel_reads_the_step_that_the_caller_states() {
        let f = vec![Frame::named(
            "load",
            vec![
                Field::time("time", vec![0, 1000]),
                Field::f64("a", vec![Some(1.0), Some(9.0)]),
            ],
        )];
        let theme = Theme::default();
        let options = PanelOptions {
            reduce: Reduce::Min,
            ..Default::default()
        };
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme).options(options);
        let out = text(&render(&p, 40, 5));
        assert!(out.contains('1'), "The panel drew this. {out}");
        assert!(!out.contains('9'), "The panel drew this. {out}");
    }

    #[test]
    fn the_panel_reads_the_bounds_of_the_column() {
        let f = vec![Frame::named(
            "load",
            vec![
                Field::time("time", vec![0, 1000]),
                Field::f64("a", vec![Some(50.0), Some(50.0)]).with_config(FieldConfig {
                    min: Some(0.0),
                    max: Some(50.0),
                    ..Default::default()
                }),
            ],
        )];
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let buf = render(&p, 40, 5);
        assert!(
            !row(&buf, 1).contains(TRACK),
            "The value reaches the largest bound of the column. {}",
            row(&buf, 1)
        );
    }

    #[test]
    fn the_panel_draws_nothing_in_an_area_that_holds_no_room() {
        let f = frames(&[("a", 1.0), ("b", 2.0)]);
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        for (w, h) in [(0, 0), (1, 1), (2, 2), (2, 20), (20, 2)] {
            let buf = render(&p, w, h);
            assert_eq!(
                text(&buf).trim(),
                "",
                "The panel must draw nothing in an area of {w} by {h} cells."
            );
        }
    }

    #[test]
    fn the_panel_draws_every_size_of_area_without_a_panic() {
        let theme = Theme::default();
        let many: Vec<(&str, f64)> = (0..12)
            .map(|i| {
                // A Japanese character takes two cells of a row. The panel
                // must hold a name of that text inside the row.
                let name = if i % 3 == 0 {
                    "日本語ですね"
                } else {
                    "throughput"
                };
                (name, f64::from(i))
            })
            .collect();
        for series in [&many[..1], &many[..3], &many[..]] {
            let f = frames(series);
            for data in [
                PanelData::Loading,
                PanelData::Error("upstream refused"),
                PanelData::Frames(&f),
            ] {
                for show in [true, false] {
                    let p = BarGaugePanel::new("load", data, &theme).show_values(show);
                    for (w, h) in [
                        (0, 0),
                        (3, 3),
                        (4, 3),
                        (8, 4),
                        (20, 3),
                        (40, 12),
                        (120, 40),
                        (1, 40),
                        (200, 1),
                    ] {
                        let _ = render(&p, w, h);
                    }
                }
            }
        }
    }

    #[test]
    fn bar_text_draws_nothing_for_a_ratio_of_zero() {
        assert_eq!(bar_text(0.0, 4), "");
    }

    #[test]
    fn bar_text_fills_every_cell_for_a_ratio_of_one() {
        assert_eq!(bar_text(1.0, 4), "████");
    }

    #[test]
    fn bar_text_ends_a_bar_inside_a_cell() {
        assert_eq!(bar_text(0.575, 8), "████▋");
        assert_eq!(bar_text(0.5, 3), "█▌");
        assert_eq!(bar_text(0.0625, 2), "▏");
    }

    #[test]
    fn bar_text_draws_nothing_in_a_width_of_zero() {
        assert_eq!(bar_text(0.0, 0), "");
        assert_eq!(bar_text(1.0, 0), "");
    }

    #[test]
    fn bar_text_holds_a_ratio_inside_the_range() {
        assert_eq!(bar_text(4.0, 3), "███");
        assert_eq!(bar_text(-2.0, 3), "");
    }

    #[test]
    fn bar_text_draws_nothing_for_a_ratio_that_is_not_finite() {
        assert_eq!(bar_text(f64::NAN, 4), "");
        assert_eq!(bar_text(f64::INFINITY, 4), "");
    }

    #[test]
    fn bar_text_never_draws_more_cells_than_the_width() {
        for width in 0..12 {
            for step in 0..=40 {
                let out = bar_text(f64::from(step) / 20.0, width);
                assert!(
                    out.chars().count() <= width,
                    "The bar holds {} cells in a width of {width}. {out}",
                    out.chars().count()
                );
            }
        }
    }

    #[test]
    fn ratio_of_returns_the_place_of_a_value_inside_a_range() {
        assert_eq!(ratio_of(0.0, 0.0, 100.0), 0.0);
        assert_eq!(ratio_of(25.0, 0.0, 100.0), 0.25);
        assert_eq!(ratio_of(100.0, 0.0, 100.0), 1.0);
        assert_eq!(ratio_of(0.0, -100.0, 100.0), 0.5);
    }

    #[test]
    fn ratio_of_holds_a_value_outside_the_range_at_a_bound() {
        assert_eq!(ratio_of(-5.0, 0.0, 100.0), 0.0);
        assert_eq!(ratio_of(500.0, 0.0, 100.0), 1.0);
    }

    #[test]
    fn ratio_of_returns_zero_for_a_range_that_has_no_width() {
        assert_eq!(ratio_of(7.0, 7.0, 7.0), 0.0);
        assert_eq!(ratio_of(7.0, 9.0, 1.0), 0.0);
    }

    #[test]
    fn ratio_of_returns_zero_for_a_value_that_is_not_finite() {
        assert_eq!(ratio_of(f64::NAN, 0.0, 1.0), 0.0);
        assert_eq!(ratio_of(f64::INFINITY, 0.0, 1.0), 0.0);
        assert_eq!(ratio_of(1.0, 0.0, f64::INFINITY), 0.0);
    }

    #[test]
    fn row_parts_gives_the_rest_of_the_row_to_the_bar() {
        assert_eq!(
            row_parts(18, 4, 3),
            Parts {
                name: 4,
                bar: 9,
                value: 3
            }
        );
    }

    #[test]
    fn row_parts_gives_the_name_no_more_than_a_third_of_the_row() {
        assert_eq!(row_parts(30, 20, 0).name, 10);
    }

    #[test]
    fn row_parts_drops_the_name_before_it_drops_the_text_of_the_value() {
        assert_eq!(
            row_parts(10, 6, 3),
            Parts {
                name: 0,
                bar: 6,
                value: 3
            }
        );
    }

    #[test]
    fn row_parts_drops_the_text_of_the_value_in_a_narrow_row() {
        assert_eq!(
            row_parts(6, 4, 4),
            Parts {
                name: 0,
                bar: 6,
                value: 0
            }
        );
    }

    #[test]
    fn row_parts_keeps_a_bar_in_a_row_of_one_cell() {
        assert_eq!(
            row_parts(1, 4, 4),
            Parts {
                name: 0,
                bar: 1,
                value: 0
            }
        );
    }

    #[test]
    fn row_parts_never_asks_for_more_cells_than_the_row_holds() {
        for width in 0..60 {
            for name in 0..14 {
                for value in 0..8 {
                    let p = row_parts(width, name, value);
                    let used = p.name
                        + p.bar
                        + p.value
                        + usize::from(p.name > 0)
                        + usize::from(p.value > 0);
                    assert_eq!(used, width, "The row is {width} cells wide. {p:?}");
                }
            }
        }
    }

    #[test]
    fn the_panel_shows_the_text_of_a_value_beside_a_name_of_japanese_text() {
        let f = frames(&[("語", 100.0)]);
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let buf = render(&p, 40, 5);
        let line = row(&buf, 1);
        assert!(
            line.contains("100"),
            "The panel must draw the whole number. {line}"
        );
    }

    #[test]
    fn a_name_of_japanese_text_keeps_the_order_of_the_bars() {
        let f = frames(&[("日本語ですね", 100.0), ("abcdef", 95.0)]);
        let theme = Theme::default();
        let options = PanelOptions {
            min: Some(0.0),
            max: Some(100.0),
            ..Default::default()
        };
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme).options(options);
        let buf = render(&p, 40, 6);
        let (first, second) = (row(&buf, 1), row(&buf, 2));
        assert!(
            bar_cells(&first) > bar_cells(&second),
            "The larger value must draw the longer bar. {first} {second}"
        );
        assert!(
            first.contains("100") && second.contains("95"),
            "The panel must draw both numbers. {first} {second}"
        );
    }

    #[test]
    fn a_name_of_japanese_text_starts_every_bar_in_the_same_column() {
        let f = frames(&[("日本語ですね", 20.0), ("abcdef", 30.0)]);
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let buf = render(&p, 40, 6);
        let start = |line: &str| {
            line.chars()
                .position(|c| c == TRACK || EIGHTHS.contains(&c))
        };
        assert_eq!(
            start(&row(&buf, 1)),
            start(&row(&buf, 2)),
            "Every bar starts in the same column. {} {}",
            row(&buf, 1),
            row(&buf, 2)
        );
    }

    #[test]
    fn a_name_of_japanese_text_keeps_the_row_inside_the_border() {
        let f = frames(&[("日本語ですね", 20.0), ("🔥svc", 30.0)]);
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let buf = render(&p, 30, 6);
        for (y, value) in [(1, "20"), (2, "30")] {
            assert_eq!(
                buf[(29, y)].symbol(),
                "│",
                "The row must not draw over the border. {}",
                row(&buf, y)
            );
            // A row that is too wide loses its right end, and the text of the
            // value sits at that end.
            assert!(
                row(&buf, y).contains(value),
                "The row must keep the text of its value. {}",
                row(&buf, y)
            );
        }
    }

    #[test]
    fn a_panel_of_one_row_draws_a_bar_and_not_a_report() {
        let f = frames(&[("a", 1.0), ("b", 2.0)]);
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let buf = render(&p, 20, 3);
        let out = text(&buf);
        assert!(
            bar_cells(&row(&buf, 1)) > 0,
            "The one row that the panel holds must draw a bar. {out}"
        );
        assert!(
            row(&buf, 1).contains('a'),
            "The panel must draw the first series. {out}"
        );
    }

    #[test]
    fn a_panel_of_two_rows_draws_a_bar_and_a_report() {
        let f = frames(&[("a", 1.0), ("b", 2.0), ("c", 3.0)]);
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let buf = render(&p, 30, 4);
        let out = text(&buf);
        assert!(
            bar_cells(&row(&buf, 1)) > 0,
            "The first row must draw a bar. {out}"
        );
        assert!(
            out.contains("2 series do not fit."),
            "The last row must report the two series. {out}"
        );
    }

    #[test]
    fn the_panel_draws_a_bar_for_a_range_of_the_largest_bounds() {
        let f = frames(&[("a", 1e308), ("b", -1e308)]);
        let theme = Theme::default();
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let buf = render(&p, 60, 6);
        let out = text(&buf);
        assert!(
            bar_cells(&row(&buf, 1)) > bar_cells(&row(&buf, 2)),
            "The largest value must draw the longer bar. {out}"
        );
        assert!(
            !row(&buf, 1).contains(TRACK),
            "The largest value must fill the whole range. {out}"
        );
    }

    #[test]
    fn the_panel_draws_inside_a_buffer_that_is_smaller_than_the_area() {
        let theme = Theme::default();
        let f = frames(&[("alpha", 10.0), ("beta", 100.0)]);
        let p = BarGaugePanel::new("load", PanelData::Frames(&f), &theme);
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 10));
        p.render(Rect::new(0, 0, 40, 6), &mut buf);
        for y in [1, 2] {
            assert_eq!(
                buf[(19, y)].symbol(),
                "│",
                "The row must stay inside the border. {}",
                row(&buf, y)
            );
        }
        assert!(
            text(&buf).contains("100"),
            "The panel must still draw the text of a value. {}",
            text(&buf)
        );
    }

    #[test]
    fn cells_counts_one_cell_for_each_letter_and_for_each_space() {
        assert_eq!(cells(""), 0);
        assert_eq!(cells("cpu"), 3);
        assert_eq!(cells("a b"), 3);
    }

    #[test]
    fn cells_counts_two_cells_for_a_japanese_character() {
        assert_eq!(cells("語"), 2);
        assert_eq!(cells("日本語"), 6);
        assert_eq!(cells("🔥svc"), 5);
    }

    #[test]
    fn cells_counts_no_cell_for_a_control_character() {
        assert_eq!(cells("\n"), 0);
        assert_eq!(cells("a\tb"), 2);
    }

    #[test]
    fn clip_cells_keeps_a_text_that_fits() {
        assert_eq!(clip_cells("cpu", 3), "cpu");
        assert_eq!(clip_cells("cpu", 8), "cpu");
        assert_eq!(clip_cells("日本", 4), "日本");
    }

    #[test]
    fn clip_cells_marks_a_text_that_it_cut() {
        assert_eq!(clip_cells("throughput", 5), "thro.");
        assert_eq!(clip_cells("throughput", 1), ".");
        assert_eq!(clip_cells("throughput", 0), "");
    }

    #[test]
    fn clip_cells_cuts_a_japanese_text_at_a_whole_character() {
        assert_eq!(clip_cells("日本語", 5), "日本.");
        assert_eq!(clip_cells("日本語", 4), "日.");
        assert_eq!(clip_cells("日本語", 2), ".");
    }

    #[test]
    fn clip_cells_never_returns_more_cells_than_the_width() {
        for text in ["", "cpu", "throughput", "日本語ですね", "🔥svc", "aｶﾞb"] {
            for width in 0..12 {
                let out = clip_cells(text, width);
                assert!(
                    cells(&out) <= width,
                    "The text takes {} cells in a width of {width}. {out}",
                    cells(&out)
                );
            }
        }
    }

    #[test]
    fn pad_after_fills_the_width_with_spaces() {
        assert_eq!(pad_after("cpu", 5), "cpu  ");
        assert_eq!(pad_after("語", 4), "語  ");
        assert_eq!(pad_after("cpu", 2), "cpu");
    }

    #[test]
    fn pad_before_fills_the_width_with_spaces() {
        assert_eq!(pad_before("42", 5), "   42");
        assert_eq!(pad_before("語", 4), "  語");
        assert_eq!(pad_before("42", 1), "42");
    }

    #[test]
    fn ratio_of_reads_a_range_of_the_largest_bounds() {
        assert_eq!(ratio_of(f64::MAX, f64::MIN, f64::MAX), 1.0);
        assert_eq!(ratio_of(f64::MIN, f64::MIN, f64::MAX), 0.0);
        assert_eq!(ratio_of(0.0, -1e308, 1e308), 0.5);
        assert!(ratio_of(1e308, -1e308, 1e308) > 0.9);
    }
}
