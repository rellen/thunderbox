//! The build of the documentation site.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::crates;

/// The largest number of broken links that the build prints.
const SHOWN: usize = 50;

/// The schemes of a link that leaves the site. The link check skips them.
pub const SCHEMES: [&str; 5] = ["http:", "https:", "mailto:", "data:", "javascript:"];

/// The start of a link that names the root of the site. It is the `site-url`.
const SITE_ROOT: &str = "/thunderbox/";

/// Builds the site in `target/book`, and checks each link of its pages.
///
/// # Errors
///
/// Returns an error in each of these cases:
///
/// - The API documentation does not exist.
/// - The program `mdbook` fails.
/// - A library crate has no API page.
/// - A page holds a broken link.
pub fn build(root: &Path) -> Result<(), String> {
    let meta = crates::metadata(root)?;
    let target = PathBuf::from(
        meta["target_directory"]
            .as_str()
            .ok_or("The output of `cargo metadata` names no target directory.")?,
    );
    let doc = target.join("doc");
    if !doc.join("crates.js").is_file() {
        return Err(format!(
            "The directory {} holds no API documentation. Run cargo doc --no-deps first.",
            doc.display()
        ));
    }
    run_mdbook(root)?;
    let book = root.join("target/book");
    let api = book.join("api");
    if api.exists() {
        std::fs::remove_dir_all(&api)
            .map_err(|e| format!("The directory {} is not removable: {e}.", api.display()))?;
    }
    copy_dir(&doc, &api)?;
    for row in crates::rows(&meta)? {
        if !api.join(&row.doc_dir).join("index.html").is_file() {
            return Err(format!(
                "The API documentation holds no page for the crate `{}`.",
                row.name
            ));
        }
    }
    let mut existing = BTreeSet::new();
    let mut pages = Vec::new();
    collect(&book, &book, &mut existing, &mut pages)?;
    let failures = check_links(&pages, &existing);
    if failures.is_empty() {
        println!("The site is in target/book.");
        return Ok(());
    }
    for failure in failures.iter().take(SHOWN) {
        eprintln!("target/book/{failure}");
    }
    if failures.len() > SHOWN {
        eprintln!("The list shows the first {SHOWN} of the broken links.");
    }
    Err(format!("The site holds {} broken links.", failures.len()))
}

/// Runs `mdbook build docs` in the root of the repository.
fn run_mdbook(root: &Path) -> Result<(), String> {
    let status = Command::new("mdbook")
        .args(["build", "docs"])
        .current_dir(root)
        .status()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "The program `mdbook` is not on the path. Run nix develop first.".to_string()
            } else {
                format!("The program `mdbook` did not start: {e}.")
            }
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("The command `mdbook build docs` failed: {status}."))
    }
}

/// Copies a directory and its content. The copy omits each file `.lock`.
fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    let fault = |e: std::io::Error| format!("The copy of {} failed: {e}.", from.display());
    std::fs::create_dir_all(to).map_err(fault)?;
    for entry in std::fs::read_dir(from).map_err(fault)? {
        let entry = entry.map_err(fault)?;
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            copy_dir(&path, &to.join(&name))?;
        } else if name != ".lock" {
            std::fs::copy(&path, to.join(&name)).map_err(fault)?;
        }
    }
    Ok(())
}

/// Lists each file under `dir`.
///
/// `existing` gets the path of each file, relative to `book`. `pages` gets the
/// path and the links of each HTML file. The function does not read the HTML
/// files of the folder `api`, because rustdoc wrote them. It records the paths
/// of those files in `existing`.
fn collect(
    book: &Path,
    dir: &Path,
    existing: &mut BTreeSet<String>,
    pages: &mut Vec<(String, Vec<String>)>,
) -> Result<(), String> {
    let fault =
        |e: std::io::Error| format!("The directory {} is not readable: {e}.", dir.display());
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(fault)?
        .map(|e| e.map(|e| e.path()).map_err(fault))
        .collect::<Result<_, _>>()?;
    entries.sort();
    for path in entries {
        let relative = path
            .strip_prefix(book)
            .map_err(|e| format!("The path {} is not in the book: {e}.", path.display()))?
            .to_string_lossy()
            .replace('\\', "/");
        if path.is_dir() {
            collect(book, &path, existing, pages)?;
        } else {
            if relative.ends_with(".html") && !relative.starts_with("api/") {
                let bytes = std::fs::read(&path)
                    .map_err(|e| format!("The file {} is not readable: {e}.", path.display()))?;
                pages.push((relative.clone(), links(&String::from_utf8_lossy(&bytes))));
            }
            existing.insert(relative);
        }
    }
    Ok(())
}

/// Returns the value of each `href` and each `src` attribute of a page.
fn links(html: &str) -> Vec<String> {
    let mut found = Vec::new();
    for attribute in ["href=\"", "src=\""] {
        for (at, _) in html.match_indices(attribute) {
            let before = html[..at].chars().next_back();
            if !before.is_some_and(char::is_whitespace) {
                continue;
            }
            let rest = &html[at + attribute.len()..];
            if let Some(end) = rest.find('"') {
                found.push(rest[..end].to_string());
            }
        }
    }
    found
}

