//! The runtime that runs the queries and holds the state.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use ratatui_core::layout::{Rect, Size};
use thunderbox_core::{
    Dashboard, PanelKind, Query, Result, TimeRange, max_data_points, now_ms, parse_duration_ms,
};
use thunderbox_source::Registry;
use thunderbox_widgets::{Marker, draws_a_panel};
use tokio::runtime::Handle;
use tokio::sync::{oneshot, watch};
use tokio::task::JoinHandle;

use crate::grid::{CollapsedRows, hidden, panel_width, places};
use crate::query::{PanelJob, PanelQueries, load_choices, run_all};
use crate::snapshot::{PanelState, Snapshot};
use crate::variable::{Choices, Variables};

/// The width of the area that holds the panels, in terminal cells, before the
/// first draw.
///
/// The runtime divides this width onto the panels of the dashboard, as
/// [`crate::DashboardView`] divides the real width. It replaces this number as
/// soon as a caller reports a real area.
///
/// The runtime holds no height of its own before that report. It asks a data
/// source about every panel of the dashboard until a caller reports an area. A
/// height that this crate guessed could be too small, and the first picture
/// would then hold no panel at all. A caller that never reports an area, such
/// as a program that draws nothing, also keeps every panel under this rule.
const ASSUMED_WIDTH: u16 = 80;

/// The time that a new area must stand still before the runtime uses it.
///
/// A caller reports the area of the panels on every frame. A person who drags
/// the edge of a terminal window therefore reports a new area many times each
/// second. The runtime holds such an area for this time, so that person sends
/// one set of queries and not one set for each frame.
///
/// The program that runs alone and the example `full_embed` each read their
/// terminal every 100 milliseconds. This time covers more than two frames of
/// such a program, and it does not grow with the refresh interval.
pub const AREA_QUIET_TIME: Duration = Duration::from_millis(250);

/// The longest time that the runtime holds a new area.
///
/// An area that changes on every frame never stands still for
/// [`AREA_QUIET_TIME`]. A panel area that grows and shrinks by one cell is one
/// example, and a scroll bar that appears and disappears makes one. The runtime
/// would hold such an area for as long as it moved, and the point limit would
/// stay wrong for that whole time. This value ends that wait.
///
/// The value is four times the quiet time. A person who drags the corner of a
/// terminal window therefore still sends few sets of queries. An area that
/// never stands still reaches the data source as well.
pub const AREA_MAX_HOLD: Duration = Duration::from_millis(1000);

/// What the runtime does with the answers that the panels hold.
///
/// A set of queries is the whole group of queries that one refresh sends. The
/// runtime keeps the answers of the set before, unless those answers describe
/// another window.
///
/// A window is the pair of moments that a time range names, and the moment
/// that named them. The range `the last five minutes` names a new window on
/// each refresh, and the runtime keeps the answers of such a range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kept {
    /// The runtime keeps the answer of each panel.
    ///
    /// A panel that the renderer drops keeps its answer as well. The new set of
    /// queries asks no data source about that panel, and it writes the answer
    /// back. A panel that holds an error is the one exception, because no later
    /// set of queries can clear such an error. [`kept_state`] states that rule.
    Answers,
    /// The runtime puts every panel back to the state that has no answer.
    Nothing,
}

/// The runtime of one dashboard.
///
/// This type behaves as a library. It never makes an async runtime, it never
/// owns the terminal, it never owns the key bindings, and it never changes
/// anything outside itself. `docs/ARCHITECTURE.md` section 12 states those
/// rules in full.
///
/// The runtime holds no lock that the code which draws must wait for. A set of
/// queries runs in a background task, and it publishes a whole [`Snapshot`]
/// when it finishes. [`DashboardRuntime::snapshot`] reads the most recent
/// snapshot and returns at once.
pub struct DashboardRuntime {
    dashboard: Dashboard,
    sources: Registry,
    handle: Handle,
    marker: Marker,
    /// The newest area whose point limits the set of queries that runs now
    /// carries.
    ///
    /// The runtime divides this area onto the panels, and each panel then holds
    /// a size of its own. [`DashboardRuntime::point_limits`] holds that step.
    ///
    /// The field holds `None` while no caller has reported an area. The runtime
    /// then asks a data source about every panel, at the width `ASSUMED_WIDTH`.
    ///
    /// An area that changes no point limit reaches this field without a set of
    /// queries. [`DashboardRuntime::take_area`] holds that step. The point
    /// limits of this field always match the point limits of the set of queries
    /// that runs now. The area itself can be a later one.
    area: Option<Size>,
    /// An area that a caller reported, and that no set of queries has used yet.
    ///
    /// The runtime holds `None` here while the newest area equals `area`.
    ///
    /// The width and the height wait together in this one field. A person who
    /// drags the corner of a terminal window changes both numbers. Two fields
    /// would then cost two sets of queries for one drag.
    pending_area: Option<Size>,
    /// The moment when a caller last reported a new area.
    area_changed_at: Instant,
    /// The moment when the wait of an area began, or `None` while no area
    /// waits and the area of the caller stands still.
    ///
    /// The runtime writes this moment when it arms an area that costs a set of
    /// queries and no wait is open. An area that costs none makes no point
    /// limit wrong, so it opens no wait.
    ///
    /// The runtime writes `None` when a set of queries starts, and also when
    /// the area of a caller stands still and no area waits.
    /// [`DashboardRuntime::end_the_wait_of_a_settled_area`] takes that second
    /// step. The runtime leaves the moment as it is at every other step.
    ///
    /// A report that needs no set of queries therefore does not end the wait.
    /// The area of one caller moves between a size that needs a set and a size
    /// that does not. Such a caller would otherwise start this wait again on
    /// every second frame. Neither [`AREA_QUIET_TIME`] nor [`AREA_MAX_HOLD`]
    /// could then pass, and that area would never reach a data source.
    area_wait_since: Option<Instant>,
    /// The row panels that are collapsed now.
    ///
    /// The runtime asks a data source for nothing about a panel that a
    /// collapsed row panel hides. Each snapshot carries a copy of this value,
    /// and the renderer reads that copy. [`DashboardRuntime::toggle_row`]
    /// changes it.
    collapsed: CollapsedRows,
    range: TimeRange,
    /// The choices of each variable, and the value that each one holds.
    variables: Variables,
    /// The channel that carries the choices of each variable that states a
    /// query.
    ///
    /// The runtime asks the data sources one time, in a task of its own. The
    /// field holds `None` while no such task runs. That covers a dashboard
    /// which declares no variable of that form, and also the time after
    /// [`DashboardRuntime::tick`] took the answer.
    choices: Option<AskedForChoices>,
    rx: watch::Receiver<Arc<Snapshot>>,
    tx: watch::Sender<Arc<Snapshot>>,
    /// The task that runs the set of queries now.
    ///
    /// The runtime stops the old task before it starts a new one. A task that
    /// already passed its last await point still finishes, and the generation
    /// number then stops it from the publication of its answer.
    task: Option<JoinHandle<()>>,
    /// The number of the set of queries that runs now.
    ///
    /// The runtime raises this number before it starts a new set of queries.
    /// Each task carries the number that started it, and [`publish`] compares
    /// the two numbers. The task runs on another thread, so the number sits in
    /// an atomic value behind an [`Arc`].
    generation: Arc<AtomicU64>,
    /// The time between two refreshes. `None` means that the dashboard
    /// refreshes only when a caller asks.
    interval: Option<Duration>,
    /// The time when the runtime last started a set of queries.
    last_refresh: Instant,
}

