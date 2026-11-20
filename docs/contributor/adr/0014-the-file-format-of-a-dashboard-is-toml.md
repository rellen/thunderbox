# ADR 0014: The file format of a dashboard is TOML

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-core/src/dashboard.rs` |
| Tests | `from_toml_and_to_toml_keep_the_same_dashboard`, `from_toml_returns_an_error_for_an_unknown_key` |
| Related | [ADR 0010](0010-the-program-that-runs-alone-is-a-dashboard-over-sysinfo.md) |

## Context

A person writes a dashboard as a document on disk. Grafana is a dashboard
program that runs in a web browser, and it stores a dashboard in a JSON format
of its own. That format is large, and another project controls
it and changes it.

## Decision

The file format of a dashboard is TOML. The project will write a reader for the
Grafana JSON format later.

## Consequences

A native format fits the panels and the data sources of this project. A reader
for the Grafana format will be an addition, and it will not change the native
format. No code holds such a reader today.

This choice keeps the model of a dashboard in this project. It does not tie
version 1 to a large format that another project controls and changes.
