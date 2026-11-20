//! Tests for the variables, from the TOML text to the text of a query.
//!
//! `thunderbox_core::variable` holds the rule that puts a value into a query,
//! and `run_panel` is the one place that applies it. These tests drive a whole
//! [`DashboardRuntime`], so they cover every step between the TOML text and
//! the text that a data source reads.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use thunderbox_core::{Dashboard, Field, Frame, QueryRequest, Result};
use thunderbox_dashboard::{DashboardRuntime, Key, PanelState, variable_key};
use thunderbox_source::{DataSource, Registry, SourceDescriptor};
use thunderbox_source_testdata::TestData;

mod common;

use common::settle;

/// Waits until the runtime holds the choices of every variable.
///
/// The runtime asks the data source of each variable that states a query one
/// time, in a task of its own. `DashboardRuntime::tick` takes that answer, and
/// a program drives `tick` from its own event loop. This helper drives it in
/// the place of that program.
///
/// The helper returns when every variable holds a choice or an error. It
/// panics after five seconds, so a data source that never answers gives a test
/// that fails and not a test that never ends.
async fn take_the_choices(runtime: &mut DashboardRuntime) {
    for _ in 0..500 {
        runtime.tick();
        let ready = runtime
            .variables()
            .states()
            .iter()
            .all(|state| !state.choices().is_empty() || state.error().is_some());
        if ready {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("The data source did not answer with the choices of a variable.");
}

/// A data source that records the text of every query that it reads.
///
/// A test reads that record back, and it then knows exactly which text left
/// thunderbox. The data source returns one series, so a panel of it holds an
/// answer.
struct Recorder {
    /// The text of each query, in the order of the queries.
    queries: Mutex<Vec<String>>,
    /// The choices that this data source holds for a variable.
    choices: Vec<String>,
    /// `true` if the descriptor states that the data source holds choices.
    holds_choices: bool,
}

impl Recorder {
    /// Returns a data source that holds no choice for a variable.
    fn new() -> Self {
        Self {
            queries: Mutex::new(Vec::new()),
            choices: Vec::new(),
            holds_choices: false,
        }
    }

    /// Returns a data source that holds these choices for a variable.
    fn with_choices(choices: &[&str]) -> Self {
        Self {
            queries: Mutex::new(Vec::new()),
            choices: choices.iter().map(|c| (*c).to_string()).collect(),
            holds_choices: true,
        }
    }

    /// Returns the text of each query that this data source read.
    fn queries(&self) -> Vec<String> {
        self.queries
            .lock()
            .expect("No test holds this lock while it panics.")
            .clone()
    }
}

#[async_trait]
impl DataSource for Recorder {
    fn descriptor(&self) -> SourceDescriptor {
        let descriptor = SourceDescriptor::new("recorder", "Recorder");
        if self.holds_choices {
            descriptor.with_label_values()
        } else {
            descriptor
        }
    }

    async fn query(&self, req: QueryRequest) -> Result<Vec<Frame>> {
        self.queries
            .lock()
            .expect("No test holds this lock while it panics.")
            .push(req.query.expr.clone());
        Ok(vec![Frame::named(
            req.query.expr.clone(),
            vec![
                Field::time("time", vec![0, 1000]),
                Field::f64(req.query.expr, vec![Some(1.0), Some(2.0)]),
            ],
        )])
    }

    async fn label_values(&self, _query: &str) -> Result<Vec<String>> {
        Ok(self.choices.clone())
    }
}

/// Returns a runtime of this dashboard, over this one data source.
fn runtime(toml: &str, source: Arc<dyn DataSource>) -> DashboardRuntime {
    let dashboard = Dashboard::from_toml(toml).expect("The test dashboard must be correct.");
    let sources = Registry::new().with("rec", source);
    DashboardRuntime::new(dashboard, sources, tokio::runtime::Handle::current())
        .expect("The runtime must start.")
}

const FIXED_OPTIONS: &str = r#"
title = "variables"
[source.rec]
kind = "recorder"

[[variable]]
name = "job"
options = ["web", "db"]
current = "web"

[[panel]]
title = "up"
kind = "timeseries"
grid = { x = 0, y = 0, w = 24, h = 8 }
[[panel.target]]
source = "rec"
expr = "up{job=\"$job\"}"
"#;

#[tokio::test]
async fn the_value_that_a_variable_holds_reaches_the_data_source() {
    let recorder = Arc::new(Recorder::new());
    let dash = runtime(FIXED_OPTIONS, recorder.clone());
    settle(&dash).await;
    assert_eq!(
        recorder.queries(),
        vec![r#"up{job="web"}"#],
        "The name of the variable must never reach the data source."
    );
}

#[tokio::test]
async fn the_snapshot_names_the_value_that_each_panel_of_it_used() {
    let dash = runtime(FIXED_OPTIONS, Arc::new(Recorder::new()));
    settle(&dash).await;
    let snapshot = dash.snapshot();
    assert_eq!(snapshot.variables["job"], "web");
    assert_eq!(snapshot.variables.len(), 1);
}

#[tokio::test]
async fn a_key_takes_the_choice_that_follows_and_the_data_source_reads_it() {
    let recorder = Arc::new(Recorder::new());
    let mut dash = runtime(FIXED_OPTIONS, recorder.clone());
    settle(&dash).await;

    let asked =
        variable_key(Key::Char(']'), dash.variables()).expect("The key must ask for a choice.");
    dash.set_variables(asked);
    settle(&dash).await;

    assert_eq!(
        recorder.queries(),
        vec![r#"up{job="web"}"#, r#"up{job="db"}"#],
        "The key of a person must reach the data source."
    );
    assert_eq!(dash.snapshot().variables["job"], "db");
    assert_eq!(dash.variables().row_text(), "[job: db]");
}

/// The answers of the set of queries before describe another question. A
/// reader would misread them beside the name of the new value.
#[tokio::test]
async fn a_new_value_puts_every_panel_back_to_the_state_that_has_no_answer() {
    let mut dash = runtime(FIXED_OPTIONS, Arc::new(Recorder::new()));
    settle(&dash).await;
    assert!(matches!(dash.snapshot().panel(0), PanelState::Ready(_)));

    let asked =
        variable_key(Key::Char(']'), dash.variables()).expect("The key must ask for a choice.");
    dash.set_variables(asked);
    assert!(
        dash.snapshot().panel(0).is_loading(),
        "A panel must hold no answer of the question before."
    );
}

const TWO_VARIABLES: &str = r#"
title = "variables"
[source.rec]
kind = "recorder"

[[variable]]
name = "job"
options = ["web", "db"]

[[variable]]
name = "region"
options = ["eu", "us"]

[[panel]]
title = "up"
kind = "timeseries"
grid = { x = 0, y = 0, w = 24, h = 8 }
[[panel.target]]
source = "rec"
expr = "up{job=\"$job\",region=\"$region\"}"
"#;

/// A move of the cursor changes no query. The runtime must therefore ask the
/// data source for nothing.
#[tokio::test]
async fn a_key_that_moves_the_cursor_starts_no_set_of_queries() {
    let recorder = Arc::new(Recorder::new());
    let mut dash = runtime(TWO_VARIABLES, recorder.clone());
    settle(&dash).await;
    assert_eq!(recorder.queries().len(), 1);

    let asked =
        variable_key(Key::Char('v'), dash.variables()).expect("The key must move the cursor.");
    dash.set_variables(asked);
    settle(&dash).await;
    assert_eq!(
        recorder.queries().len(),
        1,
        "A move of the cursor asks a data source for nothing."
    );
    assert_eq!(dash.variables().cursor(), 1);

    let asked =
        variable_key(Key::Char(']'), dash.variables()).expect("The key must ask for a choice.");
    dash.set_variables(asked);
    settle(&dash).await;
    assert_eq!(
        recorder.queries()[1],
        r#"up{job="web",region="us"}"#,
        "The key must change the variable at the cursor alone."
    );
}

const FROM_A_QUERY: &str = r#"
title = "variables"
[source.rec]
kind = "recorder"

[[variable]]
name = "job"
query = { source = "rec", expr = "job" }

[[panel]]
title = "up"
kind = "timeseries"
grid = { x = 0, y = 0, w = 24, h = 8 }
[[panel.target]]
source = "rec"
expr = "up{job=\"$job\"}"
"#;

#[tokio::test]
async fn a_variable_takes_its_choices_from_a_data_source() {
    let recorder = Arc::new(Recorder::with_choices(&["web", "db"]));
    let mut dash = runtime(FROM_A_QUERY, recorder.clone());
    take_the_choices(&mut dash).await;
    settle(&dash).await;

    assert_eq!(dash.variables().states()[0].choices(), ["web", "db"]);
    assert_eq!(dash.variables().states()[0].value(), Some("web"));
    assert!(
        recorder.queries().contains(&r#"up{job="web"}"#.to_string()),
        "The choice of the data source must reach the query. {:?}",
        recorder.queries()
    );
}

/// A variable that takes its choices from a data source holds no value until
/// that data source answers. The panel must name the variable, because a query
/// that held the name would ask a question that means nothing.
#[tokio::test]
async fn a_panel_names_the_variable_that_holds_no_value() {
    let dash = runtime(FROM_A_QUERY, Arc::new(Recorder::with_choices(&[])));
    settle(&dash).await;
    assert_eq!(
        dash.snapshot().panel(0).error(),
        Some("The variable \"job\" holds no value.")
    );
}

/// A data source that does not support the operation returns an empty list,
/// and an empty list is not an error. A person must still learn the reason,
/// so the runtime reads the descriptor of the data source.
#[tokio::test]
async fn a_data_source_that_holds_no_choice_for_a_variable_gives_a_sentence() {
    let mut dash = runtime(FROM_A_QUERY, Arc::new(Recorder::new()));
    take_the_choices(&mut dash).await;
    assert_eq!(
        dash.variables().error(),
        Some(
            "The variable \"job\" has no choice. The data source \"rec\" returns no \
             choice for a variable."
                .to_string()
        )
    );
    assert_eq!(dash.variables().row_text(), "[job: no choice]");
}

const A_SIGNAL: &str = r#"
title = "variables"
[source.rec]
kind = "testdata"

[[variable]]
name = "signal"
label = "the signal"
options = ["sine", "ramp"]

[[panel]]
title = "signal"
kind = "timeseries"
grid = { x = 0, y = 0, w = 24, h = 8 }
[[panel.target]]
source = "rec"
expr = "$signal"
"#;

/// The testdata data source reads the whole text of a query. It returns an
/// error for a text that names no signal of its own. This test therefore
/// proves that the value, and not the name of the variable, reached it.
#[tokio::test]
async fn a_data_source_that_reads_the_whole_query_holds_the_value_of_a_variable() {
    let mut dash = runtime(A_SIGNAL, Arc::new(TestData::new()));
    settle(&dash).await;
    assert!(
        matches!(dash.snapshot().panel(0), PanelState::Ready(_)),
        "The panel must hold the sine wave. {:?}",
        dash.snapshot().panel(0)
    );
    assert_eq!(dash.variables().row_text(), "[the signal: sine]");

    let asked =
        variable_key(Key::Char(']'), dash.variables()).expect("The key must ask for a choice.");
    dash.set_variables(asked);
    settle(&dash).await;
    assert!(
        matches!(dash.snapshot().panel(0), PanelState::Ready(_)),
        "The panel must hold the ramp. {:?}",
        dash.snapshot().panel(0)
    );
    assert_eq!(dash.variables().row_text(), "[the signal: ramp]");
}