impl DashboardRuntime {
    /// Returns a runtime, and starts the first set of queries.
    ///
    /// The caller supplies the `Handle`. This type never makes an async runtime
    /// of its own, so a program that already runs tasks can hold a dashboard.
    ///
    /// The caller must supply a `Handle` of an async runtime that holds the
    /// timer of tokio. The dashboard holds each query to a limit, and that
    /// limit reads the timer. `#[tokio::main]` and `Runtime::new` each build
    /// such a runtime. A caller that builds its own runtime calls `enable_time`
    /// or `enable_all` on the builder.
    ///
    /// That runtime must also run a task on a thread other than the thread that
    /// draws. This type starts a task for each panel, and a loop that draws
    /// often waits for a key without an await. Such a loop parks its thread, so
    /// a runtime of the flavour `current_thread` polls no query task. Every
    /// panel then reads `loading…` for ever, and no error and no limit ends
    /// that wait. `Builder::new_multi_thread` builds a runtime that holds this
    /// rule, and both programs of this repository call it.
    ///
    /// # Errors
    ///
    /// This function returns an error if [`Dashboard::validate`] rejects the
    /// dashboard.
    ///
    /// # Panics
    ///
    /// The task of a panel panics when the async runtime of the `Handle` holds
    /// no timer of tokio. This function cannot read that fault, because tokio
    /// offers no way to ask a `Handle` for its timer. The panic ends the task
    /// of that panel, and the panel then holds an error that names the fault.
    /// Every panel reads the timer, so every panel holds that error.
    pub fn new(dashboard: Dashboard, sources: Registry, handle: Handle) -> Result<Self> {
        dashboard.validate()?;
        let range = dashboard.time.clone();
        let interval = match &dashboard.refresh {
            None => None,
            Some(text) => {
                let millis = parse_duration_ms(text)?;
                // A period of zero would ask the data source without a pause.
                u64::try_from(millis)
                    .ok()
                    .filter(|m| *m > 0)
                    .map(Duration::from_millis)
            }
        };
        let variables = Variables::new(&dashboard.variables);
        let choices = ask_for_choices(&dashboard, &sources, &handle);
        let collapsed = CollapsedRows::of(&dashboard);
        let mut first = Snapshot::loading(dashboard.panels.len(), range.clone());
        first.variables = variables.values();
        first.collapsed = collapsed.clone();
        let (tx, rx) = watch::channel(Arc::new(first));
        let mut runtime = Self {
            dashboard,
            sources,
            handle,
            marker: Marker::default(),
            area: None,
            pending_area: None,
            area_changed_at: Instant::now(),
            area_wait_since: None,
            collapsed,
            range,
            variables,
            choices,
            rx,
            tx,
            task: None,
            generation: Arc::new(AtomicU64::new(0)),
            interval,
            last_refresh: Instant::now(),
        };
        // The channel already holds a snapshot in which every panel waits for
        // an answer. The first set of queries therefore keeps that snapshot,
        // and it needs no rule of its own.
        runtime.refresh();
        Ok(runtime)
    }

    /// Sets the glyph that a chart draws a point with.
    ///
    /// The marker decides how many points a panel asks for, so a change here
    /// starts the queries again. The panels keep the answers that they hold,
    /// because the marker changes the number of points and not the window.
    ///
    /// A program can call this method on every frame. Both programs of this
    /// repository do so. The comparison below is the reason that such a call is
    /// safe: an equal marker starts nothing. Keep that comparison. Without it,
    /// ten frames over 200 milliseconds start ten sets of queries, and no panel
    /// ever leaves [`crate::PanelState::Loading`]. No test of this repository
    /// holds this rule today, because each test that calls this method calls it
    /// once with a marker that differs.
    pub fn set_marker(&mut self, marker: Marker) {
        if self.marker != marker {
            self.marker = marker;
            self.refresh();
        }
    }

    /// Reports the area that holds the panels, in terminal cells.
    ///
    /// The caller must report the area that it gives to
    /// [`crate::DashboardView::render`]. The runtime reads the width and the
    /// height of that area, and it reads nothing else of it. A caller that
    /// reports another area makes the runtime hold a wrong belief about the
    /// screen. The panels of that caller then ask for the wrong points.
    ///
    /// The caller gives the area of every panel, and not the area of one panel.
    /// The runtime divides that area onto the panels of the dashboard, as
    /// [`crate::DashboardView`] divides it. Each panel then asks for one point
    /// for each position that the marker can address inside that panel. A
    /// caller reports the area on every frame, and the runtime reads it on
    /// every frame.
    ///
    /// The runtime asks a data source for nothing for a panel that the renderer
    /// drops. The renderer draws no panel under 3 cells wide or under 3 cells
    /// high, and it drops such a panel. That panel holds
    /// [`crate::PanelState::Loading`] until an area that draws it reaches a
    /// data source.
    ///
    /// The first area that a caller reports reaches the data source at once,
    /// unless it changes no point limit. The runtime ran its first set of
    /// queries before that report. That set carried the width `ASSUMED_WIDTH`
    /// of 80 cells, and it asked about every panel. The first picture must hold
    /// the correct number of points.
    ///
    /// A later area that changes no point limit and drops no panel costs no
    /// set of queries either. The width of an area decides the point limit of
    /// each panel, and the height decides which panels the renderer drops. A
    /// person who drags the bottom edge of a terminal window therefore reaches
    /// no data source.
    ///
    /// Every other later area waits. The runtime uses such an area when no set
    /// of queries runs, and when one of these two conditions holds:
    ///
    /// - the area has stood still for [`AREA_QUIET_TIME`];
    /// - the area has waited for [`AREA_MAX_HOLD`].
    ///
    /// [`DashboardRuntime::tick`] takes that step, and a caller that drives
    /// `tick` needs no other call. A person who drags the corner of a terminal
    /// window therefore sends one set of queries, and not one set for each
    /// frame. The second condition serves a caller whose area never stands
    /// still.
    ///
    /// Each start of a set of queries also takes the area that waits, whatever
    /// started that set. A change of the marker after a change of the area
    /// therefore sends one set of queries that carries both.
    ///
    /// The panels keep the answers that they hold, because the area changes the
    /// number of points and not the window.
    pub fn set_panel_area(&mut self, area: Size) {
        if self.area.is_none() {
            self.area = Some(area);
            // A first area that changes no point limit and drops no panel asks
            // for nothing. The set of queries that runs already carries the
            // answer that this area needs.
            if self.limits_for(Some(area)) != self.limits_for(None) {
                self.refresh();
            }
            return;
        }
        if self.newest_area() == Some(area) {
            return;
        }
        let now = Instant::now();
        self.area_changed_at = now;
        // An area that returns to the area of the set that runs asks for
        // nothing. A person who drags the corner of a window back to the place
        // where it started therefore sends no set of queries.
        self.pending_area = (Some(area) != self.area).then_some(area);
        // The wait of an area holds the time that a point limit stays wrong to
        // one length. An area that changes no point limit makes no point limit
        // wrong, so it starts no wait. A caller can move through such areas
        // for longer than [`AREA_MAX_HOLD`]. Such a caller would otherwise
        // leave a wait open. The next area that changes a point limit would
        // then reach a data source at once.
        //
        // The wait of an area starts when the first such area arrives, and a
        // later change does not start it again. An area that changes on every
        // frame therefore still reaches the data source.
        if self.area_wait_since.is_none() && self.pending_area_costs_a_set() {
            self.area_wait_since = Some(now);
        }
    }

