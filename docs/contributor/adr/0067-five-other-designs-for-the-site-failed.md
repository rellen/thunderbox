# ADR 0067: Five other designs for the documentation site failed

| field | value |
|---|---|
| Status | Rejected |
| Recorded | 2026-11-21 |
| Source | new |
| Code | No code holds a rejected design. |
| Tests | No test reads this record. |
| Related | [ADR 0066](0066-the-documentation-is-one-mdbook-site.md) |

## Context

[ADR 0066](0066-the-documentation-is-one-mdbook-site.md) records the design
that the project took for the documentation site. The project also examined
five other designs, and it took none of them. This record names the fault that
a reader measured in each one.

## Decision

The project takes none of these five designs. Each design failed on the fault
that its item in the list below names.

## Consequences

A person who proposes one of these designs again must first remove the fault
that this record names.

## Designs that the project rejected

- **A second workflow, `docs.yml`.** The workflow would build the documentation
  a second time, because `checks.yml` already runs `cargo doc --no-deps`. It
  would also publish a commit whose checks fail, because the two workflows run
  at the same time and neither one waits for the other. The job `deploy` of
  `checks.yml` waits for each job of the matrix, so it publishes a commit that
  passed.
- **Zola.** Zola is a program that builds a site from Markdown files. It reads
  a page from its folder `content` alone, and each page must start with a block
  of settings. Every document of `docs` would then need a step that copies it
  and adds the block. Each template of Zola would also be text to write and to
  check against section 1 of `CLAUDE.md`. mdBook reads `docs` as it is.
- **The root page of rustdoc.** The option `--enable-index-page` makes rustdoc
  write an `index.html` at the root of `target/doc`. Stable rustdoc rejects it.
  The command `RUSTDOCFLAGS="-Zunstable-options --enable-index-page" cargo doc
  --no-deps` printed `error: 2 nightly options were parsed`. The crate table of
  the site links to the page of each crate instead.
- **A Python script for the generator.** The comments of such a script stand
  outside the reach of `scripts/ste-lint.py`, which reads files that end in
  `.rs` and in `.md` alone. The script would also hold no test that
  `cargo test` runs. The crate `xtask` holds both.
- **A sidebar that a person writes by hand.** Each new page would then need two
  edits: the page, and the line of the sidebar. A person forgets the second
  edit, and plain mdBook then skips the page without an error. The generator
  writes the sidebar from the tables of the `README.md` files, which a person
  already edits.
