//! The line that a row panel draws.
//!
//! A row panel holds the panels below it on the grid, and a person collapses it
//! to hide them. The panel draws one line across the screen: a mark, the
//! title, and a rule to the right edge. A collapsed row panel also counts the
//! panels that it hides, at the right end of the line.
//!
//! The panel shows no data, so it takes no frames and no options. It draws in
//! an area of one line, and a person reads it as the title of the panels below
//! it.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;

use crate::Theme;
use crate::panel::{cells_of, clip};

/// The mark of a row panel that shows the panels that it holds.
const EXPANDED: &str = "▾";

/// The mark of a row panel that hides the panels that it holds.
const COLLAPSED: &str = "▸";

/// The character of the rule that fills the line after the title.
const RULE: &str = "─";

/// A line that heads the panels of a row panel.
///
/// The panel draws in the first line of its area, and it draws nothing in an
/// area of no width or of no height.
///
/// # Examples
///
/// ```
/// use ratatui_core::buffer::Buffer;
/// use ratatui_core::layout::Rect;
/// use ratatui_core::widgets::Widget;
/// use thunderbox_widgets::{RowPanel, Theme};
///
/// let theme = Theme::default();
/// let area = Rect::new(0, 0, 40, 1);
/// let mut buf = Buffer::empty(area);
/// RowPanel::new("network", &theme)
///     .collapsed(true)
///     .hidden(3)
///     .render(area, &mut buf);
/// let line: String = (0..40).map(|x| buf[(x, 0)].symbol().to_string()).collect();
/// assert!(line.starts_with("▸ network ─"));
/// assert!(line.ends_with("The row hides 3 panels."));
/// ```
pub struct RowPanel<'a> {
    /// The title that the line shows.
    title: &'a str,
    /// The colours that the panel uses.
    theme: &'a Theme,
    /// This field is `true` while the row panel hides the panels that it holds.
    collapsed: bool,
    /// The number of panels that the row panel holds.
    hidden: usize,
    /// The mark and the rule change colour when a person selects the panel.
    focused: bool,
}

impl<'a> RowPanel<'a> {
    /// Returns a row panel that shows the panels that it holds.
    pub fn new(title: &'a str, theme: &'a Theme) -> Self {
        Self {
            title,
            theme,
            collapsed: false,
            hidden: 0,
            focused: false,
        }
    }

    /// States whether the row panel hides the panels that it holds.
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Sets the number of panels that the row panel holds.
    ///
    /// The line counts those panels while the row panel is collapsed. A reader
    /// then knows how much the row panel hides.
    pub fn hidden(mut self, count: usize) -> Self {
        self.hidden = count;
        self
    }

    /// States whether a person has selected this panel. The mark and the rule
    /// change colour when a person selects a panel.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

/// Returns the text that counts the panels that a collapsed row panel hides.
///
/// The function follows the rule for a short label of `CLAUDE.md`. It returns
/// the whole sentence when the width holds it. It returns the short form `+3`
/// when the sentence does not fit, and it cuts that form when the width holds
/// no more. A row panel shares one line with its title, so the sentence often
/// does not fit.
fn hidden_text(hidden: usize, width: usize) -> String {
    let long = if hidden == 1 {
        "The row hides 1 panel.".to_string()
    } else {
        format!("The row hides {hidden} panels.")
    };
    if cells_of(&long) <= width {
        return long;
    }
    clip(&format!("+{hidden}"), width)
}

impl Widget for RowPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self).render(area, buf);
    }
}