    /// Returns the newest area of the panels, which is the area that the next
    /// set of queries carries.
    ///
    /// The method returns `None` while no caller has reported an area.
    fn newest_area(&self) -> Option<Size> {
        self.pending_area.or(self.area)
    }

    /// Returns `true` when an area that waits must reach the data source now.
    ///
    /// The runtime uses such an area when it has stood still for
    /// [`AREA_QUIET_TIME`], or when it has waited for [`AREA_MAX_HOLD`]. The
    /// second condition serves a caller whose area never stands still, and it
    /// holds the wait of an area to a known length.
    ///
    /// Both conditions also ask that no set of queries runs. A new set would
    /// otherwise stop the set that runs before its answer arrived.
    fn area_is_due(&self) -> bool {
        let quiet = self.area_changed_at.elapsed() >= AREA_QUIET_TIME;
        let waited = self
            .area_wait_since
            .is_some_and(|since| since.elapsed() >= AREA_MAX_HOLD);
        self.pending_area.is_some() && (quiet || waited) && !self.is_refreshing()
    }

    /// Returns `true` when the area that waits needs a set of queries.
    ///
    /// The height of an area decides which panels the renderer drops, and the
    /// width decides the point limit of each panel. An area that changes
    /// neither asks a data source for exactly what the set of queries before
    /// asked. A person who drags the bottom edge of a terminal window therefore
    /// starts no set of queries.
    ///
    /// The method reads the panels that the new area draws, and it reads no
    /// other panel. A panel that the new area drops asks a data source for
    /// nothing, so a point limit that it loses costs no set of queries. A
    /// height that hides one panel of a column therefore leaves every other
    /// panel of that column with the answer that it holds.
    ///
    /// A panel that the new area draws and that the area before dropped costs a
    /// set of queries. The runtime holds no point limit for such a panel, so it
    /// cannot know whether the answer that the panel holds has the right shape.
    ///
    /// The method returns `false` when no area waits. No caller reads it in
    /// that state, because [`DashboardRuntime::area_is_due`] asks that an area
    /// waits.
    fn pending_area_costs_a_set(&self) -> bool {
        let Some(area) = self.pending_area else {
            return false;
        };
        let before = self.limits_for(self.area);
        self.limits_for(Some(area))
            .into_iter()
            .zip(before)
            .any(|(after, before)| after.is_some() && after != before)
    }

    /// Takes the area that waits into the field that holds the area in use.
    ///
    /// [`DashboardRuntime::tick`] calls this method for an area that costs no
    /// set of queries. [`DashboardRuntime::start`] calls it for an area that a
    /// set of queries carries.
    ///
    /// The runtime holds the area of the set of queries that runs now, and it
    /// reads that area for each later comparison. An area that costs no set of
    /// queries must therefore still reach that field.
    ///
    /// The method leaves the wait of that area open.
    /// [`DashboardRuntime::end_the_wait_of_a_settled_area`] states which step
    /// ends it.
    fn take_area(&mut self) {
        if let Some(area) = self.pending_area.take() {
            self.area = Some(area);
        }
    }

    /// Clears the error of each panel that the area in use drops.
    ///
    /// A set of queries writes that state through [`kept_state`]. A tick that
    /// takes an area without a set of queries must write it as well. The
    /// runtime asks no data source about a panel that the renderer drops, so no
    /// later set of queries could clear such an error.
    /// [`Snapshot::error_count`] would count it, and the status row of a
    /// program would name a fault that nobody can repair.
    ///
    /// The method writes into the channel only when it changes a panel. A tick
    /// that changes nothing therefore wakes no reader of that channel.
    fn clear_the_error_of_each_panel_that_the_area_drops(&mut self) {
        let limits = self.point_limits();
        self.tx.send_if_modified(|held| {
            let drops_a_panel_that_holds_an_error = limits
                .iter()
                .enumerate()
                .any(|(index, limit)| limit.is_none() && held.panel(index).error().is_some());
            if !drops_a_panel_that_holds_an_error {
                return false;
            }
            let held = Arc::make_mut(held);
            for (index, limit) in limits.iter().enumerate() {
                if limit.is_none() && held.panels[index].error().is_some() {
                    held.panels[index] = PanelState::Loading;
                }
            }
            true
        });
    }

    /// Ends the wait of an area that stands still, and that waits for nothing.
    ///
    /// Two steps leave no area in the wait. [`DashboardRuntime::tick`] takes an
    /// area that costs no set of queries. [`DashboardRuntime::set_panel_area`]
    /// drops the area that waits when a caller reports the area in use again.
    /// Neither step ends the wait by itself, because a caller that still moves
    /// would then start that wait again on its next report.
    ///
    /// This method reads the same quiet time that
    /// [`DashboardRuntime::area_is_due`] reads. A caller that stands still for
    /// [`AREA_QUIET_TIME`] has settled, so no area waits and the wait ends. A
    /// caller that still moves keeps the wait, and [`AREA_MAX_HOLD`] therefore
    /// still holds the whole of that movement to one length.
    fn end_the_wait_of_a_settled_area(&mut self) {
        if self.pending_area.is_none() && self.area_changed_at.elapsed() >= AREA_QUIET_TIME {
            self.area_wait_since = None;
        }
    }

