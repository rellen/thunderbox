//! The part that draws a whole dashboard, and that reads a key.

use std::collections::BTreeMap;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::Widget;
use thunderbox_core::{Align, Dashboard, GridPos, Panel, PanelKind};
use thunderbox_widgets::{
    BarGaugePanel, GaugePanel, Marker, PanelData, PanelOptions, RowPanel, StatPanel, TablePanel,
    Theme, TimeSeriesPanel, TopPanel, draws_a_panel,
};

use crate::grid::{hidden, owners, places};
use crate::snapshot::{PanelState, Snapshot};

/// The answer that [`DashboardView::handle_key`] returns.
///
/// The program that holds the dashboard keeps its own key bindings. thunderbox
/// therefore reports whether it used a key. The program acts on every key that
/// thunderbox did not use.
///
/// [`DashboardView::handle_key`]: crate::DashboardView::handle_key
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consumed {
    /// thunderbox used the key. The program must not act on it.
    Yes,
    /// thunderbox did not use the key. The program can act on it.
    No,
}

impl Consumed {
    /// Returns `true` if thunderbox used the key.
    pub fn is_consumed(self) -> bool {
        self == Self::Yes
    }
}

/// The selection and the settings that the code which draws needs.
///
/// This type holds no data. The data arrives in a [`Snapshot`]. A program can
/// therefore hold one view and draw many snapshots.
#[derive(Debug, Clone)]
pub struct DashboardView {
    theme: Theme,
    marker: Marker,
    selected: usize,
    focused: bool,
}

impl Default for DashboardView {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            marker: Marker::default(),
            selected: 0,
            focused: true,
        }
    }
}

impl DashboardView {
    /// Returns a view that uses the default theme and the default marker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the colours.
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Sets the glyph that a chart draws a point with.
    pub fn with_marker(mut self, marker: Marker) -> Self {
        self.marker = marker;
        self
    }

    /// Returns the glyph that a chart draws a point with.
    pub fn marker(&self) -> Marker {
        self.marker
    }

    /// Returns the index of the panel that a person has selected.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// States whether the dashboard has the attention of the person.
    ///
    /// The program decides this. A program that holds a dashboard beside its
    /// own widgets passes `false` while the person works elsewhere. thunderbox
    /// then uses no key at all.
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Returns `true` if the dashboard has the attention of the person.
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Reads a key and reports whether thunderbox used it.
    ///
    /// thunderbox uses these keys only:
    ///
    /// - `Tab` and `BackTab` move the selection between the panels;
    /// - `m` changes the glyph that a chart draws a point with.
    ///
    /// The selection skips each panel that a collapsed row panel hides.
    /// The snapshot states which row panels are collapsed, so this method
    /// reads the snapshot that the program draws.
    ///
    /// thunderbox uses no key while the dashboard does not have the attention of
    /// the person. The program therefore keeps every key for itself at that
    /// time.
    ///
    /// The keys of the time range are not in the list above. The program binds
    /// them, and it passes them to [`crate::time_key`]. This method reports
    /// that it used none of them.
    ///
    /// The keys of a variable are not in the list above either. The program
    /// binds them, and it passes them to [`crate::variable_key`]. This method
    /// reports that it used none of them.
    ///
    /// The keys `Enter` and `Space` are not in the list above either. The
    /// program binds them, and it passes them to [`crate::row_key`]. This
    /// method reports that it used neither of them.
    pub fn handle_key(&mut self, key: Key, dashboard: &Dashboard, snapshot: &Snapshot) -> Consumed {
        if !self.focused {
            return Consumed::No;
        }
        let count = dashboard.panels.len().max(1);
        let hidden = hidden(dashboard, &snapshot.collapsed);
        // The selection skips each hidden panel. A row panel is never hidden,
        // so the loop ends before it reads any panel a second time.
        let mut step = |forward: bool| {
            for _ in 0..count {
                self.selected = if forward {
                    (self.selected + 1) % count
                } else {
                    (self.selected + count - 1) % count
                };
                if !hidden.get(self.selected).copied().unwrap_or(false) {
                    break;
                }
            }
        };
        match key {
            Key::Tab => {
                step(true);
                Consumed::Yes
            }
            Key::BackTab => {
                step(false);
                Consumed::Yes
            }
            Key::Char('m') => {
                self.marker = match self.marker {
                    Marker::Braille => Marker::Octant,
                    Marker::Octant => Marker::HalfBlock,
                    Marker::HalfBlock => Marker::Block,
                    Marker::Block => Marker::Braille,
                };
                Consumed::Yes
            }
            _ => Consumed::No,
        }
    }

