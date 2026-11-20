//! Tests for the legend rule, from the TOML text to the name of a series.
//!
//! `thunderbox_core::legend` holds the rule, and `run_panel` is the one place
//! that applies it. These tests drive a whole [`DashboardRuntime`], so they
//! cover every step between the TOML text and the name that a panel shows.

use std::sync::Arc;

use async_trait::async_trait;
use thunderbox_core::{Dashboard, Field, Frame, QueryRequest, Result};
use thunderbox_dashboard::{DashboardRuntime, PanelState};
use thunderbox_source::{DataSource, Registry, SourceDescriptor};
use thunderbox_source_sysinfo::SysinfoSource;

mod common;

use common::settle;

/// A data source that attaches no label to a column that holds a number.
///
/// The data source returns one column for each core of a processor. Each
/// column carries a number as its name, and it carries no label. A pattern on
/// such a data source reads the reserved word `{{__name__}}` and plain text. A
/// pattern that names a label finds no label, and the column then keeps its own
/// name.
struct Cores;

#[async_trait]
impl DataSource for Cores {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor::new("cores", "Cores")
    }

    async fn query(&self, _req: QueryRequest) -> Result<Vec<Frame>> {
        Ok(vec![Frame::named(
            "cpu.cores",
            vec![
                Field::time("time", vec![0, 1000]),
                Field::f64("0", vec![Some(1.0), Some(2.0)]),
                Field::f64("1", vec![Some(3.0), Some(4.0)]),
            ],
        )])
    }
}

/// Returns the name that every column of numbers in one panel shows.
async fn names(toml: &str, sources: Registry, panel: usize) -> Vec<String> {
    let dashboard = Dashboard::from_toml(toml).expect("The test dashboard must be correct.");
    let runtime = DashboardRuntime::new(dashboard, sources, tokio::runtime::Handle::current())
        .expect("The runtime must start.");
    settle(&runtime).await;
    let snapshot = runtime.snapshot();
    let PanelState::Ready(frames) = snapshot.panel(panel) else {
        panic!("The panel must hold an answer. {:?}", snapshot.panel(panel));
    };
    frames
        .iter()
        .flat_map(|f| f.numeric_fields())
        .map(Field::display_name)
        .collect()
}

const ONE_PANEL: &str = r#"
title = "cores"
[source.machine]
kind = "cores"

[[panel]]
title = "cores"
kind = "bargauge"
grid = { x = 0, y = 0, w = 24, h = 6 }
[[panel.target]]
source = "machine"
expr = "cpu.cores"
legend = "core {{__name__}}"
"#;

#[tokio::test]
async fn a_data_source_that_attaches_no_label_obeys_the_pattern_of_a_target() {
    let sources = Registry::new().with("machine", Arc::new(Cores));
    assert_eq!(
        names(ONE_PANEL, sources, 0).await,
        vec!["core 0", "core 1"],
        "The reserved word reads the name of the column."
    );
}

const NO_LEGEND: &str = r#"
title = "cores"
[source.machine]
kind = "cores"

[[panel]]
title = "cores"
kind = "bargauge"
grid = { x = 0, y = 0, w = 24, h = 6 }
[[panel.target]]
source = "machine"
expr = "cpu.cores"
"#;

#[tokio::test]
async fn a_target_that_states_no_pattern_keeps_the_name_of_the_data_source() {
    let sources = Registry::new().with("machine", Arc::new(Cores));
    assert_eq!(names(NO_LEGEND, sources, 0).await, vec!["0", "1"]);
}

const REAL_MACHINE: &str = r#"
title = "machine"
[source.machine]
kind = "sysinfo"

[[panel]]
title = "cpu"
kind = "timeseries"
grid = { x = 0, y = 0, w = 24, h = 6 }
[[panel.target]]
source = "machine"
expr = "cpu"
legend = "the {{__name__}} of this machine"
"#;

/// The sysinfo data source holds no line of code for a pattern. This test
/// proves that the rule still reaches it.
#[tokio::test]
async fn the_sysinfo_data_source_obeys_a_pattern_with_no_change_of_its_own() {
    let sources = Registry::new().with("machine", Arc::new(SysinfoSource::new()));
    assert_eq!(
        names(REAL_MACHINE, sources, 0).await,
        vec!["the cpu of this machine"],
        "The signal `cpu` returns one column, whatever the machine holds."
    );
}

const REAL_CORES: &str = r#"
title = "machine"
[source.machine]
kind = "sysinfo"

[[panel]]
title = "cores"
kind = "bargauge"
grid = { x = 0, y = 0, w = 24, h = 6 }
[[panel.target]]
source = "machine"
expr = "cpu.cores"
legend = "core {{core}}"
"#;

/// The signal `cpu.cores` returns one series for each core, and each of those
/// series carries the label `core`. A person can therefore write the name of
/// that label, and not the reserved word alone.
#[tokio::test]
async fn a_pattern_reads_the_label_that_the_sysinfo_data_source_attaches() {
    let sources = Registry::new().with("machine", Arc::new(SysinfoSource::new()));
    let names = names(REAL_CORES, sources, 0).await;
    assert!(
        !names.is_empty(),
        "The machine must report at least one core."
    );
    let expected: Vec<String> = (0..names.len()).map(|n| format!("core {n}")).collect();
    assert_eq!(
        names, expected,
        "The label must name each series, in the order of the cores."
    );
}

const REAL_CORES_NO_LEGEND: &str = r#"
title = "machine"
[source.machine]
kind = "sysinfo"

[[panel]]
title = "cores"
kind = "bargauge"
grid = { x = 0, y = 0, w = 24, h = 6 }
[[panel.target]]
source = "machine"
expr = "cpu.cores"
"#;

/// A label reaches the name of a series when the data source states no name
/// of its own. A person who writes no pattern would then read `0{core="0"}`
/// in place of `0`. The sysinfo data source states the short name, so this
/// test reads that name back.
#[tokio::test]
async fn a_target_of_the_sysinfo_data_source_that_states_no_pattern_keeps_the_short_name() {
    let sources = Registry::new().with("machine", Arc::new(SysinfoSource::new()));
    let names = names(REAL_CORES_NO_LEGEND, sources, 0).await;
    assert!(
        !names.is_empty(),
        "The machine must report at least one core."
    );
    let expected: Vec<String> = (0..names.len()).map(|n| n.to_string()).collect();
    assert_eq!(
        names, expected,
        "A person who states no pattern must read the short name of each column."
    );
}
