//! The parts that every panel shares.
//!
//! Every panel draws the same border, and every panel reports the same three
//! states: the data source has not answered, the data source returned an
//! error, and the result holds no value. This module holds those parts once,
//! so that six panels agree.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Alignment, Rect, Size};
use ratatui_core::style::{Color, Style};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;
use ratatui_widgets::block::Block;
use ratatui_widgets::borders::BorderType;
use thunderbox_core::{FieldConfig, Frame, Panel, Reduce, Threshold};

use crate::{PanelData, Theme, format_value};

/// The text that a panel shows before the data source answers.
///
/// CLAUDE.md section 1.5 keeps this string. A panel can be eight cells wide,
/// and a whole sentence does not fit.
pub const LOADING: &str = "loading…";

/// The text that a panel shows for a result that holds no value.
///
/// CLAUDE.md section 1.5 keeps this string. The same width problem applies.
pub const NO_DATA: &str = "no data";

/// The smallest panel that this crate draws, in terminal cells.
///
/// Every panel of this crate draws a border on each of its four sides. A panel
/// of this size in both directions therefore holds one cell for the data.
const MIN_CELLS: u16 = 3;

/// Returns `true` when a panel of this size holds a cell for its data.
///
/// Every panel of this crate draws nothing at all at a smaller size, in either
/// direction. A caller reads this function before it gives such a size to a
/// panel. `thunderbox_dashboard` reads it for the place of each panel, and also
/// for the point limit of each panel. It then asks a data source for nothing
/// for a panel that this function rejects.
///
/// This function is the one place that holds the smallest size. A caller and a
/// panel therefore never disagree.
///
/// A panel asks this function about the intersection of its area and the
/// buffer. A caller whose area reaches past the buffer must ask about the same
/// intersection.
///
/// # Examples
///
/// ```
/// use thunderbox_widgets::draws_a_panel;
/// use thunderbox_widgets::ratatui_core::layout::Size;
/// assert!(draws_a_panel(Size::new(3, 3)));
/// assert!(!draws_a_panel(Size::new(2, 3)));
/// ```
pub fn draws_a_panel(size: Size) -> bool {
    size.width >= MIN_CELLS && size.height >= MIN_CELLS
}

/// The settings that a panel uses to show a number.
///
/// These settings repeat the fields of [`FieldConfig`]. A data source fills the
/// column configuration, and a person writes the panel. The panel therefore
/// wins where the two disagree.
///
/// Every field is public, so a caller builds the value in the usual way:
///
/// ```
/// use thunderbox_widgets::PanelOptions;
/// let options = PanelOptions {
///     unit: Some("percent"),
///     max: Some(100.0),
///     ..Default::default()
/// };
/// assert_eq!(options.unit, Some("percent"));
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PanelOptions<'a> {
    /// The name of the unit. The values `percent`, `bytes`, and `bytes/s` are
    /// examples.
    pub unit: Option<&'a str>,
    /// The number of decimal places that the panel shows.
    pub decimals: Option<u8>,
    /// The smallest value that a meter shows.
    pub min: Option<f64>,
    /// The largest value that a meter shows.
    pub max: Option<f64>,
    /// The step that turns a column of numbers into one number.
    pub reduce: Reduce,
    /// The thresholds that change the colour of a value.
    pub thresholds: &'a [Threshold],
}

impl<'a> PanelOptions<'a> {
    /// Returns the settings that a dashboard panel states.
    ///
    /// # Examples
    ///
    /// ```
    /// use thunderbox_core::Dashboard;
    /// use thunderbox_widgets::PanelOptions;
    /// let text = r#"
    /// title = "board"
    /// [[panel]]
    /// title = "load"
    /// kind = "gauge"
    /// unit = "percent"
    /// max = 100.0
    /// grid = { x = 0, y = 0, w = 6, h = 4 }
    /// "#;
    /// let dashboard = Dashboard::from_toml(text).unwrap();
    /// let options = PanelOptions::from_panel(&dashboard.panels[0]);
    /// assert_eq!(options.unit, Some("percent"));
    /// assert_eq!(options.max, Some(100.0));
    /// ```
    pub fn from_panel(panel: &'a Panel) -> Self {
        Self {
            unit: panel.unit.as_deref(),
            decimals: panel.decimals,
            min: panel.min,
            max: panel.max,
            reduce: panel.reduce,
            thresholds: &panel.thresholds,
        }
    }

