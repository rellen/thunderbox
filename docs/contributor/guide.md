# The contributor guide

This guide is for a person who changes thunderbox. It names the documents to
read, the tools to install, and the rules that each change follows.
[CLAUDE.md](../../CLAUDE.md) holds the full rules. Claude reads that file
before each change, and the rules in it bind every contributor.

## Read these documents first

1. [The architecture](architecture.md) describes the parts of the program.
2. [The decision records](adr/README.md) state each decision, and the reason
   for it. Read the records that govern the part that you change.
3. Section 1 of [CLAUDE.md](../../CLAUDE.md) states the style of every
   document. [ADR 0002](adr/0002-write-every-document-in-simplified-technical-english.md)
   states why the project keeps that style.

## Prepare the tools

The shell of the flake holds each tool that this project needs. Run this command
from the root of the workspace:

```
nix develop
```

The shell holds these tools:

- the release of Rust that `rust-toolchain.toml` names, with `rustfmt` and
  `clippy`;
- `rust-analyzer`, which is the program that reads the code for an editor;
- `python3`, which runs the two files of `scripts`;
- `agg`, together with the font Unifont.

direnv is a program that loads a shell when a person enters a directory. The
file `.envrc` tells direnv to load the shell of the flake. Run `direnv allow`
once to permit that.

### The release of Rust

This project pins the release of Rust. The file `rust-toolchain.toml` names it.
rustup reads that file, and it then uses that release for every cargo command
inside this directory tree. The pin holds the output of clippy and of
`cargo fmt` still, for a person and for the checks that GitHub runs.

rustup installs the release by itself when the machine does not hold it. A
person who turned that behaviour off runs this command once:

```
rustup toolchain install
```

The command reads `rust-toolchain.toml`. It installs the release, the profile,
and the components that the file names.

rustup reads this file, and the flake of this project reads it also. A `rustc`
that another package manager installed ignores the file, and it reports
nothing. Install rustup from https://rustup.rs to get the pinned release, or
use the shell of the flake.

The flake takes each release of Rust from its input `rust-overlay`, and
`flake.lock` pins one commit of that input. That commit does not hold a release
that is newer than itself. Run this command when you raise the release and Nix
then reports that it does not hold it:

```
nix flake update rust-overlay
```

## Each feature holds its documentation

A change that adds a feature, or that changes what a person sees or does,
holds its documentation in the same pull request. Section 8 of
[CLAUDE.md](../../CLAUDE.md) states this rule in full. In short, the change
holds two kinds of documentation.

**The user documentation** sits in [docs/user](../user/README.md). It has four
kinds, and the change updates each kind that the feature reaches:

| kind | when the change needs it |
|---|---|
| reference | always. Each new field, key, kind of panel, data source, query, option, or message takes an entry. |
| how-to guide | when the feature serves a task that a person names. |
| explanation | when the feature changes how a person must think about thunderbox. |
| tutorial | when a beginner meets the feature on the first path through the program. |

**The contributor documentation** sits in this folder:

- one [decision record](adr/README.md) for each decision that the change takes;
- one record of the status Proposed for each question that the change leaves
  open;
- one record of the status Rejected for each design that the change examined
  and did not take. That record names the fault that a reader measured in the
  design;
- a change to [the architecture](architecture.md) when the parts or the way
  between them change, and a row in its table of the features.

Every page follows the style of section 1 of [CLAUDE.md](../../CLAUDE.md).

## Check a change

Run the seven commands of section 4 of [CLAUDE.md](../../CLAUDE.md) before each
commit. The workflow `checks` runs the same commands in the same order.

A test that passes states nothing by itself. Take each rule that the change
adds out of the code, run `cargo test --no-fail-fast`, and read which tests
fail. Section 4.3 of [CLAUDE.md](../../CLAUDE.md) states that step.

A change to a Nix file, to `flake.lock`, to `Cargo.lock`, or to
`rust-toolchain.toml` also runs `nix flake check`. Section 4.4 of
[CLAUDE.md](../../CLAUDE.md) states why.

## Show the change in a pull request

- A change that alters the screen holds a picture. [Make the pictures
  again](pictures.md) states how, and section 6 of [CLAUDE.md](../../CLAUDE.md)
  states where the picture goes.
- A pull request of more than one commit takes a title that covers every
  commit. Section 5 of [CLAUDE.md](../../CLAUDE.md) states the steps.
- The repository merges a pull request with a rebase. Section 7 of
  [CLAUDE.md](../../CLAUDE.md) states what follows from that.

## The licence of a contribution

thunderbox is free software under the MIT License or the Apache License,
version 2.0. [ADR 0057](adr/0057-license-under-mit-or-apache.md) records that
decision.

Unless you state otherwise, you license each contribution that you submit to
this project under both licences, with no other terms. The Apache License,
version 2.0, defines a contribution.
