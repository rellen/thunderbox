# ADR 0068: The site shows one version

| field | value |
|---|---|
| Status | Proposed |
| Recorded | 2026-11-21 |
| Source | new |
| Code | `.github/workflows/checks.yml`, `docs/book.toml` |
| Tests | No test holds the question. |
| Related | [ADR 0066](0066-the-documentation-is-one-mdbook-site.md), [ADR 0067](0067-five-other-designs-for-the-site-failed.md) |

## Context

The site holds the last commit of `main`. A reader of an older release of
thunderbox therefore reads the documentation of the newest commit. That
documentation can describe a field, a key, or a panel that the older release
does not hold.

docs.rs gives one page for each version of a crate. It does so only for a
crate that reaches crates.io. No crate of this workspace is there, because each
crate holds the version 0.0.0.

## Decision

The project takes no decision. The site shows one version, which is the last
commit of `main`.

## Consequences

A reader of an older release must read the files of `docs` at the tag of that
release on GitHub. The site gives that reader no help.

## The question that remains

The work that remains gives a reader of an older release the documentation of
that release. The work is worth little while the project makes no release. The
project has made none today.

Two designs could do this work, and nobody has examined either one:

- The project publishes each crate to crates.io. docs.rs then gives one page
  for each version.
- The site holds a folder for each release, and the workflow builds a folder
  when a tag arrives.

The next design answers one question. Does the project publish to crates.io, or
does the site hold a folder for each release?
