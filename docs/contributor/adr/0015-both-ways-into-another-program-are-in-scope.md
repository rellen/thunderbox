# ADR 0015: Both ways into another program are in scope

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-widgets`, `crates/thunderbox-dashboard/src/runtime.rs`, `crates/thunderbox-dashboard/src/view.rs` |
| Tests | `the_runtime_stops_its_task_when_it_drops`, `the_runtime_stops_the_task_of_each_panel_when_it_drops`, `a_read_of_a_snapshot_never_waits_for_a_query`, `the_view_leaves_a_key_that_it_does_not_use_to_the_program`, `one_data_source_that_returns_an_error_changes_one_panel_only` |
| Related | [ADR 0004](0004-design-for-the-way-inside-another-program-first.md), [ADR 0005](0005-one-trait-is-the-seam-of-every-data-source.md), [ADR 0016](0016-a-program-that-holds-a-dashboard-holds-four-rules.md), [ADR 0028](0028-the-caller-reports-one-area-as-a-size.md) |

## Context

A program that already owns a terminal can hold thunderbox in two ways. It can
hold the renderers alone, or it can hold a whole dashboard.
[ADR 0004](0004-design-for-the-way-inside-another-program-first.md) asks the
project to design for that way first.

## Decision

Both ways to put thunderbox inside another program are in scope. They are two
layers, and not two choices.

The smaller way gives the renderers to a program that already holds its own
data. That way is `thunderbox-widgets`. It runs no query, and the program
supplies each `Frame`.

The larger way gives a whole dashboard to a program, and thunderbox runs the
queries and refreshes them. That way is `thunderbox-dashboard`. It depends on
the first way, and the program supplies a `tokio::runtime::Handle`.

`thunderbox-dashboard` behaves as a library:

- **It never makes an async runtime.** It takes a `Handle`. It uses no
  `#[tokio::main]` and no `Runtime::new()`.
- **It never owns the terminal.** It sets no terminal mode, it opens no second
  screen, and it reads no standard input.
- **It never owns the key bindings.** `handle_key` reports whether it used the
  key. The program keeps its own bindings for every key that thunderbox
  ignores. The program also decides whether the dashboard has the attention of
  the person, through `DashboardView::set_focused`. `DashboardView` holds the
  selected panel, and it moves the selection on the keys `Tab` and `BackTab`.
- **It changes nothing outside itself.** It installs no logger, no panic
  handler, and no signal handler.
- **It stops its work when it drops.** It ends each background task. It leaks
  none of them.
- **It never blocks the code that draws.** A set of queries runs in a
  background task, and it publishes its result into a channel. The code that
  draws then reads the most recent result without a wait. A slow query or a
  failed query changes one panel, and never the whole picture.

## Consequences

This choice serves one aim of the project: one program and one library that
can act as a client and as a server. The same `thunderbox-dashboard` type
serves the program that runs alone, and another program that holds a
thunderbox dashboard. Only the owner of the terminal changes. A server for SSH
does not exist yet. [ADR 0013](0013-one-connection-carries-both-ways-over-ssh.md)
states the plan for it, and that server will use the same type.

The list above states what the library never does.
[ADR 0016](0016-a-program-that-holds-a-dashboard-holds-four-rules.md) states
what the program must do.
