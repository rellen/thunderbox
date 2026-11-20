# ADR 0011: Add a data source as code behind a feature

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 10 |
| Code | `crates/thunderbox-source/src/lib.rs`, `crates/thunderbox/src/sources.rs`, `crates/thunderbox/Cargo.toml` |
| Tests | `build_makes_a_prometheus_data_source`, `build_returns_an_error_for_a_type_that_this_program_does_not_hold` |
| Related | [ADR 0005](0005-one-trait-is-the-seam-of-every-data-source.md), [ADR 0012](0012-the-five-duties-of-a-data-source.md), [ADR 0013](0013-one-connection-carries-both-ways-over-ssh.md) |

## Context

Grafana is a dashboard program that runs in a web browser. Grafana runs a data
source as a separate program, and it speaks gRPC to that program. gRPC is a
protocol that calls a function in another process.

The rules that govern the layout of a Rust value in memory are not stable
between two releases of the Rust compiler.

## Decision

The project does not run a data source as a separate program now. It writes
each data source as an implementation of the trait of
[ADR 0005](0005-one-trait-is-the-seam-of-every-data-source.md), and a feature
selects it. A feature is an option of Cargo that adds code to a build.

`DataSource` stays free of any assumption that the data source runs in this
process. The request and the result are owned values, and the trait holds no
borrowed value.

thunderbox will not load a shared library at run time, because of the layout
rule above.

## Consequences

A later version can run a data source in a separate process, or as a
WebAssembly module. Neither change alters a panel.

The program `thunderbox` holds the feature `prometheus`, which is on by
default. A build without that feature reads this machine and the test data
alone. `crates/thunderbox/src/sources.rs` makes one data source for each
declaration of a dashboard, and it returns an error for a type that the build
does not hold.
