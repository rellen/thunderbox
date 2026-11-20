//! The sidebar of the documentation site.
//!
//! The functions here read the index pages of `docs` and write the text of
//! `docs/SUMMARY.md`. They read no file. They take a map from a path to the
//! text of the file at that path.

use std::collections::{BTreeMap, BTreeSet};

/// The files of `docs`.
///
/// Each key is a path relative to `docs`, written with slashes.
pub type Files = BTreeMap<String, String>;

/// The two parts of the sidebar. Each item holds the title of the part and the
/// path of its index page.
const ROOTS: [(&str, &str); 2] = [
    ("Use thunderbox", "user/README.md"),
    ("Work on thunderbox", "contributor/README.md"),
];

/// The files that no table lists. Each item holds the index page and the file
/// that the sidebar places at the end of the entries of that index page.
const EXCEPTIONS: [(&str, &str); 1] = [(
    "contributor/adr/README.md",
    "contributor/adr/0000-template.md",
)];

/// The files that the sidebar never lists.
const NOT_LISTED: [&str; 2] = ["SUMMARY.md", "index.md"];

/// One page of the sidebar, with the pages that stand below it.
struct Entry {
    title: String,
    path: String,
    children: Vec<Entry>,
}

/// The entries below one line of an index page that starts with `## `.
struct Section {
    heading: Option<String>,
    entries: Vec<Entry>,
}

/// One item that the scan of an index page finds.
enum Item {
    Heading(String),
    Link(String),
}

/// Returns the text of `docs/SUMMARY.md`.
///
/// # Errors
///
/// Returns an error in each of these cases:
///
/// - A page has no title line.
/// - A table links to a page that does not exist.
/// - A page is in no table of an index page.
pub fn build(files: &Files) -> Result<String, String> {
    let mut placed: BTreeSet<String> = BTreeSet::new();
    for (_, root) in ROOTS {
        placed.insert(root.to_string());
    }
    let mut out = String::from("# Summary\n\n[thunderbox](index.md)\n");
    for (part, root) in ROOTS {
        out.push_str(&format!(
            "\n# {part}\n\n- [{}]({root})\n",
            title(files, root)?
        ));
        for section in read_index(files, root, &mut placed)? {
            if section.entries.is_empty() {
                continue;
            }
            if let Some(heading) = section.heading {
                out.push_str(&format!("\n# {heading}\n\n"));
            }
            write_entries(&mut out, &section.entries, 0);
        }
    }
    let missing: Vec<String> = files
        .keys()
        .filter(|path| !placed.contains(*path) && !NOT_LISTED.contains(&path.as_str()))
        .map(|path| {
            format!(
                "docs/{path} is in no table of an index page. Add a row for it to docs/{}.",
                nearest_index(files, path)
            )
        })
        .collect();
    if missing.is_empty() {
        Ok(out)
    } else {
        Err(missing.join("\n"))
    }
}

/// Writes each entry as a line of a list, and writes its children below it.
fn write_entries(out: &mut String, entries: &[Entry], level: usize) {
    for entry in entries {
        out.push_str(&format!(
            "{}- [{}]({})\n",
            "  ".repeat(level),
            entry.title,
            entry.path
        ));
        write_entries(out, &entry.children, level + 1);
    }
}

/// Returns the path of the nearest index page above `path`.
fn nearest_index(files: &Files, path: &str) -> String {
    let mut dir = path.rsplit_once('/').map(|(d, _)| d);
    while let Some(d) = dir {
        let candidate = format!("{d}/README.md");
        if candidate != path && files.contains_key(&candidate) {
            return candidate;
        }
        dir = d.rsplit_once('/').map(|(parent, _)| parent);
    }
    "README.md".to_string()
}

/// Returns the first title line of a page, without the mark `# `.
fn title(files: &Files, path: &str) -> Result<String, String> {
    let text = files
        .get(path)
        .ok_or_else(|| format!("The page docs/{path} does not exist."))?;
    text.lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(|t| t.trim().to_string())
        .ok_or_else(|| {
            format!("The page docs/{path} holds no title. Add a first line that starts with `# `.")
        })
}

