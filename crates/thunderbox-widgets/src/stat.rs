//! The panel that draws one large number for each series.
//!
//! A person reads this panel from across a room. The panel therefore draws the
//! number with block characters when the tile has room for them. It falls back
//! to plain text in a tile that is too small.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Alignment, Rect};
use ratatui_core::style::{Color, Style};
use ratatui_core::symbols;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;
use ratatui_widgets::sparkline::{RenderDirection, Sparkline};
use thunderbox_core::{Frame, Reduce};

use crate::panel::{
    NO_DATA, Reduced, clip, dropped_text, frames_or_message, panel_block, reduce_frames,
    render_message,
};
use crate::{Marker, PanelData, PanelOptions, Theme, draws_a_panel};

/// The number of rows that one large character holds.
const GLYPH_ROWS: usize = 3;

/// The height of one large character, as a number of cells.
///
/// This number is [`GLYPH_ROWS`] in the type that a [`Rect`] uses.
const GLYPH_HEIGHT: u16 = 3;

/// The width of one large character, as a number of cells.
const GLYPH_WIDTH: u16 = 3;

/// The number of empty columns between two large characters.
const GLYPH_GAP: u16 = 1;

/// The smallest width of a tile, in cells.
///
/// The panel adds a second column of tiles only when both columns reach this
/// width. A name and a number do not fit in less.
const MIN_TILE_WIDTH: u16 = 8;

/// The smallest height of a tile, in cells. Two rows hold the name of the
/// series and the number.
const MIN_TILE_HEIGHT: u16 = 2;

/// The number of spare rows that the small chart needs.
const CHART_ROWS: u16 = 3;

/// The largest height of the small chart, in cells.
const CHART_MAX_ROWS: u16 = 4;

/// The largest bar height that the panel gives to the small chart.
///
/// The chart takes a whole number for each bar. The panel therefore maps the
/// range of the series onto the numbers from 0 to this one.
const CHART_SCALE: u64 = 10_000;

/// The large form of each character that a number holds.
///
/// Each entry holds [`GLYPH_ROWS`] rows, and each row holds [`GLYPH_WIDTH`]
/// characters. The table covers the digits 0 to 9, the full stop, and the minus
/// sign. It covers no other character. A number that holds another character
/// falls back to plain text.
const FONT: [(char, [&str; GLYPH_ROWS]); 12] = [
    ('0', ["▄▄▄", "█ █", "█▄█"]),
    ('1', ["▄▄ ", " █ ", "▄█▄"]),
    ('2', ["▄▄▄", "▄▄█", "█▄▄"]),
    ('3', ["▄▄▄", " ▄█", "▄▄█"]),
    ('4', ["▄ ▄", "█▄█", "  █"]),
    ('5', ["▄▄▄", "█▄▄", "▄▄█"]),
    ('6', ["▄▄▄", "█▄▄", "█▄█"]),
    ('7', ["▄▄▄", "  █", "  █"]),
    ('8', ["▄▄▄", "█▄█", "█▄█"]),
    ('9', ["▄▄▄", "█▄█", "▄▄█"]),
    ('.', ["   ", "   ", " ▄ "]),
    ('-', ["   ", "▄▄▄", "   "]),
];

/// Returns the large form of one character.
///
/// The function returns `None` when the font does not hold the character.
fn glyph(c: char) -> Option<&'static [&'static str; GLYPH_ROWS]> {
    FONT.iter().find(|(key, _)| *key == c).map(|(_, rows)| rows)
}

/// A panel that draws one large number for each series.
///
/// The panel reduces each series to one number. It then divides the panel into
/// one tile for each series. A tile holds the name of the series, the number,
/// and a small chart of the series. The tile drops a part as the area shrinks,
/// and the number is the last part that it drops.
///
/// The panel takes frames. It does not take a data source. A program that
/// already holds its own data can therefore draw this panel, and needs no part
/// of thunderbox that runs a query.
pub struct StatPanel<'a> {
    title: &'a str,
    data: PanelData<'a>,
    theme: &'a Theme,
    options: PanelOptions<'a>,
    marker: Marker,
    focused: bool,
}

impl<'a> StatPanel<'a> {
    /// Returns a panel that uses the default settings.
    pub fn new(title: &'a str, data: PanelData<'a>, theme: &'a Theme) -> Self {
        Self {
            title,
            data,
            theme,
            options: PanelOptions::default(),
            marker: Marker::default(),
            focused: false,
        }
    }

    /// Sets the unit, the number of decimal places, and the thresholds.
    pub fn options(mut self, options: PanelOptions<'a>) -> Self {
        self.options = options;
        self
    }

    /// Sets the characters that the small chart draws a bar with.
    ///
    /// The [`Marker::Block`] value selects the two characters that every font
    /// holds. Every other value selects the eight characters that give a bar a
    /// finer height.
    pub fn marker(mut self, marker: Marker) -> Self {
        self.marker = marker;
        self
    }

    /// States whether a person has selected this panel. The border changes
    /// colour when a person selects a panel.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

/// The arrangement of the tiles inside a panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Grid {
    /// The area that holds the tiles.
    body: Rect,
    /// The row that states the number of series that the panel dropped. The
    /// field is `None` when the panel drops no series, and also when the panel
    /// has no room for the row.
    note: Option<Rect>,
    /// The number of tiles across the panel.
    columns: u16,
    /// The number of tiles down the panel.
    rows: u16,
    /// The number of series that the panel draws.
    shown: usize,
    /// The number of series that the panel drops.
    dropped: usize,
}

