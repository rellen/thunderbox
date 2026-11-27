//! Tests for the dashboard that the program holds inside itself.
//!
//! A person who runs `cargo run -p thunderbox` reads this dashboard. The names of
//! its series therefore reach that person, and not a test. These tests read
//! those names back from the real data source of this machine.

use std::sync::Arc;
use std::time::Duration;

use thunderbox_core::{Dashboard, Field, Values};
use thunderbox_dashboard::{DashboardRuntime, PanelState, Snapshot};
use thunderbox_source::Registry;
use thunderbox_source_sysinfo::SysinfoSource;

/// The dashboard that the program holds inside itself.
const DEFAULT_DASHBOARD: &str = include_str!("../src/default.toml");

/// Waits until the set of queries that runs now has finished.
///
/// A set of queries is the whole group of queries that one refresh sends. The
/// helper reads the channel of the runtime, so it never returns on an answer of
/// the set of queries before.
async fn settle(runtime: &DashboardRuntime) {
    let mut rx = runtime.subscribe();
    let wait = async {
        while rx.borrow_and_update().is_refreshing() {
            rx.changed()
                .await
                .expect("The runtime must hold the channel.");
        }
    };
    tokio::time::timeout(Duration::from_secs(5), wait)
        .await
        .expect("The queries did not answer in time.");
}

/// Returns the name that every column of numbers in one panel shows.
fn names(snapshot: &Snapshot, panel: usize) -> Vec<String> {
    let PanelState::Ready(frames) = snapshot.panel(panel) else {
        panic!("The panel must hold an answer. {:?}", snapshot.panel(panel));
    };
    frames
        .iter()
        .flat_map(|f| f.numeric_fields())
        .map(Field::display_name)
        .collect()
}

/// Returns the answer of the default dashboard, over the real data source of
/// this machine.
async fn answer() -> Arc<Snapshot> {
    let dashboard =
        Dashboard::from_toml(DEFAULT_DASHBOARD).expect("The dashboard must be correct.");
    let sources = Registry::new().with("machine", Arc::new(SysinfoSource::new()));
    let runtime = DashboardRuntime::new(dashboard, sources, tokio::runtime::Handle::current())
        .expect("The runtime must start.");
    settle(&runtime).await;
    runtime.snapshot()
}

/// The sysinfo data source attaches a label to each series of the signal
/// `cpu.cores`. Both panels of the cores name each series with the pattern
/// `C{{core}}`, so the names read `C0`, `C1`, and one name for each other core.
/// A short name leaves more width to the numbers.
///
/// The store of that data source orders its names by the text, so a machine of
/// ten cores or more once drew `C0, C1, C10, C11, C2`. It now draws the cores in
/// the order of their position, and this test reads that order back.
///
/// Both panels of the cores state a pattern, and a pattern overwrites the name
/// of a column. This test therefore covers a target that states one. The test
/// `a_target_of_the_sysinfo_data_source_that_states_no_pattern_keeps_the_short_name`
/// in `thunderbox-dashboard` covers a target that states none.
#[tokio::test]
async fn the_default_dashboard_names_each_core_by_its_position() {
    let snapshot = answer().await;

    assert_eq!(
        names(&snapshot, 0),
        vec!["cpu"],
        "The panel of the processor draws one series."
    );
    assert_eq!(
        names(&snapshot, 4),
        vec!["network.rx", "network.tx"],
        "The panel of the network draws one series for each direction."
    );

    for panel in [3, 5] {
        let cores = names(&snapshot, panel);
        assert!(
            !cores.is_empty(),
            "The machine must report at least one core."
        );
        let expected: Vec<String> = (0..cores.len()).map(|n| format!("C{n}")).collect();
        assert_eq!(
            cores, expected,
            "The pattern of this panel names each series after its core. Panel {panel}."
        );
    }
}

/// The panel `processes` reads the list of the processes of this machine. The
/// process that runs this test is one of them.
#[tokio::test]
async fn the_panel_of_the_processes_lists_the_process_that_runs_this_test() {
    let snapshot = answer().await;
    let dashboard =
        Dashboard::from_toml(DEFAULT_DASHBOARD).expect("The dashboard must be correct.");
    let index = dashboard
        .panels
        .iter()
        .position(|p| p.title == "processes")
        .expect("The default dashboard must hold the panel of the processes.");
    let PanelState::Ready(frames) = snapshot.panel(index) else {
        panic!("The panel must hold an answer. {:?}", snapshot.panel(index));
    };
    let pid = frames[0]
        .fields
        .iter()
        .find(|f| f.name == "pid")
        .expect("The list must hold the column pid.");
    assert!(pid.config.identifier, "{pid:?}");
    let Values::F64(pids) = &pid.values else {
        panic!("The number of a process is a number. {pid:?}");
    };
    let me = f64::from(std::process::id());
    assert!(
        pids.contains(&Some(me)),
        "The list must hold the process that runs this test."
    );
}
