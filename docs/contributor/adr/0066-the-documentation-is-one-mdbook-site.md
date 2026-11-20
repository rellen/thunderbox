# ADR 0066: The documentation is one mdBook site, and the workflow `checks` publishes it

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-21 |
| Source | new |
| Code | `docs/book.toml`, `docs/SUMMARY.md`, `docs/index.md`, `docs/site.css`, `crates/xtask`, `.github/workflows/checks.yml`, `flake.nix` |
| Tests | `writes_the_exact_sidebar`, `returns_an_error_for_a_file_that_no_table_lists`, `writes_the_exact_table`, `reports_exactly_the_broken_links`, `keeps_a_link_in_a_fence_and_in_a_code_span` |
| Related | [ADR 0001](0001-record-architecture-decisions.md), [ADR 0067](0067-five-other-designs-for-the-site-failed.md), [ADR 0068](0068-the-site-shows-one-version.md) |

## Context

The project holds its documentation in `docs`, in plain Markdown files. A
person reads those files on GitHub, one folder at a time. No page shows the
whole tree, and no sidebar leads a reader from one page to the next.

The API documentation comes from `cargo doc --no-deps`. It measured 7.9
megabytes. No crate of the workspace is on crates.io, because each crate holds
the version 0.0.0. docs.rs builds the documentation of a crate from crates.io
alone, so docs.rs holds nothing of this project.

A reader of the project needs one address that holds the guides, the decision
records, and the API documentation together.

## Decision

The project publishes one site, and the workflow `checks` builds and publishes
it:

- mdBook builds the site from `docs`. mdBook is a program that builds a site
  from Markdown files. `docs/book.toml` sets `src = "."`, so each document and
  each picture of `docs` reaches the site, and no file moves.
- The job `deploy` of `.github/workflows/checks.yml` publishes the site to
  GitHub Pages. It runs on `main` alone, and it waits for each job of the
  matrix.
- The crate `crates/xtask` holds the Rust code of the site. The command
  `cargo xtask site generate` writes the sidebar and the crate tables. The
  command `cargo xtask site check` returns an error when a generated part is
  not current. The command `cargo xtask site build` builds the site and checks
  every internal link of the built pages. The command
  `cargo xtask mdbook-preprocessor` rewrites each relative link.
- The folder `api` of the site holds the output of `cargo doc --no-deps`.

`CLAUDE.md` section 9 states the rules that follow from this decision.

## Consequences

The generator writes three parts of the site, and no person edits them:

- `docs/SUMMARY.md`, from the tables of the `README.md` files;
- the crate table of `docs/index.md`, from the `description` field of each
  `Cargo.toml` file;
- the crate table of `docs/user/reference/crates.md`, from the same fields.

The command `cargo xtask site check` returns an error when one of these parts
is stale.

A page of `docs` also reads on GitHub, so its links name files of the
repository. The preprocessor of links changes a link to a page into a link
that works on the site. It changes a link to any other file or folder into a
link to GitHub on the branch `main`. The build checks every internal link of
the built pages.

The preprocessor does not parse Markdown in full. It does not skip a code span
that crosses a line break. It does not recognise a fence that has an indent of
four or more spaces. The link check of the built pages still catches a broken
result of either limit.

mdBook has one release in two places. The step of `checks.yml` names the
release in the line `mdbook@0.5.4`, and `flake.lock` pins the nixpkgs that
holds the mdBook of the shell. Dependabot raises the tag of the action, and it
does not raise the number after `mdbook@`. A person raises both places
together.

The site shows the last commit of `main`. [ADR 0068](0068-the-site-shows-one-version.md)
states the question that this leaves open.

## Measurements

- Plain `mdbook build` with a Markdown file that `SUMMARY.md` does not name
  returned 0, and it printed lines of the level INFO alone. The generator of
  the sidebar therefore returns an error for such a file.
- A directive of mdBook that includes a file, written inside back quotes in an
  included file, made mdBook print
  `ERROR Stack depth exceeded in a.md. Check for cyclic includes`. The page
  `docs/contributor/rules.md` includes `CLAUDE.md`, so `CLAUDE.md` holds no
  such directive.
- With `src = "."`, mdBook copies each file of `docs` that is not Markdown into
  the output.
- With `create-missing = false`, a name in `SUMMARY.md` that has no file makes
  mdBook print `ERROR failed to read chapter` and the name.
- `cargo doc --no-deps` writes no `index.html` at the root of `target/doc`. The
  crate table of the site therefore links to the page of each crate.
- A preprocessor of mdBook runs with the current directory set to the root of
  the book, which is `docs`. It reads the JSON array `[context, book]` from the
  standard input. With no `before` key, the preprocessor `site-links` ran
  after the built-in preprocessors, and it then read the included text of
  `CLAUDE.md`. `before = ["index", "links"]` makes it run first.
- The API documentation of the workspace measured 7.9 megabytes. Each crate of
  the workspace holds the version 0.0.0.
