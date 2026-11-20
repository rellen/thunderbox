//! The tasks of the repository of thunderbox.
//!
//! Run `cargo xtask site generate` to write the generated parts of the
//! documentation site. Run `cargo xtask site check` to test that they are
//! current. Run `cargo xtask site build` to build the whole site.

mod crates;
mod markers;
mod site;
mod summary;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The text that the program prints for an input that it does not know.
const USAGE: &str = "\
usage: cargo xtask site <command>

commands:
  generate   write the generated parts of the documentation site
  check      return an error when a generated part is not current
  build      build the site in target/book, and check each link
";

/// Returns the root of the repository.
///
/// # Errors
///
/// Returns an error when the path of the root does not exist.
pub fn repo_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|e| format!("The root of the repository is not readable: {e}."))
}

/// Finds each Markdown file under `dir`, in sorted order.
fn markdown_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let read = |e: std::io::Error| format!("The directory {} is not readable: {e}.", dir.display());
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(read)?
        .map(|e| e.map(|e| e.path()).map_err(read))
        .collect::<Result<_, _>>()?;
    entries.sort();
    for path in entries {
        if path.is_dir() {
            markdown_files(&path, out)?;
        } else if path.extension().is_some_and(|x| x == "md") {
            out.push(path);
        }
    }
    Ok(())
}

/// Computes the files whose text differs from the generated text.
///
/// Each item holds the path and the new text.
fn changes(root: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    let table = crates::table(&crates::rows(&crates::metadata(root)?)?);
    let docs = root.join("docs");
    let mut files = Vec::new();
    markdown_files(&docs, &mut files)?;
    let mut texts = summary::Files::new();
    let mut paths = BTreeMap::new();
    for path in files {
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("The file {} is not readable: {e}.", path.display()))?;
        let key = path
            .strip_prefix(&docs)
            .map_err(|e| format!("The file {} is not in docs: {e}.", path.display()))?
            .to_string_lossy()
            .replace('\\', "/");
        texts.insert(key.clone(), text);
        paths.insert(key, path);
    }
    let mut found = false;
    let mut changed = Vec::new();
    for (key, text) in &texts {
        let new = markers::replace(text, markers::CRATES_BEGIN, markers::CRATES_END, &table)
            .map_err(|e| format!("{}: {e}", paths[key].display()))?;
        if let Some(new) = new {
            found = true;
            if new != *text {
                changed.push((paths[key].clone(), new));
            }
        }
    }
    if !found {
        return Err(format!(
            "No file under docs holds the line `{}`.",
            markers::CRATES_BEGIN
        ));
    }
    let sidebar = summary::build(&texts)?;
    if texts.get("SUMMARY.md") != Some(&sidebar) {
        changed.push((docs.join("SUMMARY.md"), sidebar));
    }
    Ok(changed)
}

/// Writes each generated part.
fn generate(root: &Path) -> Result<(), String> {
    for (path, text) in changes(root)? {
        std::fs::write(&path, text)
            .map_err(|e| format!("The file {} is not writable: {e}.", path.display()))?;
        println!(
            "wrote {}",
            path.strip_prefix(root).unwrap_or(&path).display()
        );
    }
    Ok(())
}

/// Returns an error that names each file whose generated part is not current.
fn check(root: &Path) -> Result<(), String> {
    let changed = changes(root)?;
    if changed.is_empty() {
        return Ok(());
    }
    let names: Vec<String> = changed
        .iter()
        .map(|(p, _)| p.strip_prefix(root).unwrap_or(p).display().to_string())
        .collect();
    Err(format!(
        "These files are not current: {}. Run cargo xtask site generate.",
        names.join(", ")
    ))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let task: fn(&Path) -> Result<(), String> = match args.as_slice() {
        ["site", "generate"] => generate,
        ["site", "check"] => check,
        ["site", "build"] => site::build,
        _ => {
            eprint!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match repo_root().and_then(|root| task(&root)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(text) => {
            eprintln!("error: {text}");
            ExitCode::from(1)
        }
    }
}
