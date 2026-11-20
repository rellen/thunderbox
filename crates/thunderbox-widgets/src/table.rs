//! The panel that draws every value of the frames, as a table of rows and
//! columns.
//!
//! The panel shows the values themselves. It does not turn a column of numbers
//! into one number, so it ignores the step in the panel settings.
//!
//! A time series holds more rows than a panel can show. A person wants the most
//! recent rows. The panel therefore shows the last rows that fit. It keeps them
//! in the order of the data, so the newest row sits at the bottom. The panel
//! gives one row to the header, and it gives every other row to the data.
//!
//! Two frames can hold a different number of rows. The panel puts the last row
//! of each frame at the bottom. A frame that holds fewer rows therefore shows
//! its own most recent values. The panel fills the rows above those values with
//! the symbol of a value that is not there.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Alignment, Constraint, Rect};
use ratatui_core::style::{Color, Style};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;
use ratatui_widgets::table::{Cell, Row, Table};
use thunderbox_core::{Field, Frame, Values};

use crate::panel::{NO_DATA, clip, frames_or_message, panel_block, render_message};
use crate::{MISSING, PanelData, PanelOptions, Theme, draws_a_panel, format_time};

/// The smallest width that the panel gives to a column that needs more.
///
/// A reader can read almost nothing in a narrower column, because the panel
/// adds one full stop to a text that it cut. Four cells hold a short number
/// such as `1.5K`, or three letters of a name. The panel therefore drops a
/// column on the right before it makes such a column narrower than this width.
/// A column that needs fewer cells keeps the width that it needs.
const MIN_COLUMN: usize = 4;

/// The number of cells that the panel leaves between two columns.
const COLUMN_GAP: usize = 1;

/// A panel that draws every value of the frames, as a table of rows and
/// columns.
///
/// The panel takes frames. It does not take a data source. A program that
/// already holds its own data can therefore draw this panel, and needs no part
/// of thunderbox that runs a query.
pub struct TablePanel<'a> {
    /// The title that the border shows.
    title: &'a str,
    /// The data that the panel draws at this moment.
    data: PanelData<'a>,
    /// The colours that the panel uses.
    theme: &'a Theme,
    /// The settings that the panel uses to show a number.
    options: PanelOptions<'a>,
    /// The border changes colour when a person selects the panel.
    focused: bool,
}

impl<'a> TablePanel<'a> {
    /// Returns a panel that uses the default settings.
    pub fn new(title: &'a str, data: PanelData<'a>, theme: &'a Theme) -> Self {
        Self {
            title,
            data,
            theme,
            options: PanelOptions::default(),
            focused: false,
        }
    }

    /// Sets the unit, the number of decimal places, and the thresholds.
    ///
    /// The panel reads no step from these settings. It shows every value, and
    /// it turns no column into one number.
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
}

/// One cell of the table, and the style that the panel draws it with.
#[derive(Debug, Clone)]
struct Entry {
    /// The text that the panel draws.
    text: String,
    /// The style that the panel draws the text with.
    style: Style,
}

/// One column of the table.
#[derive(Debug, Clone)]
struct Column {
    /// The text of the header.
    header: String,
    /// One entry for each row that the panel shows.
    entries: Vec<Entry>,
    /// The panel puts a number at the right of its column. This field is `true`
    /// for a column of numbers.
    right: bool,
}

impl Column {
    /// Returns the width that the widest cell of the column needs.
    ///
    /// The method reads the header, and it reads the cells that the panel
    /// shows. It does not read a row that the panel drops.
    fn natural_width(&self) -> usize {
        let widest = self
            .entries
            .iter()
            .map(|e| display_width(&e.text))
            .max()
            .unwrap_or(0);
        widest.max(display_width(&self.header))
    }
}

/// Returns the number of cells that a text needs on a terminal.
///
/// One character can need two cells. A Chinese character is an example. A mark
/// that joins the character before it needs no cell. The panel measures a text
/// with the rule that the terminal uses. A column therefore holds the whole
/// text that the panel gives it.
fn display_width(text: &str) -> usize {
    Span::raw(text).width()
}

/// Returns the text, cut to `width` cells.
///
/// The function puts one full stop at the end of a text that it cut. A reader
/// therefore sees that the text goes on.
///
/// The shared function `clip` counts one character as one cell. That count is
/// right for a text in which each character needs one cell. This function calls
/// the shared function for such a text. It counts the cells itself for every
/// other text, because the terminal cuts a text by that measure.
fn clip_cells(text: &str, width: usize) -> String {
    let cells = display_width(text);
    if cells == text.chars().count() {
        return clip(text, width);
    }
    if width == 0 {
        return String::new();
    }
    if cells <= width {
        return text.to_string();
    }
    if width == 1 {
        return ".".to_string();
    }
    let mut kept = String::new();
    let mut used = 0;
    for c in text.chars() {
        let mut buffer = [0u8; 4];
        let need = display_width(c.encode_utf8(&mut buffer));
        if used + need > width - 1 {
            break;
        }
        kept.push(c);
        used += need;
    }
    kept.push('.');
    kept
}

/// The table that the panel draws.
#[derive(Debug, Clone)]
struct Sheet {
    /// The columns, in the order of the frames and then of the fields.
    columns: Vec<Column>,
    /// The number of rows that the panel shows. Every column holds this number
    /// of entries.
    rows: usize,
}