    /// Collapses an expanded row panel, or expands a collapsed one.
    ///
    /// The value `index` is the position of the row panel in the dashboard.
    /// The method does nothing for a position that holds no row panel.
    /// [`crate::row_key`] maps a key to that position.
    ///
    /// The method writes the new state into the snapshot that the channel
    /// holds, so the next frame draws it. It then compares the point limit of
    /// each panel before and after the change:
    ///
    /// - A panel that now draws, and that drew nothing before, starts a set of
    ///   queries. An expanded row panel shows such panels. A collapse can show
    ///   one too, because each panel that remains grows. The runtime asked no
    ///   data source about such a panel, so the answer that it holds can be
    ///   old.
    /// - A change that only hides panels starts no set of queries. A width
    ///   decides the point limit of a panel, and a collapse changes no width.
    ///   The runtime clears the error of each panel that it hides, because no
    ///   later set of queries reaches such a panel.
    ///
    /// A program calls this method from a key press. The method starts at most
    /// one set of queries for each call. A key that a person holds down
    /// therefore costs one set for each repeat.
    pub fn toggle_row(&mut self, index: usize) {
        if self.dashboard.panels.get(index).map(|p| p.kind) != Some(PanelKind::Row) {
            return;
        }
        let before = self.point_limits();
        self.collapsed.toggle(index);
        let collapsed = self.collapsed.clone();
        self.tx
            .send_modify(|held| Arc::make_mut(held).collapsed = collapsed);
        let appears = self
            .point_limits()
            .iter()
            .zip(&before)
            .any(|(after, before)| after.is_some() && before.is_none());
        if appears {
            self.refresh();
        } else {
            self.clear_the_error_of_each_panel_that_the_area_drops();
        }
    }

    /// Returns the row panels that are collapsed now.
    pub fn collapsed_rows(&self) -> &CollapsedRows {
        &self.collapsed
    }

    /// Sets the time range that every panel shares, and starts the queries
    /// again.
    ///
    /// The runtime puts every panel back to the state that has no answer when
    /// the new range differs from the range before. Those answers describe
    /// another window, and a reader would misread them.
    ///
    /// The runtime compares the two expressions of the range, and not the two
    /// windows that they name. A range that says `the last five minutes` names
    /// a later window on each refresh, and the panels keep their answers.
    ///
    /// The method starts a new set of queries on every call, even when the new
    /// range equals the range before. A caller therefore calls this method
    /// from a key press, and never on every frame. A caller that pushed a
    /// range on every frame would stop and start a set of queries on every
    /// frame.
    pub fn set_time_range(&mut self, range: TimeRange) {
        let kept = if self.range == range {
            Kept::Answers
        } else {
            Kept::Nothing
        };
        self.range = range;
        self.start(kept);
    }

    /// Starts the queries again when the refresh interval has passed.
    ///
    /// The caller drives this method from its own event loop. thunderbox starts
    /// no timer of its own, because it owns no event loop. A caller can call
    /// this method as often as it wants, and the method does nothing until the
    /// interval has passed.
    ///
    /// The method starts nothing while a set of queries still waits for an
    /// answer. A data source that answers slower than the interval therefore
    /// finishes one set of queries. The runtime does not stop each set before
    /// the answer arrives.
    ///
    /// A data source that never answers stops no later tick. The runtime holds
    /// each query to a limit, so every set of queries ends. The panel of such a
    /// data source then holds that wait as its error, and this method starts
    /// the next set. [`crate::DEFAULT_ANSWER_LIMIT`] states the limit for a
    /// data source that states none of its own.
    ///
    /// One set of queries runs its panels at the same time. A dashboard of
    /// four panels, each on a data source that never answers, therefore holds
    /// one set for that limit alone. This method starts nothing in that time.
    ///
    /// The method also gives an area that waits to a new set of queries.
    /// [`DashboardRuntime::set_panel_area`] states that rule. This step comes
    /// before the check of the interval, so a dashboard that states no refresh
    /// interval receives a new area as well.
    ///
    /// The method also takes the choices of each variable that states a query.
    /// [`DashboardRuntime::variables`] states that rule. A caller that drives
    /// this method therefore needs no other call for a variable.
    ///
    /// An area that waits and that changes no point limit reaches the runtime
    /// without a set of queries. The method takes such an area and then reads
    /// the interval, exactly as it reads the interval when no area waits.
    ///
    /// The method returns `true` when it started a new set of queries. A tick
    /// that started a set for an area therefore returns `true`. A tick that
    /// started one for the interval or for a variable returns `true` as well.
    /// A tick that took an area and started no set returns `false`.
    pub fn tick(&mut self) -> bool {
        if self.take_choices() {
            return true;
        }
        if self.area_is_due() {
            // An area that changes no point limit and drops no panel asks for
            // nothing. The set of queries that ran already carries the answer
            // that this area needs, so the runtime takes the area and starts
            // no set.
            if self.pending_area_costs_a_set() {
                self.refresh();
                return true;
            }
            self.take_area();
            self.clear_the_error_of_each_panel_that_the_area_drops();
        }
        self.end_the_wait_of_a_settled_area();
        let Some(interval) = self.interval else {
            return false;
        };
        if self.last_refresh.elapsed() < interval {
            return false;
        }
        if self.is_refreshing() {
            return false;
        }
        self.refresh();
        true
    }

    /// Returns the time between two refreshes.
    pub fn refresh_interval(&self) -> Option<Duration> {
        self.interval
    }

    /// Returns the time range that every panel shares.
    pub fn time_range(&self) -> &TimeRange {
        &self.range
    }

    /// Returns the choices of each variable, and the value that each one
    /// holds.
    ///
    /// The method returns a state that holds nothing for a dashboard which
    /// declares no variable. A program reads this state to draw the variables,
    /// and it also passes it to [`crate::variable_key`].
    ///
    /// The runtime asks the data source of each variable that states a query
    /// one time, when it starts. [`DashboardRuntime::tick`] takes that answer,
    /// so this method returns the choices of such a variable only after a
    /// caller has driven `tick`. The choices of a variable that states
    /// `options` are there from the first call.
    pub fn variables(&self) -> &Variables {
        &self.variables
    }

    /// Takes a new selection, and starts the queries again when a value
    /// changed.
    ///
    /// The caller reads the state from [`DashboardRuntime::variables`], and it
    /// asks [`crate::variable_key`] for the new state. A program therefore
    /// calls this method from a key press.
    ///
    /// The method starts a new set of queries only when a value differs from
    /// the value before. A key that moves the cursor from one variable to
    /// another asks a data source for nothing.
    ///
    /// The runtime puts every panel back to the state that has no answer when
    /// a value changed. The answers of the set before describe another
    /// question, and a reader would misread them.
    pub fn set_variables(&mut self, variables: Variables) {
        let changed = variables.values() != self.variables.values();
        self.variables = variables;
        if changed {
            self.start(Kept::Nothing);
        }
    }