    /// Draws every panel of the dashboard.
    ///
    /// The method reads the snapshot and nothing else. It never waits, and it
    /// never runs a query.
    ///
    /// The method draws no panel that a collapsed row panel hides. The field
    /// [`Snapshot::collapsed`] states which row panels are collapsed, and
    /// [`crate::DashboardRuntime`] asks no data source about such a panel.
    pub fn render(&self, dashboard: &Dashboard, snapshot: &Snapshot, area: Rect, buf: &mut Buffer) {
        let placed = places(dashboard, &snapshot.collapsed, area);
        let owners = owners(dashboard);
        let widths = self.label_widths(dashboard, snapshot, &placed);
        for (index, panel) in dashboard.panels.iter().enumerate() {
            let Some(place) = placed[index] else {
                continue;
            };
            if panel.kind == PanelKind::Row {
                if !draws_a_row(place) {
                    continue;
                }
                let held = owners.iter().filter(|owner| **owner == Some(index)).count();
                RowPanel::new(panel.title.as_str(), &self.theme)
                    .collapsed(snapshot.collapsed.contains(index))
                    .hidden(held)
                    .focused(self.focused && index == self.selected)
                    .render(place, buf);
                continue;
            }
            if !draws_a_panel(place.as_size()) {
                continue;
            }
            let width = widths.get(&column_of(panel.grid)).copied();
            self.render_panel(panel, index, snapshot, place, width, buf);
        }
    }

    /// Returns the number of panels that the renderer drops in this area.
    ///
    /// The renderer draws no panel under 3 cells wide or under 3 cells high.
    /// This crate calls such a panel a panel that the renderer drops. The
    /// method reads the rule that [`DashboardView::render`] reads, so the count
    /// describes the screen that the same area draws.
    ///
    /// A program reports this count to a person, in a row of its own. A
    /// terminal that is too small holds panels that nobody can see, and
    /// [`crate::DashboardRuntime`] asks a data source for none of them. A
    /// person who reads an empty screen learns nothing without that report.
    ///
    /// The count holds no panel that a collapsed row panel hides, because a
    /// person chose to hide it. It holds a row panel that takes no line of the
    /// terminal.
    pub fn dropped_panels(&self, dashboard: &Dashboard, snapshot: &Snapshot, area: Rect) -> usize {
        places(dashboard, &snapshot.collapsed, area)
            .into_iter()
            .zip(&dashboard.panels)
            .filter(|(place, panel)| match place {
                None => false,
                Some(place) if panel.kind == PanelKind::Row => !draws_a_row(*place),
                Some(place) => !draws_a_panel(place.as_size()),
            })
            .count()
    }

