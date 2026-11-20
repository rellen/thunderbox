//! The rule that maps a key to the row panel that it collapses or expands.
//!
//! A row panel holds the panels below it on the grid. A person selects a row
//! panel with the key `Tab`, and the key `Enter` or the key `Space` then
//! collapses it or expands it.
//!
//! The program binds both keys, and thunderbox consumes neither of them. This
//! rule follows the rule of [`crate::time_key`] and of [`crate::variable_key`].
//! The program passes a key to [`row_key`], and it passes the answer to
//! [`crate::DashboardRuntime::toggle_row`]. A program that wants another key
//! ignores this function.

use thunderbox_core::{Dashboard, PanelKind};

use crate::view::Key;

/// Returns the row panel that a key collapses or expands.
///
/// The function returns the position of the selected panel when the key is
/// `Enter` or `Space` and the selected panel is a row panel. It returns `None`
/// for every other key, and for a selection that holds a panel of another
/// kind. The program then keeps the key for itself.
///
/// The value `selected` is the position that [`crate::DashboardView::selected`]
/// returns.
///
/// The caller passes the result to [`crate::DashboardRuntime::toggle_row`]. A
/// program therefore calls this function from a key press, and never on every
/// frame.
pub fn row_key(key: Key, selected: usize, dashboard: &Dashboard) -> Option<usize> {
    if !matches!(key, Key::Enter | Key::Char(' ')) {
        return None;
    }
    let panel = dashboard.panels.get(selected)?;
    (panel.kind == PanelKind::Row).then_some(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dashboard that holds a chart, and then a row panel.
    fn dashboard() -> Dashboard {
        Dashboard::from_toml(
            r#"
title = "rows"
[[panel]]
title = "cpu"
kind = "timeseries"
grid = { x = 0, y = 0, w = 24, h = 6 }
[[panel]]
title = "details"
kind = "row"
grid = { x = 0, y = 6, w = 24, h = 1 }
"#,
        )
        .expect("The dashboard must be correct.")
    }

    #[test]
    fn enter_and_space_toggle_the_row_panel_that_a_person_selected() {
        let dash = dashboard();
        assert_eq!(row_key(Key::Enter, 1, &dash), Some(1));
        assert_eq!(row_key(Key::Char(' '), 1, &dash), Some(1));
    }

    /// The program keeps the key when the selection holds a panel of another
    /// kind. A program that holds a dashboard can bind the key `Enter` to a
    /// task of its own.
    #[test]
    fn a_selection_that_holds_no_row_panel_leaves_the_key_to_the_program() {
        let dash = dashboard();
        assert_eq!(row_key(Key::Enter, 0, &dash), None);
        assert_eq!(row_key(Key::Enter, 7, &dash), None, "No panel holds 7.");
    }

    #[test]
    fn another_key_toggles_nothing() {
        let dash = dashboard();
        for key in [Key::Tab, Key::Char('r'), Key::Left, Key::Other] {
            assert_eq!(row_key(key, 1, &dash), None, "{key:?}");
        }
    }
}