    /// Returns the unit of the panel, or the unit of the column.
    pub fn unit_for<'b>(&'b self, config: &'b FieldConfig) -> Option<&'b str> {
        self.unit.or(config.unit.as_deref())
    }

    /// Returns the number of decimal places of the panel, or of the column.
    pub fn decimals_for(&self, config: &FieldConfig) -> Option<u8> {
        self.decimals.or(config.decimals)
    }

    /// Returns the text of one value.
    pub fn text(&self, value: f64, config: &FieldConfig) -> String {
        format_value(value, self.unit_for(config), self.decimals_for(config))
    }

    /// Returns the smallest and the largest value that a meter shows.
    ///
    /// The method reads the panel first, then the column configuration, and
    /// then `data`, which holds the smallest and the largest value in the
    /// result. The method adds zero to the range that `data` gives, so that a
    /// bar starts from a fixed point. It widens a range that has no width,
    /// because a meter would otherwise divide by zero.
    pub fn bounds_for(&self, config: &FieldConfig, data: Option<(f64, f64)>) -> (f64, f64) {
        let (low, high) = data.unwrap_or((0.0, 1.0));
        let mut min = self.min.or(config.min).unwrap_or(low.min(0.0));
        let mut max = self.max.or(config.max).unwrap_or(high.max(0.0));
        if !min.is_finite() {
            min = 0.0;
        }
        if !max.is_finite() || max <= min {
            max = min + 1.0;
        }
        (min, max)
    }

    /// Returns the thresholds of the panel, or the thresholds of the column.
    pub fn thresholds_for<'b>(&'b self, config: &'b FieldConfig) -> &'b [Threshold] {
        if self.thresholds.is_empty() {
            &config.thresholds
        } else {
            self.thresholds
        }
    }

    /// Returns the colour of one value.
    ///
    /// The method returns the colour of the threshold that the value reaches.
    /// It returns `fallback` when the value reaches no threshold, and when the
    /// theme does not define the colour of the threshold.
    pub fn color_for(
        &self,
        value: f64,
        config: &FieldConfig,
        theme: &Theme,
        fallback: Color,
    ) -> Color {
        Threshold::at(self.thresholds_for(config), value)
            .and_then(|t| theme.threshold_color(&t.color))
            .unwrap_or(fallback)
    }
}

/// One series, and the one number that a panel shows for it.
#[derive(Debug, Clone)]
pub(crate) struct Reduced<'a> {
    /// The name that the panel shows for the series.
    pub name: String,
    /// The one number.
    pub value: f64,
    /// The hints that the data source attached to the column.
    pub config: &'a FieldConfig,
}

/// Returns one number for each series in the frames.
///
/// The function reads every column of numbers, in the order of the frames. It
/// drops a column that holds no value that the step can use. A panel that shows
/// one number for each series calls this function.
pub(crate) fn reduce_frames(frames: &[Frame], reduce: Reduce) -> Vec<Reduced<'_>> {
    let mut out = Vec::new();
    for frame in frames {
        for field in frame.numeric_fields() {
            let values = field.values.as_f64().unwrap_or(&[]);
            let Some(value) = reduce.apply(values) else {
                continue;
            };
            out.push(Reduced {
                name: field.display_name(),
                value,
                config: &field.config,
            });
        }
    }
    out
}