/// Returns the number of tiles that fit across an area, and down that area.
///
/// The function returns at least one tile in each direction. A panel that holds
/// one series then fills the whole area.
fn capacity(area: Rect) -> (u16, u16) {
    let columns = (area.width / MIN_TILE_WIDTH).max(1);
    let rows = (area.height / MIN_TILE_HEIGHT).max(1);
    (columns, rows)
}

/// Returns the arrangement of the tiles for a number of series.
///
/// The function prefers columns while each tile stays wide enough to read. It
/// keeps the last row of the area for a note when the series do not all fit. It
/// drops that note when the area holds one row only.
fn grid_for(count: usize, area: Rect) -> Grid {
    let count = count.max(1);
    let (columns, rows) = capacity(area);
    let room = usize::from(columns) * usize::from(rows);
    let (body, note) = if count > room && area.height >= 2 {
        (
            Rect {
                height: area.height - 1,
                ..area
            },
            Some(Rect {
                y: area.y + area.height - 1,
                height: 1,
                ..area
            }),
        )
    } else {
        (area, None)
    };
    let (max_columns, max_rows) = capacity(body);
    let room = usize::from(max_columns) * usize::from(max_rows);
    let shown = count.min(room);
    let columns = max_columns
        .min(u16::try_from(shown).unwrap_or(u16::MAX))
        .max(1);
    let rows = u16::try_from(shown.div_ceil(usize::from(columns)))
        .unwrap_or(max_rows)
        .clamp(1, max_rows.max(1));
    Grid {
        body,
        note,
        columns,
        rows,
        shown,
        dropped: count - shown,
    }
}

/// Returns the area of one tile.
///
/// The function shares each spare cell out across the tiles. The tiles
/// therefore cover the whole area, and no two tiles overlap. The function
/// returns an empty area for an index that the grid does not hold.
fn tile_of(grid: &Grid, index: usize) -> Rect {
    let columns = u64::from(grid.columns.max(1));
    let rows = u64::from(grid.rows.max(1));
    let column = (index as u64) % columns;
    let row = (index as u64) / columns;
    if row >= rows {
        return Rect {
            width: 0,
            height: 0,
            ..grid.body
        };
    }
    let x0 = u64::from(grid.body.width) * column / columns;
    let x1 = u64::from(grid.body.width) * (column + 1) / columns;
    let y0 = u64::from(grid.body.height) * row / rows;
    let y1 = u64::from(grid.body.height) * (row + 1) / rows;
    Rect {
        x: grid.body.x.saturating_add(x0 as u16),
        y: grid.body.y.saturating_add(y0 as u16),
        width: (x1 - x0) as u16,
        height: (y1 - y0) as u16,
    }
}

/// Returns the size of the smallest tile of the grid, in cells.
///
/// The tiles differ by one cell where the area does not divide evenly. The
/// panel plans every tile against the smallest one. Each tile then draws the
/// same parts. One tile does not fall back to plain text beside a tile that
/// drew a large number.
///
/// The width leaves out one column when the grid holds more than one column of
/// tiles. That column keeps two numbers apart.
fn plan_size(grid: &Grid) -> (u16, u16) {
    let gutter = u16::from(grid.columns > 1);
    (
        (grid.body.width / grid.columns.max(1)).saturating_sub(gutter),
        grid.body.height / grid.rows.max(1),
    )
}

/// The two parts of a number that the panel draws in the large form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Large<'t> {
    /// The digits, the full stop, and the minus sign. The font covers each of
    /// these characters.
    head: &'t str,
    /// The unit and the letter of the scale. The panel draws this part as plain
    /// text.
    tail: &'t str,
    /// The width of the large characters alone, in cells.
    glyphs_width: u16,
    /// The width of the whole number, which includes the tail.
    width: u16,
}

/// Returns the number, cut into the part that the font covers and the rest.
///
/// The function cuts the text at the first character that is not a digit, not a
/// full stop, and not a minus sign.
fn split_number(text: &str) -> (&str, &str) {
    let at = text
        .char_indices()
        .find(|(_, c)| !matches!(c, '0'..='9' | '.' | '-'))
        .map_or(text.len(), |(index, _)| index);
    text.split_at(at)
}

/// Returns the width of a row of large characters, in cells.
fn glyphs_width(count: usize) -> u16 {
    if count == 0 {
        return 0;
    }
    let count = count as u64;
    let width = count
        .saturating_mul(u64::from(GLYPH_WIDTH))
        .saturating_add((count - 1).saturating_mul(u64::from(GLYPH_GAP)));
    width.min(u64::from(u16::MAX)) as u16
}

/// Returns the width of the whole number in the large form, in cells.
///
/// The `count` argument is the number of large characters. The `tail` argument
/// is the number of plain characters beside them. One empty column separates
/// the two parts.
fn large_width(count: usize, tail: usize) -> u16 {
    let head = u64::from(glyphs_width(count));
    if head == 0 {
        return 0;
    }
    let extra = if tail == 0 {
        0
    } else {
        (tail as u64).saturating_add(u64::from(GLYPH_GAP))
    };
    head.saturating_add(extra).min(u64::from(u16::MAX)) as u16
}

/// Returns the plan for a number in the large form, if the number fits.
///
/// The function returns `None` when the tile is too small. It also returns
/// `None` when the font does not cover a character of the number.
fn plan_large(text: &str, width: u16, height: u16) -> Option<Large<'_>> {
    if height < GLYPH_HEIGHT {
        return None;
    }
    let (head, tail) = split_number(text);
    if head.is_empty() || head.chars().any(|c| glyph(c).is_none()) {
        return None;
    }
    let count = head.chars().count();
    let total = large_width(count, tail.chars().count());
    if total == 0 || total > width {
        return None;
    }
    Some(Large {
        head,
        tail,
        glyphs_width: glyphs_width(count),
        width: total,
    })
}