    /// Takes the choices that the data sources returned.
    ///
    /// The method returns `true` when it started a new set of queries. It
    /// starts one only when a value changed, as
    /// [`DashboardRuntime::set_variables`] does. An answer that holds the
    /// value of every variable therefore costs no second set of queries.
    ///
    /// The method reads the channel and returns at once. It never waits for a
    /// data source.
    fn take_choices(&mut self) -> bool {
        let Some(asked) = &mut self.choices else {
            return false;
        };
        let loaded = match asked.rx.try_recv() {
            Ok(loaded) => loaded,
            Err(oneshot::error::TryRecvError::Empty) => return false,
            // The task ended before it sent an answer. The runtime asks no
            // more, and each variable keeps the value that the dashboard
            // states.
            Err(oneshot::error::TryRecvError::Closed) => {
                self.choices = None;
                return false;
            }
        };
        self.choices = None;
        let before = self.variables.values();
        self.variables.take_choices(loaded);
        if self.variables.values() == before {
            return false;
        }
        self.start(Kept::Nothing);
        true
    }

    /// Returns the dashboard that this runtime draws.
    pub fn dashboard(&self) -> &Dashboard {
        &self.dashboard
    }

    /// Returns the most recent snapshot.
    ///
    /// The method returns at once. It never waits for a query. The code that
    /// draws calls this method on every frame.
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.rx.borrow().clone()
    }

    /// Returns `true` while a set of queries waits for an answer.
    ///
    /// The panels then hold the answers of the set of queries before, or they
    /// hold no answer at all. They hold no answer when this is the first set,
    /// and also when the runtime put every panel back because the window
    /// changed.
    pub fn is_refreshing(&self) -> bool {
        self.rx.borrow().is_refreshing()
    }

    /// Returns the time that the set of queries which runs now has waited.
    ///
    /// The method returns `None` when no set of queries runs. The runtime
    /// reads the clock when it starts a set, so this time describes the set
    /// itself. A program that draws therefore reports a data source that
    /// answers slowly, and it does not report the age of its own frames.
    pub fn refresh_age(&self) -> Option<Duration> {
        self.is_refreshing().then(|| self.last_refresh.elapsed())
    }

    /// Returns a receiver that reports each new snapshot.
    ///
    /// A program that draws only after a change can wait on this receiver. A
    /// program that draws on a timer does not need it.
    pub fn subscribe(&self) -> watch::Receiver<Arc<Snapshot>> {
        self.rx.clone()
    }

    /// Returns the point limit of each panel, in the order of the dashboard.
    ///
    /// The value is `None` for a panel that the renderer drops. The renderer
    /// draws no panel under 3 cells wide or under 3 cells high, and
    /// [`thunderbox_widgets::draws_a_panel`] holds that rule. The runtime asks a
    /// data source for nothing for such a panel.
    ///
    /// Every value is `Some` while no caller has reported an area. The runtime
    /// then knows no height, so it can drop no panel. It reads the width
    /// `ASSUMED_WIDTH` for the point limit of each panel.
    ///
    /// The method reads the place of each panel from [`crate::grid::places`],
    /// which is the function that the renderer reads for the place of that
    /// panel. A panel therefore asks a data source for the points of the width
    /// that it draws, and the two parts drop the same panels.
    ///
    /// The value is `None` for a row panel, which shows no data. It is `None`
    /// as well for a panel that a collapsed row panel hides, with an area or
    /// without one. The runtime knows which row panels are collapsed before
    /// any caller reports an area.
    ///
    /// A panel that holds few cells asks for few points. The default dashboard
    /// holds a panel of 8 grid columns of 24, and that panel asked for the
    /// points of the whole area before this rule. It asked for about three
    /// times the points that it draws.
    ///
    /// The width of a panel counts the border of that panel. A panel therefore
    /// asks for a few points more than it draws, and never for fewer.
    fn point_limits(&self) -> Vec<Option<u32>> {
        self.limits_for(self.area)
    }

    /// Returns the point limit that each panel would take on this area.
    ///
    /// The value `None` for `area` states that no caller has reported an area.
    /// [`DashboardRuntime::point_limits`] reads the area that the set of
    /// queries which runs now carries, and this method reads any area. A caller
    /// can therefore ask what an area would cost before it takes that area.
    fn limits_for(&self, area: Option<Size>) -> Vec<Option<u32>> {
        let subcells = self.marker.horizontal_subcells();
        let hidden = hidden(&self.dashboard, &self.collapsed);
        let placed = area.map(|area| {
            places(
                &self.dashboard,
                &self.collapsed,
                Rect::new(0, 0, area.width, area.height),
            )
        });
        self.dashboard
            .panels
            .iter()
            .enumerate()
            .map(|(index, panel)| {
                if panel.kind == PanelKind::Row || hidden[index] {
                    return None;
                }
                let Some(placed) = &placed else {
                    let width = panel_width(panel.grid, ASSUMED_WIDTH);
                    return Some(max_data_points(width, subcells));
                };
                let place = placed[index]?;
                draws_a_panel(place.as_size()).then(|| max_data_points(place.width, subcells))
            })
            .collect()
    }

    /// Returns what the next set of queries does about each panel, in the order
    /// of the dashboard.
    ///
    /// A panel that the renderer draws takes a [`PanelJob::Ask`], which holds
    /// the targets of that panel beside the point limit of that same panel. A
    /// panel that the renderer drops takes a [`PanelJob::Skip`], and the set of
    /// queries asks a data source for nothing about it.
    ///
    /// The list holds one value for each panel, and never fewer. `run_all`
    /// writes the answer of a panel at the position that the dashboard gives
    /// that panel. A list that held no entry for a panel would move every panel
    /// after it.
    ///
    /// A target holds the pattern beside the query. The code that runs the
    /// queries needs both, so this method copies the whole target.
    fn panel_jobs(&self, kept: Kept) -> Vec<PanelJob> {
        let held = self.rx.borrow();
        self.point_limits()
            .into_iter()
            .zip(&self.dashboard.panels)
            .enumerate()
            .map(|(index, (limit, panel))| match limit {
                Some(budget) => PanelJob::Ask(PanelQueries {
                    targets: panel.targets.clone(),
                    budget,
                }),
                // The job carries the state that a panel which the renderer
                // drops keeps. `kept_state` states that rule.
                None => PanelJob::Skip(kept_state(kept, held.panel(index))),
            })
            .collect()
    }

    /// Stops the set of queries that runs now, and starts a new set.
    ///
    /// The new set takes the area that waits, as every other set does.
    /// [`DashboardRuntime::set_panel_area`] states that rule. A caller that
    /// presses the key `r` while a person drags the corner of the window
    /// therefore changes the point limit as well.
    ///
    /// Each panel that the renderer draws keeps the answer that it holds. The
    /// runtime writes `true` into [`Snapshot::refreshing`] instead, and that
    /// field then reports the new answers that are on the way. One task builds
    /// a whole snapshot and publishes it in one step, so two panels never come
    /// from two different sets of queries.
    ///
    /// A panel that the renderer drops keeps the answer that it holds. The new
    /// set asks no data source about that panel, so it writes that answer back.
    /// A set that wrote [`PanelState::Loading`] there instead would draw
    /// `loading…` in that panel until the terminal grows to hold it again.
    ///
    /// The runtime raises the generation number first. A task from an older
    /// generation therefore cannot publish its answers, even when it finishes
    /// after the new task starts. It also reports no end of its own.
    pub fn refresh(&mut self) {
        self.start(Kept::Answers);
    }

    /// Starts a new set of queries, and states what the panels keep.
    ///
    /// The method writes `true` into [`Snapshot::refreshing`] before it starts
    /// the queries, and never inside the task. A caller therefore never reads a
    /// snapshot of the set before that reports no set of queries. A runtime
    /// that holds several threads can let a fast data source answer before this
    /// method returns, and the caller then reads the new answer.
    ///
    /// The method reads the clock once. It gives that moment to `last_refresh`
    /// and to each data source, so the two agree.
    ///
    /// The method takes the area that waits before it reads the point limit.
    /// Every set of queries therefore carries the newest area.
    ///
    /// The method raises the generation number before it stops the task of the
    /// set before. That task then publishes no answer, and it reports no end of
    /// its own. [`ClearOnEnd`] reports the end of a set that ends with no
    /// answer, and a task that the runtime stopped must not reach that report.
    fn start(&mut self, kept: Kept) {
        // An area that waits reaches the data source with this set, whatever
        // started this set. The wait of that area ends here.
        self.take_area();
        self.area_wait_since = None;
        // The number rises before the abort, and never after it. A task that
        // this method stops then finds another number, so it writes nothing
        // into the channel.
        let mine = self.generation.fetch_add(1, Ordering::Relaxed) + 1;
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.last_refresh = Instant::now();
        let now = now_ms();

        let panels = self.panel_jobs(kept);
        // The report of the end of this set writes an error into the panels
        // that this set asked about. It writes no error into a panel that the
        // renderer drops, because no query of such a panel ran.
        let asked: Vec<bool> = panels
            .iter()
            .map(|job| matches!(job, PanelJob::Ask(_)))
            .collect();
        let range = self.range.clone();
        let values = self.variables.values();
        let sources = self.sources.clone();
        let tx = self.tx.clone();
        let generation = self.generation.clone();
        let panel_count = self.dashboard.panels.len();

        // No task can write into the channel between the raise of the number
        // above and this report. A task of an older generation reads the number
        // that this method already raised, and the task of this set does not
        // exist yet.
        self.tx.send_if_modified(|held| {
            match kept {
                // The report touches this one field. The panels, the range,
                // and the time of the answer belong to the set of queries
                // before, and they stay together.
                Kept::Answers => Arc::make_mut(held).refreshing = true,
                Kept::Nothing => {
                    let mut no_answer = Snapshot::loading(panel_count, range.clone());
                    no_answer.variables = values.clone();
                    no_answer.collapsed = held.collapsed.clone();
                    no_answer.refreshing = true;
                    *held = Arc::new(no_answer);
                }
            }
            true
        });

        self.task = Some(self.handle.spawn(async move {
            // The value `end` reports the end of this set of queries, whether
            // the set answers or not. A data source that panics therefore
            // stops no later tick.
            let end = ClearOnEnd {
                tx,
                generation,
                mine,
                asked,
            };
            let snapshot = run_all(&sources, &panels, &range, now, &values).await;
            publish(&end.tx, &end.generation, end.mine, snapshot);
        }));
    }
}

