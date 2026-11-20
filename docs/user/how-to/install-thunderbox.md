# Install thunderbox

This guide installs the program `thunderbox`. Select one of the ways below.

## Install with Cargo from GitHub

1. Run `rustc --version`. The release must be 1.88 or later.
2. Run this command:

   ```
   cargo install --locked --git https://github.com/rellen/thunderbox thunderbox
   ```

3. Run `thunderbox --version` to check that the command works.

The program writes its version, for example `thunderbox 0.0.0`. Cargo puts the
program in the folder `~/.cargo/bin`, and rustup adds that folder to the
variable `PATH`.

## Install with Cargo from a copy of the repository

1. Run these commands:

   ```
   git clone https://github.com/rellen/thunderbox
   cd thunderbox
   cargo install --locked --path crates/thunderbox
   ```

2. Run `thunderbox --version` to check that the command works.

The file `rust-toolchain.toml` of the copy names the release of Rust that Cargo
uses inside the copy. Cargo can report that your machine does not hold that
release. When it does, run `rustup toolchain install` in the copy. Then run the
`cargo install` command again.

To run the program and not install it, run
`cargo run -p thunderbox -- my-dashboard.toml` in the copy.

## Install a build that holds no Prometheus data source

1. Add `--no-default-features` to the `cargo install` command of a section
   above.

That build holds the data sources `sysinfo` and `testdata` alone, and it holds
no client for HTTP. A dashboard file that declares a data source of the kind
`prometheus` then stops the program. The file `dashboards/prometheus.toml` gives this error:

```
Error: The program cannot make the data sources of the dashboard.

Caused by:
    The dashboard is not correct. the data source "prom" has the type "prometheus", and this program makes these types: sysinfo, testdata
```

## Install with Nix

Nix is a package manager. A flake is the file that tells Nix what a repository
offers, and the repository of thunderbox holds one.

1. Run this command to install the program for your account:

   ```
   nix profile add github:rellen/thunderbox
   ```

2. Run `thunderbox --version` to check that the command works.

An older release of Nix names the command of step 1 `nix profile install`.

The flake of the repository supplies its own release of Rust, so this way needs
no rustup. The flake offers the program for these systems:

- `x86_64-linux`;
- `aarch64-linux`;
- `aarch64-darwin`.

These commands use the features `nix-command` and `flakes` of Nix. Add
`--extra-experimental-features "nix-command flakes"` to each command when your
configuration of Nix does not enable those features.

This command runs the program one time, and it does not install it:

```
nix run github:rellen/thunderbox -- my-dashboard.toml
```

A configuration of NixOS or of home-manager can name the flake as an input,
which is a repository that the configuration reads.
[The Nix flake](../reference/nix-flake.md) states each output of the flake.

## The pages for the facts and the reasons

- On Linux, the data source `prometheus` needs the certificate of each
  certificate authority. Read
  [Give the Prometheus source a CA file](give-the-prometheus-source-a-ca-file.md).
- Read [The command line](../reference/command-line.md) for the arguments and
  the keys.