/// Returns the smallest and the largest number in a column.
///
/// The function ignores a hole in the data, and it ignores a value that is not
/// finite. It returns `None` for a column that holds no number.
fn bounds_of(points: &[Option<f64>]) -> Option<(f64, f64)> {
    points
        .iter()
        .flatten()
        .copied()
        .filter(|v| v.is_finite())
        .fold(None, |acc, v| match acc {
            None => Some((v, v)),
            Some((low, high)) => Some((low.min(v), high.max(v))),
        })
}

/// Returns the height of one bar of the small chart.
///
/// The function returns `None` for a hole in the data, and for a value that is
/// not finite. The chart then draws the symbol of an absent value in place of a
/// bar. A hole therefore stays a hole.
fn scale_point(value: Option<f64>, min: f64, max: f64) -> Option<u64> {
    let value = value?;
    if !value.is_finite() {
        return None;
    }
    let span = max - min;
    if !span.is_finite() || span <= 0.0 {
        return Some(0);
    }
    let ratio = ((value - min) / span).clamp(0.0, 1.0);
    Some((ratio * CHART_SCALE as f64) as u64)
}

/// Returns the set of bar characters that the marker asks for.
fn bar_set(marker: Marker) -> symbols::bar::Set<'static> {
    match marker {
        // A plain terminal holds the half block character and the full block
        // character only.
        Marker::Block => symbols::bar::THREE_LEVELS,
        _ => symbols::bar::NINE_LEVELS,
    }
}

/// Returns the column of numbers of each series that the panel draws.
///
/// The function walks the frames in the order that `reduce_frames` walks them,
/// and it keeps a column under the same condition. The result therefore holds
/// the points of each series at the same place as `reduce_frames` holds the
/// number of that series. The panel needs this function because `reduce_frames`
/// returns one number for each series and drops the points.
fn series_points(frames: &[Frame], reduce: Reduce) -> Vec<&[Option<f64>]> {
    let mut out = Vec::new();
    for frame in frames {
        for field in frame.numeric_fields() {
            let values = field.values.as_f64().unwrap_or(&[]);
            if reduce.apply(values).is_some() {
                out.push(values);
            }
        }
    }
    out
}

/// Draws one character in one cell.
///
/// The function draws nothing when the place sits outside the area, and when
/// the place sits outside the buffer.
fn put(buf: &mut Buffer, area: Rect, x: u16, y: u16, symbol: &str, style: Style) {
    if x < area.x || y < area.y || x >= area.right() || y >= area.bottom() {
        return;
    }
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol(symbol);
        cell.set_style(style);
    }
}

/// Draws one line of text in the middle of one row.
fn draw_centred(area: Rect, y: u16, text: &str, style: Style, buf: &mut Buffer) {
    if area.width == 0 || y >= area.bottom() {
        return;
    }
    let text = clip(text, usize::from(area.width));
    Line::from(Span::styled(text, style))
        .alignment(Alignment::Center)
        .render(
            Rect {
                y,
                height: 1,
                ..area
            },
            buf,
        );
}

/// Draws the number of series that the panel dropped.
///
/// The note takes one row. The shared function `dropped_text` supplies the
/// text, so that every panel states the same thing.
fn draw_note(area: Rect, dropped: usize, theme: &Theme, buf: &mut Buffer) {
    if dropped == 0 || area.width == 0 || area.height == 0 {
        return;
    }
    let text = dropped_text(dropped, usize::from(area.width));
    draw_centred(area, area.y, &text, theme.muted, buf);
}