/// Returns the state that a panel which the renderer drops keeps.
///
/// A panel that the renderer drops asks no data source. The panel therefore
/// holds the state that this function returns until the terminal grows again.
/// Two states would mislead a person over that time, and this function returns
/// the state that has no answer in their place.
///
/// The first is an answer of another window. `Kept::Nothing` says that the
/// window changed, and the answers of the set before describe the window
/// before. A chart that drew those frames against the new window would stop
/// before the right edge. A person reads that shape as a data source that
/// died.
///
/// The second is an error. A panel that the renderer drops reaches no data
/// source, so no later set of queries can clear such an error.
/// [`Snapshot::error_count`] would count it, and the status row would name a
/// fault that nobody can repair.
fn kept_state(kept: Kept, held: &PanelState) -> PanelState {
    match (kept, held) {
        (Kept::Nothing, _) | (Kept::Answers, PanelState::Error(_)) => PanelState::Loading,
        (Kept::Answers, other) => other.clone(),
    }
}

impl Drop for DashboardRuntime {
    /// Stops the background task.
    ///
    /// A program that holds a dashboard can drop it and make another one. This
    /// implementation makes sure that the first one leaks no task.
    fn drop(&mut self) {
        // The number rises before the abort, as it does in `start`. The task
        // that this method stops then writes nothing into the channel. A
        // receiver that a caller kept therefore reads no error of a set that
        // the runtime stopped.
        self.generation.fetch_add(1, Ordering::Relaxed);
        if let Some(task) = self.task.take() {
            task.abort();
        }
        if let Some(asked) = self.choices.take() {
            asked.task.abort();
        }
    }
}

/// The error that every panel holds when a set of queries ends with no answer.
///
/// A person reads this text in the panel. Each panel runs in a task of its own,
/// and a fault inside one of those tasks reaches that panel alone. This text
/// therefore names no data source. It reports the task that holds the whole set.
/// That task ends with no answer when the async runtime of the caller stops. It
/// also ends that way when the code that runs the set meets a fault.
const NO_ANSWER: &str = "The set of queries ended with no answer. The async runtime stopped, \
                         or the code that runs the set met a fault.";

/// Reports the end of one set of queries when the task of that set ends.
///
/// The task writes `false` into [`Snapshot::refreshing`] together with its
/// answer. A task that ends with no answer writes nothing, so this value writes
/// that `false` in its place. A fault inside the task of the set therefore
/// stops no later call to [`DashboardRuntime::tick`].
///
/// This value also gives the error [`NO_ANSWER`] to every panel that the set
/// asked about. A task that ends with no answer would otherwise change nothing
/// that a person can read. Every panel would keep `loading…` for ever when the
/// first set of queries ended that way. The status row would report no error
/// and no wait.
///
/// This value leaves a panel that the renderer drops as it is. No query of such
/// a panel ran, so [`NO_ANSWER`] would name a fault that it never met.
/// `Snapshot::error_count` would also count a panel that nobody draws. The
/// panel keeps the state that [`DashboardRuntime::start`] gave it. That state
/// is the answer that the panel held, or the state that has no answer when the
/// window changed.
///
/// A data source that panics no longer reaches this value. `run_all` runs each
/// panel in a task of its own, and it reads the end of a task that gives no
/// answer. The panel of that data source holds an error of its own, and the
/// task of the set still publishes a snapshot.
///
/// The value writes nothing while a newer set of queries runs, because the
/// generation number then differs. A task that the runtime stopped therefore
/// leaves the report of the new set as it is. [`DashboardRuntime::start`]
/// raises that number before it stops a task, so a task that the runtime
/// stopped reaches no line below.
struct ClearOnEnd {
    /// The channel that holds the snapshot.
    tx: watch::Sender<Arc<Snapshot>>,
    /// The generation of the set of queries that runs now.
    generation: Arc<AtomicU64>,
    /// The generation of the set of queries that this task runs.
    mine: u64,
    /// `true` for each panel that this set of queries asked about.
    ///
    /// The list holds one value for each panel of the dashboard, in the order
    /// of the dashboard.
    asked: Vec<bool>,
}

