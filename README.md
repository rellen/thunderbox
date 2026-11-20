# thunderbox

A dashboard platform for the terminal. It has the shape of Grafana. A person
writes a dashboard as a document, and the dashboard reads its data from data
sources that a person can add.

thunderbox stores a dashboard. It never stores a measurement. The system that
thunderbox queries keeps the measurements.

![thunderbox draws every type of panel of its built-in dashboard over the data source testdata.](docs/media/demo.gif)

## Run it

```
cargo run -p thunderbox
```

The program reads this machine. It draws the processor, each core, the memory,
the network, and the processes. The picture above draws the same panels over
the data source testdata. No picture of this project shows data of the machine
that records it.

Name a file to show your own dashboard. This command shows a dashboard of the
repository, which reads the test data:

```
cargo run -p thunderbox -- dashboards/rows.toml
```

Nix runs the program with no copy of this repository:

```
nix run github:rellen/thunderbox
```

Press `Tab` to select the next panel, and press `q` to stop the program.
[The command line](docs/user/reference/command-line.md) lists every key.

## The documentation

The site [rellen.github.io/thunderbox](https://rellen.github.io/thunderbox/)
holds the user documentation, the contributor documentation, and the API
documentation of each crate.

The documentation for a person who uses thunderbox sits in
[docs/user](docs/user/README.md), and it has four kinds:

| kind | start here |
|---|---|
| tutorials | [Watch this machine](docs/user/tutorials/watch-this-machine.md), then [Write your first dashboard](docs/user/tutorials/write-your-first-dashboard.md) |
| how-to guides | [The list of tasks](docs/user/README.md#the-how-to-guides) |
| reference | [The dashboard file](docs/user/reference/dashboard-file.md), [The panels](docs/user/reference/panels/README.md), [The data sources](docs/user/reference/data-sources/README.md) |
| explanations | [How thunderbox works](docs/user/explanation/how-thunderbox-works.md) |

The documentation for a person who changes thunderbox sits in
[docs/contributor](docs/contributor/README.md). It holds the architecture, one
record for each decision, and the rules of each change.

## Licence

thunderbox is free software. You can use it under the MIT License, or under the
Apache License, version 2.0. Select the licence that suits you. `LICENSE-MIT`
and `LICENSE-APACHE` hold the two texts. [The contributor
guide](docs/contributor/guide.md#the-licence-of-a-contribution) states the
terms of a contribution.
