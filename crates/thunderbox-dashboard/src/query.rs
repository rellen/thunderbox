//! The code that runs the queries of a dashboard.
//!
//! The module runs the query of each panel, and it also runs the query that
//! holds the choices of a variable. Both kinds of query reach one data source
//! through the same trait, and the module bounds the wait of both.
//!
//! The module runs the queries of one set at the same time. [`run_together`]
//! holds that rule, and both [`run_all`] and [`load_choices`] call it.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use thunderbox_core::{Frame, Query, QueryRequest, Target, TimeRange, Timestamp, legend, variable};
use thunderbox_source::{DataSource, Registry};
use tokio::task::JoinSet;

use crate::grid::CollapsedRows;
use crate::snapshot::{PanelState, Snapshot};
use crate::variable::Choices;

/// The longest time that the runtime waits for a data source that states no
/// limit of its own.
///
/// The third duty of a data source is a limited time, and
/// [`thunderbox_source::SourceDescriptor::answer_limit`] states that limit. The
/// runtime reads this value only for a data source whose descriptor holds
/// `None`. A data source that states its own limit keeps that limit, whether
/// that limit is shorter or longer than this value.
///
/// A person waits this long only for a data source that answers no query at
/// all. The panel then holds the wait as its error, and the dashboard
/// refreshes again.
///
/// The runtime runs the panels of one set at the same time. A dashboard of
/// four panels, each on a data source that never answers, holds one set of
/// queries for this value. It does not hold that set for four times this
/// value.
pub const DEFAULT_ANSWER_LIMIT: Duration = Duration::from_secs(60);

/// The error that a panel holds when the task of its query ends with no
/// answer.
///
/// A person reads this text in the panel. Two faults end such a task with no
/// answer: a data source panicked, or the async runtime holds no timer of
/// tokio. Each panel holds a task of its own, so this error reaches one panel
/// alone.
const PANEL_NO_ANSWER: &str = "The query of this panel ended with no answer. A data source \
                               panicked, or the async runtime holds no timer.";

/// The text that a variable holds when the task of its query ends with no
/// answer.
///
/// A program shows this text beside the name of the variable. The two faults
/// that [`PANEL_NO_ANSWER`] names end the task of a variable as well.
const VARIABLE_NO_ANSWER: &str = "The query of this variable ended with no answer. A data source \
                                  panicked, or the async runtime holds no timer.";

/// The queries of one panel, and the number of points that the panel asks for.
///
/// A set of queries holds one of these values for each panel that it asks
/// about. [`crate::DashboardRuntime`] builds them, and it reads the width of
/// each panel for the point limit of that panel.
///
/// The two fields travel together, so no panel can take the point limit of
/// another panel.
pub(crate) struct PanelQueries {
    /// The targets of the panel, in the order that the dashboard states.
    pub targets: Vec<Target>,
    /// The largest number of points that each query of this panel returns.
    pub budget: u32,
}

/// What one set of queries does about one panel of the dashboard.
///
/// A set of queries holds one of these values for each panel, in the order of
/// the dashboard. [`run_all`] therefore writes the answer of a panel at the
/// position that the dashboard gives that panel. A panel that the runtime skips
/// moves no other panel.
///
/// This type names the skip, and a skipped panel holds no empty list of
/// targets. A panel that declares no target is a different thing, and
/// [`run_panel`] gives such a panel an answer that holds no frame.
pub(crate) enum PanelJob {
    /// The renderer draws this panel. The runtime asks the data source of the
    /// panel for these queries.
    Ask(PanelQueries),
    /// The renderer drops this panel, so no data source answers it. The value
    /// holds the state that the panel already had, and the set of queries
    /// writes that state back. A panel that answered therefore keeps its
    /// answer, and it draws that answer in the first frame after the terminal
    /// grows.
    Skip(PanelState),
}