impl Widget for &RowPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let style = if self.focused {
            self.theme.border_focused
        } else {
            self.theme.border
        };
        let width = usize::from(area.width);
        let mark = if self.collapsed { COLLAPSED } else { EXPANDED };
        let mut spans = vec![Span::styled(mark, style)];
        let mut used = cells_of(mark);

        // The title takes the cells after one space.
        let title = clip(self.title, width.saturating_sub(used + 1));
        if !title.is_empty() {
            spans.push(Span::raw(" "));
            used += 1 + cells_of(&title);
            spans.push(Span::styled(title, self.theme.title));
        }

        // The note stands at the right end of the line, after one space. The
        // rule needs one space before it, so the note keeps that cell free.
        let note = if self.collapsed && self.hidden > 0 {
            hidden_text(self.hidden, width.saturating_sub(used + 2))
        } else {
            String::new()
        };
        let note_cells = if note.is_empty() {
            0
        } else {
            cells_of(&note) + 1
        };
        let rest = width.saturating_sub(used);
        if rest > note_cells {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(RULE.repeat(rest - note_cells - 1), style));
        }
        if !note.is_empty() {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(note, self.theme.muted));
        }
        Line::from(spans).render(Rect { height: 1, ..area }, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Draws the panel into a plain buffer, and returns the first line.
    fn line(panel: &RowPanel<'_>, width: u16) -> String {
        let area = Rect::new(0, 0, width, 1);
        let mut buf = Buffer::empty(area);
        panel.render(area, &mut buf);
        (0..width)
            .map(|x| buf[(x, 0)].symbol().to_string())
            .collect()
    }

    #[test]
    fn each_mark_takes_one_cell() {
        assert_eq!(cells_of(EXPANDED), 1);
        assert_eq!(cells_of(COLLAPSED), 1);
        assert_eq!(cells_of(RULE), 1);
    }

    #[test]
    fn an_expanded_row_panel_draws_its_mark_its_title_and_a_rule() {
        let theme = Theme::default();
        let panel = RowPanel::new("network", &theme).hidden(3);
        let out = line(&panel, 20);
        assert_eq!(out, "▾ network ──────────");
        assert!(
            !out.contains("hides"),
            "An expanded row panel hides nothing. {out}"
        );
    }

    #[test]
    fn a_collapsed_row_panel_counts_the_panels_that_it_hides() {
        let theme = Theme::default();
        let panel = RowPanel::new("network", &theme).collapsed(true).hidden(3);
        assert_eq!(line(&panel, 40), "▸ network ────── The row hides 3 panels.");
        assert_eq!(
            line(&panel, 20),
            "▸ network ─────── +3",
            "The short form takes the place of a sentence that does not fit."
        );
    }

    #[test]
    fn a_collapsed_row_panel_that_holds_one_panel_writes_the_singular() {
        let theme = Theme::default();
        let panel = RowPanel::new("disk", &theme).collapsed(true).hidden(1);
        assert!(line(&panel, 40).ends_with("The row hides 1 panel."));
    }

    #[test]
    fn a_collapsed_row_panel_that_holds_no_panel_writes_no_note() {
        let theme = Theme::default();
        let panel = RowPanel::new("empty", &theme).collapsed(true);
        assert_eq!(line(&panel, 12), "▸ empty ────");
    }

    #[test]
    fn hidden_text_takes_the_short_form_when_the_sentence_does_not_fit() {
        assert_eq!(hidden_text(3, 23), "The row hides 3 panels.");
        assert_eq!(hidden_text(3, 22), "+3");
        assert_eq!(hidden_text(12, 2), "+.");
        assert_eq!(hidden_text(3, 0), "");
    }

    /// The selection colours the mark and the rule. A person then sees which
    /// row panel the key `Enter` collapses.
    #[test]
    fn a_selected_row_panel_draws_its_mark_in_the_colour_of_the_selection() {
        let theme = Theme::default();
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);
        RowPanel::new("network", &theme)
            .focused(true)
            .render(area, &mut buf);
        assert_eq!(buf[(0, 0)].fg, theme.border_focused.fg.unwrap_or_default());
        let mut plain = Buffer::empty(area);
        RowPanel::new("network", &theme).render(area, &mut plain);
        assert_ne!(
            buf[(0, 0)].fg,
            plain[(0, 0)].fg,
            "The selection must change the colour of the mark."
        );
    }

    /// The line fills the width exactly. A line that ran one cell too far
    /// would damage the panel beside it.
    #[test]
    fn the_line_fills_its_area_and_writes_nothing_outside_it() {
        let theme = Theme::default();
        let titles = ["", "a", "network", "a title that is longer than the line"];
        for title in titles {
            for collapsed in [false, true] {
                for width in 0..50_u16 {
                    let panel = RowPanel::new(title, &theme).collapsed(collapsed).hidden(12);
                    let mut sentinel = ratatui_core::buffer::Cell::default();
                    sentinel.set_char('#');
                    let outer = Rect::new(0, 0, width + 6, 4);
                    let mut buf = Buffer::filled(outer, sentinel);
                    let area = Rect::new(3, 1, width, 2);
                    panel.render(area, &mut buf);
                    for y in 0..4 {
                        for x in 0..width + 6 {
                            let inside = y == 1 && (3..3 + width).contains(&x);
                            let symbol = buf[(x, y)].symbol();
                            if inside {
                                assert_ne!(
                                    symbol, "#",
                                    "The line must fill its width. {title:?} {width} {x}"
                                );
                            } else {
                                assert_eq!(
                                    symbol, "#",
                                    "The line wrote outside its area. {title:?} {width} {x} {y}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