impl Drop for ClearOnEnd {
    fn drop(&mut self) {
        self.tx.send_if_modified(|held| {
            if self.generation.load(Ordering::Relaxed) != self.mine || !held.refreshing {
                return false;
            }
            let held = Arc::make_mut(held);
            held.refreshing = false;
            for (index, panel) in held.panels.iter_mut().enumerate() {
                if self.asked.get(index).copied().unwrap_or(false) {
                    *panel = PanelState::Error(NO_ANSWER.to_string());
                }
            }
            true
        });
    }
}

/// Publishes a snapshot, and reports whether the channel took it.
///
/// The number `mine` is the generation of the set of queries that made this
/// snapshot. The function drops the snapshot when that number differs from the
/// generation that runs now. An answer of an older set of queries therefore
/// never replaces a newer answer. It also never reports that a set of queries
/// which still runs has finished.
///
/// The function reads the number and writes the snapshot inside one call to
/// `send_if_modified`. That call holds the lock of the channel for both steps,
/// so no other publisher can come between them. The lock also orders the two
/// threads, and `Relaxed` is therefore enough for the atomic number.
///
/// A channel that holds no receiver takes the snapshot as any other channel
/// does. A runtime that dropped is not an error.
fn publish(
    tx: &watch::Sender<Arc<Snapshot>>,
    generation: &AtomicU64,
    mine: u64,
    mut snapshot: Snapshot,
) -> bool {
    tx.send_if_modified(|held| {
        if generation.load(Ordering::Relaxed) != mine {
            return false;
        }
        // The runtime owns the row panels that are collapsed, and it writes
        // each change into the channel at once. A collapse that only hides
        // panels starts no set of queries, so the set that runs does not stop.
        // Its answer must not carry the state from before that collapse.
        snapshot.collapsed = held.collapsed.clone();
        *held = Arc::new(snapshot);
        true
    })
}

/// The question that the runtime asked about the choices of each variable.
///
/// The runtime asks one time, when it starts. It holds this value until
/// [`DashboardRuntime::tick`] takes the answer, and it stops the task when it
/// drops.
struct AskedForChoices {
    /// The channel that carries the answer.
    rx: oneshot::Receiver<Vec<(String, Choices)>>,
    /// The task that asks each data source.
    task: JoinHandle<()>,
}