/// Returns the smallest and the largest number across every series.
pub(crate) fn spread(series: &[Reduced<'_>]) -> Option<(f64, f64)> {
    series
        .iter()
        .map(|s| s.value)
        .filter(|v| v.is_finite())
        .fold(None, |acc, v| match acc {
            None => Some((v, v)),
            Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
        })
}

/// Returns the border of a panel.
pub(crate) fn panel_block<'a>(title: &'a str, theme: &Theme, focused: bool) -> Block<'a> {
    let border = if focused {
        theme.border_focused
    } else {
        theme.border
    };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(Line::from(Span::styled(title, theme.title)))
}

/// Returns the frames of the panel, or the text that stands in their place.
///
/// A panel draws the text when the data source has not answered, and when the
/// data source returned an error. The style comes with the text.
pub(crate) fn frames_or_message<'a>(
    data: PanelData<'a>,
    theme: &Theme,
) -> Result<&'a [Frame], (&'a str, Style)> {
    match data {
        PanelData::Loading => Err((LOADING, theme.muted)),
        PanelData::Error(text) => Err((text, theme.error)),
        PanelData::Frames(frames) => Ok(frames),
    }
}

/// Draws the border, and one line of text in the middle of the panel.
///
/// A panel uses this function when it has nothing else to draw.
pub(crate) fn render_message(
    block: Block<'_>,
    message: &str,
    style: Style,
    area: Rect,
    buf: &mut Buffer,
) {
    let inner = block.inner(area);
    block.render(area, buf);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let y = inner.y + inner.height / 2;
    Line::from(Span::styled(message, style))
        .alignment(Alignment::Center)
        .render(
            Rect {
                y,
                height: 1,
                ..inner
            },
            buf,
        );
}

/// Returns the text that states the number of series that a panel dropped.
///
/// A panel that cannot draw every series states the number that it left out. A
/// reader then knows that the panel holds more. The function returns a whole
/// sentence when the width holds one. It returns a short form in a narrow
/// panel, because no sentence fits in eight cells. CLAUDE.md section 1.5
/// records the same reason for the other short labels of this crate.
///
/// Three panels show one number for each series, and all three call this
/// function. A reader therefore meets the same words everywhere.
pub(crate) fn dropped_text(dropped: usize, width: usize) -> String {
    let long = if dropped == 1 {
        "1 series does not fit.".to_string()
    } else {
        format!("{dropped} series do not fit.")
    };
    if long.chars().count() <= width {
        return long;
    }
    clip(&format!("+{dropped}"), width)
}

/// Returns the number of cells that a text takes on a terminal.
///
/// One character of a Chinese script or a Japanese script takes two cells. A
/// mark that a writer adds to a letter takes no cell of its own. A count of the
/// characters is therefore not a count of the cells.
pub(crate) fn cells_of(text: &str) -> usize {
    Span::raw(text).width()
}

