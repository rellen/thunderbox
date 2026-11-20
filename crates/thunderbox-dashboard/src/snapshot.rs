//! The data that the code which draws reads.

use std::collections::BTreeMap;
use std::sync::Arc;

use thunderbox_core::{Frame, TimeRange, TimeWindow, Timestamp};

/// The state of one panel at one moment.
///
/// A query that returns an error becomes [`PanelState::Error`]. It never stops
/// the dashboard, and every other panel keeps its own state.
#[derive(Debug, Clone, Default)]
pub enum PanelState {
    /// No data source has answered this panel yet.
    ///
    /// A panel that the renderer drops holds this state until a data source
    /// answers it once. The renderer draws no panel under 3 cells wide or under
    /// 3 cells high, and [`crate::DashboardRuntime`] asks a data source for
    /// nothing for such a panel. No answer can therefore arrive for it. A panel
    /// that answered before the terminal became small keeps that answer
    /// instead.
    #[default]
    Loading,
    /// A data source answered. The frames hold the result.
    Ready(Arc<Vec<Frame>>),
    /// A data source returned an error. The text explains the error.
    Error(String),
}

impl PanelState {
    /// Returns `true` if this panel waits for a first answer.
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    /// Returns the text of the error, or `None` if the panel holds no error.
    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Error(text) => Some(text),
            _ => None,
        }
    }
}

/// Everything that the code which draws needs for one frame.
///
/// The runtime replaces a whole snapshot at once. The code that draws therefore
/// never sees one panel from an old query beside another panel from a new one.
///
/// A set of queries is the whole group of queries that one refresh sends. The
/// fields `panels`, `range`, `variables` and `updated_at` always come from one
/// set of queries, and a refresh never separates them. The field `refreshing`
/// is the one field that reports the set of queries which runs now.
///
/// A panel that the renderer drops is the one exception. No query of such a
/// panel ran, so its answer comes from an older set of queries. That answer
/// always describes the window in `range`, because a new window puts every
/// panel back to [`PanelState::Loading`]. Section 12 of `docs/ARCHITECTURE.md`
/// states that rule in full.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    /// The state of each panel, in the order of the dashboard.
    pub panels: Vec<PanelState>,
    /// The time range that the set of queries in `panels` used.
    pub range: TimeRange,
    /// The value that each variable held for the set of queries in `panels`.
    ///
    /// The key is the name of the variable, and the value is the choice that a
    /// person selected. Each query of that set read exactly these values. A
    /// snapshot that holds no answer yet names the values of the set of
    /// queries that runs now.
    ///
    /// The map holds no entry for a variable that holds no value. A variable
    /// that takes its choices from a data source holds no value until that
    /// data source answers.
    pub variables: BTreeMap<String, String>,
    /// The moment that the set of queries in `panels` asked about, in
    /// milliseconds after the Unix epoch.
    ///
    /// The runtime reads the clock when it starts a set of queries, and it
    /// gives that moment to each data source as the present moment. The field
    /// therefore states how old the numbers are. A data source that answers
    /// slowly does not move the field, and a refresh that waits for an answer
    /// does not move it either.
    pub updated_at: Option<Timestamp>,
    /// Reports the set of queries that runs now.
    ///
    /// The runtime writes `true` into this field before it starts a set of
    /// queries, and the task of that set writes `false` when it ends. The field
    /// is not a count, and it does not fall while the answers arrive. Read it
    /// through [`Snapshot::is_refreshing`].
    ///
    /// A value of `true` says that a set of queries runs now. The panels then
    /// hold the answers of the set before, or they hold no answer at all. They
    /// hold no answer when this is the first set, and also when the runtime put
    /// every panel back because the window changed. A panel that the renderer
    /// drops holds no answer as well.
    pub refreshing: bool,
}

impl Snapshot {
    /// Returns a snapshot in which every panel waits for a first answer.
    pub fn loading(panel_count: usize, range: TimeRange) -> Self {
        Self {
            panels: vec![PanelState::Loading; panel_count],
            range,
            ..Default::default()
        }
    }

    /// Returns the state of one panel.
    ///
    /// The method returns a state that waits for an answer when the index is
    /// outside the list. A panel that arrives before its first query therefore
    /// draws correctly.
    pub fn panel(&self, index: usize) -> &PanelState {
        const LOADING: &PanelState = &PanelState::Loading;
        self.panels.get(index).unwrap_or(LOADING)
    }