/// Returns the text without a control character.
///
/// A data source can return a text that holds a line break. Such a character
/// would damage the row that the panel draws. The function puts one space in
/// the place of each such character.
fn plain(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// Returns the cell that stands for a value that is not there.
fn missing_entry(theme: &Theme) -> Entry {
    Entry {
        text: MISSING.to_string(),
        style: theme.muted,
    }
}

/// Returns the cell of one column at one row.
///
/// The function returns the symbol of a value that is not there in each of
/// these conditions:
///
/// - the column holds a hole at this row;
/// - the value is not finite;
/// - the column is shorter, and it does not reach this row.
fn entry(field: &Field, row: usize, options: &PanelOptions<'_>, theme: &Theme) -> Entry {
    let missing = || missing_entry(theme);
    let plain_entry = |text: String| Entry {
        text,
        style: theme.text,
    };
    match &field.values {
        Values::Time(values) => match values.get(row) {
            Some(t) => plain_entry(format_time(*t)),
            None => missing(),
        },
        Values::F64(values) => match values.get(row).copied().flatten().filter(|v| v.is_finite()) {
            Some(v) => {
                let fallback = theme.text.fg.unwrap_or(Color::Reset);
                let color = options.color_for(v, &field.config, theme, fallback);
                Entry {
                    text: options.text(v, &field.config),
                    style: theme.text.fg(color),
                }
            }
            None => missing(),
        },
        Values::Str(values) => match values.get(row).and_then(Option::as_ref) {
            Some(text) => plain_entry(plain(text)),
            None => missing(),
        },
        Values::Bool(values) => match values.get(row).copied().flatten() {
            Some(true) => plain_entry("true".to_string()),
            Some(false) => plain_entry("false".to_string()),
            None => missing(),
        },
    }
}

/// Returns the table that the panel draws.
///
/// The function builds one column for each field of each frame, in the order of
/// the frames and then of the fields. The header of a column is the display
/// name of the field. The function puts the name of the frame and a full stop
/// in front of that name when more than one frame holds a column. Two fields
/// that carry the same name therefore stay apart.
///
/// The longest frame states the number of rows. The function puts the last row
/// of each frame at the bottom. A frame that holds fewer rows therefore shows
/// its own most recent values. The rows above those values hold the symbol of a
/// value that is not there. A short column inside a frame keeps the place of
/// the row that it holds. The other columns of that frame describe the same
/// rows, so the column must not move.
///
/// The `limit` argument states how many rows the panel can show. The function
/// builds the last `limit` rows only, and it keeps them in the order of the
/// data.
///
/// The function returns `None` when the frames hold no column, or when every
/// column holds no row. The panel then reports that it holds no data.
fn collect(
    frames: &[Frame],
    options: &PanelOptions<'_>,
    theme: &Theme,
    limit: usize,
) -> Option<Sheet> {
    let total = frames.iter().map(Frame::len).max().unwrap_or(0);
    if total == 0 {
        return None;
    }
    let rows = total.min(limit);
    let first = total - rows;
    let named = frames.iter().filter(|f| !f.fields.is_empty()).count() > 1;

    let mut columns = Vec::new();
    for frame in frames {
        // The frame ends at the last row of the panel. A frame that holds fewer
        // rows therefore starts later than the longest frame.
        let offset = total.saturating_sub(frame.len());
        for field in &frame.fields {
            let name = field.display_name();
            let header = match &frame.name {
                Some(frame_name) if named => format!("{frame_name}.{name}"),
                _ => name,
            };
            columns.push(Column {
                header: plain(&header),
                entries: (first..total)
                    .map(|row| match row.checked_sub(offset) {
                        Some(index) => entry(field, index, options, theme),
                        None => missing_entry(theme),
                    })
                    .collect(),
                right: matches!(field.values, Values::F64(_)),
            });
        }
    }
    Some(Sheet { columns, rows })
}

/// Returns the width of each column that the panel draws.
///
/// The rule has four parts:
///
/// - the function gives every column the width that it needs, when the panel is
///   wide enough for every column;
/// - the function shares the width in proportion when the panel is not wide
///   enough, and it gives each column [`MIN_COLUMN`] cells;
/// - the function gives a column that needs fewer cells the width that it
///   needs;
/// - the function drops the columns on the right that have no room.
///
/// The number of widths that the function returns is therefore the number of
/// columns that the panel draws. The list is empty when the panel is too narrow
/// for one column. The caller cuts each cell to the width that this function
/// returns.
fn share_widths(natural: &[usize], available: usize) -> Vec<usize> {
    if natural.is_empty() || available == 0 {
        return Vec::new();
    }
    let gaps = |count: usize| count.saturating_sub(1) * COLUMN_GAP;
    if natural.iter().sum::<usize>() + gaps(natural.len()) <= available {
        return natural.to_vec();
    }
    // The smallest width of a column is MIN_COLUMN, or the width that the
    // column needs when the column needs fewer cells. The panel keeps a column
    // while that smallest width and one gap still fit.
    let floor = |want: usize| want.min(MIN_COLUMN);
    let mut keep = 0;
    let mut used = 0;
    for want in natural {
        let cost = if keep == 0 {
            floor(*want)
        } else {
            COLUMN_GAP + floor(*want)
        };
        if used + cost > available {
            break;
        }
        used += cost;
        keep += 1;
    }
    if keep == 0 {
        return Vec::new();
    }
    let kept = &natural[..keep];
    let budget = available.saturating_sub(gaps(keep));
    let mut widths: Vec<usize> = kept.iter().map(|w| floor(*w)).collect();
    let spare = budget.saturating_sub(widths.iter().sum::<usize>());
    let extra: Vec<usize> = kept
        .iter()
        .zip(&widths)
        .map(|(want, small)| want.saturating_sub(*small))
        .collect();
    let total_extra: usize = extra.iter().sum();
    if total_extra == 0 {
        return widths;
    }
    for ((width, want), full) in widths.iter_mut().zip(&extra).zip(kept) {
        // No column grows past the width that it needs.
        *width = (*width + want * spare / total_extra).min(*full);
    }
    // The division above leaves a few cells over. The columns on the left take
    // them, and no column grows past the width that it needs.
    let mut left = budget.saturating_sub(widths.iter().sum::<usize>());
    for (width, want) in widths.iter_mut().zip(kept) {
        if left == 0 {
            break;
        }
        let give = want.saturating_sub(*width).min(left);
        *width += give;
        left -= give;
    }
    widths
}

/// Returns one cell of the table.
///
/// The function puts a number at the right of its column, because a reader
/// compares a column of numbers by its last digit. It puts every other value at
/// the left.
fn cell(text: String, style: Style, right: bool) -> Cell<'static> {
    let alignment = if right {
        Alignment::Right
    } else {
        Alignment::Left
    };
    Cell::new(Line::from(Span::styled(text, style)).alignment(alignment))
}

