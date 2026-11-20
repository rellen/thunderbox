//! The arithmetic that divides the grid of a dashboard onto the cells of a
//! terminal.
//!
//! The renderer draws each panel inside the place that [`places`] returns.
//! The runtime asks the data source of that panel for the points of the same
//! width. One module holds both numbers, so the two can never differ.
//!
//! [`thunderbox_widgets::draws_a_panel`] states whether a panel of a size holds a
//! cell for its data. A panel that this function rejects is a panel that the
//! renderer drops. The renderer reads that function for each panel, and the
//! runtime reads it for the same panel. A panel that the renderer drops
//! therefore asks a data source for nothing.
//!
//! `thunderbox-widgets` holds that function, because a panel of that crate draws
//! nothing at a smaller size. The smallest size therefore belongs to the panel
//! that draws, and this crate reads it.
//!
//! # Row panels
//!
//! A row panel holds each panel below it on the grid, up to the next row
//! panel. [`places`] gives each row panel one line of the terminal, and it
//! scales the other rows of the grid onto the lines that remain. A collapsed
//! row panel hides the panels that it holds. The rows of the grid that those
//! panels held then leave the scale, so the panels below move up and every
//! panel that remains grows.
//!
//! A dashboard that holds no row panel gives every panel the place that it
//! held before row panels existed. The test
//! `a_dashboard_with_no_row_panel_keeps_every_place` holds that rule.

use std::collections::BTreeSet;

use ratatui_core::layout::Rect;
use thunderbox_core::{Dashboard, GRID_COLUMNS, GridPos, PanelKind};

/// The row panels that are collapsed now, by their position in the dashboard.
///
/// [`crate::DashboardRuntime`] holds this value, and each [`crate::Snapshot`]
/// carries a copy of it. [`crate::DashboardView`] draws the copy of the
/// snapshot, so the renderer and the runtime hide the same panels.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CollapsedRows(BTreeSet<usize>);

impl CollapsedRows {
    /// Returns the row panels that the dashboard states as collapsed.
    ///
    /// The field `collapsed` of a row panel states its first state.
    pub fn of(dashboard: &Dashboard) -> Self {
        Self(
            dashboard
                .panels
                .iter()
                .enumerate()
                .filter(|(_, panel)| panel.kind == PanelKind::Row && panel.collapsed == Some(true))
                .map(|(index, _)| index)
                .collect(),
        )
    }

    /// Returns `true` when the panel at this position is a collapsed row panel.
    pub fn contains(&self, index: usize) -> bool {
        self.0.contains(&index)
    }

    /// Collapses an expanded row panel, or expands a collapsed one.
    pub(crate) fn toggle(&mut self, index: usize) {
        if !self.0.remove(&index) {
            self.0.insert(index);
        }
    }
}

/// Returns the row panel that holds each panel, in the order of the dashboard.
///
/// A row panel holds each panel that starts below its line, up to the line of
/// the next row panel. A panel above the first row panel belongs to no row
/// panel, and neither does a row panel itself. `Dashboard::validate` rejects a
/// panel that crosses the line of a row panel, so each panel belongs to one
/// place.
pub(crate) fn owners(dashboard: &Dashboard) -> Vec<Option<usize>> {
    let rows = row_lines(dashboard);
    dashboard
        .panels
        .iter()
        .map(|panel| {
            if panel.kind == PanelKind::Row {
                return None;
            }
            rows.iter()
                .rev()
                .find(|row| row.y < panel.grid.y)
                .map(|row| row.index)
        })
        .collect()
}

/// Returns `true` for each panel that a collapsed row panel hides, in the order
/// of the dashboard.
pub(crate) fn hidden(dashboard: &Dashboard, collapsed: &CollapsedRows) -> Vec<bool> {
    owners(dashboard)
        .into_iter()
        .map(|owner| owner.is_some_and(|row| collapsed.contains(row)))
        .collect()
}

/// One row panel, and the rows of the grid that its line covers.
#[derive(Debug, Clone, Copy)]
struct RowLine {
    /// The position of the row panel in the dashboard.
    index: usize,
    /// The first row of the grid that the line covers.
    y: u16,
    /// The number of rows of the grid that the line covers.
    h: u16,
}

