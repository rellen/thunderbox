# ADR 0004: Design for the way inside another program first

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 1 |
| Code | `crates/thunderbox-widgets/Cargo.toml`, `crates/thunderbox-widgets/examples/panel_embed.rs` |
| Tests | No test holds this decision. The table `[dependencies]` of `crates/thunderbox-widgets/Cargo.toml` holds the first constraint, and no test reads that table. |
| Related | [ADR 0005](0005-one-trait-is-the-seam-of-every-data-source.md), [ADR 0013](0013-one-connection-carries-both-ways-over-ssh.md), [ADR 0015](0015-both-ways-into-another-program-are-in-scope.md), [ADR 0059](0059-version-1-may-need-no-server.md) |

## Context

The design names four ways to run thunderbox. These four ways are not four
products. Two questions separate them, and one more question applies to all of
them. The code holds two of the four ways today. The other two ways are plans.

| way to run | number of processes | where the program draws | who owns the terminal | in the code today |
|---|---|---|---|---|
| alone | one | in the same process | thunderbox | yes |
| client and server | two | in the client | thunderbox | no |
| through SSH | two | in the server | thunderbox | no |
| inside another program | one | in the same process | the other program | yes |

The place where the data lives is not a question in this table. A Prometheus
data source reads another machine. A `sysinfo` data source reads this machine.
Neither choice changes the shape of thunderbox. The choice of a data source
answers that question, and nothing else does.

The project doubts the "client and server" way. thunderbox is a dashboard
layer, and not a store. A thunderbox server that only passes a query to
Prometheus adds a step and adds no value. Grafana is a dashboard program that
runs in a web browser, and it works in that way for one reason only. A web
browser cannot hold a password, and a web browser cannot reach a private
network. Three reasons for a thunderbox server remain:

- the viewer cannot reach the data source, but the server can;
- one collector serves many viewers, which matters when collection is
  expensive;
- the server holds a cache, or limits how often it queries the data source.

The SSH way would answer the first reason, and it is simpler.

The way inside another program is the hardest of the four. An async runtime is
a library that runs concurrent tasks. A project that ignores the way inside
another program must later separate the async runtime from the code that draws.

## Decision

The project designs for the way inside another program first. The other three
ways then follow. The rule gives two constraints:

1. **`thunderbox-widgets` depends on `ratatui-core` and `ratatui-widgets`
   only.** A terminal backend is the code that writes to a real terminal. The
   crate uses no terminal backend, no `crossterm`, and no `tokio`, and it reads
   no file. A program that already owns a terminal must be able to call
   `f.render_widget(TimeSeriesPanel::new("cpu", PanelData::Frames(&frames), &theme), area)`,
   and nothing else. The crate `chandelier` of another project draws its
   charts in that way today.
2. **The code that draws is synchronous, and it reads data that it owns.** No
   `async` code appears where the program draws a frame. Another task fills the
   data before the program draws. Only that step changes between the four ways
   to run thunderbox.

The project treats "client and server" as a later option on the layer of the
data source. It is not a fourth way to run thunderbox. The project will build
it only when a need appears that the SSH way cannot meet.

## Consequences

`thunderbox-widgets` also depends on `thunderbox-core`. That crate holds no
ratatui and no tokio, as [ADR 0005](0005-one-trait-is-the-seam-of-every-data-source.md)
states.

The example `panel_embed` of `thunderbox-widgets` is a program that owns its
terminal. It draws thunderbox panels beside its own text with
`f.render_widget`.

[ADR 0015](0015-both-ways-into-another-program-are-in-scope.md) states the two
ways into another program, and the rules that the library then holds.
[ADR 0059](0059-version-1-may-need-no-server.md) asks whether version 1 needs a
server at all.
