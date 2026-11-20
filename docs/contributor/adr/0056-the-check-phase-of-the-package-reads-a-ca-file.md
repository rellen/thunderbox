# ADR 0056: The check phase of the package reads a CA file

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `CLAUDE.md` section 4.4 |
| Code | `default.nix`, `crates/thunderbox-source-prometheus/src/lib.rs` |
| Tests | No Rust test holds the decision. `nix flake check` holds it, because 22 tests fail in the sandbox without `cacert`. The test `the_error_of_the_client_names_its_cause` shows the fault, and it passes with or without `cacert`. |
| Related | [ADR 0055](0055-build-the-package-with-a-nix-flake.md), [ADR 0064](0064-plain-http-needs-no-ca-file.md) |

## Context

The check phase is the step of the build of the package that runs
`cargo test` for the whole workspace. The sandbox of Nix runs that step, and
the sandbox holds no certificate of a CA.

On Linux, a client of the data source prometheus reads the certificates of
each CA from the machine. It returns an error when it finds none, even for an
address of plain HTTP.

## Decision

The check phase needs a file of CA certificates. `nativeCheckInputs` of
`default.nix` therefore names `cacert`.

A setup hook is a script that a package of Nix runs in each build that takes
that package as an input. The setup hook of `cacert` sets the variable
`SSL_CERT_FILE`. The check phase then reads a file of certificates through
that variable.

## Consequences

`cargo test` cannot show the fault on a machine that holds a CA certificate.
Cargo replaces a variable `SSL_CERT_FILE` that names a file which does not
exist. The test then reads the file of the machine.

Run the program of a test directly to see the fault. The test
`the_error_of_the_client_names_its_cause` starts such a program.

The need for a file of certificates stays.
[ADR 0064](0064-plain-http-needs-no-ca-file.md) asks whether a client of a plain
HTTP address must read any certificate at all.

## Measurements

Without a CA certificate, 22 tests fail. The log of the workflow `flake` named
3 of them, because `cargo test` stops after the first target that fails.