/// Starts the task that asks for the choices of each variable.
///
/// The function returns `None` for a dashboard in which no variable states a
/// query. The runtime then reads no channel, and it starts no task.
///
/// The runtime asks one time, when it starts. A dashboard that must ask again
/// stops and starts. The choices of a variable change rarely, and a question
/// on each refresh would reach the data source as often as the panels do.
fn ask_for_choices(
    dashboard: &Dashboard,
    sources: &Registry,
    handle: &Handle,
) -> Option<AskedForChoices> {
    let asks: Vec<(String, Query)> = dashboard
        .variables
        .iter()
        .filter_map(|var| var.query.clone().map(|query| (var.name.clone(), query)))
        .collect();
    if asks.is_empty() {
        return None;
    }
    let (tx, rx) = oneshot::channel();
    let sources = sources.clone();
    let task = handle.spawn(async move {
        // The runtime drops the receiver when a person drops the dashboard.
        // The answer then has no reader, and that is not an error.
        let _ = tx.send(load_choices(&sources, asks).await);
    });
    Some(AskedForChoices { rx, task })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::PanelState;

    /// Returns a snapshot whose one panel holds a text as its error.
    ///
    /// A test reads that text back, and it then knows which snapshot the
    /// channel holds.
    fn with_error(text: &str) -> Snapshot {
        let mut snapshot = Snapshot::loading(1, TimeRange::default());
        snapshot.panels[0] = PanelState::Error(text.to_string());
        snapshot
    }

    /// The decision of [`publish`] has no test outside this crate. A task
    /// that lost the race passes its last await point and then publishes. A
    /// test in `tests/` cannot make that happen at a chosen moment, so this
    /// test drives the decision itself.
    ///
    /// This test writes the generation number that it reads. The test
    /// `each_set_of_queries_takes_a_new_number` covers the other half of the
    /// guard, which is the number that the runtime itself writes.
    #[test]
    fn an_answer_of_an_older_set_of_queries_never_replaces_a_newer_answer() {
        let start = Arc::new(Snapshot::loading(1, TimeRange::default()));
        let (tx, _rx) = watch::channel(start);
        let generation = AtomicU64::new(2);

        assert!(
            publish(&tx, &generation, 2, with_error("the new answer")),
            "The runtime must publish the answer of the set of queries that runs now."
        );
        assert!(
            !publish(&tx, &generation, 1, with_error("the old answer")),
            "The runtime must drop the answer of an older set of queries."
        );
        assert_eq!(
            tx.borrow().panel(0).error(),
            Some("the new answer"),
            "The channel must still hold the newer answer."
        );
    }

    #[test]
    fn an_answer_of_an_older_set_never_reports_the_end_of_a_newer_set() {
        let mut waits = Snapshot::loading(1, TimeRange::default());
        waits.refreshing = true;
        let (tx, _rx) = watch::channel(Arc::new(waits));
        let generation = AtomicU64::new(5);

        assert!(
            !publish(&tx, &generation, 4, with_error("the old answer")),
            "The runtime must drop the answer of an older set of queries."
        );
        assert!(
            tx.borrow().is_refreshing(),
            "An older answer must not report that the newer set of queries has finished."
        );
    }

    /// The dashboard of the test below. It holds one panel and no target.
    const ONE_PANEL: &str = r#"
title = "test"
[[panel]]
title = "first"
kind = "timeseries"
grid = { x = 0, y = 0, w = 12, h = 8 }
"#;

    /// `kept_state` must take an error from a panel that the renderer drops.
    ///
    /// Two paths write that state. A set of queries writes it through
    /// `panel_jobs`, and a tick that takes an area writes it through
    /// `clear_the_error_of_each_panel_that_the_area_drops`. This test holds the
    /// rule itself, so it holds both paths to the same words.
    #[test]
    fn kept_state_takes_an_error_from_a_panel_that_the_renderer_drops() {
        let error = PanelState::Error("the machine did not answer".to_string());
        assert!(
            kept_state(Kept::Answers, &error).is_loading(),
            "A panel that holds an error must lose it."
        );
        assert!(
            matches!(
                kept_state(Kept::Answers, &PanelState::Ready(Arc::new(Vec::new()))),
                PanelState::Ready(_)
            ),
            "A panel that holds an answer must keep it."
        );
        assert!(
            kept_state(Kept::Nothing, &PanelState::Ready(Arc::new(Vec::new()))).is_loading(),
            "A new window must take the answer of every panel."
        );
    }

    /// `take_area` must leave the wait of an area open.
    ///
    /// `tick` calls `take_area` for an area that costs no set of queries. A
    /// caller that moves between such an area and an area that costs one would
    /// otherwise lose the wait on every second frame. The area that costs a set
    /// of queries would then reach a data source after two of the longest
    /// holds, and not after one.
    #[tokio::test]
    async fn taking_an_area_leaves_the_wait_of_that_area_open() {
        let dashboard = Dashboard::from_toml(ONE_PANEL).expect("The dashboard must be correct.");
        let mut runtime = DashboardRuntime::new(dashboard, Registry::new(), Handle::current())
            .expect("The runtime must start.");
        let opened = Instant::now();
        runtime.area = Some(Size::new(100, 40));
        runtime.pending_area = Some(Size::new(100, 41));
        runtime.area_wait_since = Some(opened);

        runtime.take_area();
        assert_eq!(
            runtime.area,
            Some(Size::new(100, 41)),
            "The area that waited must reach the field of the area in use."
        );
        assert_eq!(
            runtime.pending_area, None,
            "No area may wait after the runtime took the area that waited."
        );
        assert_eq!(
            runtime.area_wait_since,
            Some(opened),
            "The wait of that area must stay open."
        );
    }

    /// A set of queries that runs holds `area_is_due`, so an area can wait
    /// while the area of the caller stands still.
    /// `end_the_wait_of_a_settled_area` must keep the wait of that area.
    ///
    /// A runtime that ended it would measure the longest hold from a later
    /// moment. The area that waits would then reach a data source up to one
    /// longest hold after the moment that `AREA_MAX_HOLD` states.
    #[tokio::test]
    async fn the_wait_of_an_area_that_still_waits_does_not_end() {
        let dashboard = Dashboard::from_toml(ONE_PANEL).expect("The dashboard must be correct.");
        let mut runtime = DashboardRuntime::new(dashboard, Registry::new(), Handle::current())
            .expect("The runtime must start.");
        // The area of the caller stands still, and an area waits.
        let opened = Instant::now() - AREA_QUIET_TIME * 2;
        runtime.area = Some(Size::new(100, 40));
        runtime.pending_area = Some(Size::new(120, 40));
        runtime.area_changed_at = opened;
        runtime.area_wait_since = Some(opened);

        runtime.end_the_wait_of_a_settled_area();
        assert_eq!(
            runtime.area_wait_since,
            Some(opened),
            "The wait of an area that still waits must not end."
        );

        // The runtime takes that area, so no area waits.
        runtime.pending_area = None;
        runtime.end_the_wait_of_a_settled_area();
        assert!(
            runtime.area_wait_since.is_none(),
            "The wait must end when no area waits and the area of the caller \
             stands still."
        );
    }

    /// The runtime must give each set of queries a new number. The two tests
    /// above give [`publish`] a number of their own, so neither of them reads
    /// the number that the runtime writes.
    #[tokio::test]
    async fn each_set_of_queries_takes_a_new_number() {
        let dashboard = Dashboard::from_toml(ONE_PANEL).expect("The dashboard must be correct.");
        let mut runtime = DashboardRuntime::new(dashboard, Registry::new(), Handle::current())
            .expect("The runtime must start.");
        let first = runtime.generation.load(Ordering::Relaxed);
        runtime.refresh();
        let second = runtime.generation.load(Ordering::Relaxed);
        assert!(
            second > first,
            "A new set of queries must take a new number. Before: {first}. After: {second}."
        );
    }

    /// A task that ends with no answer must report the end of its own set of
    /// queries. The runtime otherwise waits for a set that no longer runs.
    #[test]
    fn the_end_of_a_task_reports_the_end_of_its_own_set_of_queries() {
        let start = Arc::new(Snapshot::loading(1, TimeRange::default()));
        let (tx, _rx) = watch::channel(start);
        let generation = Arc::new(AtomicU64::new(1));
        tx.send_modify(|held| Arc::make_mut(held).refreshing = true);

        drop(ClearOnEnd {
            tx: tx.clone(),
            generation: generation.clone(),
            mine: 1,
            asked: vec![true],
        });
        assert!(
            !tx.borrow().is_refreshing(),
            "The end of the task must report the end of the set of queries."
        );

        tx.send_modify(|held| Arc::make_mut(held).refreshing = true);
        generation.store(2, Ordering::Relaxed);
        drop(ClearOnEnd {
            tx: tx.clone(),
            generation,
            mine: 1,
            asked: vec![true],
        });
        assert!(
            tx.borrow().is_refreshing(),
            "The end of an older task must not report the end of a newer set."
        );
    }

    /// A task that ends with no answer gives an error to each panel that its
    /// set of queries asked about. A panel that the renderer drops met no
    /// fault, because no query of that panel ran. The value must therefore
    /// leave such a panel as it is, and `Snapshot::error_count` must count it
    /// as no error.
    #[test]
    fn the_end_of_a_task_gives_no_error_to_a_panel_that_the_renderer_drops() {
        let start = Arc::new(Snapshot::loading(2, TimeRange::default()));
        let (tx, _rx) = watch::channel(start);
        let generation = Arc::new(AtomicU64::new(1));
        // The second panel holds an answer. A panel that the renderer drops
        // holds such a state after a refresh of the same window.
        tx.send_modify(|held| {
            let held = Arc::make_mut(held);
            held.refreshing = true;
            held.panels[1] = PanelState::Ready(Arc::new(Vec::new()));
        });

        drop(ClearOnEnd {
            tx: tx.clone(),
            generation,
            mine: 1,
            asked: vec![true, false],
        });
        let held = tx.borrow();
        assert_eq!(
            held.panel(0).error(),
            Some(NO_ANSWER),
            "The panel that the set asked about must hold the error."
        );
        assert!(
            matches!(held.panel(1), PanelState::Ready(_)),
            "The panel that the renderer drops must keep its answer. {:?}",
            held.panel(1)
        );
        assert_eq!(
            held.error_count(),
            1,
            "The status row must report one error, and not two."
        );
    }
}
