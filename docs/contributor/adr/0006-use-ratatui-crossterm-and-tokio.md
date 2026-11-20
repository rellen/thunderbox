# ADR 0006: Use ratatui, crossterm, and tokio, and no other library of that kind

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 4 |
| Code | `Cargo.toml`, the `Cargo.toml` file of each crate |
| Tests | No test holds this decision. The `Cargo.toml` files hold this decision, and no test reads them. |
| Related | [ADR 0004](0004-design-for-the-way-inside-another-program-first.md), [ADR 0008](0008-draw-each-chart-through-a-wrapper-of-the-project.md), [ADR 0013](0013-one-connection-carries-both-ways-over-ssh.md) |

## Context

[The research](../research.md) records the evidence for the choice of each
library. Each download count there is the count for 90 days, from crates.io,
on 26 August 2026.

ratatui is the only Rust library for a terminal user interface that supplies
charts. It has a `Chart` type, a `Dataset` type, an `Axis` type, a
`Sparkline`, a `BarChart`, and a `Canvas` that can draw inside one terminal
cell. Every other library supplies boxes and text, and a project must then
write its own charts. ratatui had 16,749,238 downloads in 90 days.

Version 0.30.0 of ratatui arrived on 26 December 2025, and it added the parts
that this project needs:

- `GraphType::Area` and `Dataset::fill_to_y(f64)` fill the area below a line.
- The markers `Octant`, `Sextant`, `Quadrant`, and `Custom(char)` join
  `Braille`.
- A braille character now draws above a block character.
- ratatui divided itself into `ratatui-core`, `ratatui-widgets`, and
  `ratatui-macros`, and it moved each terminal backend into its own crate. This
  change is the reason why a program can hold a thunderbox panel.
- The `Canvas` type now rounds a position correctly.

The project built its first five panels against ratatui alone. No panel needed
another library, and no panel needed a copy of a ratatui file. The top panel
and the row panel came later, and they follow the same rule.

`russh` 0.63.1 is an SSH client and an SSH server. Its authors wrote it in Rust
alone, and it runs on tokio. It had 2,480,749 downloads in 90 days. Its
repository holds two examples that draw with ratatui.

## Decision

The project uses these libraries. The block holds the lines of the workspace
`Cargo.toml`:

```toml
ratatui = { version = "0.30.2", default-features = false, features = ["crossterm", "layout-cache"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "time"] }
# in thunderbox-widgets alone:
ratatui-core = "0.1"
ratatui-widgets = "0.3"
```

crossterm is the library that controls the terminal. No crate of the workspace
declares it directly. The program and the two examples read it as `ratatui::crossterm`, and
`Cargo.lock` holds `crossterm` 0.29.0.

The way through SSH will add two libraries. No crate holds them today, and
[ADR 0013](0013-one-connection-carries-both-ways-over-ssh.md) states that way:

- `russh` 0.63, for the SSH server;
- `termwiz` 0.23, to read the keys alone. The function
  `crossterm::event::read()` reads the standard input, and an SSH server cannot
  use the standard input.

thunderbox uses no other chart library, and no other library for a terminal
user interface.

## Consequences

The first form of this decision, in `docs/ARCHITECTURE.md`, named two more
items. It named `crossterm` 0.29 as a dependency of the workspace, and it
turned on the feature `unstable-backend-writer` of ratatui. No commit of this
repository declared either item in a `Cargo.toml` file. The block above
therefore holds the lines of `Cargo.toml`, so that a reader who copies it gets
the real declarations.

ratatui holds these limits:

- A `Dataset` holds one `Style` for a whole series. A point therefore cannot
  have its own colour.
- Unicode added the octant and sextant characters recently. A terminal can show
  a replacement character in their place, so braille stays the default marker.
- A chart can need four parts that `Chart` does not hold:
  - a time axis that ends at the right;
  - a type for the bound of an axis;
  - a legend in a chosen place;
  - a logarithmic axis.

  [ADR 0008](0008-draw-each-chart-through-a-wrapper-of-the-project.md) states
  how the project can add those parts later.

## Designs that the project rejected

The project rejected these libraries for a terminal user interface:

- **cursive** 0.21.1 supplies no chart, and it made no release in two years.
- **iocraft** 0.8.5 writes text to a command line. It supplies no chart, and it
  cannot draw inside one terminal cell.
- **tuie** 0.2.4 had 444 downloads in 90 days. It is the work of one person,
  and it is not a base for a project.

The project rejected these chart crates: `tui-piechart`, `chandelier`,
`ratatui-plt`, `malevich`, `ratatui-stacked-bar`, `waveformchart`, `trazo`,
and `tuika-charts`. None of them is worth a dependency here. The project can
examine `malevich` again later, because it holds code for statistics, and it
works with `ratatui-core`. The crates `ratatui-chart`, `ratatui-barchart`,
`ratatui-sparkline`, and `ratatui-treemap` hold no code.

The project rejected these crates that wrap `russh`: `frittura-ssh-core`,
`chai-framework`, `charmed-wish`, `sshui`, `sshdance`, and `tuihost`. Each is
the work of one person, and few people use any of them. The example of `russh`
is about 270 lines, so the project will copy it.

OpenSSH with `ForceCommand` is the other method for the SSH way. `ForceCommand`
is the option of OpenSSH that runs one program for each login. OpenSSH supplies
the management of the keys, the check of a password, and PAM. The project takes
`russh` when one of these conditions applies:

- one process must serve many people from shared state;
- thunderbox must check the password itself;
- the project must ship one file.

A dashboard meets those conditions.

[The research](../research.md) gives the count of the downloads of each crate,
and the full reason for each choice.