/// Returns the text, cut to `width` cells.
///
/// The function counts cells, and it does not count characters. A name of a
/// Chinese script that passes a count of characters still holds too many cells
/// for the row. A [`Line`] that does not fit its area drops the end of the
/// text, and it marks nothing. Two names that differ would then draw one
/// picture.
///
/// The function puts one full stop at the end of a text that it cut. A reader
/// therefore sees that the name goes on.
pub(crate) fn clip(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if cells_of(text) <= width {
        return text.to_string();
    }
    if width == 1 {
        return ".".to_string();
    }
    // The full stop takes the last cell. The name takes the cells that remain.
    let room = width - 1;
    let mut kept = String::new();
    for c in text.chars() {
        kept.push(c);
        if cells_of(&kept) > room {
            kept.pop();
            break;
        }
    }
    kept.push('.');
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use thunderbox_core::{Field, Frame};

    fn config() -> FieldConfig {
        FieldConfig {
            unit: Some("bytes".into()),
            decimals: Some(2),
            min: Some(1.0),
            max: Some(9.0),
            thresholds: vec![Threshold {
                value: 5.0,
                color: "critical".into(),
            }],
            ..Default::default()
        }
    }

    #[test]
    fn the_panel_wins_over_the_column_configuration() {
        let options = PanelOptions {
            unit: Some("percent"),
            decimals: Some(0),
            min: Some(0.0),
            max: Some(100.0),
            ..Default::default()
        };
        let config = config();
        assert_eq!(options.unit_for(&config), Some("percent"));
        assert_eq!(options.decimals_for(&config), Some(0));
        assert_eq!(options.bounds_for(&config, None), (0.0, 100.0));
        assert_eq!(options.text(42.4, &config), "42%");
    }

    #[test]
    fn the_column_configuration_serves_a_panel_that_states_nothing() {
        let options = PanelOptions::default();
        let config = config();
        assert_eq!(options.unit_for(&config), Some("bytes"));
        assert_eq!(options.decimals_for(&config), Some(2));
        assert_eq!(options.bounds_for(&config, None), (1.0, 9.0));
    }

    #[test]
    fn bounds_for_reads_the_data_when_nothing_else_states_a_range() {
        let options = PanelOptions::default();
        let empty = FieldConfig::default();
        assert_eq!(options.bounds_for(&empty, Some((10.0, 40.0))), (0.0, 40.0));
        assert_eq!(
            options.bounds_for(&empty, Some((-30.0, -10.0))),
            (-30.0, 0.0)
        );
    }

    #[test]
    fn bounds_for_widens_a_range_that_has_no_width() {
        let options = PanelOptions::default();
        let empty = FieldConfig::default();
        let (min, max) = options.bounds_for(&empty, Some((7.0, 7.0)));
        assert!(max > min, "A meter would divide by zero. {min} {max}");
        let (min, max) = options.bounds_for(&empty, None);
        assert!(max > min, "{min} {max}");
    }

    #[test]
    fn bounds_for_repairs_a_value_that_is_not_finite() {
        let options = PanelOptions {
            min: Some(f64::NAN),
            max: Some(f64::INFINITY),
            ..Default::default()
        };
        let (min, max) = options.bounds_for(&FieldConfig::default(), None);
        assert!(min.is_finite() && max.is_finite() && max > min);
    }

    #[test]
    fn color_for_reads_the_thresholds_of_the_panel_first() {
        let theme = Theme::default();
        let list = [Threshold {
            value: 90.0,
            color: "warning".into(),
        }];
        let options = PanelOptions {
            thresholds: &list,
            ..Default::default()
        };
        let config = config();
        // The column would call 6.0 critical. The panel wins, and 6.0 reaches
        // no threshold of the panel.
        assert_eq!(
            options.color_for(6.0, &config, &theme, Color::Cyan),
            Color::Cyan
        );
        assert_eq!(
            options.color_for(95.0, &config, &theme, Color::Cyan),
            theme.warning
        );
    }

    #[test]
    fn color_for_reads_the_thresholds_of_the_column_when_the_panel_has_none() {
        let theme = Theme::default();
        let options = PanelOptions::default();
        assert_eq!(
            options.color_for(6.0, &config(), &theme, Color::Cyan),
            theme.critical
        );
    }

    #[test]
    fn color_for_returns_the_fallback_for_a_colour_that_the_theme_does_not_define() {
        let theme = Theme::default();
        let list = [Threshold {
            value: 0.0,
            color: "mauve".into(),
        }];
        let options = PanelOptions {
            thresholds: &list,
            ..Default::default()
        };
        let empty = FieldConfig::default();
        assert_eq!(
            options.color_for(1.0, &empty, &theme, Color::Green),
            Color::Green
        );
    }

    fn frames() -> Vec<Frame> {
        vec![Frame::named(
            "f",
            vec![
                Field::time("time", vec![0, 1000, 2000]),
                Field::f64("a", vec![Some(1.0), Some(5.0), Some(3.0)]),
                Field::f64("b", vec![None, None, None]),
                Field::f64("c", vec![Some(8.0)]),
            ],
        )]
    }

    #[test]
    fn reduce_frames_returns_one_number_for_each_series() {
        let f = frames();
        let out = reduce_frames(&f, Reduce::Last);
        assert_eq!(out.len(), 2, "The column of holes must not appear.");
        assert_eq!(out[0].name, "a");
        assert_eq!(out[0].value, 3.0);
        assert_eq!(out[1].value, 8.0);
    }

    #[test]
    fn reduce_frames_reads_the_step_that_the_caller_names() {
        let f = frames();
        let out = reduce_frames(&f, Reduce::Max);
        assert_eq!(out[0].value, 5.0);
    }

    #[test]
    fn reduce_frames_returns_nothing_for_frames_that_hold_no_number() {
        assert!(reduce_frames(&[], Reduce::Last).is_empty());
        let f = vec![Frame::new(vec![Field::time("time", vec![0])])];
        assert!(reduce_frames(&f, Reduce::Last).is_empty());
    }

    #[test]
    fn spread_returns_the_smallest_and_the_largest_number() {
        let f = frames();
        let out = reduce_frames(&f, Reduce::Last);
        assert_eq!(spread(&out), Some((3.0, 8.0)));
        assert_eq!(spread(&[]), None);
    }

    #[test]
    fn dropped_text_states_the_number_as_a_whole_sentence() {
        assert_eq!(dropped_text(1, 30), "1 series does not fit.");
        assert_eq!(dropped_text(4, 30), "4 series do not fit.");
        assert_eq!(dropped_text(4, 20), "4 series do not fit.");
    }

    #[test]
    fn dropped_text_returns_a_short_form_in_a_narrow_panel() {
        assert_eq!(dropped_text(4, 19), "+4");
        assert_eq!(dropped_text(12, 3), "+12");
        assert_eq!(dropped_text(12, 2), "+.");
        assert_eq!(dropped_text(4, 0), "");
    }

    #[test]
    fn cells_of_counts_the_cells_and_not_the_characters() {
        assert_eq!(cells_of("cpu"), 3);
        // Each of these characters takes two cells on a terminal.
        assert_eq!(cells_of("中央処理"), 8);
        assert_eq!(cells_of(""), 0);
    }

    #[test]
    fn clip_cuts_a_name_by_the_cells_that_it_takes() {
        let name = "中央処理装置";
        assert_eq!(cells_of(name), 12);
        assert_eq!(clip(name, 8), "中央処.");
        assert_eq!(clip("中央", 4), "中央");
        assert_eq!(clip("中央", 3), "中.");
        assert_eq!(clip("中", 1), ".");
        assert_eq!(clip("中", 0), "");
    }

    #[test]
    fn clip_never_returns_a_text_that_is_wider_than_the_width() {
        for text in ["cpu", "throughput", "中央処理装置", "a中b央c", ""] {
            for width in 0..14 {
                let out = clip(text, width);
                assert!(
                    cells_of(&out) <= width,
                    "The text {out:?} takes more than {width} cells."
                );
            }
        }
    }

    #[test]
    fn clip_keeps_a_text_that_fits() {
        assert_eq!(clip("cpu", 3), "cpu");
        assert_eq!(clip("cpu", 8), "cpu");
    }

    #[test]
    fn clip_marks_a_text_that_it_cut() {
        assert_eq!(clip("throughput", 5), "thro.");
        assert_eq!(clip("throughput", 1), ".");
        assert_eq!(clip("throughput", 0), "");
    }

    /// A panel of two cells holds a border and nothing else. Every panel of
    /// this crate draws nothing at such a size, and a caller reads this
    /// function before it gives that size to a panel.
    #[test]
    fn a_panel_under_three_cells_holds_no_cell_for_its_data() {
        assert!(
            draws_a_panel(Size::new(3, 3)),
            "A panel of 3 by 3 cells holds one cell for its data."
        );
        assert!(
            !draws_a_panel(Size::new(2, 20)),
            "A panel of 2 cells wide holds no cell for its data."
        );
        assert!(
            !draws_a_panel(Size::new(20, 2)),
            "A panel of 2 cells high holds no cell for its data."
        );
        assert!(
            !draws_a_panel(Size::new(0, 0)),
            "A panel of no cell holds no cell for its data."
        );
    }
}
