//! The colours that a panel uses.

use ratatui_core::style::{Color, Style};

/// The colours that a panel uses.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// The border of a panel that a person has not selected.
    pub border: Style,
    /// The border of the panel that a person has selected.
    pub border_focused: Style,
    /// The title in the border of a panel.
    pub title: Style,
    /// The lines and the labels of an axis.
    pub axis: Style,
    /// Ordinary text inside a panel.
    pub text: Style,
    /// Text that is less important than ordinary text.
    pub muted: Style,
    /// Text that reports an error.
    pub error: Style,
    /// One colour for each series, in order.
    pub series: Vec<Color>,
    /// The colour of a value that reaches the warning threshold.
    pub warning: Color,
    /// The colour of a value that reaches the critical threshold.
    pub critical: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            border: Style::default().fg(Color::DarkGray),
            border_focused: Style::default().fg(Color::Cyan),
            title: Style::default().fg(Color::Gray),
            axis: Style::default().fg(Color::DarkGray),
            text: Style::default().fg(Color::Gray),
            muted: Style::default().fg(Color::DarkGray),
            error: Style::default().fg(Color::Red),
            series: vec![
                Color::Cyan,
                Color::Magenta,
                Color::Green,
                Color::Yellow,
                Color::Blue,
                Color::LightRed,
            ],
            warning: Color::Yellow,
            critical: Color::Red,
        }
    }
}

impl Theme {
    /// Returns the colour of one series.
    ///
    /// The method starts again at the first colour when a chart holds more
    /// series than the theme holds colours. It returns cyan when the theme
    /// holds no colour at all.
    pub fn series_color(&self, n: usize) -> Color {
        if self.series.is_empty() {
            return Color::Cyan;
        }
        self.series[n % self.series.len()]
    }

    /// Returns the colour that has this name.
    ///
    /// The method returns `None` when the theme does not define the name. The
    /// caller then uses the colour of the series.
    pub fn threshold_color(&self, name: &str) -> Option<Color> {
        match name {
            "warning" => Some(self.warning),
            "critical" => Some(self.critical),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_color_starts_again_at_the_first_colour() {
        let t = Theme::default();
        let n = t.series.len();
        assert_eq!(t.series_color(0), t.series_color(n));
        assert_eq!(t.series_color(1), t.series_color(n + 1));
    }

    #[test]
    fn series_color_returns_cyan_for_an_empty_list() {
        let t = Theme {
            series: vec![],
            ..Default::default()
        };
        assert_eq!(t.series_color(3), Color::Cyan);
    }

    #[test]
    fn threshold_color_returns_a_colour_for_a_known_name() {
        let t = Theme::default();
        assert_eq!(t.threshold_color("critical"), Some(Color::Red));
        assert_eq!(t.threshold_color("mauve"), None);
    }
}