/// Returns each row panel of the dashboard, from the top to the bottom.
fn row_lines(dashboard: &Dashboard) -> Vec<RowLine> {
    let mut rows: Vec<RowLine> = dashboard
        .panels
        .iter()
        .enumerate()
        .filter(|(_, panel)| panel.kind == PanelKind::Row)
        .map(|(index, panel)| RowLine {
            index,
            y: panel.grid.y,
            h: panel.grid.h,
        })
        .collect();
    rows.sort_by_key(|row| (row.y, row.index));
    rows
}

/// Returns the place of each panel inside an area, in the order of the
/// dashboard.
///
/// The value is `None` for a panel that a collapsed row panel hides. Every
/// other panel takes a place, and that place can be too small to draw.
/// [`thunderbox_widgets::draws_a_panel`] decides that for a panel that shows
/// data. A row panel draws when its place holds one line.
///
/// The function gives each row panel one line, and it scales the other rows of
/// the grid onto the lines that remain. Two sets of rows of the grid leave that
/// scale:
///
/// - the rows that the line of a row panel covers;
/// - the rows that a collapsed row panel holds.
///
/// A panel below a collapsed row panel therefore moves up, and each panel that
/// remains grows.
///
/// A terminal can hold fewer lines than the dashboard holds row panels. The
/// last row panels then take no line at all, and every other panel takes no
/// height.
///
/// The renderer draws each panel at this place, and the runtime asks the data
/// source of each panel for the points of the same width. One function returns
/// both numbers, so the two parts cannot differ.
///
/// The function holds every place inside the area, and it saturates each sum.
/// A dashboard that no caller validated therefore cannot make it panic.
pub(crate) fn places(
    dashboard: &Dashboard,
    collapsed: &CollapsedRows,
    area: Rect,
) -> Vec<Option<Rect>> {
    let total = rows_of_grid(dashboard);
    let rows = row_lines(dashboard);
    let hidden = hidden(dashboard, collapsed);

    // This list holds the rows of the grid that leave the scale, as pairs of a
    // start and an end. A collapsed row panel holds the rows up to the next row
    // panel, or up to the bottom of the grid.
    let mut gone: Vec<(u16, u16)> = Vec::new();
    for (k, row) in rows.iter().enumerate() {
        let line_end = row.y.saturating_add(row.h);
        gone.push((row.y, line_end));
        if collapsed.contains(row.index) {
            let end = rows.get(k + 1).map_or(total, |next| next.y).max(line_end);
            gone.push((line_end, end));
        }
    }
    // This closure returns the rows of the grid above `y` that stay in the
    // scale.
    let kept = |y: u16| -> u16 {
        let left: u16 = gone
            .iter()
            .map(|(start, end)| (*end).min(y).saturating_sub(*start))
            .fold(0, u16::saturating_add);
        y.saturating_sub(left)
    };
    let units = kept(total);
    let lines = u16::try_from(rows.len()).unwrap_or(u16::MAX);
    let pool = area.height.saturating_sub(lines);
    // This closure returns the number of row panels above `y`. Each one takes
    // one line.
    let lines_above = |y: u16| -> u16 {
        u16::try_from(rows.iter().filter(|row| row.y < y).count()).unwrap_or(u16::MAX)
    };
    // This closure returns the distance from the top of the area to the start
    // of a grid row. It holds that distance inside the area.
    let offset = |above: u16, y: u16| -> u16 {
        above
            .saturating_add(scale(kept(y), units, pool))
            .min(area.height)
    };

    dashboard
        .panels
        .iter()
        .zip(hidden)
        .map(|(panel, hidden)| {
            if hidden {
                return None;
            }
            let g = panel.grid;
            let above = lines_above(g.y);
            let top = offset(above, g.y);
            let height = if panel.kind == PanelKind::Row {
                u16::from(top < area.height)
            } else {
                offset(above, g.y.saturating_add(g.h)).saturating_sub(top)
            };
            Some(Rect {
                x: area.x + scale(g.x, GRID_COLUMNS, area.width),
                y: area.y + top,
                width: panel_width(g, area.width),
                height,
            })
        })
        .collect()
}

/// Returns the number of grid rows that a dashboard holds.
///
/// The rows of a grid belong to the dashboard, and the area holds none of them.
/// The panel that reaches furthest down states the number. The renderer then
/// scales that number onto the height of the area, so the tallest panel reaches
/// the bottom of the area.
///
/// The function returns at least one row. A dashboard that holds no panel would
/// otherwise divide by zero.
///
/// The function adds `y` and `h` with `u16::saturating_add`. A dashboard that
/// no caller validated therefore cannot make this function panic.
pub(crate) fn rows_of_grid(dashboard: &Dashboard) -> u16 {
    dashboard
        .panels
        .iter()
        .map(|panel| panel.grid.y.saturating_add(panel.grid.h))
        .max()
        .unwrap_or(1)
        .max(1)
}