/// Runs the queries of every panel and returns one snapshot.
///
/// The function runs the panels at the same time. [`run_together`] holds one
/// task for each panel that the caller asks about. A dashboard that reads a
/// server on another machine therefore waits for the slowest panel. It does not
/// wait for the sum of its panels.
///
/// The answers keep the order of `panels`. [`Snapshot::panel`] reads a panel
/// at the position that the dashboard gives it, so that order matters. The
/// snapshot holds one state for each element of `panels`, and a
/// [`PanelJob::Skip`] therefore moves no other panel.
///
/// A [`PanelJob::Skip`] names a panel that the renderer drops. The function
/// starts no task for such a panel. The snapshot holds the state that the
/// [`PanelJob::Skip`] carries, so a panel that answered keeps its answer.
///
/// Each panel carries a point limit of its own, and the queries of that panel
/// take that limit. A narrow panel therefore asks a data source for fewer
/// points than a wide panel asks for.
///
/// A query that returns an error becomes the state of that panel alone. The
/// function never stops early, so one data source that does not answer cannot
/// hide the answers of the others. The limit of [`run_panel`] holds that rule
/// for a data source that never answers, so one such data source stops no
/// other panel.
///
/// A panel whose task ends with no answer takes [`PANEL_NO_ANSWER`] as its
/// error. A data source that panics ends the task of its own panel, and every
/// other panel keeps its answer.
///
/// The function holds no limit of its own. One set of queries can therefore
/// take the longest of the limits of its panels. A dashboard of four panels,
/// each on a data source that never answers, holds one set of queries for
/// [`DEFAULT_ANSWER_LIMIT`]. A limit on the whole set would throw away every
/// panel that had already answered. It would also cut a dashboard whose panels
/// answer slowly and correctly.
///
/// The caller gives the present moment as `now`. Each data source reads that
/// moment, and the snapshot keeps it as `updated_at`. The snapshot holds
/// `false` in `refreshing`, because no query of this set still runs when the
/// function returns. A panel that meets an error stops at that target, and it
/// does not run the targets after it.
///
/// The caller gives the value of each variable as `values`. Every query of the
/// set reads the same values, and the snapshot keeps them beside the answers.
/// A reader of the snapshot therefore learns which values the panels used.
///
/// # Panics
///
/// The function panics when no async runtime of tokio runs on the thread of
/// the caller. It spawns one task for each panel, and [`JoinSet::spawn`] needs
/// that runtime. [`crate::DashboardRuntime`] runs this function inside a task
/// of the `Handle` that its caller supplied, so a dashboard always holds one.
pub async fn run_all(
    sources: &Registry,
    panels: &[PanelJob],
    range: &TimeRange,
    now: Timestamp,
    values: &BTreeMap<String, String>,
) -> Snapshot {
    // A panel that the renderer drops keeps the state that its job carries.
    // Every other panel starts in the state that has no answer, and a task of
    // this set replaces it.
    let mut answers: Vec<PanelState> = panels
        .iter()
        .map(|job| match job {
            PanelJob::Skip(held) => held.clone(),
            PanelJob::Ask(_) => PanelState::Loading,
        })
        .collect();
    let mut asked: Vec<usize> = Vec::new();
    let mut jobs = Vec::new();
    for (index, job) in panels.iter().enumerate() {
        let PanelJob::Ask(one_panel) = job else {
            continue;
        };
        // A task holds no borrow, so each job takes a copy of what it reads. A
        // copy of the registry copies no data source, because the registry
        // holds each one behind an `Arc`.
        let sources = sources.clone();
        let targets = one_panel.targets.clone();
        let budget = one_panel.budget;
        let range = range.clone();
        let values = values.clone();
        asked.push(index);
        jobs.push(async move { run_panel(&sources, &targets, &range, budget, now, &values).await });
    }
    for (index, answer) in asked.into_iter().zip(run_together(jobs).await) {
        answers[index] = answer.unwrap_or_else(|| PanelState::Error(PANEL_NO_ANSWER.to_string()));
    }
    Snapshot {
        panels: answers,
        range: range.clone(),
        variables: values.clone(),
        updated_at: Some(now),
        refreshing: false,
        // The runtime owns the row panels that are collapsed. `publish` writes
        // the state that the channel holds into this snapshot, so a set of
        // queries never carries a state of its own.
        collapsed: CollapsedRows::default(),
    }
}

/// Runs every job at the same time, and returns the answers in the order of
/// the jobs.
///
/// The function spawns one task for each job, and it returns when every one of
/// those tasks has ended. A job whose task ends with no answer takes `None`,
/// and every other job keeps its own answer. A panic inside one job therefore
/// stops no other job.
///
/// The function gives each task the position of its job, and it writes the
/// answer of that task at that position. A task that answers first therefore
/// moves no other answer.
///
/// The function holds the tasks in a [`JoinSet`]. The drop of that set stops
/// each task that still runs. A caller that drops this future therefore stops
/// every task that this function spawned. [`crate::DashboardRuntime`] stops the
/// task of a set of queries when a person refreshes the dashboard, and when the
/// dashboard drops. That stop reaches the task of each panel through this rule.
///
/// # Panics
///
/// The function panics when no async runtime of tokio runs on the thread of
/// the caller, because [`JoinSet::spawn`] needs that runtime.
async fn run_together<F>(jobs: Vec<F>) -> Vec<Option<F::Output>>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let mut answers: Vec<Option<F::Output>> = (0..jobs.len()).map(|_| None).collect();
    let mut set = JoinSet::new();
    for (index, job) in jobs.into_iter().enumerate() {
        set.spawn(async move { (index, job.await) });
    }
    while let Some(ended) = set.join_next().await {
        // A task that panicked returns an error here. The answer of that job
        // stays `None`, and the caller writes the text that a person reads.
        if let Ok((index, answer)) = ended {
            answers[index] = Some(answer);
        }
    }
    answers
}

/// Runs every query of one panel and returns the state of that panel.
///
/// A panel that holds several queries joins the frames of all of them. One
/// query that returns an error makes the whole panel show that error. A chart
/// that hides a series which is not there would mislead a reader.
///
/// The function takes a [`Target`] and not a [`thunderbox_core::Query`], because
/// a target holds both the query and the pattern. This function is the
/// one place that sees both a target and the answer of that target. It calls
/// [`legend::apply`] on that answer alone, so a pattern never reaches the
/// frames of another target.
///
/// The function holds each query to a limit. [`answer_limit`] states that
/// limit. A data source that passes the limit gives its panel an error that
/// names the data source and the wait. The task of the set of queries then
/// always ends, and [`crate::DashboardRuntime::tick`] starts the next set.
///
/// The limit reads the timer of tokio. The caller must therefore supply a
/// `Handle` of a runtime that holds that timer, and
/// [`crate::DashboardRuntime::new`] states that duty. The limit also stops a
/// query at an await point alone. A data source that holds a thread of the
/// async runtime, and that awaits nothing, therefore runs to its end. The third
/// duty of a data source covers that condition, and ADR 0012 in `docs/contributor/adr`
/// states it.
///
/// The function puts the value of each variable into the text of the query.
/// [`Target::to_query`] holds that step. A target that refers to a variable
/// which holds no value gives the panel an error that names that variable. The
/// panel would otherwise ask the data source about the name itself, and the
/// answer of that question means nothing.
async fn run_panel(
    sources: &Registry,
    targets: &[Target],
    range: &TimeRange,
    budget: u32,
    now: Timestamp,
    values: &BTreeMap<String, String>,
) -> PanelState {
    if targets.is_empty() {
        return PanelState::Ready(Arc::new(Vec::new()));
    }
    let mut frames: Vec<Frame> = Vec::new();
    for target in targets {
        if let Some(name) = missing_value(&target.expr, values) {
            return PanelState::Error(format!("The variable {name:?} holds no value."));
        }
        let source = match sources.get(&target.source) {
            Ok(source) => source.clone(),
            Err(error) => return PanelState::Error(error.to_string()),
        };
        let request = QueryRequest::new(target.to_query(values), range, now, budget);
        let limit = answer_limit(source.as_ref());
        let Ok(answer) = tokio::time::timeout(limit, source.query(request)).await else {
            return PanelState::Error(format!(
                "The data source {:?} did not answer in {}.",
                target.source,
                wait_text(limit)
            ));
        };
        match answer {
            Ok(mut answer) => {
                if let Some(error) = uneven_frame(&target.source, &answer) {
                    return PanelState::Error(error);
                }
                legend::apply(target.legend.as_deref(), &mut answer);
                frames.append(&mut answer);
            }
            Err(error) => return PanelState::Error(error.to_string()),
        }
    }
    PanelState::Ready(Arc::new(frames))
}

