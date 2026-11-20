//! The mdBook preprocessor that makes each relative link work on the site.
//!
//! A page under `docs` also reads well on GitHub. Its links therefore name
//! files in the repository, and some of those files are not pages of the site.
//! The preprocessor changes each such link into a link that works on the site.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;

use crate::site::SCHEMES;

/// The address of the site. A link that starts with it becomes a relative link.
const SITE: &str = "https://rellen.github.io/thunderbox/";

/// The address of the repository on GitHub, for a link to a file or a folder.
const REPOSITORY: &str = "https://github.com/rellen/thunderbox";

/// The kind of a path in the repository.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The path names a file.
    File,
    /// The path names a folder.
    Folder,
    /// The path names nothing.
    Absent,
}

/// Returns the path of a file or a folder that a link names.
///
/// `dir` is the folder that holds the link. The function returns `None` when
/// the path leaves the repository. Both paths are relative to the root.
fn join(dir: &str, target: &str) -> Option<String> {
    let mut parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    for segment in target.split('/') {
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

/// Returns the relative path from the folder `from` to the path `to`.
///
/// A `to` that ends in a slash keeps that slash. An empty result becomes `./`.
fn relative(from: &str, to: &str) -> String {
    let from: Vec<&str> = from.split('/').filter(|p| !p.is_empty()).collect();
    let to_parts: Vec<&str> = to.split('/').filter(|p| !p.is_empty()).collect();
    let limit = to_parts.len().saturating_sub(1).min(from.len());
    let common = (0..limit).take_while(|&i| from[i] == to_parts[i]).count();
    let mut out = "../".repeat(from.len() - common);
    out.push_str(&to_parts[common..].join("/"));
    if to.ends_with('/') && !out.ends_with('/') {
        out.push('/');
    }
    if out.is_empty() {
        out.push_str("./");
    }
    out
}

/// Returns the folder of a path. The folder of a path without a slash is empty.
fn folder(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

/// Returns the path of a page of the site, with `README.md` renamed.
///
/// The `index` preprocessor of mdBook renames each `README.md` chapter to
/// `index.md`. The preprocessor of this module runs before it.
fn site_name(path: &str) -> String {
    match path.rsplit_once('/') {
        Some((dir, "README.md")) => format!("{dir}/index.md"),
        None if path == "README.md" => "index.md".to_string(),
        _ => path.to_string(),
    }
}

/// Finds the files that the chapters include with `{{#include <path>}}`.
///
/// Each chapter holds its path inside `docs`, and its text. The result maps
/// the path of an included file, relative to the root of the repository, to the
/// path of the chapter that includes it.
pub fn includes(chapters: &[(String, String)]) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    for (chapter, content) in chapters {
        let dir = join("docs", folder(chapter)).unwrap_or_default();
        for line in content.lines() {
            let Some(rest) = line.trim().strip_prefix("{{#include ") else {
                continue;
            };
            let Some(path) = rest.strip_suffix("}}") else {
                continue;
            };
            let path = path.trim().split(':').next().unwrap_or("");
            if let Some(full) = join(&dir, path) {
                found.entry(full).or_insert_with(|| chapter.clone());
            }
        }
    }
    found
}

/// Rewrites the links of one chapter.
///
/// - `chapter` is the path of the chapter inside `docs`.
/// - `pages` holds the path of each chapter of the book inside `docs`.
/// - `kind` tells whether a path of the repository is a file, a folder, or
///   absent. The path is relative to the root of the repository.
/// - `included` maps a file outside the chapters to the chapter that includes
///   it. Use [`includes`] to build it.
///
/// The function returns the new text and one error for each link that reaches
/// nothing. A link inside a fenced code block or a code span stays as it is.
pub fn rewrite(
    chapter: &str,
    content: &str,
    pages: &BTreeSet<String>,
    kind: &dyn Fn(&str) -> Kind,
    included: &BTreeMap<String, String>,
) -> (String, Vec<String>) {
    let mut errors = Vec::new();
    let mut change = |target: &str| -> Option<String> {
        match map_target(chapter, target, pages, kind, included) {
            Ok(new) => new,
            Err(()) => {
                errors.push(format!(
                    "docs/{chapter}: the link {target} reaches nothing."
                ));
                None
            }
        }
    };
    let mut out = String::with_capacity(content.len());
    let mut fence: Option<(char, usize)> = None;
    for line in content.split_inclusive('\n') {
        match fence_of(line) {
            Some((mark, count, rest_is_empty)) => match fence {
                None => fence = Some((mark, count)),
                Some((open, size)) if open == mark && count >= size && rest_is_empty => {
                    fence = None;
                }
                Some(_) => {}
            },
            None => {
                if fence.is_none() {
                    scan_line(line, &mut out, &mut change);
                    continue;
                }
            }
        }
        out.push_str(line);
    }
    (out, errors)
}

/// Reads the fence marker at the start of a line.
///
/// Returns the character, the length of the run, and whether the rest of the
/// line is empty. It returns `None` when the line holds no fence marker.
fn fence_of(line: &str) -> Option<(char, usize, bool)> {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    let mark = trimmed.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let count = trimmed.chars().take_while(|&c| c == mark).count();
    if count < 3 {
        return None;
    }
    Some((mark, count, trimmed[count..].trim().is_empty()))
}

/// Copies one line to `out`, and changes the target of each link.
///
/// The function skips each inline code span. A span stays inside one line.
///
/// The scan has two limits. It does not skip a code span that crosses a line
/// break. It does not read a fence that has an indent of four or more spaces.
/// The link check of `cargo xtask site build` still catches a broken result.
fn scan_line(line: &str, out: &mut String, change: &mut dyn FnMut(&str) -> Option<String>) {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let run = bytes[i..].iter().take_while(|&&b| b == b'`').count();
            let mut j = i + run;
            let mut end = None;
            while j < bytes.len() {
                if bytes[j] == b'`' {
                    let size = bytes[j..].iter().take_while(|&&b| b == b'`').count();
                    if size == run {
                        end = Some(j + size);
                        break;
                    }
                    j += size;
                } else {
                    j += 1;
                }
            }
            let stop = end.unwrap_or(i + run);
            out.push_str(&line[i..stop]);
            i = stop;
        } else if line[i..].starts_with("](") {
            let start = i + 2;
            let mut depth = 1;
            let mut stop = start;
            for (offset, c) in line[start..].char_indices() {
                stop = start + offset;
                if c == '(' {
                    depth += 1;
                } else if c == ')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                } else if c.is_whitespace() {
                    break;
                }
                stop = start + offset + c.len_utf8();
            }
            let target = &line[start..stop];
            out.push_str("](");
            match change(target) {
                Some(new) => out.push_str(&new),
                None => out.push_str(target),
            }
            i = stop;
        } else {
            let c = line[i..].chars().next().unwrap_or(' ');
            out.push(c);
            i += c.len_utf8();
        }
    }
}

