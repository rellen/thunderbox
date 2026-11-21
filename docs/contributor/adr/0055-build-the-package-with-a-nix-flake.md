# ADR 0055: Build the package with a Nix flake

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `CLAUDE.md` section 4.4 |
| Code | `flake.nix`, `default.nix`, `flake.lock`, `rust-toolchain.toml`, `.github/workflows/flake.yml` |
| Tests | No Rust test holds the decision. `nix flake check` builds the package, which runs `cargo test`, and it builds the shell. |
| Related | [ADR 0056](0056-the-check-phase-of-the-package-reads-a-ca-file.md), [ADR 0064](0064-plain-http-needs-no-ca-file.md) |

## Context

Nix is a package manager. A flake is the file that tells Nix what a repository
offers. nixpkgs is the collection of the packages of Nix.

`rust-toolchain.toml` pins one release of Rust. rustup is the program that
reads that file and selects that release. A machine that holds no rustup runs
the seven commands of `CLAUDE.md` section 4 with another release, and it
reports nothing.

## Decision

The project builds its package, and it supplies its tools, through a Nix
flake:

- `flake.nix` offers the package of the program, and `default.nix` holds that
  package.
- `flake.nix` also offers a shell that holds each tool of `CLAUDE.md` section 4
  and section 6.
- `README.md` gives the command that runs the program through Nix.

`nix develop` starts the shell. The flake reads the release of Rust from
`rust-toolchain.toml`, so the seven commands then run with the pinned release.

The shell adds two components to that release, `rust-src` and `rust-analyzer`,
for an editor. It also holds `python3`, and it holds `agg` with the directory
of the font Unifont.

Run this command after a change to a Nix file, to `flake.lock`, to
`Cargo.lock`, or to `rust-toolchain.toml`:

```
nix flake check
```

The command builds each check of `flake.nix`. The first check is the package,
and the build of the package runs `cargo test` for the whole workspace. The
second check is the shell.

### Workflow

`.github/workflows/flake.yml` runs that command on Linux. `checks.yml` reads no
Nix file, so the workflow `flake` is the one check that builds the flake.

The workflow starts only for a change to a file that its list `paths` names. A
change under `crates` starts no run. Press the manual button of that workflow
after such a change when the change can alter the build.

### Systems

The flake offers three systems, and one of them has no build:

| system | who built the package |
|---|---|
| `x86_64-linux` | The workflow `flake` builds it. |
| `aarch64-darwin` | A person built it, with the sandbox of Nix on. |
| `aarch64-linux` | Nobody built it. |

The sandbox of Nix separates a build from the network and from the files of
the machine.

## Consequences

Four rules follow from the way that the flake builds the package.
[ADR 0056](0056-the-check-phase-of-the-package-reads-a-ca-file.md) states a
fifth rule.

- **The package names each path that the build reads.** The list `fileset` of
  `default.nix` holds `Cargo.toml`, `Cargo.lock`, `crates`, and `dashboards`.
  `crates/thunderbox/src/sources.rs` includes three files of `dashboards`. Code
  that includes a file from another directory compiles with `cargo`, and
  `nix build` then returns an error. Add the path to that list.
- **The overlay does not use the pinned release.** An overlay adds a package to
  the package set of another configuration. `packages.default` gives
  `default.nix` the release that `rust-toolchain.toml` names.
  `overlays.default` gives it the nixpkgs of the configuration that takes the
  overlay, and that nixpkgs holds a release of its own. No check builds the
  package of the overlay. A person built it one time, with the nixpkgs that
  `flake.lock` pins.
- **A new release of Rust can need a new `flake.lock`.** The flake takes each
  release from its input `rust-overlay`, and `flake.lock` pins one commit of
  that input. Dependabot raises the release in `rust-toolchain.toml` alone. Run
  `nix flake update rust-overlay` when Nix reports that it does not hold the
  release.
- **The package links one C library that `cargo` alone does not link.**
  nixpkgs tells the crate `aws-lc-sys` to link the library `aws-lc` of
  nixpkgs. `cargo` alone compiles that library from the source inside the
  crate. The package therefore needs `pkg-config` and `aws-lc`, and
  `default.nix` names both.

The build of the package runs a test that starts a server on the machine. The
sandbox of macOS forbids that, so `default.nix` sets
`__darwinAllowLocalNetworking`.

## Measurements

One machine held no rustup, and its `cargo` reported release 1.96.1. Inside
the shell on that machine, `cargo` reported release 1.98.0.