impl StatPanel<'_> {
    /// Draws one tile.
    ///
    /// The `named` argument states whether the tile shows the name of the
    /// series. The `points` argument holds the column of numbers that the small
    /// chart draws.
    fn render_tile(
        &self,
        grid: &Grid,
        index: usize,
        series: &Reduced<'_>,
        points: &[Option<f64>],
        named: bool,
        buf: &mut Buffer,
    ) {
        let (plan_width, plan_height) = plan_size(grid);
        let whole = tile_of(grid, index);
        // The last column of a tile stays empty where the panel holds more than
        // one column of tiles. Two numbers beside each other then stay apart.
        let tile = Rect {
            width: plan_width.min(whole.width),
            ..whole
        };
        if tile.width == 0 || tile.height == 0 {
            return;
        }
        let color = self.options.color_for(
            series.value,
            series.config,
            self.theme,
            self.theme.series_color(index),
        );
        // The name costs one row. A tile of one row keeps the number instead.
        let name_rows = u16::from(named && plan_height > 1);
        if name_rows == 1 {
            draw_centred(tile, tile.y, &series.name, self.theme.muted, buf);
        }
        let body = Rect {
            y: tile.y.saturating_add(name_rows),
            height: tile.height - name_rows,
            ..tile
        };
        if body.height == 0 {
            return;
        }
        let text = self.options.text(series.value, series.config);
        let large = plan_large(&text, plan_width, plan_height.saturating_sub(name_rows));
        let value_rows = if large.is_some() { GLYPH_HEIGHT } else { 1 };
        let spare = plan_height
            .saturating_sub(name_rows)
            .saturating_sub(value_rows);
        let chart_rows = if spare >= CHART_ROWS {
            spare.min(CHART_MAX_ROWS)
        } else {
            0
        };
        let value_area = Rect {
            height: body.height.saturating_sub(chart_rows).max(1),
            ..body
        };
        match large {
            Some(plan) => self.draw_large(&plan, value_area, color, buf),
            None => {
                let y = value_area.y + (value_area.height - 1) / 2;
                draw_centred(value_area, y, &text, Style::default().fg(color), buf);
            }
        }
        if chart_rows > 0 {
            let chart = Rect {
                y: value_area.y.saturating_add(value_area.height),
                height: chart_rows,
                ..body
            };
            // The chart must stay inside the tile. A cell outside the buffer
            // would stop the program.
            self.draw_chart(chart.intersection(body), series, points, color, buf);
        }
    }

    /// Draws the number in the large form.
    ///
    /// The method draws the head with the block characters of the font. It
    /// draws the tail as plain text, beside the head, along the bottom row.
    fn draw_large(&self, plan: &Large<'_>, area: Rect, color: Color, buf: &mut Buffer) {
        if area.width < plan.width || area.height < GLYPH_HEIGHT {
            return;
        }
        let style = Style::default().fg(color);
        let x0 = area.x + (area.width - plan.width) / 2;
        let y0 = area.y + (area.height - GLYPH_HEIGHT) / 2;
        let mut x = x0;
        for c in plan.head.chars() {
            let Some(rows) = glyph(c) else { continue };
            for (row, line) in rows.iter().enumerate() {
                for (column, symbol) in line.chars().enumerate() {
                    if symbol == ' ' {
                        continue;
                    }
                    let mut bytes = [0_u8; 4];
                    put(
                        buf,
                        area,
                        x.saturating_add(column as u16),
                        y0.saturating_add(row as u16),
                        symbol.encode_utf8(&mut bytes),
                        style,
                    );
                }
            }
            x = x.saturating_add(GLYPH_WIDTH + GLYPH_GAP);
        }
        let mut x = x0
            .saturating_add(plan.glyphs_width)
            .saturating_add(GLYPH_GAP);
        let y = y0.saturating_add(GLYPH_HEIGHT - 1);
        for c in plan.tail.chars() {
            let mut bytes = [0_u8; 4];
            put(buf, area, x, y, c.encode_utf8(&mut bytes), style);
            x = x.saturating_add(1);
        }
    }

    /// Draws the small chart of one series.
    ///
    /// The chart shows the most recent points that fit. It scales each point
    /// against the bounds that the panel settings return.
    fn draw_chart(
        &self,
        area: Rect,
        series: &Reduced<'_>,
        points: &[Option<f64>],
        color: Color,
        buf: &mut Buffer,
    ) {
        // The chart writes a cell without a check of its own. The area must
        // therefore stay inside the buffer.
        let area = area.intersection(buf.area);
        if area.width == 0 || area.height == 0 || points.is_empty() {
            return;
        }
        let (min, max) = self.options.bounds_for(series.config, bounds_of(points));
        let first = points.len().saturating_sub(usize::from(area.width));
        // The chart draws the most recent point at the right edge, as the time
        // series panel does. The most recent point therefore comes first.
        let data: Vec<Option<u64>> = points[first..]
            .iter()
            .rev()
            .map(|v| scale_point(*v, min, max))
            .collect();
        Sparkline::default()
            .data(data)
            .direction(RenderDirection::RightToLeft)
            .max(CHART_SCALE)
            .style(Style::default().fg(color))
            .absent_value_style(self.theme.muted)
            .absent_value_symbol(symbols::shade::LIGHT)
            .bar_set(bar_set(self.marker))
            .render(area, buf);
    }
}

impl Widget for StatPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self).render(area, buf);
    }
}

