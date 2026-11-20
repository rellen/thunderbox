//! The panel that draws a list of rows in order by one column, with the
//! largest value first.
//!
//! The program `top` shows the processes of a machine in this form. This panel
//! knows nothing of a process. A data source returns a frame in which each row
//! describes one thing, and the panel puts those rows in order. A process, a
//! disk, and a container are three examples of such a thing.
//!
//! The panel differs from the table panel in three ways:
//!
//! - The table panel keeps the order of the data, and it shows the last rows
//!   that fit. This panel puts the rows in order by one column, and it shows
//!   the first rows that fit.
//! - The table panel puts the columns of two frames side by side. This panel
//!   puts the rows of every frame into one list. A column of one frame joins
//!   the column of another frame that shows the same header.
//! - The settings of the panel describe the column that gives the order. Every
//!   other column keeps the settings that its data source states. A list of
//!   processes holds a share of the processor beside a count of bytes, and one
//!   unit cannot describe both.
//!
//! The two panels draw a sheet with one function, so they share one layout.

use std::cmp::Ordering;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Alignment, Rect};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;
use thunderbox_core::{Field, Frame};

use crate::panel::{NO_DATA, frames_or_message, panel_block, render_message};
use crate::table::{Column, Counted, Sheet, draw_sheet, dropped_text, entry, missing_entry, plain};
use crate::{PanelData, PanelOptions, Theme, draws_a_panel};

/// A panel that draws a list of rows in order by one column, with the largest
/// value first.
///
/// The panel takes frames. It does not take a data source. A program that
/// already holds its own data can therefore draw this panel, and needs no part
/// of thunderbox that runs a query.
///
/// # Examples
///
/// ```
/// use ratatui_core::buffer::Buffer;
/// use ratatui_core::layout::Rect;
/// use ratatui_core::widgets::Widget;
/// use thunderbox_core::{Field, Frame, Values};
/// use thunderbox_widgets::{PanelData, Theme, TopPanel};
///
/// let frames = vec![Frame::new(vec![
///     Field {
///         name: "name".into(),
///         values: Values::Str(vec![Some("idle".into()), Some("busy".into())]),
///         ..Default::default()
///     },
///     Field::f64("cpu", vec![Some(1.0), Some(90.0)]),
/// ])];
/// let theme = Theme::default();
/// let area = Rect::new(0, 0, 20, 5);
/// let mut buf = Buffer::empty(area);
/// TopPanel::new("processes", PanelData::Frames(&frames), &theme)
///     .sort_by("cpu")
///     .render(area, &mut buf);
/// // The border takes the first row, and the header takes the second row.
/// let first: String = (0..20).map(|x| buf[(x, 2)].symbol().to_string()).collect();
/// assert!(first.contains("busy"));
/// ```
pub struct TopPanel<'a> {
    /// The title that the border shows.
    title: &'a str,
    /// The data that the panel draws at this moment.
    data: PanelData<'a>,
    /// The colours that the panel uses.
    theme: &'a Theme,
    /// The settings that the panel uses to show the column that gives the
    /// order.
    options: PanelOptions<'a>,
    /// The header of the column that gives the order. The panel takes its
    /// first column of numbers when this field holds `None`.
    sort: Option<&'a str>,
    /// The border changes colour when a person selects the panel.
    focused: bool,
}

impl<'a> TopPanel<'a> {
    /// Returns a panel that puts its rows in order by its first column of
    /// numbers.
    pub fn new(title: &'a str, data: PanelData<'a>, theme: &'a Theme) -> Self {
        Self {
            title,
            data,
            theme,
            options: PanelOptions::default(),
            sort: None,
            focused: false,
        }
    }

    /// Sets the column that puts the rows in order.
    ///
    /// The name is the text that the header of the column shows. The panel
    /// puts the row that holds the largest number of that column first. Two
    /// rows that hold the same number keep the order of the data. A row that
    /// holds no number in that column comes last.
    ///
    /// The panel draws an error in place of the rows when no column of numbers
    /// shows this header.
    pub fn sort_by(mut self, column: &'a str) -> Self {
        self.sort = Some(column);
        self
    }