/// Reads the entries of an index page, in the order of its tables.
fn read_index(
    files: &Files,
    index: &str,
    placed: &mut BTreeSet<String>,
) -> Result<Vec<Section>, String> {
    let text = files
        .get(index)
        .ok_or_else(|| format!("The page docs/{index} does not exist."))?;
    let mut sections = vec![Section {
        heading: None,
        entries: Vec::new(),
    }];
    for item in scan(text) {
        match item {
            Item::Heading(heading) => sections.push(Section {
                heading: Some(heading),
                entries: Vec::new(),
            }),
            Item::Link(target) => {
                let Some(path) = resolve(index, &target) else {
                    continue;
                };
                if !placed.insert(path.clone()) {
                    continue;
                }
                let entry = make_entry(files, path, placed)?;
                if let Some(last) = sections.last_mut() {
                    last.entries.push(entry);
                }
            }
        }
    }
    for (owner, file) in EXCEPTIONS {
        if owner == index && placed.insert(file.to_string()) {
            let entry = make_entry(files, file.to_string(), placed)?;
            if let Some(last) = sections.last_mut() {
                last.entries.push(entry);
            }
        }
    }
    Ok(sections)
}

/// Makes the entry of a page. An index page also gets the entries of its tables.
fn make_entry(files: &Files, path: String, placed: &mut BTreeSet<String>) -> Result<Entry, String> {
    let title = title(files, &path)?;
    let children = if path.rsplit('/').next() == Some("README.md") {
        read_index(files, &path, placed)?
            .into_iter()
            .flat_map(|section| section.entries)
            .collect()
    } else {
        Vec::new()
    };
    Ok(Entry {
        title,
        path,
        children,
    })
}

/// Finds each line that starts with `## `, and the first link of each table
/// row. The scan skips the header row of a table and the lines of a fence.
fn scan(text: &str) -> Vec<Item> {
    let mut items = Vec::new();
    let mut in_fence = false;
    let mut after_row = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            after_row = false;
        } else if in_fence {
            continue;
        } else if let Some(heading) = line.strip_prefix("## ") {
            items.push(Item::Heading(heading.trim().to_string()));
            after_row = false;
        } else if trimmed.starts_with('|') {
            let header = !after_row;
            after_row = true;
            let rule = trimmed.chars().all(|c| "|-: ".contains(c));
            if header || rule {
                continue;
            }
            if let Some(target) = first_link(trimmed) {
                items.push(Item::Link(target));
            }
        } else {
            after_row = false;
        }
    }
    items
}

/// Returns the target of the first Markdown link of a line.
fn first_link(line: &str) -> Option<String> {
    let rest = &line[line.find("](")? + 2..];
    let target = &rest[..rest.find(')')?];
    target.split_whitespace().next().map(str::to_string)
}

/// Resolves a link of an index page to a path that the index page may list.
///
/// The function returns `None` for a link that is not an entry. An entry is a
/// relative link to a Markdown file in the folder of the index page, or below
/// that folder.
fn resolve(index: &str, target: &str) -> Option<String> {
    if target.contains(':') || target.starts_with('/') {
        return None;
    }
    let target = target.split('#').next().unwrap_or("");
    if !target.ends_with(".md") {
        return None;
    }
    let dir = index.rsplit_once('/').map_or("", |(d, _)| d);
    let mut parts: Vec<&str> = if dir.is_empty() {
        Vec::new()
    } else {
        dir.split('/').collect()
    };
    for segment in target.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            name => parts.push(name),
        }
    }
    let path = parts.join("/");
    let prefix = if dir.is_empty() {
        String::new()
    } else {
        format!("{dir}/")
    };
    path.starts_with(&prefix).then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(pairs: &[(&str, &str)]) -> Files {
        pairs
            .iter()
            .map(|(path, text)| (path.to_string(), text.to_string()))
            .collect()
    }

    const USER: &str = "\
# The user docs

| kind | folder |
|---|---|
| [tutorials](tutorials) | a folder |
| [Intro](intro.md) | The first page. |

## The guides