/// Computes the new target of one link.
///
/// Returns `Ok(None)` when the target stays as it is, and `Err(())` when the
/// target reaches nothing.
fn map_target(
    chapter: &str,
    target: &str,
    pages: &BTreeSet<String>,
    kind: &dyn Fn(&str) -> Kind,
    included: &BTreeMap<String, String>,
) -> Result<Option<String>, ()> {
    let dir = folder(chapter);
    if let Some(rest) = target.strip_prefix(SITE) {
        let at = rest.find(['?', '#']).unwrap_or(rest.len());
        let (path, tail) = rest.split_at(at);
        return Ok(Some(format!("{}{tail}", relative(dir, path))));
    }
    if target.is_empty()
        || target.starts_with(['#', '/'])
        || SCHEMES.iter().any(|s| target.starts_with(s))
    {
        return Ok(None);
    }
    let at = target.find(['?', '#']).unwrap_or(target.len());
    let (path, tail) = target.split_at(at);
    let full = join(&join("docs", dir).ok_or(())?, path).ok_or(())?;
    if let Some(inner) = full.strip_prefix("docs/")
        && pages.contains(inner)
    {
        let new = site_name(path);
        return Ok((new != path).then(|| format!("{new}{tail}")));
    }
    if let Some(page) = included.get(&full) {
        let new = relative(dir, &site_name(page));
        return Ok(Some(format!("{new}{tail}")));
    }
    let known = kind(&full);
    let is_markdown = full.ends_with(".md");
    match known {
        Kind::Absent => Err(()),
        Kind::File if full.starts_with("docs/") && !is_markdown => Ok(None),
        Kind::File => Ok(Some(format!("{REPOSITORY}/blob/main/{full}{tail}"))),
        Kind::Folder => Ok(Some(format!("{REPOSITORY}/tree/main/{full}{tail}"))),
    }
}

