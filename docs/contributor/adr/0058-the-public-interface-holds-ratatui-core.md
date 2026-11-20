# ADR 0058: The public interface holds `ratatui-core`

| field | value |
|---|---|
| Status | Proposed |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 13 item 1 |
| Code | `crates/thunderbox-widgets/src/lib.rs`, `crates/thunderbox-widgets/Cargo.toml`, `Cargo.toml` |
| Tests | No test holds this decision. |
| Related | [ADR 0004](0004-design-for-the-way-inside-another-program-first.md), [ADR 0006](0006-use-ratatui-crossterm-and-tokio.md), [ADR 0008](0008-draw-each-chart-through-a-wrapper-of-the-project.md) |

## Context

`ratatui-core` version 0.1.x appears in the public interface of
`thunderbox-widgets`. `Cargo.lock` holds version 0.1.2 today.

A program that holds a thunderbox panel therefore depends on that crate. Every
release of ratatui that breaks its interface also breaks the interface of
thunderbox for such a program.

## Decision

The project has not taken a decision. Three steps can limit that effect:

- keep the set of public types small;
- export `ratatui_core` under a second name;
- record the version.

## Consequences

The crate already takes the second step. `crates/thunderbox-widgets/src/lib.rs`
holds `pub use ratatui_core;`, and the comment on that line calls it a second
name for `ratatui_core`. A program can read the version of ratatui that
thunderbox uses from that name.

## The question that remains

The first step and the third step remain. Each one limits what a release of
ratatui breaks in a program that holds a thunderbox panel.

The next design answers one question. Which types of `ratatui-core` must the
public interface of `thunderbox-widgets` hold, and where does the project
record the version of that crate?
