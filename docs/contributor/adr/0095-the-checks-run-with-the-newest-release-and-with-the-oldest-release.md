# ADR 0095: The checks run with the newest release and with the oldest release

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-27 |
| Source | new |
| Code | `.github/workflows/checks.yml`, `rust-toolchain.toml`, `Cargo.toml`, `.github/dependabot.yml` |
| Related | [ADR 0055](0055-build-the-package-with-a-nix-flake.md), [ADR 0066](0066-the-documentation-is-one-mdbook-site.md) |

## Context

`Cargo.toml` declares `rust-version = "1.88"`. That is the oldest release that
compiles the code. The workspace uses edition 2024, which needs release 1.85.
The example `crates/thunderbox-widgets/examples/panel_embed.rs` holds a let
chain, which release 1.88 made stable. No record stated that reason.

`rust-toolchain.toml` pins `channel = "1.100.0"`, the newest stable release on
2026-11-27. `.github/dependabot.yml` asked Dependabot to raise that pin once
each month.

The workflow `checks` ran the nine checks of `CLAUDE.md` section 4 with the
pinned release on Linux and on macOS. One more step ran `cargo check` with
release 1.88.0 on Linux alone. The workflow therefore compiled the code with the
oldest release, and it never ran a test with that release.

A plain `cargo` that another package manager installed ignores
`rust-toolchain.toml`, and it reports nothing. On 2026-11-27, the `cargo` of
Homebrew on one machine reported release 1.96.1 while the pin named release
1.100.0. The checks on that machine did not run with the pinned release.

A person asked for two changes. Local tests and local work use the newest
release. The checks of each merge and of each pull request run the oldest
release and the newest release as a matrix.

## Decision

The project uses two names:

- The newest release is the release that `rust-toolchain.toml` names. The
  project keeps that pin at the newest stable release of Rust.
- The oldest release is the release that the field `rust-version` of
  `Cargo.toml` names.

The workflow `checks` runs a matrix of the operating system (`ubuntu-latest` and
`macos-latest`) and the release (`newest` and `oldest`). The name of a job shows
both, for example `checks on ubuntu-latest with the newest release`.

- The newest job runs the nine checks of `CLAUDE.md` section 4, in the same
  order as before.
- The oldest job reads the number from `Cargo.toml`, and it installs that
  release. It writes `RUSTUP_TOOLCHAIN` to `GITHUB_ENV`, so each later step of
  the job uses that release. A guard compares `rustc --version` with the number,
  and it fails when the two differ. The job then runs
  `cargo check --locked --workspace --all-features --all-targets`,
  `cargo test --locked`, and `cargo test --locked --doc`, and it runs nothing
  else.
- The steps of the site, and the upload of the site, run for Linux with the
  newest release alone. The job `deploy` waits for every job of the matrix.

`.github/dependabot.yml` asks Dependabot to look for a new release each week.
The pin then reaches a new release within a week.

`rust-toolchain.toml` keeps its pin. A comment of `Cargo.toml` states why the
number is 1.88. The comment also states that a person raises the number on
purpose, in a commit of its own, with a new record.

Every cargo command of `CLAUDE.md` runs with the newest release. A person runs
it inside the shell of the flake, for example `nix develop -c cargo test`, or in
a shell that direnv loaded, or with rustup. A machine that holds no rustup runs
the oldest check with this command:

```
nix shell --inputs-from . rust-overlay#rust_1_88_0 -c cargo test --locked
```

The name `rust_1_88_0` follows the number of `Cargo.toml`.

## Consequences

- Each run holds four jobs where it held two. The oldest jobs cost minutes on
  Linux and on macOS. The `fail-fast` default still cancels the macOS jobs when
  a Linux job fails.
- A test that passes only on a new release now fails an oldest job. The
  oldest job also runs the tests that read the machine through
  `thunderbox-source-sysinfo`.
- The pin still decides the output of clippy and of `cargo fmt`. The oldest job
  runs neither tool.
- A person who raises `rust-version` also changes the name `rust_1_88_0` in the
  command above, and writes a new record.
- A new release of Rust opens a pull request within a week. That pull request
  also starts the workflow `flake`, because `flake.nix` reads the pin.

## Measurements

The first measurement ran the oldest job on a macOS machine before the push. The
command was:

```
nix shell --inputs-from . rust-overlay#rust_1_88_0 -c sh -c 'cargo --version && CARGO_TARGET_DIR=target/oldest cargo check --locked --workspace --all-features --all-targets && CARGO_TARGET_DIR=target/oldest cargo test --locked && CARGO_TARGET_DIR=target/oldest cargo test --locked --doc'
```

`cargo --version` reported `cargo 1.88.0 (873a06493 2025-05-10)`. The check
compiled every target. All 36 result lines of `cargo test` and of
`cargo test --doc` reported `0 failed`, and the command returned the code 0. No
test failed with the oldest release.

The second measurement is one run of the workflow on GitHub, from the manual
button, on the branch of this change. All four jobs passed:

| job | result | time |
|---|---|---|
| Linux with the newest release | passed | 66 seconds |
| macOS with the newest release | passed | 90 seconds |
| Linux with the oldest release | passed | 128 seconds |
| macOS with the oldest release | passed | 251 seconds |

Each job with the oldest release installed release 1.88.0, and it passed the
check, the tests, and the doc tests. No job had a cache, so a later run takes
less time.

## Designs that the project rejected

- **The word `stable` in `rust-toolchain.toml` or in the matrix.** A new lint of
  clippy would then fail a pull request that did not cause it. Dependabot opens
  no pull request for that word, so nobody would read the new faults on
  purpose.
- **The oldest release on Linux alone.** The tests read the machine through a
  crate that holds a separate implementation for each operating system. A test
  can pass on one system and fail on the other.
- **Every check with the oldest release.** clippy and `cargo fmt` change their
  output with the release. The oldest release would report faults that the
  newest release does not report, or it would hide faults that it does report.
