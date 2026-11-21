# The Nix flake

Nix is a package manager. A flake is the file that tells Nix what a repository
offers. The file `flake.nix` at the root of this repository is the flake of
thunderbox, and the file `default.nix` holds the package of the program.

The guide [Install thunderbox](../how-to/install-thunderbox.md) installs the
program with Nix.

## Inputs

An input is a repository that a flake reads.

| input | what the flake takes from it |
|---|---|
| `nixpkgs` | The package set, from the branch `nixpkgs-unstable`. |
| `rust-overlay` | Each release of Rust. This input follows the `nixpkgs` of the flake. |

The file `flake.lock` pins one commit of each input.

## Systems

The flake makes each output for these systems:

- `x86_64-linux`;
- `aarch64-linux`;
- `aarch64-darwin`.

In the names below, `SYSTEM` stands for the name of one of these systems.

## Outputs

| output | what it is |
|---|---|
| `packages.SYSTEM.default` | The program `thunderbox`. |
| `devShells.SYSTEM.default` | A shell that holds each tool that the project needs. |
| `checks.SYSTEM.default` | The package. The build of the package runs the tests. |
| `checks.SYSTEM.shell` | The shell. |
| `formatter.SYSTEM` | The program `nixfmt-tree`, which gives one format to each Nix file. |
| `overlays.default` | An overlay that adds `pkgs.thunderbox` to the package set of a configuration. |

### Output `packages.SYSTEM.default`

- The package uses the release of Rust that `rust-toolchain.toml` names. The
  package, the shell, and rustup therefore use one release.
- The package holds the feature `prometheus`, so it holds every data source.
- The build reads the files `Cargo.toml` and `Cargo.lock`, and the directories
  `crates` and `dashboards`. A change to any other file starts no new build.
- The build links the library `aws-lc` of nixpkgs. The data source
  `prometheus` needs that library.
- The check phase is the part of the build that runs the tests. It runs
  `cargo test` for the whole workspace. The package `cacert` gives that phase a
  file of certificates through the environment variable `SSL_CERT_FILE`.
- The name of the program is `thunderbox`. Its licence is the licence of the
  file `LICENSE-MIT`, or the licence of the file `LICENSE-APACHE`.

A person can replace an argument of the package with `override`. This
expression builds the program with another `rustPlatform`:

```nix
packages.SYSTEM.default.override { rustPlatform = another; }
```

### Output `devShells.SYSTEM.default`

The shell holds these tools:

- the release of Rust that `rust-toolchain.toml` names, with `rustfmt` and
  `clippy`;
- `rust-src` and `rust-analyzer`. An editor reads the code through
  `rust-analyzer`;
- `python3`, which runs the files of `scripts`;
- `agg`, which makes a picture in the GIF format from a cast file. A cast file
  holds what a terminal drew. This `agg` reads the font Unifont from the store
  of Nix, which is the directory where Nix keeps each package.

The shell holds no copy of the package, so `nix develop` compiles nothing.

The file `.envrc` tells the program direnv to load this shell. direnv loads a
shell when a person enters a directory. Run `direnv allow` one time to permit
it.

### Output `overlays.default`

- A configuration of NixOS or of home-manager can take this overlay. The
  configuration then holds `pkgs.thunderbox`.
- That package comes from the nixpkgs of the configuration. It uses the release
  of Rust of that nixpkgs.
- Cargo returns an error when that release is older than 1.88. The field
  `rust-version` of `Cargo.toml` names that release.

## Commands

| command | what it does |
|---|---|
| `nix run github:rellen/thunderbox` | Build the program, and start it with the dashboard inside the program. The command needs no copy of this repository. |
| `nix run github:rellen/thunderbox -- my-dashboard.toml` | Build the program, and start it with a dashboard file. |
| `nix profile add github:rellen/thunderbox` | Install the program for one person. An older release of Nix names this command `nix profile install`. |
| `nix build` | Build the package from a copy of this repository. |
| `nix develop` | Start the shell of the flake. Run it from the root of the workspace. |
| `nix flake check` | Build each check: the package, with its tests, and the shell. |
| `nix fmt` | Give one format to each Nix file of this repository. |
| `nix flake update rust-overlay` | Move the input `rust-overlay` to its newest commit. |

The input `rust-overlay` holds no release of Rust that is newer than its own
commit. Nix reports a fault when `rust-toolchain.toml` names such a release. The
command `nix flake update rust-overlay` then repairs the fault.

## Pages that give the reason

- [Architecture decision record 0055](../../contributor/adr/0055-build-the-package-with-a-nix-flake.md)
  states why the project builds the package with a flake.
- [Architecture decision record 0056](../../contributor/adr/0056-the-check-phase-of-the-package-reads-a-ca-file.md)
  states why the check phase reads a file of certificates.
