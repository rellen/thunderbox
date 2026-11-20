//! The panel renderers of thunderbox.
//!
//! This crate is the smaller of the two ways to put thunderbox inside another
//! program. A program that already holds its own data can draw a thunderbox panel
//! with this crate alone. The program does not need a terminal backend, an
//! async runtime, or any part of thunderbox that runs a query. This crate depends
//! only on `ratatui-core` and `ratatui-widgets`.
//!
//! ```no_run
//! # use thunderbox_widgets::{PanelData, PanelOptions, Theme, TimeSeriesPanel};
//! # let (frames, theme, area, buf) = (vec![], Theme::default(),
//! #     ratatui_core::layout::Rect::ZERO,
//! #     &mut ratatui_core::buffer::Buffer::empty(ratatui_core::layout::Rect::ZERO));
//! use ratatui_core::widgets::Widget;
//! TimeSeriesPanel::new("CPU", PanelData::Frames(&frames), &theme).render(area, buf);
//! ```
//!
//! # The panels
//!
//! Each panel takes a title, the data, and a theme. Each panel also takes a
//! [`PanelOptions`] value, which states the unit, the number of decimal places,
//! the range of a meter, and the thresholds.
//!
//! - [`TimeSeriesPanel`] draws one or more values against time.
//! - [`StatPanel`] draws one large number for each series.
//! - [`GaugePanel`] draws one value against a range, as a meter.
//! - [`BarGaugePanel`] draws one bar for each series, against a range.
//! - [`TablePanel`] draws the columns of the frames, as rows and columns.
//! - [`TopPanel`] draws a list of rows in order by one column, with the largest
//!   value first. The program `top` shows the processes of a machine in this
//!   form.

mod bargauge;
mod gauge;
mod marker;
mod panel;
mod stat;
mod table;
mod theme;
mod timeseries;
mod top;
mod value;

pub use bargauge::BarGaugePanel;
pub use gauge::GaugePanel;
pub use marker::Marker;
pub use panel::{LOADING, NO_DATA, PanelOptions, draws_a_panel};
pub use stat::StatPanel;
pub use table::TablePanel;
pub use theme::Theme;
pub use timeseries::{TimeSeriesPanel, last_value};
pub use top::TopPanel;
pub use value::{MISSING, format_from_now, format_time, format_value, format_window};

use thunderbox_core::Frame;

/// The data that a panel shows at this moment.
///
/// An error from a data source belongs to one panel. It never stops the
/// dashboard. One query that returns an error therefore changes one panel, and
/// every other panel continues to draw.
#[derive(Debug, Clone, Copy)]
pub enum PanelData<'a> {
    /// The data source has not answered the query yet.
    Loading,
    /// The data source answered the query.
    Frames(&'a [Frame]),
    /// The data source returned an error. The text explains the error.
    Error(&'a str),
}

/// A second name for `ratatui_core`.
///
/// A program that holds a thunderbox panel can read the version of ratatui that
/// thunderbox uses from this name. The program therefore does not need to state
/// that version a second time.
pub use ratatui_core;