/// Returns the error for an answer that holds a frame of two lengths.
///
/// The fifth duty of a data source asks that every column of one frame hold
/// the same number of rows. The crate comment of `thunderbox-source` states
/// that duty. A panel reads a frame of two lengths in its own way, so the
/// function returns an error and no panel draws such a frame.
///
/// The function reads every frame of `answer`, and every column of each frame.
/// It reads the time column, each column of numbers, each column of text, and
/// each column of true and false values. A frame of zero rows passes, and so
/// does a frame with no column. The error names `source`, the length of the
/// shortest column, and the length of the longest column. ADR 0081 in
/// `docs/contributor/adr` states the reason.
fn uneven_frame(source: &str, answer: &[Frame]) -> Option<String> {
    answer.iter().find_map(|frame| {
        let lengths = frame.fields.iter().map(|field| field.len());
        let shortest = lengths.clone().min()?;
        let longest = lengths.max()?;
        (shortest != longest).then(|| {
            format!(
                "The data source {source:?} answered a frame with a column of {} and a \
                 column of {}. Each column of one frame must hold the same number of rows.",
                plural(shortest as u128, "row"),
                plural(longest as u128, "row"),
            )
        })
    })
}

/// Returns the choices of each variable that states a query.
///
/// The function asks the data source of every variable at the same time, as
/// [`run_all`] asks the data source of every panel. It returns one answer for
/// each entry of `asks`, and it keeps that order. A variable whose data source
/// returns an error takes [`Choices::Failed`], and every other variable keeps
/// its own answer.
///
/// The function bounds the wait of each query, as [`run_panel`] does. A data
/// source that never answers therefore ends its own query at that bound, and
/// every other variable still takes its choices. A variable whose task ends
/// with no answer takes [`VARIABLE_NO_ANSWER`].
///
/// # Panics
///
/// The function panics when no async runtime of tokio runs on the thread of
/// the caller. [`run_together`] states the reason.
pub(crate) async fn load_choices(
    sources: &Registry,
    asks: Vec<(String, Query)>,
) -> Vec<(String, Choices)> {
    let mut names = Vec::with_capacity(asks.len());
    let mut jobs = Vec::with_capacity(asks.len());
    for (name, query) in asks {
        names.push(name);
        // A task holds no borrow. The job therefore takes a copy of the
        // registry, as a job of `run_all` does.
        let sources = sources.clone();
        jobs.push(async move { one_variable(&sources, &query).await });
    }
    names
        .into_iter()
        .zip(run_together(jobs).await)
        .map(|(name, answer)| {
            let answer = answer.unwrap_or_else(|| Choices::Failed(VARIABLE_NO_ANSWER.to_string()));
            (name, answer)
        })
        .collect()
}

/// Returns the choices of one variable.
///
/// The function returns [`Choices::Failed`] for a data source that does not
/// support the operation. [`thunderbox_source::SourceDescriptor`] states that
/// support. An empty list from a data source that supports the operation is
/// not an error, and the variable then holds no choice.
async fn one_variable(sources: &Registry, query: &Query) -> Choices {
    let source = match sources.get(&query.source) {
        Ok(source) => source.clone(),
        Err(error) => return Choices::Failed(error.to_string()),
    };
    let descriptor = source.descriptor();
    if !descriptor.supports_label_values {
        return Choices::Failed(format!(
            "The data source {:?} returns no choice for a variable.",
            query.source
        ));
    }
    let limit = answer_limit(source.as_ref());
    let Ok(answer) = tokio::time::timeout(limit, source.label_values(&query.expr)).await else {
        return Choices::Failed(format!(
            "The data source {:?} did not answer in {}.",
            query.source,
            wait_text(limit)
        ));
    };
    match answer {
        Ok(choices) => Choices::Found(choices),
        Err(error) => Choices::Failed(error.to_string()),
    }
}

/// Returns the name of the first variable that the text refers to and that
/// holds no value.
///
/// The function returns `None` when every reference of the text names a value.
/// [`thunderbox_core::Dashboard::validate`] rejects a target that names a
/// variable which the dashboard does not declare. A variable that holds no
/// value is therefore the one condition that reaches this function.
fn missing_value<'a>(expr: &'a str, values: &BTreeMap<String, String>) -> Option<&'a str> {
    variable::references(expr)
        .into_iter()
        .find(|name| !values.contains_key(*name))
}

/// Returns the longest time that the runtime waits for one query.
///
/// A data source states that time in its descriptor. The function returns
/// [`DEFAULT_ANSWER_LIMIT`] for a data source that states none.
fn answer_limit(source: &dyn DataSource) -> Duration {
    source
        .descriptor()
        .answer_limit
        .unwrap_or(DEFAULT_ANSWER_LIMIT)
}

