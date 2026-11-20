//! The arithmetic that divides the grid of a dashboard onto the cells of a
//! terminal.
//!
//! The renderer draws each panel inside the size that [`panel_size`] returns.
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

use ratatui_core::layout::Size;
use thunderbox_core::{Dashboard, GRID_COLUMNS, GridPos};

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

/// Returns the size of one panel, in terminal cells.
///
/// The value `rows` is the number of grid rows of the dashboard, which
/// [`rows_of_grid`] returns. The value `area` is the size of the area that
/// holds every panel. The function scales the cells of one panel onto that
/// area.
///
/// The height of a panel follows both `y` and `h`, because [`scale`] multiplies
/// before it divides. The width follows both `x` and `w` for the same reason,
/// and [`panel_width`] states that rule.
///
/// The function adds `y` and `h` with `u16::saturating_add`. A dashboard that
/// no caller validated therefore cannot make this function panic.
pub(crate) fn panel_size(grid: GridPos, rows: u16, area: Size) -> Size {
    let top = scale(grid.y, rows, area.height);
    let bottom = scale(grid.y.saturating_add(grid.h), rows, area.height);
    Size {
        width: panel_width(grid, area.width),
        height: bottom.saturating_sub(top),
    }
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
    use thunderbox_core::max_data_points;
    use thunderbox_widgets::draws_a_panel;

    /// Returns the position of a panel of one row.
    fn at(x: u16, w: u16) -> GridPos {
        GridPos { x, y: 0, w, h: 1 }
    }

    /// The five widths of the default dashboard, on a terminal of 110 cells.
    /// `docs/ARCHITECTURE.md` section 12 holds the same numbers.
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
        let rows = rows_of_grid(dashboard);
        dashboard
            .panels
            .iter()
            .filter(|panel| draws_a_panel(panel_size(panel.grid, rows, area)))
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
        let size = panel_size(
            dashboard.panels[0].grid,
            rows_of_grid(&dashboard),
            Size::new(100, 40),
        );
        assert!(
            size.height <= 40,
            "A panel must hold no cell past the area. It held {} cells.",
            size.height
        );
    }

    /// The renderer drops one panel of eight on a terminal of 24 rows.
    /// `docs/ARCHITECTURE.md` section 12 holds the same numbers.
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
    /// 9 rows. `docs/ARCHITECTURE.md` section 12 holds the same numbers.
    #[test]
    fn the_default_dashboard_drops_a_panel_only_under_a_terminal_of_ten_rows() {
        let dashboard = Dashboard::from_toml(SIX_PANELS).expect("The dashboard must be correct.");
        let counts: Vec<usize> = (7..=11)
            .map(|terminal| drawn(&dashboard, Size::new(110, terminal - 1)))
            .collect();
        assert_eq!(
            counts,
            vec![0, 1, 4, 6, 6],
            "A terminal of 9 rows draws four of the six panels, and a terminal of 10 rows \
             draws every panel."
        );
    }

    /// The dashboard that holds the grid of the default dashboard. The panels
    /// of that dashboard hold 16, 8, 8, 8, 8 and 24 columns of the grid, and
    /// they stand in three rows.
    const SIX_PANELS: &str = r#"
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
grid = { x = 0, y = 13, w = 24, h = 6 }
"#;
}