| page | text |
|---|---|
| [One](guides/one.md) | a page |
| [One again](guides/one.md) | a duplicate |
| [Two](guides/two.md#part) | a link with an anchor |
| [Away](../contributor/outside.md) | a page outside the folder |

## The empty section

No table follows.

## The reference

| page | text |
|---|---|
| [Nested](ref/README.md) | an index page |
";

    const REF: &str = "\
# The reference index

| page | text |
|---|---|
| [A](a.md) | x |
| [B](b.md) | y |
";

    const CONTRIBUTOR: &str = "\
# The contributor docs

| page | text |
|---|---|
| [Outside](outside.md) | x |
| [Decisions](adr/README.md) | y |
";

    const ADR: &str = "\
# The records

## The records

| record | title |
|---|---|
| [0001](0001-x.md) | z |
";

    fn tree() -> Files {
        files(&[
            ("index.md", "# thunderbox\n"),
            ("SUMMARY.md", "old text\n"),
            ("user/README.md", USER),
            ("user/intro.md", "# Intro page\n"),
            ("user/guides/one.md", "# Guide one\n"),
            ("user/guides/two.md", "# Guide two\n"),
            ("user/ref/README.md", REF),
            ("user/ref/a.md", "# A page\n"),
            ("user/ref/b.md", "# B page\n"),
            ("contributor/README.md", CONTRIBUTOR),
            ("contributor/outside.md", "# Outside page\n"),
            ("contributor/adr/README.md", ADR),
            ("contributor/adr/0000-template.md", "# ADR NNNN: T\n"),
            ("contributor/adr/0001-x.md", "# ADR 0001: X\n"),
        ])
    }

    #[test]
    fn writes_the_exact_sidebar() {
        let expected = "\
# Summary

[thunderbox](index.md)

# Use thunderbox

- [The user docs](user/README.md)
- [Intro page](user/intro.md)

# The guides

- [Guide one](user/guides/one.md)
- [Guide two](user/guides/two.md)

# The reference

- [The reference index](user/ref/README.md)
  - [A page](user/ref/a.md)
  - [B page](user/ref/b.md)

# Work on thunderbox

- [The contributor docs](contributor/README.md)
- [Outside page](contributor/outside.md)
- [The records](contributor/adr/README.md)
  - [ADR 0001: X](contributor/adr/0001-x.md)
  - [ADR NNNN: T](contributor/adr/0000-template.md)
";
        assert_eq!(
            build(&tree()).unwrap(),
            expected,
            "The sidebar text must match."
        );
    }

    #[test]
    fn returns_an_error_for_a_file_that_no_table_lists() {
        let mut tree = tree();
        tree.insert("contributor/adr/stray.md".into(), "# Stray\n".into());
        assert_eq!(
            build(&tree).unwrap_err(),
            "docs/contributor/adr/stray.md is in no table of an index page. \
             Add a row for it to docs/contributor/adr/README.md.",
            "The error must name the file and the index page."
        );
    }

    #[test]
    fn returns_an_error_for_a_page_with_no_title_line() {
        let mut tree = tree();
        tree.insert("user/intro.md".into(), "No title here.\n".into());
        assert_eq!(
            build(&tree).unwrap_err(),
            "The page docs/user/intro.md holds no title. Add a first line that starts with `# `.",
            "The error must name the page."
        );
    }

    #[test]
    fn returns_an_error_for_a_link_to_a_page_that_does_not_exist() {
        let mut tree = tree();
        tree.remove("user/guides/two.md");
        assert_eq!(
            build(&tree).unwrap_err(),
            "The page docs/user/guides/two.md does not exist.",
            "The error must name the missing page."
        );
    }

    #[test]
    fn resolves_only_a_link_in_the_folder_of_the_index_or_below_it() {
        assert_eq!(
            resolve("user/README.md", "a/b.md#c"),
            Some("user/a/b.md".to_string()),
            "A link below the folder is an entry."
        );
        assert_eq!(
            resolve("user/README.md", "../contributor/x.md"),
            None,
            "A link outside the folder is not an entry."
        );
        assert_eq!(
            resolve("user/README.md", "../user2/x.md"),
            None,
            "A folder with the same start is not the folder."
        );
        assert_eq!(
            resolve("user/README.md", "https://example.com/x.md"),
            None,
            "An address is not an entry."
        );
        assert_eq!(
            resolve("user/README.md", "folder"),
            None,
            "A link to a folder is not an entry."
        );
    }
}