/// Returns the width of one panel, in terminal cells.
///
/// The grid holds [`GRID_COLUMNS`] columns. The value `area_width` is the width
/// of the area that holds every panel. The function scales the columns of one
/// panel onto that area.
///
/// The width of a panel follows both `x` and `w`, because [`scale`] multiplies
/// before it divides. Two panels that hold the same number of columns can
/// therefore differ by one cell. That happens on an area whose width the grid
/// does not divide.
///
/// The function returns 0 for a panel that the area is too narrow to hold. The
/// renderer draws no such panel, and [`thunderbox_core::max_data_points`] raises
/// that width to one cell. No panel therefore asks for zero points.
///
/// The function adds `x` and `w` with `u16::saturating_add`. A dashboard that
/// no caller validated therefore cannot make this function panic.
pub(crate) fn panel_width(grid: GridPos, area_width: u16) -> u16 {
    let left = scale(grid.x, GRID_COLUMNS, area_width);
    let right = scale(grid.x.saturating_add(grid.w), GRID_COLUMNS, area_width);
    right.saturating_sub(left)
}

/// Scales `value` from a grid of `whole` units onto `span` cells.
///
/// The function uses one multiplication before the division, so that two panels
/// that touch on the grid also touch on the screen. It leaves no gap and no
/// overlap.
///
/// The function holds the result to `span` cells. A `value` that is larger than
/// `whole` belongs to a dashboard that no caller validated. The result of such
/// a value stays inside the area.
pub(crate) fn scale(value: u16, whole: u16, span: u16) -> u16 {
    if whole == 0 {
        return 0;
    }
    let cells = (u32::from(value) * u32::from(span)) / u32::from(whole);
    cells.min(u32::from(span)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::layout::Size;
    use thunderbox_core::max_data_points;
    use thunderbox_widgets::draws_a_panel;

    /// Returns the position of a panel of one row.
    fn at(x: u16, w: u16) -> GridPos {
        GridPos { x, y: 0, w, h: 1 }
    }

    /// The five widths of the default dashboard, on a terminal of 110 cells.
    /// ADR 0023 in `docs/contributor/adr` holds the same numbers.
    #[test]
    fn the_default_dashboard_holds_these_widths_on_a_terminal_of_110_cells() {
        assert_eq!(panel_width(at(0, 16), 110), 73, "The panel cpu.");
        assert_eq!(panel_width(at(16, 8), 110), 37, "The panel cpu now.");
        assert_eq!(panel_width(at(0, 8), 110), 36, "The panel memory.");
        assert_eq!(panel_width(at(8, 8), 110), 37, "The panel cores.");
        assert_eq!(panel_width(at(0, 24), 110), 110, "The panel of the table.");
    }

    /// A row of panels covers every cell of the area. The renderer leaves no
    /// gap and no overlap, so the widths must add up to the width of the area.
    #[test]
    fn the_panels_of_one_row_cover_every_cell_of_the_area() {
        for area in 0..=400_u16 {
            let two: u16 = [at(0, 16), at(16, 8)]
                .iter()
                .map(|grid| panel_width(*grid, area))
                .sum();
            assert_eq!(two, area, "Two panels must cover the area of {area} cells.");
            let three: u16 = [at(0, 8), at(8, 8), at(16, 8)]
                .iter()
                .map(|grid| panel_width(*grid, area))
                .sum();
            assert_eq!(
                three, area,
                "Three panels must cover the area of {area} cells."
            );
        }
    }

    /// A panel of the whole grid holds the whole area. The point limit of such
    /// a panel therefore does not change under this rule.
    #[test]
    fn a_panel_of_the_whole_grid_holds_the_whole_area() {
        for area in 0..=400_u16 {
            assert_eq!(panel_width(at(0, GRID_COLUMNS), area), area);
        }
    }

    /// No panel asks a data source for zero points. A width of zero reaches
    /// `max_data_points`, and that function raises it to one cell.
    #[test]
    fn no_panel_asks_for_zero_points() {
        for area in 0..=200_u16 {
            for w in 1..=GRID_COLUMNS {
                for x in 0..=(GRID_COLUMNS - w) {
                    let points = max_data_points(panel_width(at(x, w), area), 2);
                    assert!(
                        points >= 1,
                        "A panel of x={x} and w={w} on {area} cells asked for {points} points."
                    );
                }
            }
        }
    }

    /// No panel holds more cells than the area holds.
    #[test]
    fn no_panel_is_wider_than_the_area() {
        for area in 0..=200_u16 {
            for w in 1..=GRID_COLUMNS {
                for x in 0..=(GRID_COLUMNS - w) {
                    let width = panel_width(at(x, w), area);
                    assert!(
                        width <= area,
                        "A panel of x={x} and w={w} held {width} cells of {area}."
                    );
                }
            }
        }
    }

    /// A panel that no caller validated must return a width, and it must not
    /// end the program.
    #[test]
    fn a_panel_that_spans_past_the_grid_holds_no_cell_past_the_area() {
        assert_eq!(panel_width(at(u16::MAX, u16::MAX), 100), 0);
        assert_eq!(panel_width(at(0, u16::MAX), 100), 100);
    }

    /// Returns the number of panels that the renderer draws in this area.
    fn drawn(dashboard: &Dashboard, area: Size) -> usize {
        places(
            dashboard,
            &CollapsedRows::default(),
            Rect::new(0, 0, area.width, area.height),
        )
        .into_iter()
        .flatten()
        .filter(|place| draws_a_panel(place.as_size()))
        .count()
    }

    /// Returns a dashboard of `count` panels, one above the other.
    ///
    /// Each panel holds the whole width of the grid and 8 rows of it.
    fn column(count: u16) -> Dashboard {
        let mut text = String::from("title = \"test\"\n");
        for index in 0..count {
            text.push_str(&format!(
                "[[panel]]\ntitle = \"panel {index}\"\nkind = \"timeseries\"\n\
                 grid = {{ x = 0, y = {}, w = 24, h = 8 }}\n",
                index * 8
            ));
        }
        Dashboard::from_toml(&text).expect("The dashboard must be correct.")
    }

    /// The rows of a grid belong to the dashboard. The panel that reaches
    /// furthest down states the number, and the area states none of it.
    #[test]
    fn the_tallest_panel_states_the_rows_of_the_grid() {
        assert_eq!(rows_of_grid(&column(1)), 8);
        assert_eq!(rows_of_grid(&column(8)), 64);
    }

    /// A dashboard that no caller validated must return a number of rows, and
    /// it must not end the program. `Dashboard::validate` bounds `x` and `w`
    /// against the columns of the grid, and it bounds neither `y` nor `h`.
    #[test]
    fn a_dashboard_that_spans_past_the_grid_holds_a_number_of_rows() {
        let text = format!(
            "title = \"test\"\n[[panel]]\ntitle = \"one\"\nkind = \"timeseries\"\n\
             grid = {{ x = 0, y = {}, w = 24, h = {} }}\n",
            u16::MAX,
            u16::MAX
        );
        let dashboard = Dashboard::from_toml(&text).expect("The dashboard must be correct.");
        assert!(
            dashboard.validate().is_ok(),
            "This dashboard passes the check that the runtime makes."
        );
        assert_eq!(rows_of_grid(&dashboard), u16::MAX);
        let place = places(
            &dashboard,
            &CollapsedRows::default(),
            Rect::new(0, 0, 100, 40),
        )[0]
        .expect("No row panel hides this panel.");
        assert!(
            place.bottom() <= 40,
            "A panel must hold no cell past the area. It reached the row {}.",
            place.bottom()
        );
    }

    /// The renderer drops one panel of eight on a terminal of 24 rows.
    /// ADR 0024 in `docs/contributor/adr` holds the same numbers.
    ///
    /// The area of the panels is one row shorter than the terminal, because the
    /// program that runs alone keeps one row for the status row.
    #[test]
    fn a_column_of_eight_panels_drops_these_panels_on_each_terminal() {
        let dashboard = column(8);
        let counts: Vec<usize> = (17..=25)
            .map(|terminal| drawn(&dashboard, Size::new(110, terminal - 1)))
            .collect();
        assert_eq!(
            counts,
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
            "A terminal of 17 rows draws no panel, and a terminal of 25 rows draws every panel."
        );
    }

    /// The renderer drops no panel of the default dashboard above a terminal of
    /// 9 rows. ADR 0024 in `docs/contributor/adr` holds the same numbers.
    #[test]
    fn the_default_dashboard_drops_a_panel_only_under_a_terminal_of_ten_rows() {
        let dashboard = Dashboard::from_toml(SEVEN_PANELS).expect("The dashboard must be correct.");
        let counts: Vec<usize> = (7..=11)
            .map(|terminal| drawn(&dashboard, Size::new(110, terminal - 1)))
            .collect();
        assert_eq!(
            counts,
            vec![0, 2, 5, 7, 7],
            "A terminal of 9 rows draws five of the seven panels, and a terminal of 10 rows \
             draws every panel."
        );
    }

    /// The dashboard that holds the grid of the default dashboard. The panels
    /// of that dashboard hold 16, 8, 8, 8, 8, 12 and 12 columns of the grid,
    /// and they stand in three rows.
    const SEVEN_PANELS: &str = r#"
title = "test"
[[panel]]
title = "cpu"
kind = "timeseries"
grid = { x = 0, y = 0, w = 16, h = 7 }
[[panel]]
title = "cpu now"
kind = "stat"
grid = { x = 16, y = 0, w = 8, h = 7 }
[[panel]]
title = "memory"
kind = "gauge"
grid = { x = 0, y = 7, w = 8, h = 6 }
[[panel]]
title = "cores"
kind = "bargauge"
grid = { x = 8, y = 7, w = 8, h = 6 }
[[panel]]
title = "network"
kind = "timeseries"
grid = { x = 16, y = 7, w = 8, h = 6 }
[[panel]]
title = "the most recent rows for each core"
kind = "table"
grid = { x = 0, y = 13, w = 12, h = 6 }
[[panel]]
title = "processes"
kind = "top"
grid = { x = 12, y = 13, w = 12, h = 6 }
"#;

    /// A dashboard of two row panels. The first row panel holds one chart, and
    /// the second holds two panels side by side.
    ///
    /// The panels stand at these positions:
    ///
    /// - 0 is the chart above both row panels;
    /// - 1 is the first row panel, and 2 is its chart;
    /// - 3 is the second row panel, and 4 and 5 are its two panels.
    const ROWS: &str = r#"
title = "rows"
[[panel]]
title = "top"
kind = "timeseries"
grid = { x = 0, y = 0, w = 24, h = 6 }
[[panel]]
title = "first"
kind = "row"
grid = { x = 0, y = 6, w = 24, h = 1 }
[[panel]]
title = "a"
kind = "timeseries"
grid = { x = 0, y = 7, w = 24, h = 6 }
[[panel]]
title = "second"
kind = "row"
grid = { x = 0, y = 13, w = 24, h = 1 }
[[panel]]
title = "b"
kind = "timeseries"
grid = { x = 0, y = 14, w = 12, h = 6 }
[[panel]]
title = "c"
kind = "table"
grid = { x = 12, y = 14, w = 12, h = 6 }
"#;

    fn rows() -> Dashboard {
        Dashboard::from_toml(ROWS).expect("The dashboard must be correct.")
    }

    /// Returns the row panels that are collapsed, from a list of positions.
    fn collapsed(positions: &[usize]) -> CollapsedRows {
        let mut set = CollapsedRows::default();
        for position in positions {
            set.toggle(*position);
        }
        set
    }

    /// The layout that the renderer used before row panels existed. A
    /// dashboard that holds no row panel must keep each of these places.
    fn old_place(dashboard: &Dashboard, grid: GridPos, area: Rect) -> Rect {
        let rows = rows_of_grid(dashboard);
        let top = scale(grid.y, rows, area.height);
        let bottom = scale(grid.y.saturating_add(grid.h), rows, area.height);
        Rect {
            x: area.x + scale(grid.x, GRID_COLUMNS, area.width),
            y: area.y + top,
            width: panel_width(grid, area.width),
            height: bottom.saturating_sub(top),
        }
    }

    #[test]
    fn a_dashboard_with_no_row_panel_keeps_every_place() {
        for dashboard in [column(3), column(8)] {
            for width in [0_u16, 7, 80, 133] {
                for height in 0..=70_u16 {
                    let area = Rect::new(3, 2, width, height);
                    let placed = places(&dashboard, &CollapsedRows::default(), area);
                    for (panel, place) in dashboard.panels.iter().zip(placed) {
                        assert_eq!(
                            place,
                            Some(old_place(&dashboard, panel.grid, area)),
                            "{} on {width} by {height}.",
                            panel.title
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_row_panel_holds_each_panel_below_it_up_to_the_next_row_panel() {
        assert_eq!(
            owners(&rows()),
            vec![None, None, Some(1), None, Some(3), Some(3)],
            "The chart above both row panels belongs to no row panel."
        );
    }

    #[test]
    fn each_row_panel_takes_one_line_at_every_height() {
        let dash = rows();
        for height in 2..=60_u16 {
            let placed = places(
                &dash,
                &CollapsedRows::default(),
                Rect::new(0, 0, 80, height),
            );
            for index in [1, 3] {
                let place = placed[index].expect("A row panel is never hidden.");
                assert_eq!(
                    place.height, 1,
                    "The row panel {index} must take one line of {height} lines."
                );
                assert_eq!(place.width, 80, "A row panel spans the whole area.");
            }
        }
    }

    /// The panel above a row panel ends where the line of that row panel
    /// starts, and the panel below starts where the line ends. The renderer
    /// leaves no gap and no overlap.
    #[test]
    fn the_panels_around_a_row_panel_touch_its_line() {
        let dash = rows();
        for height in 2..=60_u16 {
            let placed = places(
                &dash,
                &CollapsedRows::default(),
                Rect::new(0, 5, 80, height),
            );
            let at = |index: usize| placed[index].expect("No row panel is collapsed.");
            assert_eq!(at(0).bottom(), at(1).y, "{height}");
            assert_eq!(at(1).bottom(), at(2).y, "{height}");
            assert_eq!(at(2).bottom(), at(3).y, "{height}");
            assert_eq!(at(3).bottom(), at(4).y, "{height}");
            assert_eq!(
                at(4).y,
                at(5).y,
                "The two panels share one row of the grid."
            );
            assert_eq!(
                at(4).bottom(),
                5 + height,
                "The last panels reach the bottom."
            );
        }
    }

    #[test]
    fn a_collapsed_row_panel_hides_the_panels_that_it_holds() {
        let placed = places(&rows(), &collapsed(&[3]), Rect::new(0, 0, 80, 30));
        assert!(placed[4].is_none() && placed[5].is_none(), "{placed:?}");
        assert!(placed[3].is_some(), "The row panel itself stays.");
        assert!(placed[0].is_some() && placed[2].is_some(), "{placed:?}");
    }

    /// The person chose this behaviour: the panels below a collapsed row panel
    /// move up, and every panel that remains grows to fill the terminal.
    #[test]
    fn the_panels_below_a_collapsed_row_panel_move_up_and_every_panel_grows() {
        let dash = rows();
        let area = Rect::new(0, 0, 80, 30);
        let open = places(&dash, &CollapsedRows::default(), area);
        let shut = places(&dash, &collapsed(&[1]), area);
        let (open_top, shut_top) = (open[0].unwrap(), shut[0].unwrap());
        assert!(
            shut_top.height > open_top.height,
            "{open_top:?} {shut_top:?}"
        );
        let (open_b, shut_b) = (open[4].unwrap(), shut[4].unwrap());
        assert!(
            shut_b.y < open_b.y,
            "The panel below moves up. {open_b:?} {shut_b:?}"
        );
        assert!(shut_b.height > open_b.height, "{open_b:?} {shut_b:?}");
        assert_eq!(
            shut[1].unwrap().bottom(),
            shut[3].unwrap().y,
            "The two lines must touch."
        );
        assert_eq!(
            shut_b.bottom(),
            30,
            "The last panels still reach the bottom."
        );
    }

    /// A terminal can hold fewer lines than the dashboard holds row panels. No
    /// place may then reach past the area, and no two places may overlap.
    #[test]
    fn every_place_stays_inside_the_area_and_no_two_places_overlap() {
        let dash = rows();
        for state in [vec![], vec![1], vec![3], vec![1, 3]] {
            for height in 0..=40_u16 {
                let area = Rect::new(2, 3, 50, height);
                let placed: Vec<Rect> = places(&dash, &collapsed(&state), area)
                    .into_iter()
                    .flatten()
                    .filter(|place| place.height > 0)
                    .collect();
                for place in &placed {
                    assert!(
                        area.contains(place.as_position()) && place.bottom() <= area.bottom(),
                        "{place:?} lies outside {area:?}. {state:?}"
                    );
                }
                for (i, one) in placed.iter().enumerate() {
                    for other in &placed[i + 1..] {
                        assert!(
                            !one.intersects(*other),
                            "{one:?} and {other:?} overlap at {height} lines. {state:?}"
                        );
                    }
                }
            }
        }
    }
}