/// Collects the chapters of a book, with the path and the text of each.
///
/// A draft chapter, which has no path, is not in the result.
fn collect(items: &[serde_json::Value], out: &mut Vec<(String, String)>) {
    for item in items {
        let Some(chapter) = item.get("Chapter") else {
            continue;
        };
        if let (Some(path), Some(content)) = (
            chapter["source_path"]
                .as_str()
                .filter(|_| !chapter["path"].is_null()),
            chapter["content"].as_str(),
        ) {
            out.push((path.to_string(), content.to_string()));
        }
        if let Some(sub) = chapter["sub_items"].as_array() {
            collect(sub, out);
        }
    }
}

/// Writes the new text into each chapter, and appends each error to `errors`.
fn apply(
    items: &mut [serde_json::Value],
    pages: &BTreeSet<String>,
    kind: &dyn Fn(&str) -> Kind,
    included: &BTreeMap<String, String>,
    errors: &mut Vec<String>,
) {
    for item in items {
        let Some(chapter) = item.get_mut("Chapter") else {
            continue;
        };
        let source = chapter["source_path"].as_str().map(str::to_string);
        let content = chapter["content"].as_str().map(str::to_string);
        if let (Some(source), Some(content)) = (source, content)
            && !chapter["path"].is_null()
        {
            let (new, mut found) = rewrite(&source, &content, pages, kind, included);
            errors.append(&mut found);
            chapter["content"] = serde_json::Value::String(new);
        }
        if let Some(sub) = chapter["sub_items"].as_array_mut() {
            apply(sub, pages, kind, included, errors);
        }
    }
}