    /// Returns `true` if no panel holds an answer.
    ///
    /// A refresh keeps the answer of each panel, so this method returns `true`
    /// in three cases only:
    ///
    /// - the time before the first set of queries answers;
    /// - the time after the runtime put every panel back because the window
    ///   changed;
    /// - the whole time in which the renderer drops every panel.
    ///
    /// A window is the pair of moments that a time range names, and the moment
    /// that named them. The range `the last five minutes` names a new window
    /// on each refresh, and the runtime keeps the answers of such a range.
    ///
    /// The third case reaches a terminal that was too small before the first
    /// answers arrived. It does not reach a terminal that shrinks after the
    /// panels answered. The runtime keeps the answer of a panel that the
    /// renderer drops, so each panel stays at [`PanelState::Ready`] and this
    /// method returns `false`. A program must therefore not build its message
    /// for a small terminal on this method alone.
    ///
    /// The third case has no end of its own. The renderer draws no panel under
    /// 3 cells wide or under 3 cells high, and the runtime asks a data source
    /// for nothing for such a panel. A terminal that is too small for every
    /// panel therefore holds this method at `true` for as long as it stays that
    /// small. A program that draws a message about a small terminal reads
    /// [`crate::DashboardView::dropped_panels`], and it reads that method
    /// whatever this one returns. `crates/thunderbox/src/main.rs` does so.
    pub fn is_loading(&self) -> bool {
        self.panels.iter().all(PanelState::is_loading)
    }

    /// Returns `true` while a set of queries waits for an answer.
    ///
    /// The panels then hold the answers of the set of queries before, or they
    /// hold no answer at all. A program that draws can tell a reader that new
    /// answers are on the way.
    pub fn is_refreshing(&self) -> bool {
        self.refreshing
    }

    /// Returns the window that the set of queries in `panels` asked about.
    ///
    /// A window is the pair of moments that a time range names, and the moment
    /// that named them. The method returns `None` before the first set of
    /// queries answers, because no moment named a window yet.
    ///
    /// Each data source received exactly this window. The runtime reads the
    /// clock once when it starts a set of queries. It gives that one moment to
    /// each data source, and it also holds it in `updated_at`.
    pub fn window(&self) -> Option<TimeWindow> {
        self.updated_at.map(|now| self.range.window(now))
    }

    /// Returns the number of panels that hold an error.
    pub fn error_count(&self) -> usize {
        self.panels.iter().filter(|p| p.error().is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_snapshot_waits_for_every_panel() {
        let snap = Snapshot::loading(3, TimeRange::default());
        assert_eq!(snap.panels.len(), 3);
        assert!(snap.is_loading());
        assert_eq!(snap.error_count(), 0);
    }

    #[test]
    fn panel_returns_a_loading_state_outside_the_list() {
        let snap = Snapshot::loading(1, TimeRange::default());
        assert!(
            snap.panel(99).is_loading(),
            "A panel that has no state yet must still draw."
        );
    }

    #[test]
    fn a_snapshot_reports_the_set_of_queries_that_runs_now() {
        let mut snap = Snapshot::loading(2, TimeRange::default());
        assert!(
            !snap.is_refreshing(),
            "A snapshot that waits for no answer must report no set of queries."
        );
        snap.refreshing = true;
        assert!(
            snap.is_refreshing(),
            "A snapshot must report the set of queries that waits for an answer."
        );
    }

    #[test]
    fn a_snapshot_that_holds_no_answer_names_no_window() {
        let mut snap = Snapshot::loading(1, TimeRange::default());
        assert!(
            snap.window().is_none(),
            "No moment named a window before the first answer."
        );
        snap.updated_at = Some(1_000_000);
        let window = snap.window().expect("The answer names a window.");
        assert_eq!(
            (window.from, window.to, window.now),
            (700_000, 1_000_000, 1_000_000)
        );
    }

    #[test]
    fn error_count_counts_only_the_panels_that_hold_an_error() {
        let mut snap = Snapshot::loading(3, TimeRange::default());
        snap.panels[0] = PanelState::Error("no route to host".into());
        snap.panels[1] = PanelState::Ready(Arc::new(vec![]));
        assert_eq!(snap.error_count(), 1);
        assert!(!snap.is_loading());
        assert_eq!(snap.panels[0].error(), Some("no route to host"));
    }
}