/// Returns the text of one cell, in a column of `width` cells.
///
/// The function cuts a text that is too long, and it marks the cut with one
/// full stop. It returns `None` for a number that is too long, because the
/// panel must not cut a number. A number that lost its last digits states a
/// different number, and a reader believes that number. The caller draws the
/// symbol of a value that is not there in the place of such a number.
fn cell_text(text: &str, width: usize, number: bool) -> Option<String> {
    if number && display_width(text) > width {
        return None;
    }
    Some(clip_cells(text, width))
}

/// Returns the text of a note that states how many columns the panel dropped.
///
/// The panel draws this note at the right of the header row. A reader then
/// knows that the panel holds more columns. The function returns a short form
/// for a narrow panel. It returns an empty text for a panel that has no room
/// for the note.
fn dropped_text(dropped: usize, width: usize) -> String {
    let long = if dropped == 1 {
        "1 column does not fit.".to_string()
    } else {
        format!("{dropped} columns do not fit.")
    };
    if display_width(&long) <= width {
        return long;
    }
    clip_cells(&format!("+{dropped}"), width)
}

/// Returns the width of each column, and the note about the columns that the
/// panel dropped.
///
/// The function takes the width of the note back from the widths that
/// [`share_widths`] returns. The note therefore has room. The function keeps
/// the columns when the note leaves room for none of them. A column of values
/// helps a reader more than the note does.
fn plan_widths(natural: &[usize], available: usize) -> (Vec<usize>, String) {
    let mut widths = share_widths(natural, available);
    if widths.len() >= natural.len() {
        return (widths, String::new());
    }
    let short = display_width(&format!("+{}", natural.len() - widths.len()));
    let narrow = share_widths(natural, available.saturating_sub(short + COLUMN_GAP));
    if !narrow.is_empty() {
        widths = narrow;
    }
    let used = widths.iter().sum::<usize>() + widths.len().saturating_sub(1) * COLUMN_GAP;
    let room = available.saturating_sub(used + COLUMN_GAP);
    let note = dropped_text(natural.len() - widths.len(), room);
    (widths, note)
}

impl Widget for TablePanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self).render(area, buf);
    }
}