/// Reads a book from the standard input, and writes the new book.
///
/// mdBook calls this function with the standard input `[context, book]`.
///
/// # Errors
///
/// Returns an error when the input is not a book, or when a link reaches
/// nothing.
pub fn run(root: &Path) -> Result<(), String> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| format!("The standard input is not readable: {e}."))?;
    let value: serde_json::Value = serde_json::from_str(&input)
        .map_err(|e| format!("The standard input is not JSON: {e}."))?;
    let mut book = value
        .get(1)
        .cloned()
        .ok_or("The standard input holds no book.")?;
    let items = book["items"]
        .as_array_mut()
        .ok_or("The book holds no list of items.")?;
    let mut chapters = Vec::new();
    collect(items, &mut chapters);
    let pages: BTreeSet<String> = chapters.iter().map(|(p, _)| p.clone()).collect();
    let included = includes(&chapters);
    let kind = |path: &str| {
        let full = root.join(path);
        if full.is_dir() {
            Kind::Folder
        } else if full.is_file() {
            Kind::File
        } else {
            Kind::Absent
        }
    };
    let mut errors = Vec::new();
    apply(items, &pages, &kind, &included, &mut errors);
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    let text = serde_json::to_string(&book)
        .map_err(|e| format!("The book is not writable as JSON: {e}."))?;
    println!("{text}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILES: [&str; 4] = [
        "README.md",
        "CLAUDE.md",
        "docs/media/x.gif",
        "docs/notes.md",
    ];
    const FOLDERS: [&str; 3] = ["docs", "docs/user", "crates"];

    fn kind(path: &str) -> Kind {
        if FILES.contains(&path) {
            Kind::File
        } else if FOLDERS.contains(&path) {
            Kind::Folder
        } else {
            Kind::Absent
        }
    }

    fn pages() -> BTreeSet<String> {
        [
            "index.md",
            "user/README.md",
            "user/a.md",
            "contributor/rules.md",
        ]
        .iter()
        .map(|p| p.to_string())
        .collect()
    }

    fn included() -> BTreeMap<String, String> {
        BTreeMap::from([("CLAUDE.md".to_string(), "contributor/rules.md".to_string())])
    }

    fn run_rewrite(chapter: &str, content: &str) -> (String, Vec<String>) {
        rewrite(chapter, content, &pages(), &kind, &included())
    }

    #[test]
    fn rewrites_a_site_address_to_a_relative_path() {
        let text = "[a](https://rellen.github.io/thunderbox/api/x/index.html#top) \
                    [b](https://rellen.github.io/thunderbox/)";
        assert_eq!(
            run_rewrite("user/reference/crates.md", text).0,
            "[a](../../api/x/index.html#top) [b](../../)",
            "The site address must become a path relative to the chapter."
        );
    }

    #[test]
    fn keeps_a_scheme_and_an_anchor() {
        let text = "[a](https://example.com/x) [b](mailto:a@b.c) [c](#part)";
        assert_eq!(
            run_rewrite("user/a.md", text),
            (text.to_string(), vec![]),
            "A link with a scheme, or an anchor alone, must stay."
        );
    }

    #[test]
    fn keeps_a_link_to_a_page_and_renames_a_readme_page() {
        let text = "[a](a.md#x) [b](README.md) [c](../index.md) [d](../user/README.md#y)";
        assert_eq!(
            run_rewrite("user/a.md", text).0,
            "[a](a.md#x) [b](index.md) [c](../index.md) [d](../user/index.md#y)",
            "Only a README.md page must change its name."
        );
    }

    #[test]
    fn keeps_a_link_to_a_file_that_is_not_markdown() {
        let text = "[a](../media/x.gif)";
        assert_eq!(
            run_rewrite("user/a.md", text).0,
            "[a](../media/x.gif)",
            "A file inside docs that is not Markdown must stay."
        );
    }

    #[test]
    fn links_a_folder_inside_docs_to_github() {
        assert_eq!(
            run_rewrite("user/a.md", "[a](../user) [b](../user/#z)").0,
            "[a](https://github.com/rellen/thunderbox/tree/main/docs/user) \
             [b](https://github.com/rellen/thunderbox/tree/main/docs/user#z)",
            "A folder inside docs must become a tree link."
        );
    }

    #[test]
    fn links_an_included_file_to_its_chapter() {
        assert_eq!(
            run_rewrite("user/a.md", "[a](../../CLAUDE.md#one)").0,
            "[a](../contributor/rules.md#one)",
            "An included file must become a link to the chapter that includes it."
        );
    }

    #[test]
    fn links_any_other_file_outside_docs_to_github() {
        assert_eq!(
            run_rewrite("user/a.md", "[a](../../README.md#q) [b](../../crates)").0,
            "[a](https://github.com/rellen/thunderbox/blob/main/README.md#q) \
             [b](https://github.com/rellen/thunderbox/tree/main/crates)",
            "A file or a folder outside docs must become a GitHub link."
        );
    }

    #[test]
    fn links_a_markdown_file_that_is_not_a_page_to_github() {
        assert_eq!(
            run_rewrite("user/a.md", "[a](../notes.md)").0,
            "[a](https://github.com/rellen/thunderbox/blob/main/docs/notes.md)",
            "A Markdown file that the book omits must become a blob link."
        );
    }

    #[test]
    fn reports_a_link_that_reaches_nothing() {
        let (text, errors) = run_rewrite("user/a.md", "[a](gone.md) [b](../../../up.md)");
        assert_eq!(
            text, "[a](gone.md) [b](../../../up.md)",
            "The text must stay."
        );
        assert_eq!(
            errors,
            [
                "docs/user/a.md: the link gone.md reaches nothing.",
                "docs/user/a.md: the link ../../../up.md reaches nothing."
            ],
            "Each link that reaches nothing must give one error."
        );
    }

    #[test]
    fn keeps_a_link_in_a_fence_and_in_a_code_span() {
        let text = "```\n[a](gone.md)\n```\n`[b](gone.md)` and ``[c](gone.md)``\n\
                    ~~~~\n[d](gone.md)\n~~~\n[e](gone.md)\n~~~~\n[f](README.md)\n";
        assert_eq!(
            run_rewrite("user/a.md", text),
            (text.replace("[f](README.md)", "[f](index.md)"), vec![]),
            "A fence or a code span must hide its links."
        );
    }

    #[test]
    fn keeps_an_image_and_a_title() {
        let text = "![alt](../media/x.gif) [a](README.md \"the title\")";
        assert_eq!(
            run_rewrite("user/a.md", text).0,
            "![alt](../media/x.gif) [a](index.md \"the title\")",
            "An image must stay, and a title must not join the target."
        );
    }

    #[test]
    fn finds_the_included_files() {
        let chapters = vec![
            (
                "contributor/rules.md".to_string(),
                "text\n{{#include ../../CLAUDE.md}}\n".to_string(),
            ),
            (
                "user/a.md".to_string(),
                "{{#include ../../other.md:2:5}}\n`{{#include x}}`\n".to_string(),
            ),
        ];
        assert_eq!(
            includes(&chapters),
            BTreeMap::from([
                ("CLAUDE.md".to_string(), "contributor/rules.md".to_string()),
                ("other.md".to_string(), "user/a.md".to_string()),
            ]),
            "The map must name each file and the chapter that includes it."
        );
    }

    #[test]
    fn computes_a_relative_path() {
        assert_eq!(relative("user", "user/index.md"), "index.md");
        assert_eq!(relative("", "api/x/"), "api/x/");
        assert_eq!(relative("a/b", "c.md"), "../../c.md");
        assert_eq!(relative("a", ""), "../");
        assert_eq!(relative("", ""), "./");
    }
}
