//! The helper that more than one test file needs.
//!
//! Three test files drive a whole [`DashboardRuntime`], and each must wait for
//! a set of queries. One copy of that rule keeps the three files in step.

use std::time::Duration;

use thunderbox_dashboard::DashboardRuntime;

/// Waits until the set of queries that runs now has finished.
///
/// A set of queries is the whole group of queries that one refresh sends. The
/// helper reads the channel of the runtime, so it never returns on an answer of
/// the set of queries before. The timeout turns a data source that never
/// answers into a test that fails at once.
pub async fn settle(runtime: &DashboardRuntime) {
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