impl Widget for &TablePanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !draws_a_panel(area.as_size()) {
            return;
        }

        let block = panel_block(self.title, self.theme, self.focused);
        let frames = match frames_or_message(self.data, self.theme) {
            Ok(frames) => frames,
            Err((message, style)) => return render_message(block, message, style, area, buf),
        };

        let inner = block.inner(area);
        // The header takes one row. The data takes every other row.
        let limit = usize::from(inner.height).saturating_sub(1);
        let Some(sheet) = collect(frames, &self.options, self.theme, limit) else {
            return render_message(block, NO_DATA, self.theme.muted, area, buf);
        };

        let natural: Vec<usize> = sheet.columns.iter().map(Column::natural_width).collect();
        let (widths, note) = plan_widths(&natural, usize::from(inner.width));
        if widths.is_empty() {
            // The panel is too narrow for one column. It draws the border and
            // the title, and it draws no column.
            return block.render(area, buf);
        }
        let columns = || widths.iter().zip(&sheet.columns);

        let header = Row::new(
            columns()
                .map(|(width, column)| {
                    let text = clip_cells(&column.header, *width);
                    cell(text, self.theme.title, column.right)
                })
                .collect::<Vec<Cell<'_>>>(),
        )
        .style(self.theme.title);

        let rows: Vec<Row<'_>> = (0..sheet.rows)
            .map(|row| {
                Row::new(
                    columns()
                        .map(|(width, column)| {
                            let Some(found) = column.entries.get(row) else {
                                return cell(String::new(), self.theme.text, column.right);
                            };
                            match cell_text(&found.text, *width, column.right) {
                                Some(text) => cell(text, found.style, column.right),
                                // The number does not fit. The panel draws the
                                // symbol of a value that it cannot show.
                                None => cell(
                                    clip_cells(MISSING, *width),
                                    self.theme.muted,
                                    column.right,
                                ),
                            }
                        })
                        .collect::<Vec<Cell<'_>>>(),
                )
            })
            .collect();

        let constraints: Vec<Constraint> = widths
            .iter()
            .map(|w| Constraint::Length(u16::try_from(*w).unwrap_or(u16::MAX)))
            .collect();

        Table::new(rows, constraints)
            .header(header)
            .block(block)
            .column_spacing(u16::try_from(COLUMN_GAP).unwrap_or(1))
            .render(area, buf);

        if !note.is_empty() {
            // The note sits at the right of the header row, in the cells that
            // the columns left free.
            Line::from(Span::styled(note, self.theme.muted))
                .alignment(Alignment::Right)
                .render(Rect { height: 1, ..inner }, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thunderbox_core::{FieldConfig, Threshold};

    /// Returns a list that holds one frame. The frame holds a time column and
    /// one column of numbers.
    fn frames(values: Vec<Option<f64>>) -> Vec<Frame> {
        let stamps: Vec<i64> = (0..values.len() as i64).map(|i| i * 1000).collect();
        vec![Frame::named(
            "cpu",
            vec![Field::time("time", stamps), Field::f64("usage", values)],
        )]
    }

    /// Draws the panel into a plain buffer. This test needs no terminal backend
    /// and no terminal. That fact is the promise of this crate.
    fn render(panel: &TablePanel<'_>, w: u16, h: u16) -> Buffer {
        let area = Rect::new(0, 0, w, h);
        let mut buf = Buffer::empty(area);
        panel.render(area, &mut buf);
        buf
    }

    /// Draws the panel at a place inside a larger buffer, and checks that the
    /// panel wrote inside its own area only.
    ///
    /// A dashboard gives every panel one part of one buffer. A panel that
    /// writes outside its area damages the panel beside it. The buffer of the
    /// helper above hides such a fault, because ratatui drops a write that
    /// falls outside the buffer.
    fn render_beside_a_neighbour(panel: &TablePanel<'_>, w: u16, h: u16) {
        let mut sentinel = ratatui_core::buffer::Cell::default();
        sentinel.set_char('#');
        let outer = Rect::new(0, 0, w + 10, h + 10);
        let mut buf = Buffer::filled(outer, sentinel);
        let area = Rect::new(3, 4, w, h);
        panel.render(area, &mut buf);
        for y in outer.y..outer.bottom() {
            for x in outer.x..outer.right() {
                if area.contains(ratatui_core::layout::Position::new(x, y)) {
                    continue;
                }
                assert_eq!(
                    buf[(x, y)].symbol(),
                    "#",
                    "The panel wrote outside its area at the column {x} and the row {y}."
                );
            }
        }
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
        let p = TablePanel::new("CPU", PanelData::Loading, &theme);
        assert!(text(&render(&p, 30, 8)).contains("loading"));
    }

    #[test]
    fn the_panel_shows_the_text_of_an_error() {
        let theme = Theme::default();
        let p = TablePanel::new("CPU", PanelData::Error("upstream refused"), &theme);
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
        let p = TablePanel::new("CPU", PanelData::Frames(&empty), &theme);
        assert!(text(&render(&p, 30, 8)).contains("no data"));
    }

    #[test]
    fn the_panel_reports_no_data_for_a_frame_that_holds_no_column() {
        let theme = Theme::default();
        let f = vec![Frame::new(vec![])];
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        assert!(text(&render(&p, 30, 8)).contains("no data"));
    }

    #[test]
    fn the_panel_reports_no_data_for_a_column_that_holds_no_row() {
        let theme = Theme::default();
        let f = vec![Frame::new(vec![Field::f64("usage", vec![])])];
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        assert!(text(&render(&p, 30, 8)).contains("no data"));
    }

    #[test]
    fn the_panel_draws_the_header_and_the_values() {
        let theme = Theme::default();
        let f = frames(vec![Some(10.0), Some(42.0)]);
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 30, 6));
        assert!(!out.contains("no data"), "The panel holds data. {out}");
        assert!(out.contains("time"), "The panel must draw a header. {out}");
        assert!(out.contains("usage"), "The panel must draw a header. {out}");
        assert!(
            out.contains("00:00:00"),
            "The panel must draw a time. {out}"
        );
        assert!(
            out.contains("00:00:01"),
            "The panel must draw a time. {out}"
        );
        assert!(out.contains("10"), "The panel must draw a number. {out}");
        assert!(out.contains("42"), "The panel must draw a number. {out}");
    }

    #[test]
    fn the_panel_shows_the_last_rows_and_puts_the_newest_row_at_the_bottom() {
        let theme = Theme::default();
        let f = frames(vec![Some(1.0), Some(2.0), Some(3.0), Some(4.0)]);
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        // The panel is five cells tall. The border takes two rows, and the
        // header takes one row. Two rows of data therefore fit.
        let out = text(&render(&p, 30, 5));
        assert!(
            !out.contains("00:00:00") && !out.contains("00:00:01"),
            "The panel must drop the oldest rows. {out}"
        );
        let lines: Vec<&str> = out.lines().collect();
        assert!(
            lines[2].contains("00:00:02"),
            "The older row must sit above. {out}"
        );
        assert!(
            lines[3].contains("00:00:03"),
            "The newest row must sit at the bottom. {out}"
        );
    }

    #[test]
    fn the_panel_draws_the_header_alone_when_one_row_fits() {
        let theme = Theme::default();
        let f = frames(vec![Some(1.0)]);
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 30, 3));
        assert!(out.contains("usage"), "The header must stay. {out}");
        assert!(
            !out.contains("00:00:00"),
            "No row of data fits, so the panel draws none. {out}"
        );
    }

    #[test]
    fn the_panel_draws_a_column_for_each_field_of_each_frame() {
        let theme = Theme::default();
        let f = vec![
            Frame::named("a", vec![Field::f64("up", vec![Some(1.0)])]),
            Frame::named("b", vec![Field::f64("up", vec![Some(2.0)])]),
        ];
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 30, 5));
        assert!(
            out.contains("a.up") && out.contains("b.up"),
            "The name of the frame must keep two headers apart. {out}"
        );
    }

    #[test]
    fn the_panel_keeps_the_header_of_one_frame_short() {
        let theme = Theme::default();
        let f = frames(vec![Some(1.0)]);
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 30, 5));
        assert!(
            !out.contains("cpu.usage"),
            "One frame needs no name in front of the header. {out}"
        );
        assert!(out.contains("usage"), "{out}");
    }

    #[test]
    fn the_panel_draws_the_symbol_for_a_hole_and_for_a_value_that_is_not_finite() {
        let theme = Theme::default();
        let f = frames(vec![None, Some(f64::NAN)]);
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 30, 6));
        assert!(
            !out.contains("no data"),
            "The time column holds values. {out}"
        );
        assert_eq!(
            out.matches(MISSING).count(),
            2,
            "The panel must draw one symbol for each value that is not there. {out}"
        );
        assert!(
            !out.contains(" 0"),
            "The panel must never draw a zero in place of a hole. {out}"
        );
    }

    #[test]
    fn the_panel_draws_a_symbol_for_a_column_that_holds_only_holes() {
        let theme = Theme::default();
        let f = vec![Frame::new(vec![Field::f64("usage", vec![None, None])])];
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 30, 6));
        assert!(
            out.contains(MISSING),
            "The column holds two rows, and both are holes. {out}"
        );
        assert!(
            !out.contains("no data"),
            "The column holds two rows, so the panel draws two rows. {out}"
        );
    }

    #[test]
    fn the_panel_fills_a_short_column_with_the_symbol() {
        let theme = Theme::default();
        let f = vec![Frame::new(vec![
            Field::f64("long", vec![Some(1.0), Some(2.0)]),
            Field::f64("short", vec![Some(3.0)]),
        ])];
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 30, 6));
        assert_eq!(
            out.matches(MISSING).count(),
            1,
            "The short column must show one symbol. {out}"
        );
    }

    #[test]
    fn the_panel_draws_the_words_true_and_false() {
        let theme = Theme::default();
        let f = vec![Frame::new(vec![Field {
            name: "up".into(),
            values: Values::Bool(vec![Some(true), Some(false), None]),
            ..Default::default()
        }])];
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 30, 7));
        assert!(out.contains("true") && out.contains("false"), "{out}");
        assert!(out.contains(MISSING), "A hole keeps its symbol. {out}");
    }

    #[test]
    fn the_panel_draws_a_text_value() {
        let theme = Theme::default();
        let f = vec![Frame::new(vec![Field {
            name: "host".into(),
            values: Values::Str(vec![Some("alpha".into()), None]),
            ..Default::default()
        }])];
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 30, 6));
        assert!(out.contains("alpha"), "{out}");
        assert!(out.contains(MISSING), "A hole keeps its symbol. {out}");
    }

    #[test]
    fn the_panel_puts_a_number_at_the_right_of_its_column() {
        let theme = Theme::default();
        let f = vec![Frame::new(vec![Field::f64(
            "x",
            vec![Some(1500.0), Some(5.0)],
        )])];
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        // The widest value is four cells wide, so the column is four cells
        // wide, and the panel needs six cells with the border.
        let out = text(&render(&p, 6, 5));
        assert!(out.contains("1.5K"), "{out}");
        assert!(
            out.lines().any(|line| line.contains("   5")),
            "The panel must put the short number at the right. {out}"
        );
    }

    #[test]
    fn the_panel_reads_the_unit_of_the_panel_and_not_of_the_column() {
        let theme = Theme::default();
        let f = vec![Frame::new(vec![
            Field::f64("x", vec![Some(42.0)]).with_config(FieldConfig {
                unit: Some("bytes".into()),
                ..Default::default()
            }),
        ])];
        let options = PanelOptions {
            unit: Some("percent"),
            ..Default::default()
        };
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme).options(options);
        let out = text(&render(&p, 20, 5));
        assert!(out.contains("42%"), "The panel wins over the column. {out}");
    }

    #[test]
    fn the_panel_colours_a_number_that_reaches_a_threshold() {
        let theme = Theme::default();
        let f = frames(vec![Some(95.0)]);
        let list = [Threshold {
            value: 90.0,
            color: "critical".into(),
        }];
        let options = PanelOptions {
            thresholds: &list,
            ..Default::default()
        };
        let plain_panel = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        let hot = TablePanel::new("CPU", PanelData::Frames(&f), &theme).options(options);
        let red = |buf: &Buffer| {
            buf.content
                .iter()
                .any(|c| c.fg == theme.critical && c.symbol() != " ")
        };
        assert!(
            !red(&render(&plain_panel, 30, 5)),
            "The panel names no threshold, so no cell is red."
        );
        assert!(
            red(&render(&hot, 30, 5)),
            "The value reaches the threshold, so the cell is red."
        );
    }

    #[test]
    fn the_panel_drops_a_column_that_has_no_room() {
        let theme = Theme::default();
        let fields: Vec<Field> = (0..8)
            .map(|i| Field::f64(format!("column{i}"), vec![Some(f64::from(i))]))
            .collect();
        let f = vec![Frame::new(fields)];
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 14, 4));
        assert!(out.contains("col"), "The first column must stay. {out}");
        assert!(
            !out.contains("column7"),
            "The last column has no room. {out}"
        );
    }

    #[test]
    fn the_panel_states_the_number_of_columns_that_it_dropped() {
        let theme = Theme::default();
        let fields: Vec<Field> = (0..500)
            .map(|i| Field::f64(format!("m{i}"), vec![Some(f64::from(i))]))
            .collect();
        let f = vec![Frame::new(fields)];
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 30, 5));
        let header = out.lines().nth(1).unwrap_or_default().to_string();
        assert!(
            header.contains('+'),
            "The panel must state the columns that it dropped. {out}"
        );
        assert!(
            header.contains("49"),
            "The note must state how many columns the panel dropped. {out}"
        );
        assert!(out.contains("m0"), "The first column must stay. {out}");
    }

    #[test]
    fn the_panel_keeps_a_narrow_column_that_still_fits() {
        let theme = Theme::default();
        let column = |name: &str, value: &str| Field {
            name: name.into(),
            values: Values::Str(vec![Some(value.into())]),
            ..Default::default()
        };
        let f = vec![Frame::new(vec![
            column("a", "x"),
            column("b", "y"),
            column("c", "hello world"),
        ])];
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        // The panel holds nine cells inside its border. The three columns need
        // one, one, and four cells, and the two gaps need two cells.
        let out = text(&render(&p, 11, 5));
        assert!(
            out.contains('c'),
            "The third column needs four cells, and it fits. {out}"
        );
        assert!(
            out.contains("hell"),
            "The panel must draw the value of the third column. {out}"
        );
    }

    #[test]
    fn the_panel_draws_no_number_that_it_cannot_show_in_full() {
        let theme = Theme::default();
        let f = vec![Frame::new(vec![
            Field::f64("aaaa", vec![Some(98_765.0)]),
            Field::f64("bbbb", vec![Some(98_765.0)]),
        ])];
        let options = PanelOptions {
            unit: Some("percent"),
            decimals: Some(0),
            ..Default::default()
        };
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme).options(options);
        // Each column holds four cells, and the value needs six.
        let out = text(&render(&p, 11, 5));
        assert!(
            !out.contains("987"),
            "A number that lost its last digits states a different number. {out}"
        );
        assert_eq!(
            out.matches(MISSING).count(),
            2,
            "The panel must draw the symbol of a value that it cannot show. {out}"
        );
    }

    #[test]
    fn the_panel_gives_a_wide_character_the_cells_that_it_needs() {
        let theme = Theme::default();
        let f = vec![Frame::new(vec![Field {
            name: "host".into(),
            values: Values::Str(vec![Some("北京机房一号".into()), Some("東京".into())]),
            ..Default::default()
        }])];
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 30, 6));
        // A character that needs two cells fills the cell beside it with a
        // space. The test removes each space, and it then reads the value.
        let row = out.lines().nth(2).unwrap_or_default().replace(' ', "");
        assert!(
            row.contains("北京机房一号"),
            "Each of the six characters needs two cells, and the panel holds them. {out}"
        );
    }

    #[test]
    fn the_panel_shows_the_last_rows_of_a_frame_that_holds_fewer_rows() {
        let theme = Theme::default();
        let long: Vec<Option<f64>> = (0..100).map(|i| Some(f64::from(i))).collect();
        let f = vec![
            Frame::named("a", vec![Field::f64("v", long)]),
            Frame::named(
                "b",
                vec![Field::f64(
                    "v",
                    vec![Some(7000.0), Some(8000.0), Some(9000.0)],
                )],
            ),
        ];
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        // The panel holds five rows of data. The short frame holds three rows,
        // and those three rows sit at the bottom.
        let out = text(&render(&p, 24, 8));
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[4].contains("7K"), "{out}");
        assert!(lines[5].contains("8K"), "{out}");
        assert!(lines[6].contains("9K"), "{out}");
        assert_eq!(
            out.matches(MISSING).count(),
            2,
            "The short frame reaches no value in the two rows above. {out}"
        );
    }

    #[test]
    fn the_panel_cuts_a_name_that_is_too_long() {
        let theme = Theme::default();
        let f = vec![Frame::new(vec![Field::f64(
            "a_very_long_column_name",
            vec![Some(1.0)],
        )])];
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 12, 4));
        assert!(
            out.contains("a_very_lo."),
            "The panel must mark the name that it cut. {out}"
        );
    }

    #[test]
    fn the_panel_draws_the_border_alone_when_no_column_fits() {
        let theme = Theme::default();
        let f = frames(vec![Some(1.0)]);
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 4, 4));
        assert!(!out.contains("no data"), "The panel holds data. {out}");
        assert!(
            !out.contains("0"),
            "No column fits, so the panel draws no value. {out}"
        );
    }

    #[test]
    fn the_panel_draws_nothing_in_a_small_area() {
        let f = frames(vec![Some(1.0), Some(2.0)]);
        let theme = Theme::default();
        let p = TablePanel::new("CPU", PanelData::Frames(&f), &theme);
        for (w, h) in [(0, 0), (1, 1), (2, 2), (2, 20), (20, 2)] {
            let out = text(&render(&p, w, h));
            assert!(out.chars().all(|c| c == ' ' || c == '\n'), "{out}");
        }
    }

    #[test]
    fn the_panel_draws_every_size_of_area_without_a_panic() {
        let theme = Theme::default();
        let long: Vec<Option<f64>> = (0..500).map(|i| Some(f64::from(i))).collect();
        let mut fields = vec![Field::time("time", (0..500).map(|i| i * 1000).collect())];
        fields.push(Field::f64("usage", long));
        fields.push(Field {
            name: "host".into(),
            values: Values::Str(vec![Some("alpha".into()); 500]),
            ..Default::default()
        });
        fields.push(Field {
            name: "up".into(),
            values: Values::Bool(vec![Some(true); 3]),
            ..Default::default()
        });
        let f = vec![
            Frame::named("first", fields),
            Frame::named("second", vec![Field::f64("empty", vec![])]),
        ];
        let p = TablePanel::new(
            "A table with a very long title",
            PanelData::Frames(&f),
            &theme,
        )
        .focused(true);
        for (w, h) in [
            (0, 0),
            (3, 3),
            (4, 3),
            (5, 3),
            (8, 4),
            (20, 3),
            (40, 12),
            (120, 40),
            (200, 1),
            (1, 200),
        ] {
            let _ = render(&p, w, h);
        }
        // Every small size follows. A panel that a person makes smaller passes
        // through each of them. The panel draws beside a neighbour at each
        // size, so that the test also finds a write outside the area.
        for w in 0..40_u16 {
            for h in 0..10_u16 {
                let _ = render(&p, w, h);
                render_beside_a_neighbour(&p, w, h);
            }
        }
    }

    #[test]
    fn the_panel_writes_no_cell_outside_its_area() {
        let theme = Theme::default();
        let f = vec![
            Frame::named(
                "first",
                vec![
                    Field::time("time", (0..40).map(|i| i * 1000).collect()),
                    Field::f64("usage", (0..40).map(|i| Some(f64::from(i))).collect()),
                ],
            ),
            Frame::named("second", vec![Field::f64("short", vec![Some(1.0)])]),
        ];
        let p = TablePanel::new("A table", PanelData::Frames(&f), &theme).focused(true);
        for (w, h) in [(3, 3), (4, 3), (8, 4), (11, 5), (20, 3), (40, 12), (45, 12)] {
            render_beside_a_neighbour(&p, w, h);
        }
    }

    #[test]
    fn collect_returns_none_for_frames_that_hold_no_row() {
        let theme = Theme::default();
        let options = PanelOptions::default();
        assert!(collect(&[], &options, &theme, 10).is_none());
        let f = vec![Frame::new(vec![Field::f64("x", vec![])])];
        assert!(collect(&f, &options, &theme, 10).is_none());
    }

    #[test]
    fn collect_builds_the_last_rows_only() {
        let theme = Theme::default();
        let options = PanelOptions::default();
        let f = frames(vec![Some(1.0), Some(2.0), Some(3.0)]);
        let sheet = collect(&f, &options, &theme, 2).expect("The frames hold rows.");
        assert_eq!(sheet.rows, 2);
        assert_eq!(sheet.columns.len(), 2);
        assert_eq!(sheet.columns[1].entries[0].text, "2");
        assert_eq!(sheet.columns[1].entries[1].text, "3");
    }

    #[test]
    fn collect_builds_no_row_for_a_panel_that_shows_none() {
        let theme = Theme::default();
        let options = PanelOptions::default();
        let f = frames(vec![Some(1.0)]);
        let sheet = collect(&f, &options, &theme, 0).expect("The frames hold rows.");
        assert_eq!(sheet.rows, 0);
        assert_eq!(
            sheet.columns[0].natural_width(),
            4,
            "The header is four cells wide."
        );
    }

    #[test]
    fn natural_width_reads_the_header_and_the_widest_cell() {
        let theme = Theme::default();
        let options = PanelOptions::default();
        let f = vec![Frame::new(vec![Field::f64("x", vec![Some(1500.0)])])];
        let sheet = collect(&f, &options, &theme, 4).expect("The frames hold rows.");
        assert_eq!(sheet.columns[0].natural_width(), 4, "The cell is 1.5K.");
    }

    #[test]
    fn share_widths_gives_each_column_the_width_that_it_needs() {
        assert_eq!(share_widths(&[4, 6, 5], 40), vec![4, 6, 5]);
        // The three columns and the two gaps need 17 cells.
        assert_eq!(share_widths(&[4, 6, 5], 17), vec![4, 6, 5]);
    }

    #[test]
    fn share_widths_shares_the_width_in_proportion() {
        let out = share_widths(&[10, 20], 15);
        assert_eq!(out.len(), 2);
        assert_eq!(
            out.iter().sum::<usize>(),
            14,
            "One cell is the gap. {out:?}"
        );
        assert!(
            out[1] > out[0],
            "The wider column keeps more cells. {out:?}"
        );
    }

    #[test]
    fn share_widths_gives_each_column_that_it_keeps_the_smallest_width() {
        let out = share_widths(&[20, 20, 20, 20], 14);
        assert!(
            out.iter().all(|w| *w >= MIN_COLUMN),
            "No column is narrower than the smallest width. {out:?}"
        );
        assert_eq!(out.len(), 3, "Three columns fit in fourteen cells. {out:?}");
    }

    #[test]
    fn share_widths_drops_the_columns_on_the_right() {
        // One column takes four cells. Two columns take nine cells, because one
        // cell is the gap between them.
        assert_eq!(share_widths(&[8, 8, 8], 4), vec![4]);
        assert_eq!(share_widths(&[8, 8, 8], 8), vec![8]);
        assert_eq!(share_widths(&[8, 8, 8], 9), vec![4, 4]);
        assert_eq!(share_widths(&[8, 8, 8], 14).len(), 3);
    }

    #[test]
    fn share_widths_returns_nothing_for_a_panel_that_is_too_narrow() {
        assert!(share_widths(&[8, 8], 3).is_empty());
        assert!(share_widths(&[8, 8], 0).is_empty());
        assert!(share_widths(&[], 40).is_empty());
    }

    #[test]
    fn share_widths_keeps_a_narrow_column_that_costs_less_than_the_smallest_width() {
        // The three columns need one, one, and eight cells. The two narrow
        // columns cost one cell each, so the third column has room.
        let out = share_widths(&[1, 1, 8], 9);
        assert_eq!(out.len(), 3, "Every column fits. {out:?}");
        assert_eq!(out[0], 1, "A column grows no wider than it needs. {out:?}");
        let used = out.iter().sum::<usize>() + out.len().saturating_sub(1) * COLUMN_GAP;
        assert!(used <= 9, "The columns must fit. {out:?}");
    }

    #[test]
    fn share_widths_gives_no_column_more_than_it_needs() {
        // The first column needs five cells, and the second needs twenty. Only
        // the first column fits, and it keeps five of the seven cells.
        assert_eq!(share_widths(&[5, 20], 7), vec![5]);
    }

    #[test]
    fn share_widths_never_returns_more_than_the_width_of_the_panel() {
        for available in 0..60_usize {
            for count in 1..8_usize {
                for width in [1_usize, 3, 7, 30] {
                    let natural = vec![width; count];
                    let out = share_widths(&natural, available);
                    let used = out.iter().sum::<usize>() + out.len().saturating_sub(1) * COLUMN_GAP;
                    assert!(
                        used <= available,
                        "The columns must fit. {out:?} {available} {count} {width}"
                    );
                    assert!(
                        out.iter().zip(&natural).all(|(got, want)| got <= want),
                        "No column grows past the width that it needs. {out:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn plan_widths_leaves_room_for_the_note() {
        let natural = vec![7; 8];
        let (widths, note) = plan_widths(&natural, 20);
        assert!(!note.is_empty(), "The panel drops a column. {widths:?}");
        let used = widths.iter().sum::<usize>() + widths.len().saturating_sub(1) * COLUMN_GAP;
        assert!(
            used + COLUMN_GAP + display_width(&note) <= 20,
            "The note must not cover a column. {widths:?} {note}"
        );
    }

    #[test]
    fn plan_widths_writes_no_note_when_every_column_fits() {
        let (widths, note) = plan_widths(&[4, 5], 20);
        assert_eq!(widths, vec![4, 5]);
        assert!(note.is_empty(), "No column is missing. {note}");
    }

    #[test]
    fn plan_widths_keeps_a_column_when_the_note_leaves_no_room() {
        // The one column needs four cells, and the panel holds five. The note
        // would leave no room for the column, so the column stays.
        let (widths, _) = plan_widths(&[4, 4], 5);
        assert_eq!(
            widths,
            vec![4],
            "The column helps a reader more. {widths:?}"
        );
    }

    #[test]
    fn dropped_text_states_the_number_of_columns() {
        assert_eq!(dropped_text(1, 30), "1 column does not fit.");
        assert_eq!(dropped_text(4, 30), "4 columns do not fit.");
    }

    #[test]
    fn dropped_text_returns_a_short_form_for_a_narrow_panel() {
        assert_eq!(dropped_text(4, 21), "4 columns do not fit.");
        assert_eq!(dropped_text(4, 20), "+4");
        assert_eq!(dropped_text(12, 2), "+.");
        assert_eq!(dropped_text(4, 0), "");
    }

    #[test]
    fn cell_text_cuts_a_name_and_marks_the_cut() {
        assert_eq!(cell_text("hostname", 4, false).as_deref(), Some("hos."));
        assert_eq!(cell_text("host", 4, false).as_deref(), Some("host"));
    }

    #[test]
    fn cell_text_returns_nothing_for_a_number_that_does_not_fit() {
        assert_eq!(cell_text("98765%", 6, true).as_deref(), Some("98765%"));
        assert_eq!(cell_text("98765%", 4, true), None);
        assert_eq!(cell_text("-1.2M", 4, true), None);
    }

    #[test]
    fn display_width_counts_the_cells_of_a_character() {
        assert_eq!(display_width("host"), 4);
        assert_eq!(display_width("東京"), 4, "Each character needs two cells.");
        assert_eq!(display_width(MISSING), 1, "The symbol needs one cell.");
    }

    #[test]
    fn clip_cells_keeps_a_text_that_fits() {
        assert_eq!(clip_cells("cpu", 3), "cpu");
        assert_eq!(clip_cells("cpu", 8), "cpu");
        assert_eq!(clip_cells("東京", 4), "東京");
    }

    #[test]
    fn clip_cells_cuts_a_text_by_the_cells_that_it_needs() {
        assert_eq!(clip_cells("throughput", 5), "thro.");
        assert_eq!(clip_cells("throughput", 1), ".");
        assert_eq!(clip_cells("throughput", 0), "");
        // The two characters need four cells. Three cells hold one character
        // and the full stop.
        assert_eq!(clip_cells("東京", 3), "東.");
        assert!(display_width(&clip_cells("北京机房", 5)) <= 5);
    }

    #[test]
    fn plain_puts_a_space_in_the_place_of_a_line_break() {
        assert_eq!(plain("a\nb\tc"), "a b c");
        assert_eq!(plain("alpha"), "alpha");
    }
}