    /// Sets the unit, the number of decimal places, and the thresholds of the
    /// column that gives the order.
    ///
    /// Every other column keeps the settings that its data source states. The
    /// panel reads no step from these settings, and it turns no column into one
    /// number.
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

/// The rows that the panel draws, and the count of the rows that it drops.
#[derive(Debug, Clone)]
struct Ranked {
    /// The rows that fit, in order.
    sheet: Sheet,
    /// The count of the rows that do not fit.
    dropped: usize,
}

/// Returns the number that one row holds in one column.
///
/// The function returns `None` for a hole, for a value that is not finite, and
/// for a column that holds no numbers. Such a row comes last.
fn number(field: Option<&Field>, row: usize) -> Option<f64> {
    field?
        .values
        .as_f64()?
        .get(row)
        .copied()
        .flatten()
        .filter(|v| v.is_finite())
}

/// Puts the larger number first, and a row that holds no number last.
///
/// Two rows that hold the same number compare as equal. A stable sort then
/// keeps the order of the data for those two rows.
fn largest_first(a: Option<f64>, b: Option<f64>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => b.partial_cmp(&a).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Returns the rows that the panel draws.
///
/// The function builds one column for each header that a field of the frames
/// shows, in the order in which the headers first appear. It puts the rows of
/// every frame into one list, and it puts that list in order by the column
/// that `sort` names. A frame that holds no field for a header shows the
/// symbol of a value that is not there in that column.
///
/// The `limit` argument states how many rows the panel can show. The function
/// builds the first `limit` rows only, and it counts the rest.
///
/// The function returns `Ok(None)` when the frames hold no row. The panel then
/// reports that it holds no data. That check comes first, because a data
/// source can return an empty answer that holds no column at all.
///
/// # Errors
///
/// The function returns the text of an error when `sort` names a header that
/// no column of numbers shows.
fn rank(
    frames: &[Frame],
    sort: Option<&str>,
    options: &PanelOptions<'_>,
    theme: &Theme,
    limit: usize,
) -> Result<Option<Ranked>, String> {
    let total: usize = frames.iter().map(Frame::len).sum();
    if total == 0 {
        return Ok(None);
    }

    // The header of each column, and whether a field of that column holds
    // numbers.
    let mut headers: Vec<(String, bool)> = Vec::new();
    // For each frame, the field of each column. A frame that holds no such
    // field holds `None`, and the first field of a header wins.
    let mut lookup: Vec<Vec<Option<&Field>>> = Vec::with_capacity(frames.len());
    for frame in frames {
        let mut fields: Vec<Option<&Field>> = vec![None; headers.len()];
        for field in &frame.fields {
            let header = plain(&field.display_name());
            let index = match headers.iter().position(|(h, _)| *h == header) {
                Some(index) => index,
                None => {
                    headers.push((header, false));
                    fields.push(None);
                    headers.len() - 1
                }
            };
            headers[index].1 |= field.is_numeric();
            if fields[index].is_none() {
                fields[index] = Some(field);
            }
        }
        lookup.push(fields);
    }
    // A frame that came before a new header holds no field of that header.
    for fields in &mut lookup {
        fields.resize(headers.len(), None);
    }

    let key = match sort {
        Some(name) => {
            let found = headers
                .iter()
                .position(|(header, numeric)| *numeric && header == name);
            match found {
                Some(index) => Some(index),
                None => return Err(format!("No column of numbers shows the header {name:?}.")),
            }
        }
        None => headers.iter().position(|(_, numeric)| *numeric),
    };

    // One place for each row: the index of the frame, and the row inside it.
    let mut places: Vec<(Option<f64>, usize, usize)> = Vec::with_capacity(total);
    for (index, frame) in frames.iter().enumerate() {
        for row in 0..frame.len() {
            let value = key.and_then(|k| number(lookup[index][k], row));
            places.push((value, index, row));
        }
    }
    // `sort_by` is a stable sort. Two rows that hold the same number therefore
    // keep the order of the data, and a list that refreshes stays calm.
    places.sort_by(|a, b| largest_first(a.0, b.0));

    let shown = total.min(limit);
    let plain_options = PanelOptions::default();
    let columns = headers
        .into_iter()
        .enumerate()
        .map(|(column, (header, right))| {
            let sorted = key == Some(column);
            // The settings of the panel describe the column that gives the
            // order. Every other column keeps the settings of its data source.
            let settings = if sorted { options } else { &plain_options };
            Column {
                header,
                entries: places[..shown]
                    .iter()
                    .map(|(_, frame, row)| match lookup[*frame][column] {
                        Some(field) => entry(field, *row, settings, theme),
                        None => missing_entry(theme),
                    })
                    .collect(),
                right,
                sorted,
            }
        })
        .collect();
    Ok(Some(Ranked {
        sheet: Sheet {
            columns,
            rows: shown,
        },
        dropped: total - shown,
    }))
}

impl Widget for TopPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self).render(area, buf);
    }
}