impl Widget for &StatPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !draws_a_panel(area.as_size()) {
            return;
        }
        let block = panel_block(self.title, self.theme, self.focused);
        let frames = match frames_or_message(self.data, self.theme) {
            Ok(frames) => frames,
            Err((message, style)) => return render_message(block, message, style, area, buf),
        };
        let series = reduce_frames(frames, self.options.reduce);
        if series.is_empty() {
            return render_message(block, NO_DATA, self.theme.muted, area, buf);
        }
        let points = series_points(frames, self.options.reduce);
        let inner = block.inner(area);
        block.render(area, buf);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        let grid = grid_for(series.len(), inner);
        let named = series.len() > 1;
        for (index, one) in series.iter().enumerate().take(grid.shown) {
            let column = points.get(index).copied().unwrap_or(&[]);
            self.render_tile(&grid, index, one, column, named, buf);
        }
        if let Some(note) = grid.note {
            draw_note(note, grid.dropped, self.theme, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::buffer::Cell;
    use ratatui_core::layout::Position;
    use thunderbox_core::{Field, FieldConfig, Threshold};

    /// Returns one frame that holds one series of numbers.
    fn frames(values: Vec<Option<f64>>) -> Vec<Frame> {
        let stamps: Vec<i64> = (0..values.len() as i64).map(|i| i * 1000).collect();
        vec![Frame::named(
            "cpu",
            vec![Field::time("time", stamps), Field::f64("usage", values)],
        )]
    }

    /// Returns one frame that holds one series for each name.
    fn many_frames(names: &[&str]) -> Vec<Frame> {
        let mut fields = vec![Field::time("time", vec![0, 1000, 2000])];
        for (i, name) in names.iter().enumerate() {
            let base = i as f64 + 1.0;
            fields.push(Field::f64(
                *name,
                vec![Some(base), Some(base * 2.0), Some(base * 3.0)],
            ));
        }
        vec![Frame::named("board", fields)]
    }

    /// Returns one frame that holds a number of series.
    fn wide_frames(count: usize) -> Vec<Frame> {
        let names: Vec<String> = (0..count).map(|i| format!("series{i}")).collect();
        let borrowed: Vec<&str> = names.iter().map(String::as_str).collect();
        many_frames(&borrowed)
    }

    /// Draws the panel into a plain buffer. This test needs no terminal backend
    /// and no terminal. That fact is the promise of this crate.
    fn render(panel: &StatPanel<'_>, w: u16, h: u16) -> Buffer {
        let area = Rect::new(0, 0, w, h);
        let mut buf = Buffer::empty(area);
        panel.render(area, &mut buf);
        buf
    }

    /// Draws the panel at a place inside a larger buffer.
    ///
    /// The buffer holds three empty cells on each side of the panel. A cell
    /// that the panel draws outside its own area is therefore visible. The
    /// function returns the buffer and the area of the panel.
    fn render_inside(panel: &StatPanel<'_>, w: u16, h: u16) -> (Buffer, Rect) {
        let area = Rect::new(3, 3, w, h);
        let mut buf = Buffer::empty(Rect::new(0, 0, w.saturating_add(6), h.saturating_add(6)));
        panel.render(area, &mut buf);
        (buf, area)
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

    /// Returns the characters of one row of the buffer, from one place.
    fn cells(buf: &Buffer, x: u16, y: u16, count: u16) -> String {
        (x..x.saturating_add(count))
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect()
    }

    #[test]
    fn the_panel_shows_that_the_data_source_has_not_answered() {
        let theme = Theme::default();
        let p = StatPanel::new("CPU", PanelData::Loading, &theme);
        assert!(text(&render(&p, 30, 8)).contains("loading"));
    }

    #[test]
    fn the_panel_shows_the_text_of_an_error() {
        let theme = Theme::default();
        let p = StatPanel::new("CPU", PanelData::Error("upstream refused"), &theme);
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
        let p = StatPanel::new("CPU", PanelData::Frames(&empty), &theme);
        assert!(text(&render(&p, 30, 8)).contains("no data"));
    }

    #[test]
    fn the_panel_reports_no_data_for_a_column_of_holes() {
        let f = frames(vec![None, None, None]);
        let theme = Theme::default();
        let p = StatPanel::new("CPU", PanelData::Frames(&f), &theme);
        assert!(text(&render(&p, 30, 8)).contains("no data"));
    }

    #[test]
    fn the_panel_reports_no_data_for_a_value_that_is_not_finite() {
        let f = frames(vec![Some(f64::NAN), Some(f64::INFINITY)]);
        let theme = Theme::default();
        let p = StatPanel::new("CPU", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 30, 8));
        assert!(out.contains("no data"), "The panel drew this. {out}");
    }

    #[test]
    fn the_panel_draws_the_shape_of_the_digit_eight() {
        let f = frames(vec![Some(8.0)]);
        let theme = Theme::default();
        let p = StatPanel::new("n", PanelData::Frames(&f), &theme);
        let buf = render(&p, 20, 8);
        assert_eq!(cells(&buf, 8, 1, 3), "▄▄▄");
        assert_eq!(cells(&buf, 8, 2, 3), "█▄█");
        assert_eq!(cells(&buf, 8, 3, 3), "█▄█");
    }

    #[test]
    fn the_font_holds_the_shape_of_each_digit() {
        assert_eq!(glyph('8'), Some(&["▄▄▄", "█▄█", "█▄█"]));
        assert_eq!(glyph('1'), Some(&["▄▄ ", " █ ", "▄█▄"]));
        for c in "0123456789.-".chars() {
            let rows = glyph(c).unwrap_or_else(|| panic!("The font must hold {c}."));
            for row in rows {
                assert_eq!(
                    row.chars().count(),
                    usize::from(GLYPH_WIDTH),
                    "Every row must hold the same number of characters. {c}"
                );
            }
        }
        assert_eq!(glyph('K'), None, "The font holds no letter.");
        assert_eq!(usize::from(GLYPH_HEIGHT), GLYPH_ROWS);
    }

    #[test]
    fn the_panel_draws_each_digit_on_its_own() {
        let theme = Theme::default();
        for digit in 0..=9_u32 {
            let f = frames(vec![Some(f64::from(digit))]);
            let p = StatPanel::new("n", PanelData::Frames(&f), &theme);
            let buf = render(&p, 20, 8);
            let c = char::from_digit(digit, 10).unwrap_or('0');
            let rows = glyph(c).unwrap_or(&["", "", ""]);
            for (row, line) in rows.iter().enumerate() {
                assert_eq!(
                    cells(&buf, 8, 1 + row as u16, GLYPH_WIDTH),
                    *line,
                    "The panel must draw the digit {digit}."
                );
            }
        }
    }

    #[test]
    fn the_panel_draws_the_unit_beside_the_large_number() {
        let f = frames(vec![Some(42.0)]);
        let theme = Theme::default();
        let options = PanelOptions {
            unit: Some("percent"),
            ..Default::default()
        };
        let p = StatPanel::new("n", PanelData::Frames(&f), &theme).options(options);
        let buf = render(&p, 20, 8);
        // The head is "42", which is seven cells wide. The tail follows one
        // cell later, on the bottom row of the large form.
        assert_eq!(cells(&buf, 13, 3, 1), "%");
        assert_eq!(
            cells(&buf, 5, 1, 3),
            "▄ ▄",
            "The panel must draw the digit four."
        );
    }

    #[test]
    fn the_panel_draws_plain_text_in_a_tile_that_is_too_short() {
        let f = frames(vec![Some(8.0)]);
        let theme = Theme::default();
        let p = StatPanel::new("n", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 20, 4));
        assert!(out.contains('8'), "The panel must draw the number. {out}");
        assert!(
            !out.contains('█'),
            "A tile of two rows holds no large number. {out}"
        );
    }

    #[test]
    fn the_panel_draws_plain_text_for_a_number_that_the_font_does_not_cover() {
        // The em dash stands for a value that is not finite. The font holds no
        // em dash.
        assert_eq!(plan_large(crate::MISSING, 40, 10), None);
        let f = frames(vec![Some(1.0)]);
        let theme = Theme::default();
        let p = StatPanel::new("n", PanelData::Frames(&f), &theme);
        assert!(text(&render(&p, 20, 8)).contains('▄'));
    }

    #[test]
    fn the_panel_draws_one_tile_for_each_series() {
        let f = many_frames(&["alpha", "beta", "gamma"]);
        let theme = Theme::default();
        let p = StatPanel::new("board", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 40, 12));
        for name in ["alpha", "beta", "gamma"] {
            assert!(out.contains(name), "The panel must draw {name}. {out}");
        }
    }

    #[test]
    fn the_panel_draws_no_name_for_one_series() {
        let f = many_frames(&["alpha"]);
        let theme = Theme::default();
        let p = StatPanel::new("board", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 40, 12));
        assert!(
            !out.contains("alpha"),
            "One series fills the tile, so the name adds nothing. {out}"
        );
    }

    #[test]
    fn the_panel_keeps_two_names_of_wide_characters_apart() {
        let theme = Theme::default();
        let first = many_frames(&["x", "中央処理装置", "z"]);
        let second = many_frames(&["x", "上央処理装置", "z"]);
        let p = StatPanel::new("board", PanelData::Frames(&first), &theme);
        let q = StatPanel::new("board", PanelData::Frames(&second), &theme);
        let one = render(&p, 30, 8);
        let two = render(&q, 30, 8);
        // The tile of the second series starts at the column 10, and it holds
        // eight cells. A wide character takes two cells, and the buffer keeps
        // the second cell of that character empty. The name therefore reads
        // as three characters and a full stop.
        assert_eq!(
            cells(&one, 10, 1, 7),
            "中 央 処 .",
            "The panel must cut the name at the end, and mark the cut."
        );
        assert_ne!(
            text(&one),
            text(&two),
            "Two names that differ must draw two pictures that differ."
        );
    }

    #[test]
    fn the_panel_states_the_number_of_series_that_it_dropped() {
        let f = wide_frames(100);
        let theme = Theme::default();
        let p = StatPanel::new("board", PanelData::Frames(&f), &theme);
        let wide = grid_for(100, Rect::new(1, 1, 38, 10));
        assert!(wide.dropped > 0, "The panel cannot draw 100 series here.");
        let out = text(&render(&p, 40, 12));
        assert!(
            out.contains(&dropped_text(wide.dropped, 38)),
            "A wide panel must state the number in a whole sentence. {out}"
        );
        let narrow = grid_for(100, Rect::new(1, 1, 10, 6));
        let out = text(&render(&p, 12, 8));
        assert!(
            out.contains(&dropped_text(narrow.dropped, 10)),
            "A narrow panel must state the number in the short form. {out}"
        );
    }

    #[test]
    fn dropped_text_states_the_count() {
        assert_eq!(dropped_text(4, 30), "4 series do not fit.");
        assert_eq!(dropped_text(4, 19), "+4");
    }

    #[test]
    fn the_panel_draws_the_small_chart_under_the_number() {
        let f = frames(vec![Some(1.0), Some(5.0), Some(9.0)]);
        let theme = Theme::default();
        let p = StatPanel::new("n", PanelData::Frames(&f), &theme);
        let buf = render(&p, 20, 10);
        let out = text(&buf);
        // The number takes the rows 1 to 3, and the chart takes the rows 5 to
        // 8. The chart draws the most recent point at the right edge. The
        // largest point fills the four rows. The smallest point fills a part
        // of one row.
        assert_eq!(
            cells(&buf, 16, 8, 3),
            "▃██",
            "The chart must draw one bar for each point. {out}"
        );
        assert_eq!(
            cells(&buf, 16, 5, 3),
            "  █",
            "Only the largest point reaches the top row of the chart. {out}"
        );
        assert!(
            FONT.iter()
                .all(|(_, rows)| rows.iter().all(|row| !row.contains('▃'))),
            "The font must hold no bar of this height. A large number therefore \
             cannot pass this test in place of a chart."
        );
    }

    #[test]
    fn the_small_chart_keeps_a_hole_as_a_hole() {
        let f = frames(vec![Some(1.0), None, Some(9.0)]);
        let theme = Theme::default();
        let p = StatPanel::new("n", PanelData::Frames(&f), &theme);
        let out = text(&render(&p, 20, 10));
        assert!(
            out.contains('░'),
            "The chart must draw the symbol of an absent value. {out}"
        );
    }

    #[test]
    fn the_panel_counts_a_column_that_holds_only_holes() {
        let f = frames(vec![None, None, None]);
        let theme = Theme::default();
        let options = PanelOptions {
            reduce: Reduce::Count,
            ..Default::default()
        };
        let p = StatPanel::new("n", PanelData::Frames(&f), &theme).options(options);
        let out = text(&render(&p, 20, 10));
        assert!(
            out.contains("█ █"),
            "The count of a column of holes is zero. {out}"
        );
        assert!(
            out.contains('░'),
            "Every point of the chart is a hole. {out}"
        );
    }

    #[test]
    fn the_panel_reads_the_threshold_of_the_column() {
        let mut f = frames(vec![Some(95.0)]);
        f[0].fields[1] = f[0].fields[1].clone().with_config(FieldConfig {
            thresholds: vec![Threshold {
                value: 90.0,
                color: "critical".into(),
            }],
            ..Default::default()
        });
        let theme = Theme::default();
        let p = StatPanel::new("n", PanelData::Frames(&f), &theme);
        let buf = render(&p, 20, 8);
        assert_eq!(
            buf[(6, 1)].style().fg,
            Some(theme.critical),
            "The panel must draw the number in the colour of the threshold."
        );
    }

    #[test]
    fn split_number_cuts_the_text_at_the_first_letter() {
        assert_eq!(split_number("1.5KB"), ("1.5", "KB"));
        assert_eq!(split_number("42%"), ("42", "%"));
        assert_eq!(split_number("-1.5K"), ("-1.5", "K"));
        assert_eq!(split_number("7"), ("7", ""));
        assert_eq!(split_number(crate::MISSING), ("", crate::MISSING));
        assert_eq!(split_number(""), ("", ""));
    }

    #[test]
    fn glyphs_width_adds_one_column_between_two_characters() {
        assert_eq!(glyphs_width(0), 0);
        assert_eq!(glyphs_width(1), 3);
        assert_eq!(glyphs_width(2), 7);
        assert_eq!(glyphs_width(4), 15);
    }

    #[test]
    fn large_width_adds_the_tail_to_the_large_characters() {
        assert_eq!(large_width(0, 2), 0);
        assert_eq!(large_width(1, 0), 3);
        assert_eq!(large_width(2, 1), 9);
        assert_eq!(large_width(3, 2), 14);
    }

    #[test]
    fn large_width_holds_a_long_number_inside_the_type() {
        assert_eq!(glyphs_width(usize::MAX), u16::MAX);
        assert_eq!(large_width(usize::MAX, usize::MAX), u16::MAX);
    }

    #[test]
    fn plan_large_refuses_a_tile_that_is_too_small() {
        assert!(plan_large("42%", 9, 3).is_some());
        assert_eq!(
            plan_large("42%", 8, 3),
            None,
            "The width is one cell short."
        );
        assert_eq!(
            plan_large("42%", 9, 2),
            None,
            "The height is one row short."
        );
    }

    #[test]
    fn one_series_fills_the_whole_panel() {
        let grid = grid_for(1, Rect::new(1, 1, 38, 10));
        assert_eq!((grid.columns, grid.rows), (1, 1));
        assert_eq!((grid.shown, grid.dropped), (1, 0));
        assert_eq!(tile_of(&grid, 0), Rect::new(1, 1, 38, 10));
        assert!(grid.note.is_none());
    }

    #[test]
    fn the_grid_prefers_columns_while_a_tile_stays_wide_enough() {
        let grid = grid_for(4, Rect::new(0, 0, 40, 10));
        assert_eq!((grid.columns, grid.rows), (4, 1));
        let narrow = grid_for(4, Rect::new(0, 0, 6, 12));
        assert_eq!(narrow.columns, 1, "A narrow panel holds one tile across.");
    }

    #[test]
    fn the_grid_adds_a_row_when_the_series_do_not_fit_across() {
        // The area holds five tiles of eight cells across.
        let grid = grid_for(6, Rect::new(0, 0, 40, 10));
        assert_eq!((grid.columns, grid.rows), (5, 2));
        assert_eq!(grid.dropped, 0);
    }

    #[test]
    fn the_grid_keeps_one_row_for_the_note_when_it_drops_a_series() {
        let grid = grid_for(100, Rect::new(0, 0, 10, 6));
        assert!(grid.dropped > 0, "The panel cannot draw 100 series here.");
        assert_eq!(grid.body.height, 5);
        assert_eq!(grid.note.map(|r| r.y), Some(5));
        assert_eq!(grid.note.map(|r| r.height), Some(1));
        assert_eq!(grid.shown + grid.dropped, 100);
    }

    #[test]
    fn the_grid_drops_the_note_when_the_panel_holds_one_row() {
        let grid = grid_for(50, Rect::new(0, 0, 10, 1));
        assert!(grid.note.is_none(), "One row holds a tile, not a note.");
        assert_eq!(grid.shown, 1);
    }

    #[test]
    fn plan_size_returns_the_smallest_tile_and_keeps_two_tiles_apart() {
        let one = grid_for(1, Rect::new(0, 0, 20, 6));
        assert_eq!(plan_size(&one), (20, 6), "One tile needs no empty column.");
        // The width 58 does not divide by seven. The smallest tile is eight
        // cells wide, and one of those cells keeps two tiles apart.
        let seven = grid_for(7, Rect::new(0, 0, 58, 12));
        assert_eq!((seven.columns, seven.rows), (7, 1));
        assert_eq!(plan_size(&seven), (7, 12));
    }

    #[test]
    fn every_tile_draws_the_number_in_the_same_form() {
        // The last value of the first series is 3, and of the second series
        // is 6. Both tiles hold the large form, at the same rows.
        let f = many_frames(&["alpha", "beta"]);
        let theme = Theme::default();
        let p = StatPanel::new("board", PanelData::Frames(&f), &theme);
        let buf = render(&p, 30, 8);
        let three = glyph('3').unwrap_or(&["", "", ""]);
        let six = glyph('6').unwrap_or(&["", "", ""]);
        assert_eq!(cells(&buf, 6, 4, GLYPH_WIDTH), three[1]);
        assert_eq!(cells(&buf, 20, 4, GLYPH_WIDTH), six[1]);
    }

    #[test]
    fn the_tiles_cover_the_area_and_no_two_tiles_overlap() {
        let area = Rect::new(2, 3, 21, 7);
        let grid = grid_for(6, area);
        let mut seen = vec![0_u8; usize::from(area.width) * usize::from(area.height)];
        for index in 0..grid.shown {
            let tile = tile_of(&grid, index);
            for y in tile.y..tile.bottom() {
                for x in tile.x..tile.right() {
                    let at =
                        usize::from(y - area.y) * usize::from(area.width) + usize::from(x - area.x);
                    seen[at] += 1;
                }
            }
        }
        assert!(
            seen.iter().all(|count| *count == 1),
            "Each cell belongs to one tile. {seen:?}"
        );
    }

    #[test]
    fn tile_of_returns_an_empty_area_for_an_index_that_the_grid_does_not_hold() {
        let grid = grid_for(2, Rect::new(0, 0, 20, 4));
        let tile = tile_of(&grid, 99);
        assert_eq!((tile.width, tile.height), (0, 0));
    }

    #[test]
    fn bounds_of_ignores_a_hole_and_a_value_that_is_not_finite() {
        assert_eq!(
            bounds_of(&[Some(3.0), None, Some(f64::NAN), Some(1.0)]),
            Some((1.0, 3.0))
        );
        assert_eq!(bounds_of(&[None, None]), None);
        assert_eq!(bounds_of(&[]), None);
    }

    #[test]
    fn scale_point_keeps_a_hole_as_a_hole() {
        assert_eq!(scale_point(None, 0.0, 10.0), None);
        assert_eq!(scale_point(Some(f64::NAN), 0.0, 10.0), None);
        assert_eq!(scale_point(Some(f64::INFINITY), 0.0, 10.0), None);
    }

    #[test]
    fn scale_point_maps_the_range_onto_the_scale() {
        assert_eq!(scale_point(Some(0.0), 0.0, 10.0), Some(0));
        assert_eq!(scale_point(Some(5.0), 0.0, 10.0), Some(CHART_SCALE / 2));
        assert_eq!(scale_point(Some(10.0), 0.0, 10.0), Some(CHART_SCALE));
    }

    #[test]
    fn scale_point_holds_a_value_inside_the_range() {
        assert_eq!(scale_point(Some(-5.0), 0.0, 10.0), Some(0));
        assert_eq!(scale_point(Some(99.0), 0.0, 10.0), Some(CHART_SCALE));
        assert_eq!(
            scale_point(Some(4.0), 5.0, 5.0),
            Some(0),
            "A range of no width must not divide by zero."
        );
    }

    #[test]
    fn series_points_returns_one_column_for_each_series() {
        let f = many_frames(&["alpha", "beta"]);
        let points = series_points(&f, Reduce::Last);
        let series = reduce_frames(&f, Reduce::Last);
        assert_eq!(points.len(), series.len());
        assert_eq!(points[0], &[Some(1.0), Some(2.0), Some(3.0)]);
        assert_eq!(points[1], &[Some(2.0), Some(4.0), Some(6.0)]);
    }

    #[test]
    fn series_points_drops_the_column_that_reduce_frames_drops() {
        let f = vec![Frame::named(
            "cpu",
            vec![
                Field::time("time", vec![0, 1000]),
                Field::f64("holes", vec![None, None]),
                Field::f64("usage", vec![Some(1.0), Some(2.0)]),
            ],
        )];
        let points = series_points(&f, Reduce::Last);
        let series = reduce_frames(&f, Reduce::Last);
        assert_eq!(points.len(), 1);
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].name, "usage");
        assert_eq!(points[0], &[Some(1.0), Some(2.0)]);
    }

    #[test]
    fn bar_set_reads_the_marker() {
        assert_eq!(bar_set(Marker::Block), symbols::bar::THREE_LEVELS);
        assert_eq!(bar_set(Marker::Braille), symbols::bar::NINE_LEVELS);
    }

    #[test]
    fn the_panel_draws_nothing_outside_its_own_area() {
        let theme = Theme::default();
        let one = frames(vec![Some(1.0), None, Some(3000.0)]);
        let some = many_frames(&["alpha", "beta", "gamma"]);
        let lots = wide_frames(60);
        let sets = [
            PanelData::Loading,
            PanelData::Error("upstream refused"),
            PanelData::Frames(&one),
            PanelData::Frames(&some),
            PanelData::Frames(&lots),
        ];
        let sizes = [
            (3, 3),
            (4, 3),
            (8, 4),
            (20, 3),
            (40, 12),
            (120, 40),
            (1, 1),
            (3, 60),
            (200, 3),
        ];
        for set in sets {
            for (w, h) in sizes {
                let p = StatPanel::new("board", set, &theme).focused(true);
                let (buf, area) = render_inside(&p, w, h);
                for y in buf.area.top()..buf.area.bottom() {
                    for x in buf.area.left()..buf.area.right() {
                        if area.contains(Position::new(x, y)) {
                            continue;
                        }
                        assert_eq!(
                            buf[(x, y)],
                            Cell::EMPTY,
                            "The panel must draw nothing outside its own area. \
                             The area is {area:?}, and the cell is {x} {y}."
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_panel_does_not_panic_in_any_area() {
        let theme = Theme::default();
        let empty: Vec<Frame> = vec![];
        let one = frames(vec![Some(1.0), None, Some(3000.0)]);
        let some = many_frames(&["alpha", "beta", "gamma"]);
        let lots = wide_frames(60);
        let sets: [&[Frame]; 4] = [&empty, &one, &some, &lots];
        let sizes = [
            (3, 3),
            (4, 3),
            (8, 4),
            (20, 3),
            (40, 12),
            (120, 40),
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 60),
            (200, 3),
        ];
        for set in sets {
            for (w, h) in sizes {
                for marker in [Marker::Braille, Marker::Block] {
                    let p = StatPanel::new("board", PanelData::Frames(set), &theme)
                        .marker(marker)
                        .focused(true);
                    let _ = render(&p, w, h);
                }
            }
            for w in 0..=24_u16 {
                for h in 0..=12_u16 {
                    let p = StatPanel::new("board", PanelData::Frames(set), &theme);
                    let _ = render(&p, w, h);
                }
            }
        }
    }
}