/// Returns the text of a wait, for the error of a panel.
///
/// The function writes seconds for a wait of a whole number of seconds. It
/// writes milliseconds for every other wait. A data source can state a limit of
/// 1500 milliseconds, and no whole number of seconds names that wait.
fn wait_text(wait: Duration) -> String {
    let millis = wait.as_millis();
    if millis > 0 && millis.is_multiple_of(1000) {
        return plural(millis / 1000, "second");
    }
    plural(millis, "millisecond")
}

/// Returns a count and the name of its unit, in the singular or the plural.
fn plural(count: u128, unit: &str) -> String {
    if count == 1 {
        format!("{count} {unit}")
    } else {
        format!("{count} {unit}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use thunderbox_core::{Field, RelativeTime, Result, Values};
    use thunderbox_source::{DataSource, SourceDescriptor};
    use tokio::sync::Barrier;

    /// A data source that names one column after the text of the query.
    ///
    /// The data source attaches the label `instance` when `labelled` is
    /// `true`. It attaches no label when that field is `false`. The sysinfo
    /// data source answers in that second form for a signal that returns one
    /// series, and the signal `cpu` is an example.
    struct Echo {
        labelled: bool,
    }

    #[async_trait]
    impl DataSource for Echo {
        fn descriptor(&self) -> SourceDescriptor {
            SourceDescriptor::new("echo", "Echo")
        }

        async fn query(&self, req: QueryRequest) -> Result<Vec<Frame>> {
            let mut column = Field::f64(req.query.expr.clone(), vec![Some(1.0), Some(2.0)]);
            if self.labelled {
                column = column.with_label("instance", "web-1");
            }
            Ok(vec![Frame::named(
                req.query.expr,
                vec![Field::time("time", vec![0, 1000]), column],
            )])
        }
    }

    /// A data source that names one column after the point limit of the query.
    ///
    /// A test reads that name back, and it then knows the number of points that
    /// the panel of the query asked for.
    struct Counts;

    #[async_trait]
    impl DataSource for Counts {
        fn descriptor(&self) -> SourceDescriptor {
            SourceDescriptor::new("counts", "Counts")
        }

        async fn query(&self, req: QueryRequest) -> Result<Vec<Frame>> {
            let name = req.max_data_points.to_string();
            Ok(vec![Frame::named(
                name.clone(),
                vec![
                    Field::time("time", vec![0, 1000]),
                    Field::f64(name, vec![Some(1.0), Some(2.0)]),
                ],
            )])
        }
    }

    /// Returns the job of each panel, and gives every panel one point limit.
    ///
    /// The runtime asks about every panel of these tests. A test that reads
    /// the point limit of one panel builds its own values.
    fn asks(panels: Vec<Vec<Target>>, budget: u32) -> Vec<PanelJob> {
        panels
            .into_iter()
            .map(|targets| PanelJob::Ask(PanelQueries { targets, budget }))
            .collect()
    }

    /// Each panel holds a width of its own, so each panel holds a point limit
    /// of its own. The queries of one panel must carry the limit of that panel,
    /// and never the limit of another panel.
    #[tokio::test]
    async fn run_all_gives_the_queries_of_a_panel_the_point_limit_of_that_panel() {
        let sources = Registry::new().with("s", Arc::new(Counts));
        let panels = vec![
            PanelJob::Ask(PanelQueries {
                targets: vec![target("wide", None)],
                budget: 146,
            }),
            PanelJob::Ask(PanelQueries {
                targets: vec![target("narrow", None)],
                budget: 74,
            }),
        ];
        let snapshot = run_all(&sources, &panels, &range(), 1_000_000, &no_values()).await;
        assert_eq!(
            first_name(&snapshot, 0),
            "146",
            "The wide panel must ask for the points of its own width."
        );
        assert_eq!(
            first_name(&snapshot, 1),
            "74",
            "The narrow panel must ask for the points of its own width."
        );
    }

    /// A panel that the renderer drops holds no query, and it must move no
    /// other panel. [`Snapshot::panel`] reads a panel at the position that the
    /// dashboard gives it. A set of queries that held no entry for the skipped
    /// panel would give panel 1 the answer of panel 2. The last panel would
    /// then hold nothing.
    #[tokio::test]
    async fn a_panel_that_the_renderer_drops_moves_no_other_panel() {
        let sources = Registry::new().with("s", Arc::new(Echo { labelled: false }));
        let panels = vec![
            PanelJob::Ask(PanelQueries {
                targets: vec![target("first", None)],
                budget: 10,
            }),
            PanelJob::Skip(PanelState::Loading),
            PanelJob::Ask(PanelQueries {
                targets: vec![target("third", None)],
                budget: 10,
            }),
        ];
        let snapshot = run_all(&sources, &panels, &range(), 1_000_000, &no_values()).await;
        assert_eq!(
            snapshot.panels.len(),
            3,
            "The snapshot must hold one state for each panel."
        );
        assert_eq!(
            first_name(&snapshot, 0),
            "first",
            "The first panel must hold its own answer."
        );
        assert!(
            snapshot.panel(1).is_loading(),
            "A panel that the renderer drops must hold the state that its job \
             carries. {:?}",
            snapshot.panel(1)
        );
        assert_eq!(
            first_name(&snapshot, 2),
            "third",
            "The third panel must hold its own answer."
        );
    }

    /// A data source that never answers, and that states a short limit.
    ///
    /// A test that drives this data source waits for the limit, and not for a
    /// period that the test itself chose. The limit is short, so the test is
    /// short.
    struct Silent;

    /// The limit that the data source above states.
    const SILENT_LIMIT: Duration = Duration::from_millis(20);

    #[async_trait]
    impl DataSource for Silent {
        fn descriptor(&self) -> SourceDescriptor {
            SourceDescriptor::new("silent", "Silent")
                .with_answer_limit(SILENT_LIMIT)
                .with_label_values()
        }

        async fn query(&self, _req: QueryRequest) -> Result<Vec<Frame>> {
            tokio::time::sleep(Duration::from_secs(3600)).await;
            Ok(Vec::new())
        }

        async fn label_values(&self, _query: &str) -> Result<Vec<String>> {
            tokio::time::sleep(Duration::from_secs(3600)).await;
            Ok(Vec::new())
        }
    }

    /// A data source that answers no query until three queries have arrived.
    ///
    /// The text of a query is a number of milliseconds. The data source waits
    /// at a barrier until three queries have arrived, and it then waits for
    /// that number. A barrier is a point at which several tasks wait for each
    /// other.
    ///
    /// A test learns two things from this data source. Three queries that ran
    /// one after another would never pass the barrier. The query of the longest
    /// wait also answers last, so the caller must write each answer at the
    /// position of its own query.
    struct Together {
        /// The barrier that holds each query until three have arrived.
        gate: Arc<Barrier>,
    }

    impl Together {
        /// Waits at the barrier, and then waits for the period that the text
        /// names.
        async fn wait(&self, expr: &str) {
            self.gate.wait().await;
            let millis: u64 = expr
                .parse()
                .expect("The test writes a number of milliseconds.");
            tokio::time::sleep(Duration::from_millis(millis)).await;
        }
    }

    #[async_trait]
    impl DataSource for Together {
        fn descriptor(&self) -> SourceDescriptor {
            SourceDescriptor::new("together", "Together").with_label_values()
        }

        async fn query(&self, req: QueryRequest) -> Result<Vec<Frame>> {
            self.wait(&req.query.expr).await;
            Ok(vec![Frame::named(
                req.query.expr.clone(),
                vec![
                    Field::time("time", vec![0, 1000]),
                    Field::f64(req.query.expr, vec![Some(1.0), Some(2.0)]),
                ],
            )])
        }

        async fn label_values(&self, query: &str) -> Result<Vec<String>> {
            self.wait(query).await;
            Ok(vec![query.to_string()])
        }
    }

    /// The three periods that a test of [`Together`] gives its three queries.
    ///
    /// The first period is the longest one, so the first query answers last.
    const BACKWARDS: [&str; 3] = ["30", "20", "10"];

    /// Returns a registry that holds one [`Together`] under the name `s`.
    fn together() -> Registry {
        Registry::new().with(
            "s",
            Arc::new(Together {
                gate: Arc::new(Barrier::new(BACKWARDS.len())),
            }),
        )
    }

    /// The longest time that a test of [`Together`] waits.
    ///
    /// Three queries that run at the same time answer inside this time. Three
    /// queries that run one after another never answer at all, because the
    /// third one never reaches the barrier. The test then fails at this limit,
    /// and it does not run for ever.
    const TOGETHER_LIMIT: Duration = Duration::from_secs(5);

    /// A data source that panics on every query.
    ///
    /// A test that runs this data source writes the report of a panic on the
    /// output of the test. That report belongs to the test, and it is not a
    /// fault.
    struct Panics;

    #[async_trait]
    impl DataSource for Panics {
        fn descriptor(&self) -> SourceDescriptor {
            SourceDescriptor::new("panics", "Panics").with_label_values()
        }

        async fn query(&self, _req: QueryRequest) -> Result<Vec<Frame>> {
            panic!("This data source panics on every query.");
        }

        async fn label_values(&self, _query: &str) -> Result<Vec<String>> {
            panic!("This data source panics on every query.");
        }
    }

    /// A data source that holds the choices of a variable.
    struct Choosy;

    #[async_trait]
    impl DataSource for Choosy {
        fn descriptor(&self) -> SourceDescriptor {
            SourceDescriptor::new("choosy", "Choosy").with_label_values()
        }

        async fn query(&self, _req: QueryRequest) -> Result<Vec<Frame>> {
            Ok(Vec::new())
        }

        async fn label_values(&self, _query: &str) -> Result<Vec<String>> {
            Ok(vec!["web".into(), "db".into()])
        }
    }

    /// Returns a registry that holds one data source under the name `s`.
    fn registry(labelled: bool) -> Registry {
        Registry::new().with("s", Arc::new(Echo { labelled }))
    }

    /// Returns a target of the data source `s`.
    fn target(expr: &str, pattern: Option<&str>) -> Target {
        Target {
            source: "s".into(),
            expr: expr.into(),
            legend: pattern.map(str::to_string),
        }
    }

    /// Returns the range that every test in this module uses.
    fn range() -> TimeRange {
        TimeRange {
            from: RelativeTime::Offset(-1000),
            to: RelativeTime::Now,
        }
    }

    /// Returns the values that a test which reads no variable passes.
    fn no_values() -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    /// Returns the name that each column of numbers shows.
    async fn names(sources: &Registry, targets: &[Target]) -> Vec<String> {
        let state = run_panel(sources, targets, &range(), 10, 1_000_000, &no_values()).await;
        let PanelState::Ready(frames) = state else {
            panic!("The data source must answer.");
        };
        frames
            .iter()
            .flat_map(|f| f.numeric_fields())
            .map(Field::display_name)
            .collect()
    }

    #[tokio::test]
    async fn a_pattern_reads_a_label_that_the_series_holds() {
        let out = names(&registry(true), &[target("up", Some("{{instance}}"))]).await;
        assert_eq!(out, vec!["web-1"]);
    }

    #[tokio::test]
    async fn a_pattern_that_names_a_label_which_is_not_there_keeps_the_old_name() {
        let out = names(&registry(true), &[target("up", Some("{{nope}}"))]).await;
        assert_eq!(
            out,
            vec![r#"up{instance="web-1"}"#],
            "A panel must still show a name for the series."
        );
    }

    #[tokio::test]
    async fn a_pattern_reads_the_name_of_a_series_that_holds_no_label() {
        let out = names(&registry(false), &[target("0", Some("core {{__name__}}"))]).await;
        assert_eq!(
            out,
            vec!["core 0"],
            "A data source that attaches no label must still obey a pattern."
        );
    }

    #[tokio::test]
    async fn a_target_that_states_no_pattern_keeps_every_name() {
        let out = names(&registry(true), &[target("up", None)]).await;
        assert_eq!(out, vec![r#"up{instance="web-1"}"#]);
    }

    #[tokio::test]
    async fn each_target_of_a_panel_reads_its_own_pattern() {
        let targets = [
            target("rx", Some("received on {{instance}}")),
            target("tx", Some("sent by {{__name__}}")),
        ];
        let out = names(&registry(true), &targets).await;
        assert_eq!(
            out,
            vec!["received on web-1", "sent by tx"],
            "A pattern must reach the frames of its own query alone."
        );
    }

    /// The third duty of a data source is a limited time. A data source that
    /// does not keep that duty must not hold the runtime. The panel therefore
    /// holds an error that names the data source and the wait.
    #[tokio::test]
    async fn a_data_source_that_never_answers_gives_its_panel_an_error() {
        let sources = Registry::new().with("s", Arc::new(Silent));
        let state = run_panel(
            &sources,
            &[target("up", None)],
            &range(),
            10,
            1_000_000,
            &no_values(),
        )
        .await;
        let error = state
            .error()
            .expect("The panel must hold the error of the wait.");
        assert!(
            error.contains("\"s\""),
            "The message must name the data source. {error}"
        );
        assert!(
            error.contains("20 milliseconds"),
            "The message must state the wait. {error}"
        );
    }

    /// ADR 0015 in `docs/contributor/adr` states this rule. A slow query changes one panel,
    /// and never the whole picture.
    #[tokio::test]
    async fn a_data_source_that_never_answers_stops_no_other_panel() {
        let sources = Registry::new()
            .with("s", Arc::new(Echo { labelled: false }))
            .with("silent", Arc::new(Silent));
        let silent = Target {
            source: "silent".into(),
            expr: "up".into(),
            legend: None,
        };
        let panels = vec![vec![silent], vec![target("up", None)]];
        let snapshot = run_all(
            &sources,
            &asks(panels, 10),
            &range(),
            1_000_000,
            &no_values(),
        )
        .await;
        assert!(
            snapshot.panel(0).error().is_some(),
            "The panel of the data source that never answers must hold an error."
        );
        assert!(
            matches!(snapshot.panel(1), PanelState::Ready(_)),
            "The other panel must hold its answer. {:?}",
            snapshot.panel(1)
        );
    }

    /// Returns the name of the first column of numbers that a panel holds.
    fn first_name(snapshot: &Snapshot, panel: usize) -> String {
        let PanelState::Ready(frames) = snapshot.panel(panel) else {
            panic!("The panel must hold an answer. {:?}", snapshot.panel(panel));
        };
        frames
            .iter()
            .flat_map(|f| f.numeric_fields())
            .map(Field::display_name)
            .next()
            .expect("The answer must hold a column of numbers.")
    }

    /// Returns one panel for each period of [`BACKWARDS`].
    fn backwards_panels() -> Vec<Vec<Target>> {
        BACKWARDS
            .iter()
            .map(|expr| vec![target(expr, None)])
            .collect()
    }

    /// Runs three panels of [`Together`], and returns the snapshot.
    async fn run_backwards() -> Snapshot {
        let panels = backwards_panels();
        tokio::time::timeout(
            TOGETHER_LIMIT,
            run_all(
                &together(),
                &asks(panels, 10),
                &range(),
                1_000_000,
                &no_values(),
            ),
        )
        .await
        .expect("Three panels that run at the same time must all answer.")
    }

    /// ADR 0045 in `docs/contributor/adr` states this rule. The data source of this test
    /// answers no query until three queries have arrived. A set that
    /// ran its panels one after another would therefore never end.
    #[tokio::test]
    async fn run_all_runs_the_panels_of_one_set_at_the_same_time() {
        let snapshot = run_backwards().await;
        assert_eq!(
            snapshot.error_count(),
            0,
            "Every panel of the set must hold an answer. {snapshot:?}"
        );
    }

    /// `Snapshot::panel` reads a panel at the position that the dashboard gives
    /// it. The panels answer in the reverse order here, so this test fails for
    /// a function that returns the answers in the order of their arrival.
    #[tokio::test]
    async fn run_all_keeps_the_order_of_the_panels() {
        let snapshot = run_backwards().await;
        let names: Vec<String> = (0..BACKWARDS.len())
            .map(|panel| first_name(&snapshot, panel))
            .collect();
        assert_eq!(
            names, BACKWARDS,
            "Each answer must reach the panel of its own query."
        );
    }

    /// The variables follow the rule of the panels. The data source of this
    /// test answers no query until three queries have arrived, and it answers
    /// them in the reverse order.
    #[tokio::test]
    async fn load_choices_asks_every_variable_at_the_same_time() {
        let asks: Vec<(String, Query)> = BACKWARDS
            .iter()
            .map(|expr| ((*expr).to_string(), query_of("s", expr)))
            .collect();
        let out = tokio::time::timeout(TOGETHER_LIMIT, load_choices(&together(), asks))
            .await
            .expect("Three variables that run at the same time must all answer.");
        let want: Vec<(String, Choices)> = BACKWARDS
            .iter()
            .map(|expr| ((*expr).to_string(), Choices::Found(vec![(*expr).into()])))
            .collect();
        assert_eq!(
            out, want,
            "Each answer must reach the variable of its own query."
        );
    }

    /// Each panel runs in a task of its own. A data source that panics
    /// therefore ends the task of its own panel, and every other panel keeps
    /// its answer.
    ///
    /// This test writes the report of a panic on the output of the test. That
    /// report belongs to the test, and it is not a fault.
    #[tokio::test]
    async fn a_data_source_that_panics_gives_its_panel_an_error() {
        let sources = Registry::new()
            .with("s", Arc::new(Echo { labelled: false }))
            .with("panics", Arc::new(Panics));
        let panics = Target {
            source: "panics".into(),
            expr: "up".into(),
            legend: None,
        };
        let panels = vec![vec![panics], vec![target("up", None)]];
        let snapshot = run_all(
            &sources,
            &asks(panels, 10),
            &range(),
            1_000_000,
            &no_values(),
        )
        .await;
        assert_eq!(
            snapshot.panel(0).error(),
            Some(PANEL_NO_ANSWER),
            "The panel of the data source that panics must hold an error."
        );
        assert!(
            matches!(snapshot.panel(1), PanelState::Ready(_)),
            "The other panel must keep its answer. {:?}",
            snapshot.panel(1)
        );
    }

    /// Each variable runs in a task of its own, as each panel does. A data
    /// source that panics therefore leaves every other variable with its
    /// choices.
    ///
    /// This test writes the report of a panic on the output of the test. That
    /// report belongs to the test, and it is not a fault.
    #[tokio::test]
    async fn a_data_source_that_panics_gives_its_variable_no_choice() {
        let sources = Registry::new()
            .with("panics", Arc::new(Panics))
            .with("choosy", Arc::new(Choosy));
        let asks = vec![
            ("job".to_string(), query_of("panics", "job")),
            ("region".to_string(), query_of("choosy", "region")),
        ];
        let out = load_choices(&sources, asks).await;
        assert_eq!(
            out[0].1,
            Choices::Failed(VARIABLE_NO_ANSWER.to_string()),
            "The variable of the data source that panics must hold an error."
        );
        assert_eq!(
            out[1].1,
            Choices::Found(vec!["web".into(), "db".into()]),
            "The other variable must keep its choices."
        );
    }

    #[test]
    fn a_data_source_that_states_no_limit_takes_the_limit_of_the_runtime() {
        assert_eq!(
            answer_limit(&Echo { labelled: false }),
            DEFAULT_ANSWER_LIMIT,
            "The runtime must hold a data source that states no limit to a limit of its own."
        );
    }

    #[test]
    fn a_data_source_that_states_a_limit_keeps_that_limit() {
        assert_eq!(
            answer_limit(&Silent),
            SILENT_LIMIT,
            "The runtime must take the limit that the data source states."
        );
    }

    #[test]
    fn the_text_of_a_wait_names_the_unit_of_the_number() {
        assert_eq!(wait_text(Duration::from_secs(30)), "30 seconds");
        assert_eq!(wait_text(Duration::from_secs(1)), "1 second");
        assert_eq!(wait_text(Duration::from_millis(20)), "20 milliseconds");
        assert_eq!(wait_text(Duration::from_millis(1)), "1 millisecond");
        assert_eq!(wait_text(Duration::from_millis(1500)), "1500 milliseconds");
    }

    /// Returns the values of one variable.
    fn one_value(name: &str, value: &str) -> BTreeMap<String, String> {
        let mut values = BTreeMap::new();
        values.insert(name.to_string(), value.to_string());
        values
    }

    /// The data source names each column after the text of the query that it
    /// received. This test therefore reads back the text that reached that
    /// data source.
    #[tokio::test]
    async fn the_value_of_a_variable_reaches_the_data_source() {
        let state = run_panel(
            &registry(false),
            &[target(r#"up{job="$job"}"#, None)],
            &range(),
            10,
            1_000_000,
            &one_value("job", "web"),
        )
        .await;
        let PanelState::Ready(frames) = state else {
            panic!("The data source must answer. {state:?}");
        };
        assert_eq!(
            frames[0].name.as_deref(),
            Some(r#"up{job="web"}"#),
            "The name of the variable must never reach the data source."
        );
    }

    /// A variable that takes its choices from a data source holds no value
    /// until that data source answers. The panel must then name the variable.
    /// A query that held the name would ask a question that means nothing.
    #[tokio::test]
    async fn a_target_that_names_a_variable_which_holds_no_value_gives_the_panel_an_error() {
        let state = run_panel(
            &registry(false),
            &[target(r#"up{job="$job"}"#, None)],
            &range(),
            10,
            1_000_000,
            &no_values(),
        )
        .await;
        let error = state.error().expect("The panel must hold an error.");
        assert_eq!(error, "The variable \"job\" holds no value.");
    }

    #[tokio::test]
    async fn a_snapshot_holds_the_value_that_each_panel_of_it_used() {
        let panels = vec![vec![target("up", None)]];
        let values = one_value("job", "web");
        let snapshot = run_all(
            &sources_of_one(),
            &asks(panels, 10),
            &range(),
            1_000_000,
            &values,
        )
        .await;
        assert_eq!(snapshot.variables, values);
    }

    /// Returns a registry that holds the data source `s` alone.
    fn sources_of_one() -> Registry {
        registry(false)
    }

    #[tokio::test]
    async fn load_choices_returns_the_values_that_a_data_source_holds() {
        let sources = Registry::new().with("s", Arc::new(Choosy));
        let out = load_choices(&sources, vec![("job".into(), query_of("s", "job"))]).await;
        assert_eq!(
            out,
            vec![(
                "job".to_string(),
                Choices::Found(vec!["web".into(), "db".into()])
            )]
        );
    }

    /// A data source that does not support the operation returns an empty
    /// list, and an empty list is not an error. The runtime must therefore
    /// read the descriptor, so that a person learns the reason.
    #[tokio::test]
    async fn load_choices_names_a_data_source_that_holds_no_choice_for_a_variable() {
        let sources = Registry::new().with("s", Arc::new(Echo { labelled: false }));
        let out = load_choices(&sources, vec![("job".into(), query_of("s", "job"))]).await;
        let Choices::Failed(error) = &out[0].1 else {
            panic!("The data source must report that it holds no choice. {out:?}");
        };
        assert_eq!(
            error,
            "The data source \"s\" returns no choice for a variable."
        );
    }

    #[tokio::test]
    async fn load_choices_names_a_data_source_that_the_registry_does_not_hold() {
        let out = load_choices(&Registry::new(), vec![("job".into(), query_of("s", "job"))]).await;
        let Choices::Failed(error) = &out[0].1 else {
            panic!("The registry holds no such data source. {out:?}");
        };
        assert_eq!(error, "The registry does not hold the data source s.");
    }

    /// The third duty of a data source covers a variable as well as a panel.
    /// A data source that never answers must therefore hold no variable.
    #[tokio::test]
    async fn load_choices_bounds_the_wait_of_one_variable() {
        let sources = Registry::new().with("s", Arc::new(Silent));
        let out = load_choices(&sources, vec![("job".into(), query_of("s", "job"))]).await;
        let Choices::Failed(error) = &out[0].1 else {
            panic!("The data source never answers. {out:?}");
        };
        assert!(
            error.contains("20 milliseconds"),
            "The message must state the wait. {error}"
        );
    }

    /// Returns a query of one data source.
    fn query_of(source: &str, expr: &str) -> Query {
        Query {
            source: source.into(),
            expr: expr.into(),
        }
    }

    #[tokio::test]
    async fn a_panel_that_holds_no_target_returns_no_frame() {
        let state = run_panel(&registry(true), &[], &range(), 10, 1_000_000, &no_values()).await;
        let PanelState::Ready(frames) = state else {
            panic!("A panel that holds no target must hold no error.");
        };
        assert!(frames.is_empty());
    }

    /// A data source that gives the same frames to every query.
    struct Fixed(Vec<Frame>);

    #[async_trait]
    impl DataSource for Fixed {
        fn descriptor(&self) -> SourceDescriptor {
            SourceDescriptor::new("fixed", "Fixed")
        }

        async fn query(&self, _req: QueryRequest) -> Result<Vec<Frame>> {
            Ok(self.0.clone())
        }
    }

    /// Returns the state of a panel whose one target reads these frames.
    async fn state_of(frames: Vec<Frame>) -> PanelState {
        let sources = Registry::new().with("s", Arc::new(Fixed(frames)));
        run_panel(
            &sources,
            &[target("up", None)],
            &range(),
            10,
            1_000_000,
            &no_values(),
        )
        .await
    }

    /// Returns a column of these values, with no name.
    fn column(values: Values) -> Field {
        Field {
            values,
            ..Default::default()
        }
    }

    /// Returns a frame that holds a time column and a column of numbers.
    fn series(stamps: Vec<Timestamp>, numbers: Vec<Option<f64>>) -> Frame {
        Frame::new(vec![
            Field::time("time", stamps),
            Field::f64("usage", numbers),
        ])
    }

    #[tokio::test]
    async fn a_number_column_shorter_than_the_time_column_gives_the_panel_an_error() {
        let state = state_of(vec![series(
            vec![0, 1000, 2000],
            vec![Some(1.0), Some(2.0)],
        )])
        .await;
        let error = state
            .error()
            .expect("A frame of two lengths must give the panel an error.");
        assert!(
            error.contains("\"s\""),
            "The message must name the data source. {error}"
        );
        assert!(
            error.contains("2 rows") && error.contains("3 rows"),
            "The message must state the shortest and the longest column. {error}"
        );
        assert!(
            error.contains("Each column of one frame must hold the same number of rows."),
            "The message must state the rule. {error}"
        );
    }

    #[tokio::test]
    async fn a_column_of_text_or_of_true_and_false_values_of_another_length_gives_an_error() {
        let short = [
            column(Values::Str(vec![Some("a".into()), Some("b".into())])),
            column(Values::Bool(vec![Some(true), Some(false)])),
        ];
        for other in short {
            let mut frame = series(vec![0, 1000, 2000], vec![Some(1.0), Some(2.0), Some(3.0)]);
            frame.fields.push(other.clone());
            let state = state_of(vec![frame]).await;
            assert!(
                state.error().is_some(),
                "The check must read every column of the frame. {other:?}"
            );
        }
    }

    #[tokio::test]
    async fn an_answer_whose_second_frame_holds_two_lengths_gives_the_panel_an_error() {
        let state = state_of(vec![
            series(vec![0, 1000], vec![Some(1.0), Some(2.0)]),
            series(vec![0, 1000], vec![Some(1.0)]),
        ])
        .await;
        let error = state
            .error()
            .expect("The check must read every frame of the answer.");
        assert!(
            error.contains("1 row and") && error.contains("2 rows"),
            "The message must state the lengths of the second frame. {error}"
        );
    }

    #[tokio::test]
    async fn a_frame_of_one_length_a_frame_of_zero_rows_and_a_frame_with_no_column_pass() {
        let state = state_of(vec![
            series(vec![0, 1000], vec![Some(1.0), Some(2.0)]),
            series(vec![], vec![]),
            Frame::new(vec![]),
        ])
        .await;
        let PanelState::Ready(frames) = state else {
            panic!("Each frame keeps the fifth duty, so the panel holds the answer.");
        };
        assert_eq!(frames.len(), 3, "The panel must keep every frame.");
    }
}