    /// Returns the width that the labels of each stack need.
    ///
    /// A stack is the set of panels that hold the same columns of the grid, one
    /// above another. The key of the map is the pair of grid columns that a
    /// stack holds.
    ///
    /// The method returns an empty map for a dashboard that states
    /// [`Align::None`]. A dashboard that states no rule uses that rule. The
    /// method measures every chart panel of a dashboard that states
    /// [`Align::Column`], and it keeps the widest measurement of each stack.
    ///
    /// The method measures the panels that the renderer draws. A panel that is
    /// too small for a chart therefore widens no other panel.
    fn label_widths(
        &self,
        dashboard: &Dashboard,
        snapshot: &Snapshot,
        placed: &[Option<Rect>],
    ) -> BTreeMap<(u16, u16), u16> {
        let mut widths = BTreeMap::new();
        if dashboard.align != Align::Column {
            return widths;
        }
        for (index, panel) in dashboard.panels.iter().enumerate() {
            if panel.kind != PanelKind::TimeSeries {
                continue;
            }
            let Some(place) = placed[index] else {
                continue;
            };
            if !draws_a_panel(place.as_size()) {
                continue;
            }
            let need = self
                .time_series(panel, index, snapshot, false, None)
                .measure_label_width();
            let width = widths.entry(column_of(panel.grid)).or_insert(0);
            *width = (*width).max(need);
        }
        widths
    }

    /// Returns the chart panel that draws one panel of the dashboard.
    ///
    /// The renderer builds this value two times for one panel of a dashboard
    /// that gives one width to a stack. It measures the labels first, and it
    /// draws second. The value borrows the data, and it copies nothing.
    fn time_series<'p>(
        &'p self,
        panel: &'p Panel,
        index: usize,
        snapshot: &'p Snapshot,
        selected: bool,
        label_width: Option<u16>,
    ) -> TimeSeriesPanel<'p> {
        let mut chart = TimeSeriesPanel::new(
            panel.title.as_str(),
            self.data(index, snapshot),
            &self.theme,
        )
        .marker(self.marker)
        .options(PanelOptions::from_panel(panel))
        .focused(selected);
        // The panel draws the window that a person asked for, and not the
        // window that the data happens to cover. A snapshot that holds no
        // answer yet names no window, and that panel draws a message in place
        // of an axis.
        if let Some(window) = snapshot.window() {
            chart = chart.window(window);
        }
        if let Some(width) = label_width {
            chart = chart.label_width(width);
        }
        chart
    }

    /// Returns the data that one panel shows at this moment.
    fn data<'p>(&self, index: usize, snapshot: &'p Snapshot) -> PanelData<'p> {
        match snapshot.panel(index) {
            PanelState::Loading => PanelData::Loading,
            PanelState::Error(text) => PanelData::Error(text),
            PanelState::Ready(frames) => PanelData::Frames(frames),
        }
    }

    /// Draws one panel.
    fn render_panel(
        &self,
        panel: &Panel,
        index: usize,
        snapshot: &Snapshot,
        area: Rect,
        label_width: Option<u16>,
        buf: &mut Buffer,
    ) {
        let data = self.data(index, snapshot);
        let selected = self.focused && index == self.selected;
        let options = PanelOptions::from_panel(panel);
        let title = panel.title.as_str();
        match panel.kind {
            PanelKind::TimeSeries => self
                .time_series(panel, index, snapshot, selected, label_width)
                .render(area, buf),
            PanelKind::Stat => StatPanel::new(title, data, &self.theme)
                .marker(self.marker)
                .options(options)
                .focused(selected)
                .render(area, buf),
            PanelKind::Gauge => GaugePanel::new(title, data, &self.theme)
                .options(options)
                .focused(selected)
                .render(area, buf),
            PanelKind::BarGauge => BarGaugePanel::new(title, data, &self.theme)
                .options(options)
                .focused(selected)
                .render(area, buf),
            PanelKind::Table => TablePanel::new(title, data, &self.theme)
                .options(options)
                .left(&panel.left)
                .focused(selected)
                .render(area, buf),
            PanelKind::Top => {
                let mut top = TopPanel::new(title, data, &self.theme)
                    .options(options)
                    .left(&panel.left)
                    .focused(selected);
                if let Some(column) = panel.sort.as_deref() {
                    top = top.sort_by(column);
                }
                top.render(area, buf);
            }
            // `render` draws each row panel itself, because a row panel needs
            // the count of the panels that it holds and no data.
            PanelKind::Row => {}
        }
    }
}