/// Returns the broken links of the pages, as `page: link`.
///
/// Each page holds its path relative to the book, and its links. `existing`
/// holds the path of each file of the book. A link to a folder works when the
/// folder holds a file `index.html`. The function skips a link that leaves the
/// site, and a link to a place in the same page.
fn check_links(pages: &[(String, Vec<String>)], existing: &BTreeSet<String>) -> Vec<String> {
    let mut failures = Vec::new();
    for (page, links) in pages {
        for link in links {
            if link.starts_with('#')
                || link.starts_with("//")
                || SCHEMES.iter().any(|s| link.starts_with(s))
            {
                continue;
            }
            let works = resolve(page, link).is_some_and(|path| {
                let index = if path.is_empty() {
                    "index.html".to_string()
                } else {
                    format!("{path}/index.html")
                };
                existing.contains(&path) || existing.contains(&index)
            });
            if !works {
                failures.push(format!("{page}: {link}"));
            }
        }
    }
    failures
}

/// Resolves a link against the folder of its page.
///
/// The function removes the query and the anchor. A link that starts with
/// `/thunderbox/` resolves against the root of the book, which is the
/// `site-url` of `book.toml`. The function returns `None` for a link that
/// leaves the book, and for any other link that starts with a slash.
fn resolve(page: &str, link: &str) -> Option<String> {
    let link = link.split(['?', '#']).next().unwrap_or("");
    let mut parts: Vec<&str> = page
        .rsplit_once('/')
        .map_or_else(Vec::new, |(dir, _)| dir.split('/').collect());
    let link = if let Some(rest) = link.strip_prefix(SITE_ROOT) {
        if rest.is_empty() {
            return Some(String::new());
        }
        parts.clear();
        rest
    } else if link.starts_with('/') {
        return None;
    } else {
        link
    };
    if link.is_empty() {
        parts.push(page.rsplit('/').next().unwrap_or(page));
        return Some(parts.join("/"));
    }
    for segment in link.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            name => parts.push(name),
        }
    }
    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_records_api_paths_and_reads_no_api_page() {
        let book = std::env::temp_dir().join(format!("xtask-collect-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&book);
        std::fs::create_dir_all(book.join("api")).expect("The test folder is writable.");
        std::fs::write(book.join("index.html"), "<a href=\"x.html\">").expect("A write works.");
        std::fs::write(book.join("api/a.html"), "<a href=\"gone.html\">").expect("A write works.");
        let mut existing = BTreeSet::new();
        let mut pages = Vec::new();
        collect(&book, &book, &mut existing, &mut pages).expect("The collect call works.");
        let _ = std::fs::remove_dir_all(&book);
        assert!(
            existing.contains("api/a.html"),
            "The paths under api are recorded."
        );
        assert_eq!(pages.len(), 1, "The collect call reads no page under api.");
        assert_eq!(pages[0].0, "index.html");
    }

    fn set(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|p| p.to_string()).collect()
    }

    fn page(path: &str, links: &[&str]) -> (String, Vec<String>) {
        (
            path.to_string(),
            links.iter().map(|l| l.to_string()).collect(),
        )
    }

    #[test]
    fn reports_exactly_the_broken_links() {
        let existing = set(&[
            "index.html",
            "user/index.html",
            "user/a.html",
            "user/deep/page/index.html",
            "css/site.css",
        ]);
        let pages = vec![
            page(
                "user/a.html",
                &[
                    "index.html",
                    "../index.html",
                    "../css/site.css",
                    "missing.html",
                    "#top",
                    "https://example.com/x.html",
                    "deep/page/",
                    "deep/page?x=1#y",
                    "b.html?x=1",
                    "a.html#part",
                    "../../outside.html",
                ],
            ),
            page("index.html", &["user/", "user/a.html", "user/b.html"]),
        ];
        assert_eq!(
            check_links(&pages, &existing),
            [
                "user/a.html: missing.html",
                "user/a.html: b.html?x=1",
                "user/a.html: ../../outside.html",
                "index.html: user/b.html",
            ],
            "The check must report the broken links, in order."
        );
    }

    #[test]
    fn skips_only_a_link_that_starts_with_a_scheme() {
        let existing = set(&["a.html"]);
        let pages = vec![page(
            "a.html",
            &[
                "https://example.com/x",
                "mailto:a@b.c",
                "data:text/plain,x",
                "javascript:void(0)",
                "b.html:c",
                "x:y.html",
            ],
        )];
        assert_eq!(
            check_links(&pages, &existing),
            ["a.html: b.html:c", "a.html: x:y.html"],
            "A colon inside a path must not hide a broken link."
        );
    }

    #[test]
    fn resolves_a_site_root_link_against_the_root_of_the_book() {
        let existing = set(&["index.html", "x.html", "user/y.html"]);
        let pages = vec![page(
            "user/y.html",
            &[
                "/thunderbox/",
                "/thunderbox/x.html",
                "/thunderbox/user/y.html#a",
                "/thunderbox/gone.html",
                "/other/x.html",
                "/x.html",
            ],
        )];
        assert_eq!(
            check_links(&pages, &existing),
            [
                "user/y.html: /thunderbox/gone.html",
                "user/y.html: /other/x.html",
                "user/y.html: /x.html"
            ],
            "A link must resolve against the root only when it starts with /thunderbox/."
        );
    }

    #[test]
    fn finds_the_value_of_each_href_and_src() {
        let html =
            "<a href=\"a.html\">x</a> <img src=\"b.png\"> <a data-href=\"c\"> <p href=\"d\">";
        assert_eq!(
            links(html),
            ["a.html", "d", "b.png"],
            "The scan must skip an attribute that only ends in the name."
        );
    }
}