impl Widget for &TopPanel<'_> {
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
        let ranked = match rank(frames, self.sort, &self.options, self.theme, limit) {
            Ok(Some(ranked)) => ranked,
            Ok(None) => return render_message(block, NO_DATA, self.theme.muted, area, buf),
            Err(text) => return render_message(block, &text, self.theme.error, area, buf),
        };
        draw_sheet(&ranked.sheet, block, self.theme, area, buf);

        if ranked.dropped > 0 {
            // The note sits at the right of the bottom border. It leaves the
            // two corners of the border as they are.
            let room = area.width.saturating_sub(2);
            let note = dropped_text(ranked.dropped, usize::from(room), Counted::Row);
            Line::from(Span::styled(note, self.theme.muted))
                .alignment(Alignment::Right)
                .render(
                    Rect {
                        x: area.x + 1,
                        y: area.bottom() - 1,
                        width: room,
                        height: 1,
                    },
                    buf,
                );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::style::Modifier;
    use thunderbox_core::{FieldConfig, Threshold, Values};

    use crate::MISSING;

    /// Returns a column of text.
    fn text_field(name: &str, values: &[&str]) -> Field {
        Field {
            name: name.into(),
            values: Values::Str(values.iter().map(|v| Some((*v).to_string())).collect()),
            ..Default::default()
        }
    }

    /// Returns a column of numbers in this unit.
    fn number_field(name: &str, unit: &str, values: Vec<Option<f64>>) -> Field {
        Field::f64(name, values).with_config(FieldConfig {
            unit: Some(unit.into()),
            ..Default::default()
        })
    }

    /// Returns one frame that holds four processes.
    ///
    /// The column `cpu` puts the rows in the order cargo, editor, shell, init.
    /// The column `memory` puts them in the order init, editor, cargo, shell.
    /// The two orders differ, so a test can tell which column gave the order.
    fn processes() -> Vec<Frame> {
        vec![Frame::named(
            "processes",
            vec![
                text_field("pid", &["1", "2", "3", "4"]),
                text_field("name", &["init", "shell", "cargo", "editor"]),
                number_field(
                    "cpu",
                    "percent",
                    vec![Some(0.5), Some(2.0), Some(97.0), Some(12.0)],
                ),
                number_field(
                    "memory",
                    "bytes",
                    vec![Some(9.0e8), Some(8.0e6), Some(1.5e8), Some(3.0e8)],
                ),
            ],
        )]
    }

    /// Draws the panel into a plain buffer. This test needs no terminal backend
    /// and no terminal. That fact is the promise of this crate.
    fn render(panel: &TopPanel<'_>, w: u16, h: u16) -> Buffer {
        let area = Rect::new(0, 0, w, h);
        let mut buf = Buffer::empty(area);
        panel.render(area, &mut buf);
        buf
    }

    /// Draws the panel at a place inside a larger buffer, and checks that the
    /// panel wrote inside its own area only.
    ///
    /// The panel writes a note on its bottom border. A note that started one
    /// cell too far to the right would damage the panel beside it. The buffer
    /// of the helper above would hide that fault.
    fn render_beside_a_neighbour(panel: &TopPanel<'_>, w: u16, h: u16) {
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
            .map(|y| line(buf, y))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Returns the text of one row of the buffer.
    fn line(buf: &Buffer, y: u16) -> String {
        (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect()
    }

    /// Returns the names of the processes, in the order that the panel drew
    /// them. The border takes the first row, and the header takes the second.
    fn names_in_order(buf: &Buffer) -> Vec<&'static str> {
        let mut out = Vec::new();
        for y in 2..buf.area.height {
            let row = line(buf, y);
            for name in ["init", "shell", "cargo", "editor"] {
                if row.contains(name) {
                    out.push(name);
                }
            }
        }
        out
    }

    #[test]
    fn the_panel_shows_that_the_data_source_has_not_answered() {
        let theme = Theme::default();
        let p = TopPanel::new("processes", PanelData::Loading, &theme);
        assert!(text(&render(&p, 30, 8)).contains("loading"));
    }

    #[test]
    fn the_panel_shows_the_text_of_an_error() {
        let theme = Theme::default();
        let p = TopPanel::new("processes", PanelData::Error("upstream refused"), &theme);
        let out = text(&render(&p, 40, 8));
        assert!(out.contains("upstream refused"), "{out}");
        assert!(
            out.contains("processes"),
            "The panel must still draw the title."
        );
    }

    #[test]
    fn the_panel_reports_that_it_has_no_data() {
        let theme = Theme::default();
        let empty: Vec<Frame> = vec![];
        let p = TopPanel::new("processes", PanelData::Frames(&empty), &theme).sort_by("cpu");
        assert!(
            text(&render(&p, 30, 8)).contains("no data"),
            "An answer that holds no row is not a fault of the sort column."
        );
    }

    /// The sysinfo data source returns this answer for a window that does not
    /// hold the present moment. The frame holds every column, and no row.
    #[test]
    fn the_panel_reports_no_data_for_columns_that_hold_no_row() {
        let theme = Theme::default();
        let f = vec![Frame::new(vec![
            text_field("name", &[]),
            number_field("cpu", "percent", vec![]),
        ])];
        let p = TopPanel::new("processes", PanelData::Frames(&f), &theme).sort_by("cpu");
        assert!(text(&render(&p, 30, 8)).contains("no data"));
    }

    #[test]
    fn the_panel_puts_the_largest_value_of_the_column_first() {
        let theme = Theme::default();
        let f = processes();
        let by_cpu = TopPanel::new("processes", PanelData::Frames(&f), &theme).sort_by("cpu");
        assert_eq!(
            names_in_order(&render(&by_cpu, 40, 8)),
            vec!["cargo", "editor", "shell", "init"],
            "The column cpu gives the order."
        );
        let by_memory = TopPanel::new("processes", PanelData::Frames(&f), &theme).sort_by("memory");
        assert_eq!(
            names_in_order(&render(&by_memory, 40, 8)),
            vec!["init", "editor", "cargo", "shell"],
            "The column memory gives the order."
        );
    }

    #[test]
    fn a_panel_that_names_no_column_takes_its_first_column_of_numbers() {
        let theme = Theme::default();
        let f = processes();
        let p = TopPanel::new("processes", PanelData::Frames(&f), &theme);
        assert_eq!(
            names_in_order(&render(&p, 40, 8)),
            vec!["cargo", "editor", "shell", "init"],
            "The first column of numbers is cpu. The text columns before it give no order."
        );
    }

    /// Most processes of a machine use no share of the processor. A list that
    /// refreshed would put those rows in a new order each time, and the rows
    /// would jump about. The panel keeps the order of the data instead.
    #[test]
    fn two_rows_that_hold_the_same_number_keep_the_order_of_the_data() {
        let theme = Theme::default();
        let f = vec![Frame::new(vec![
            text_field("name", &["init", "shell", "cargo", "editor"]),
            number_field("cpu", "percent", vec![Some(0.0); 4]),
        ])];
        let p = TopPanel::new("processes", PanelData::Frames(&f), &theme).sort_by("cpu");
        assert_eq!(
            names_in_order(&render(&p, 40, 8)),
            vec!["init", "shell", "cargo", "editor"]
        );
    }

    #[test]
    fn a_row_that_holds_no_number_comes_last() {
        let theme = Theme::default();
        let f = vec![Frame::new(vec![
            text_field("name", &["init", "shell", "cargo", "editor"]),
            number_field(
                "cpu",
                "percent",
                vec![None, Some(1.0), Some(f64::NAN), Some(5.0)],
            ),
        ])];
        let p = TopPanel::new("processes", PanelData::Frames(&f), &theme).sort_by("cpu");
        let buf = render(&p, 40, 8);
        assert_eq!(
            names_in_order(&buf),
            vec!["editor", "shell", "init", "cargo"],
            "A hole and a value that is not finite come last, in the order of the data."
        );
        assert_eq!(
            text(&buf).matches(MISSING).count(),
            2,
            "The panel draws the symbol for each row that holds no number."
        );
    }

    #[test]
    fn the_panel_reports_a_sort_column_that_the_data_does_not_hold() {
        let theme = Theme::default();
        let f = processes();
        for name in ["threads", "name"] {
            let p = TopPanel::new("processes", PanelData::Frames(&f), &theme).sort_by(name);
            let out = text(&render(&p, 70, 8));
            assert!(
                out.contains("No column of numbers shows the header"),
                "The panel must name the fault. {name} {out}"
            );
            assert!(
                out.contains(name),
                "The message must name the column. {name} {out}"
            );
            assert!(
                !out.contains("cargo"),
                "The panel draws no rows in an order that nobody asked for. {out}"
            );
        }
    }

    #[test]
    fn the_panel_shows_the_first_rows_and_counts_the_rows_that_do_not_fit() {
        let theme = Theme::default();
        let f = processes();
        let p = TopPanel::new("processes", PanelData::Frames(&f), &theme).sort_by("cpu");
        // The border takes two rows, and the header takes one. Two rows fit.
        let buf = render(&p, 40, 5);
        assert_eq!(names_in_order(&buf), vec!["cargo", "editor"]);
        assert!(
            line(&buf, 4).contains("2 rows do not fit."),
            "The bottom border must count the rows that the panel dropped. {}",
            text(&buf)
        );
        assert!(
            line(&buf, 4).starts_with('╰') && line(&buf, 4).ends_with('╯'),
            "The note must leave the corners of the border. {}",
            text(&buf)
        );
    }

    #[test]
    fn the_note_takes_a_short_form_in_a_narrow_panel() {
        let theme = Theme::default();
        let f = processes();
        let p = TopPanel::new("processes", PanelData::Frames(&f), &theme).sort_by("cpu");
        let buf = render(&p, 12, 5);
        assert!(line(&buf, 4).contains("+2"), "{}", text(&buf));
    }

    #[test]
    fn the_panel_writes_no_note_when_every_row_fits() {
        let theme = Theme::default();
        let f = processes();
        let p = TopPanel::new("processes", PanelData::Frames(&f), &theme).sort_by("cpu");
        let buf = render(&p, 40, 8);
        assert!(
            !text(&buf).contains("fit") && !text(&buf).contains('+'),
            "{}",
            text(&buf)
        );
    }

    /// The note counts the rows of the data and not the rows that the panel
    /// draws. A panel that holds room for the header alone drops every row.
    #[test]
    fn a_panel_that_holds_room_for_the_header_alone_counts_every_row() {
        let theme = Theme::default();
        let f = processes();
        let p = TopPanel::new("processes", PanelData::Frames(&f), &theme).sort_by("cpu");
        let buf = render(&p, 40, 3);
        assert!(
            line(&buf, 1).contains("cpu"),
            "The header must stay. {}",
            text(&buf)
        );
        assert!(
            line(&buf, 2).contains("4 rows do not fit."),
            "{}",
            text(&buf)
        );
    }

    #[test]
    fn the_rows_of_two_frames_join_into_one_list() {
        let theme = Theme::default();
        let f = vec![
            Frame::named(
                "this",
                vec![
                    text_field("pid", &["1", "2"]),
                    text_field("name", &["init", "shell"]),
                    number_field("cpu", "percent", vec![Some(3.0), Some(1.0)]),
                ],
            ),
            Frame::named(
                "that",
                vec![
                    text_field("name", &["cargo", "editor"]),
                    number_field("cpu", "percent", vec![Some(50.0), Some(2.0)]),
                    text_field("host", &["beta", "beta"]),
                ],
            ),
        ];
        let p = TopPanel::new("processes", PanelData::Frames(&f), &theme).sort_by("cpu");
        let buf = render(&p, 40, 8);
        assert_eq!(
            names_in_order(&buf),
            vec!["cargo", "init", "editor", "shell"],
            "The panel ranks the rows of both frames together. {}",
            text(&buf)
        );
        let header = line(&buf, 1);
        for column in ["pid", "name", "cpu", "host"] {
            assert!(
                header.contains(column),
                "One header for each name. {column} {header}"
            );
        }
        assert_eq!(
            text(&buf).matches(MISSING).count(),
            4,
            "Each frame lacks one column for two rows. {}",
            text(&buf)
        );
    }

    /// A list of processes holds a share of the processor beside a count of
    /// bytes. The unit of the panel would turn the count of bytes into a share
    /// if it reached every column.
    #[test]
    fn the_settings_of_the_panel_describe_the_column_that_gives_the_order() {
        let theme = Theme::default();
        let f = processes();
        let options = PanelOptions {
            unit: Some("percent"),
            decimals: Some(1),
            ..Default::default()
        };
        let p = TopPanel::new("processes", PanelData::Frames(&f), &theme)
            .sort_by("cpu")
            .options(options);
        let out = text(&render(&p, 40, 8));
        assert!(
            out.contains("97.0%"),
            "The panel sets the column cpu. {out}"
        );
        assert!(
            out.contains("150MB"),
            "The column memory keeps the unit of its data source. {out}"
        );
    }

    #[test]
    fn a_threshold_of_the_panel_colours_the_column_that_gives_the_order_alone() {
        let theme = Theme::default();
        let f = processes();
        let list = [Threshold {
            value: 50.0,
            color: "critical".into(),
        }];
        let options = PanelOptions {
            thresholds: &list,
            ..Default::default()
        };
        let p = TopPanel::new("processes", PanelData::Frames(&f), &theme)
            .sort_by("cpu")
            .options(options);
        let buf = render(&p, 40, 8);
        let red: String = buf
            .content
            .iter()
            .filter(|c| c.fg == theme.critical && c.symbol() != " ")
            .map(|c| c.symbol().to_string())
            .collect();
        assert_eq!(
            red, "97%",
            "Each count of bytes passes 50, and no count of bytes may turn red."
        );
    }

    #[test]
    fn the_header_of_the_column_that_gives_the_order_carries_a_line() {
        let theme = Theme::default();
        let f = processes();
        for (column, other) in [("cpu", "memory"), ("memory", "cpu")] {
            let p = TopPanel::new("processes", PanelData::Frames(&f), &theme).sort_by(column);
            let buf = render(&p, 40, 8);
            let marked: String = (0..buf.area.width)
                .map(|x| &buf[(x, 1)])
                .filter(|c| c.modifier.contains(Modifier::UNDERLINED))
                .map(|c| c.symbol().to_string())
                .collect();
            assert!(
                marked.contains(column) && !marked.contains(other),
                "The line marks {column} alone. It marks {marked:?}."
            );
        }
    }

    #[test]
    fn a_panel_that_holds_no_column_of_numbers_keeps_the_order_of_the_data() {
        let theme = Theme::default();
        let f = vec![Frame::new(vec![text_field(
            "name",
            &["init", "shell", "cargo", "editor"],
        )])];
        let p = TopPanel::new("processes", PanelData::Frames(&f), &theme);
        let buf = render(&p, 40, 8);
        assert_eq!(
            names_in_order(&buf),
            vec!["init", "shell", "cargo", "editor"]
        );
        assert!(
            !buf.content
                .iter()
                .any(|c| c.modifier.contains(Modifier::UNDERLINED)),
            "No column gives the order, so the panel marks none."
        );
    }

    #[test]
    fn the_panel_draws_nothing_in_a_small_area() {
        let theme = Theme::default();
        let f = processes();
        let p = TopPanel::new("processes", PanelData::Frames(&f), &theme);
        for (w, h) in [(0, 0), (1, 1), (2, 2), (2, 20), (20, 2)] {
            let out = text(&render(&p, w, h));
            assert!(out.chars().all(|c| c == ' ' || c == '\n'), "{out}");
        }
    }

    #[test]
    fn the_panel_draws_every_size_of_area_inside_its_area() {
        let theme = Theme::default();
        let many: Vec<Option<f64>> = (0..500).map(|i| Some(f64::from(i % 37))).collect();
        let names: Vec<String> = (0..500).map(|i| format!("process{i}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let f = vec![
            Frame::new(vec![
                text_field("name", &names),
                number_field("cpu", "percent", many),
            ]),
            Frame::new(vec![number_field("cpu", "percent", vec![None])]),
        ];
        let p = TopPanel::new(
            "A list of processes with a very long title",
            PanelData::Frames(&f),
            &theme,
        )
        .sort_by("cpu")
        .focused(true);
        for (w, h) in [(120, 40), (200, 1), (1, 200), (200, 3)] {
            let _ = render(&p, w, h);
        }
        // Every small size follows. A panel that a person makes smaller passes
        // through each of them.
        for w in 0..40_u16 {
            for h in 0..10_u16 {
                render_beside_a_neighbour(&p, w, h);
            }
        }
    }

    #[test]
    fn largest_first_puts_a_row_that_holds_no_number_last() {
        assert_eq!(largest_first(Some(2.0), Some(1.0)), Ordering::Less);
        assert_eq!(largest_first(Some(1.0), Some(2.0)), Ordering::Greater);
        assert_eq!(largest_first(Some(1.0), Some(1.0)), Ordering::Equal);
        assert_eq!(largest_first(Some(0.0), None), Ordering::Less);
        assert_eq!(largest_first(None, Some(0.0)), Ordering::Greater);
        assert_eq!(largest_first(None, None), Ordering::Equal);
    }

    #[test]
    fn rank_counts_the_rows_that_do_not_fit() {
        let theme = Theme::default();
        let f = processes();
        let ranked = rank(&f, Some("cpu"), &PanelOptions::default(), &theme, 3)
            .expect("The data holds the column cpu.")
            .expect("The data holds rows.");
        assert_eq!(ranked.sheet.rows, 3);
        assert_eq!(ranked.dropped, 1);
        assert!(
            ranked.sheet.columns.iter().all(|c| c.entries.len() == 3),
            "Each column holds one entry for each row that the panel shows."
        );
    }
}
