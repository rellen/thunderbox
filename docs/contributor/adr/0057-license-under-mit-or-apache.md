# ADR 0057: License the project under MIT or Apache 2.0

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 13 item 5 |
| Code | `Cargo.toml`, `LICENSE-MIT`, `LICENSE-APACHE`, `README.md`, `docs/contributor/guide.md`, `default.nix` |
| Tests | No test holds this decision. |
| Related | None. |

## Context

The list of the decisions that remained open held the licence of the project
as one item. The repository now states its licence, so the project closed that
item.

## Decision

A person can use thunderbox under the MIT License, or under the Apache
License, version 2.0. That person selects one of the two.

- The table `[workspace.package]` of `Cargo.toml` sets
  `license = "MIT OR Apache-2.0"`, and every crate takes that field.
- `LICENSE-MIT` and `LICENSE-APACHE` hold the two texts.
- The section "Licence" of `README.md` states the choice.
- The section "The licence of a contribution" of `docs/contributor/guide.md`
  states the terms of a contribution.

## Consequences

`LICENSE-MIT` names The Thunderbox contributors as the holder of the
copyright. `LICENSE-APACHE` names no holder. Its appendix keeps the blank form
`Copyright [yyyy] [name of copyright owner]`.

A contributor licenses each contribution under both licences, with no other
terms, unless that contributor states otherwise.

The package of `default.nix` states both licences in its field `meta.license`.
