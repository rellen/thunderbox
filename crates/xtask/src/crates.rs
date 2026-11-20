//! The table of the library crates of the workspace.

use std::collections::BTreeSet;
use std::fmt::Write;
use std::process::Command;

use serde_json::Value;

/// The target kinds that make a library.
const LIBRARY_KINDS: [&str; 6] = ["lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"];

/// The address of the API pages on the site.
const API_BASE: &str = "https://rellen.github.io/thunderbox/api";

/// One library crate of the workspace.
#[derive(Debug, PartialEq, Eq)]
pub struct Row {
    /// The name of the package.
    pub name: String,
    /// The description, with each run of white space reduced to one space.
    pub description: String,
    /// The name of the directory that rustdoc writes for the crate.
    pub doc_dir: String,
    /// The names of the workspace members that this crate depends on.
    pub depends_on: Vec<String>,
}

/// Runs `cargo metadata` in the directory `root` and returns the parsed output.
///
/// # Errors
///
/// Returns an error when the command fails or when its output is not JSON.
pub fn metadata(root: &std::path::Path) -> Result<Value, String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("The command `cargo metadata` did not start: {e}."))?;
    if !output.status.success() {
        let text = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "The command `cargo metadata` failed: {}",
            text.trim()
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("The output of `cargo metadata` is not JSON: {e}."))
}

/// Reads the rows of the table from the output of `cargo metadata`.
///
/// # Errors
///
/// Returns an error when the output has an unexpected shape, or when a library
/// crate has no description.
pub fn rows(meta: &Value) -> Result<Vec<Row>, String> {
    let shape = || "The output of `cargo metadata` has an unexpected shape.".to_string();
    let members: BTreeSet<&str> = meta["workspace_members"]
        .as_array()
        .ok_or_else(shape)?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let packages = meta["packages"].as_array().ok_or_else(shape)?;
    let names: BTreeSet<&str> = packages
        .iter()
        .filter(|p| p["id"].as_str().is_some_and(|id| members.contains(id)))
        .filter_map(|p| p["name"].as_str())
        .collect();
    let mut rows = Vec::new();
    for package in packages {
        let id = package["id"].as_str().ok_or_else(shape)?;
        if !members.contains(id) {
            continue;
        }
        let name = package["name"].as_str().ok_or_else(shape)?;
        let library = package["targets"]
            .as_array()
            .ok_or_else(shape)?
            .iter()
            .find(|t| {
                t["kind"].as_array().is_some_and(|kinds| {
                    kinds
                        .iter()
                        .filter_map(Value::as_str)
                        .any(|k| LIBRARY_KINDS.contains(&k))
                })
            });
        let Some(library) = library else { continue };
        let description = package["description"]
            .as_str()
            .map(|d| d.split_whitespace().collect::<Vec<_>>().join(" "))
            .unwrap_or_default();
        if description.is_empty() {
            return Err(format!("The crate `{name}` has no description."));
        }
        let target = library["name"].as_str().ok_or_else(shape)?;
        let depends_on: BTreeSet<String> = package["dependencies"]
            .as_array()
            .ok_or_else(shape)?
            .iter()
            .filter(|d| d["kind"].is_null())
            .filter_map(|d| d["name"].as_str())
            .filter(|d| names.contains(d))
            .map(str::to_string)
            .collect();
        rows.push(Row {
            name: name.to_string(),
            description,
            doc_dir: target.replace('-', "_"),
            depends_on: depends_on.into_iter().collect(),
        });
    }
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(rows)
}

/// Writes the table of the rows in the format of Markdown.
#[must_use]
pub fn table(rows: &[Row]) -> String {
    let mut out = String::from(
        "| crate | what it holds | the crates of thunderbox that it depends on |\n|---|---|---|\n",
    );
    for row in rows {
        let depends = if row.depends_on.is_empty() {
            "none".to_string()
        } else {
            row.depends_on
                .iter()
                .map(|d| format!("`{d}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let _ = writeln!(
            out,
            "| [`{name}`]({API_BASE}/{dir}/index.html) | {description} | {depends} |",
            name = row.name,
            dir = row.doc_dir,
            description = row.description.replace('|', "\\|"),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture(second_description: &str) -> Value {
        json!({
            "workspace_members": ["a 0.1.0", "b 0.1.0", "c 0.1.0"],
            "packages": [
                {"id": "a 0.1.0", "name": "alpha-core", "description": "The   core\n types.",
                 "targets": [{"name": "alpha-core", "kind": ["lib"]}], "dependencies": []},
                {"id": "b 0.1.0", "name": "beta", "description": second_description,
                 "targets": [{"name": "beta", "kind": ["lib"]}],
                 "dependencies": [
                    {"name": "alpha-core", "kind": null},
                    {"name": "prog", "kind": "dev"},
                    {"name": "serde", "kind": null}]},
                {"id": "c 0.1.0", "name": "prog", "description": "A program.",
                 "targets": [{"name": "prog", "kind": ["bin"]}], "dependencies": []},
                {"id": "d 0.1.0", "name": "outsider", "description": "Not a member.",
                 "targets": [{"name": "outsider", "kind": ["lib"]}], "dependencies": []}
            ]
        })
    }

    #[test]
    fn reads_only_the_libraries_of_the_workspace() {
        let rows = rows(&fixture("The beta.")).unwrap();
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names,
            ["alpha-core", "beta"],
            "The rows must hold the two libraries."
        );
    }

    #[test]
    fn ignores_a_dev_dependency_and_a_crate_outside_the_workspace() {
        let rows = rows(&fixture("The beta.")).unwrap();
        assert_eq!(
            rows[1].depends_on,
            ["alpha-core"],
            "The list must hold one normal dependency of the workspace."
        );
    }

    #[test]
    fn writes_the_directory_with_an_underscore_and_reduces_white_space() {
        let rows = rows(&fixture("The beta.")).unwrap();
        assert_eq!(
            rows[0].doc_dir, "alpha_core",
            "The directory must use an underscore."
        );
        assert_eq!(
            rows[0].description, "The core types.",
            "White space must reduce to one space."
        );
    }

    #[test]
    fn returns_an_error_that_names_a_crate_with_no_description() {
        let error = rows(&fixture("  ")).unwrap_err();
        assert!(
            error.contains("`beta`"),
            "The error must name the crate: {error}"
        );
    }

    #[test]
    fn writes_the_exact_table() {
        let rows = vec![
            Row {
                name: "x-one".into(),
                description: "A | B.".into(),
                doc_dir: "x_one".into(),
                depends_on: vec![],
            },
            Row {
                name: "x-two".into(),
                description: "C.".into(),
                doc_dir: "x_two".into(),
                depends_on: vec!["x-one".into(), "y".into()],
            },
        ];
        let expected = "\
| crate | what it holds | the crates of thunderbox that it depends on |
|---|---|---|
| [`x-one`](https://rellen.github.io/thunderbox/api/x_one/index.html) | A \\| B. | none |
| [`x-two`](https://rellen.github.io/thunderbox/api/x_two/index.html) | C. | `x-one`, `y` |
";
        assert_eq!(table(&rows), expected, "The table text must match.");
    }
}