/// A key that a program passes to thunderbox.
///
/// This type holds no dependency on a terminal backend. A program converts its
/// own key type into this one, so that this crate needs no backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// The tab key. It selects the next panel.
    Tab,
    /// The tab key with the shift key. It selects the panel before.
    BackTab,
    /// The left arrow key.
    ///
    /// [`DashboardView::handle_key`] never uses this key. The program keeps
    /// it, and it can pass the key to [`crate::time_key`].
    Left,
    /// The right arrow key.
    ///
    /// [`DashboardView::handle_key`] never uses this key. The program keeps
    /// it, and it can pass the key to [`crate::time_key`].
    Right,
    /// The key `Enter`.
    ///
    /// [`DashboardView::handle_key`] never uses this key. The program keeps
    /// it, and it can pass the key to [`crate::row_key`].
    Enter,
    /// A key that holds one character.
    Char(char),
    /// A key that thunderbox does not name.
    Other,
}

/// Returns the pair of grid columns that a panel holds.
///
/// Two panels that hold the same pair stand one above the other, and they cover
/// the same cells across the screen. [`DashboardView::render`] gives one width
/// to the charts of such a pair.
fn column_of(grid: GridPos) -> (u16, u16) {
    (grid.x, grid.w)
}

/// Returns `true` when the place of a row panel holds its line.
///
/// A row panel draws one line, so it needs one line and one cell. A panel that
/// shows data needs more, and [`draws_a_panel`] states how much.
fn draws_a_row(place: Rect) -> bool {
    place.width > 0 && place.height > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{CollapsedRows, panel_width};

    /// Returns the place of one panel, while every row panel is expanded.
    fn place_of(dashboard: &Dashboard, index: usize, area: Rect) -> Rect {
        places(dashboard, &CollapsedRows::default(), area)[index]
            .expect("No row panel hides a panel while every row panel is expanded.")
    }

    /// A dashboard that holds a panel of each width that the default dashboard
    /// holds. The three panels of the second row start at three grid columns.
    const WIDTHS: &str = r#"
title = "test"
[[panel]]
title = "wide"
kind = "timeseries"
grid = { x = 0, y = 0, w = 16, h = 7 }
[[panel]]
title = "narrow"
kind = "stat"
grid = { x = 16, y = 0, w = 8, h = 7 }
[[panel]]
title = "left"
kind = "gauge"
grid = { x = 0, y = 7, w = 8, h = 6 }
[[panel]]
title = "middle"
kind = "bargauge"
grid = { x = 8, y = 7, w = 8, h = 6 }
[[panel]]
title = "right"
kind = "timeseries"
grid = { x = 16, y = 7, w = 8, h = 6 }
[[panel]]
title = "whole"
kind = "table"
grid = { x = 0, y = 13, w = 24, h = 6 }
"#;

    /// The renderer draws a panel at the width that [`panel_width`] returns.
    /// The runtime asks the data source for the points of that same width. The
    /// two numbers must therefore agree at every width of a terminal.
    #[test]
    fn the_place_of_a_panel_holds_the_width_that_the_point_limit_reads() {
        let dashboard = Dashboard::from_toml(WIDTHS).expect("The dashboard must be correct.");
        for width in 0..=300_u16 {
            let area = Rect {
                x: 4,
                y: 2,
                width,
                height: 40,
            };
            for (index, panel) in dashboard.panels.iter().enumerate() {
                assert_eq!(
                    place_of(&dashboard, index, area).width,
                    panel_width(panel.grid, width),
                    "The panel {:?} on an area of {width} cells.",
                    panel.title
                );
            }
        }
    }

    /// The panels of one row cover every cell of the area. The renderer left no
    /// gap and no overlap before this change, and it must leave none now.
    #[test]
    fn the_panels_of_one_row_still_touch_on_the_screen() {
        let dashboard = Dashboard::from_toml(WIDTHS).expect("The dashboard must be correct.");
        for width in 0..=300_u16 {
            let area = Rect {
                x: 0,
                y: 0,
                width,
                height: 40,
            };
            let row: Vec<Rect> = (2..5)
                .map(|index| place_of(&dashboard, index, area))
                .collect();
            assert_eq!(row[0].x, 0, "The first panel must start at the left edge.");
            assert_eq!(
                row[0].x + row[0].width,
                row[1].x,
                "The second panel must start where the first one ends. {width}"
            );
            assert_eq!(
                row[1].x + row[1].width,
                row[2].x,
                "The third panel must start where the second one ends. {width}"
            );
            assert_eq!(
                row[2].x + row[2].width,
                width,
                "The third panel must end at the right edge. {width}"
            );
        }
    }

    /// Returns `true` if the renderer wrote a cell inside this place.
    ///
    /// The function compares the buffer that the renderer wrote against a
    /// buffer that holds nothing. The renderer writes no cell of the place of a
    /// panel that it drops, so the two buffers agree there.
    fn holds_a_cell(drawn: &Buffer, empty: &Buffer, place: Rect) -> bool {
        for y in place.y..place.y.saturating_add(place.height) {
            for x in place.x..place.x.saturating_add(place.width) {
                if drawn[(x, y)] != empty[(x, y)] {
                    return true;
                }
            }
        }
        false
    }

    /// A program reads [`DashboardView::dropped_panels`] and writes the count
    /// into a row that a person reads. The count must therefore hold the panels
    /// that [`DashboardView::render`] leaves empty, at every size of a
    /// terminal.
    #[test]
    fn the_count_of_the_dropped_panels_holds_the_panels_that_the_renderer_leaves_empty() {
        let dashboard = Dashboard::from_toml(WIDTHS).expect("The dashboard must be correct.");
        let view = DashboardView::default();
        let snapshot = Snapshot::default();
        for width in [0_u16, 1, 3, 8, 11, 24, 47, 80, 130] {
            for height in 0..=42_u16 {
                let area = Rect {
                    x: 0,
                    y: 0,
                    width,
                    height,
                };
                let empty = Buffer::empty(area);
                let mut drawn = Buffer::empty(area);
                view.render(&dashboard, &snapshot, area, &mut drawn);
                let left_empty = (0..dashboard.panels.len())
                    .filter(|index| {
                        let place = place_of(&dashboard, *index, area);
                        !holds_a_cell(&drawn, &empty, place)
                    })
                    .count();
                assert_eq!(
                    view.dropped_panels(&dashboard, &snapshot, area),
                    left_empty,
                    "A terminal of {width} cells by {height} cells."
                );
            }
        }
    }

    /// The renderer drops a panel that holds under 3 cells in each direction.
    /// [`DashboardView::dropped_panels`] must count exactly those panels, and
    /// the count must hold for an area that does not start at the first cell.
    #[test]
    fn the_count_of_the_dropped_panels_holds_the_panels_that_are_under_three_cells() {
        let dashboard = Dashboard::from_toml(WIDTHS).expect("The dashboard must be correct.");
        let view = DashboardView::default();
        for width in 0..=130_u16 {
            for height in 0..=42_u16 {
                let area = Rect {
                    x: 7,
                    y: 5,
                    width,
                    height,
                };
                let small = (0..dashboard.panels.len())
                    .filter(|index| {
                        let place = place_of(&dashboard, *index, area);
                        place.width < 3 || place.height < 3
                    })
                    .count();
                assert_eq!(
                    view.dropped_panels(&dashboard, &Snapshot::default(), area),
                    small,
                    "A terminal of {width} cells by {height} cells."
                );
            }
        }
    }
}
