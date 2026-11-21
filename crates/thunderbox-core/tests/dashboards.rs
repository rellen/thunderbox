//! The test that each dashboard file of the repository still loads.
//!
//! `Dashboard::validate` rejects more values over time. A new rule can reject a
//! value that a file of `dashboards` holds. The first command that a person
//! runs would then return an error, so this test reads each such file.

use std::path::Path;

use thunderbox_core::Dashboard;

#[test]
fn each_file_under_dashboards_loads() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dashboards");
    let mut count = 0;
    for entry in std::fs::read_dir(&folder).expect("The test must be able to read the folder.") {
        let path = entry
            .expect("The test must be able to read an entry.")
            .path();
        if path.extension().is_none_or(|extension| extension != "toml") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("The test must be able to read the file.");
        if let Err(error) = Dashboard::from_toml(&text) {
            panic!("The file {} does not load. {error}", path.display());
        }
        count += 1;
    }
    assert!(
        count >= 3,
        "The folder must hold the three dashboards of the repository. It held {count}."
    );
}
