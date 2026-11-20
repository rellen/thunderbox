# ADR 0016: A program that holds a dashboard holds four rules

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-20 |
| Source | `docs/ARCHITECTURE.md` section 12 |
| Code | `crates/thunderbox-dashboard/src/runtime.rs`, `crates/thunderbox-core/src/dashboard.rs`, `crates/thunderbox/src/main.rs`, `crates/thunderbox-dashboard/examples/full_embed.rs` |
| Tests | No test holds the first rule, the second rule, or the fourth rule. The test `a_dashboard_that_names_no_data_source_returns_an_error_in_its_panel` shows the effect of the third rule: a name that the registry does not hold reaches each panel as an error. The dashboard of that test declares a `[source]` table, so the test does not hold the third rule itself. |
| Related | [ADR 0015](0015-both-ways-into-another-program-are-in-scope.md), [ADR 0019](0019-the-runtime-holds-one-query-to-a-limit.md), [ADR 0031](0031-a-snapshot-waits-in-a-third-case.md) |

## Context

[ADR 0015](0015-both-ways-into-another-program-are-in-scope.md) states what the
library never does. No file stated what the program must do before this rule.

## Decision

A program that holds a thunderbox dashboard must itself hold four rules.

1. **The async runtime must run a task on a thread other than the thread that
   draws.** `DashboardRuntime::new` asks for a `Handle` that holds the timer of
   tokio, and a runtime of the flavour `current_thread` holds that timer. That
   timer is still not enough. The runtime starts a task for each panel, and the
   loop that draws never awaits. `crates/thunderbox/src/main.rs` waits for a
   key inside `event::poll`, which stops the thread until a key arrives or the
   time of the call passes. A runtime of one thread therefore polls no query
   task, and every panel reads `loading…` for ever. No error reaches the
   screen, and no limit ends the wait.
2. **A method that starts a new set of queries needs a guard for the value that
   did not change.** Both programs call `set_marker` on every frame.
   `DashboardRuntime::set_marker` compares the new marker against the marker
   that it holds, and it starts nothing when the two are equal. That comparison
   is the reason that the call is safe on every frame.
3. **`Dashboard::validate` checks a source name only for a dashboard that
   declares a `[source]` table.** Both checks of the name run only when
   `!self.sources.is_empty()` holds. One check reads the target of a panel, and
   the other reads the query of a variable. A dashboard that declares no such
   table therefore accepts any name at all.
4. **A `watch::Receiver` that outlives the runtime reports `refreshing` for
   ever.** `Drop for DashboardRuntime` raises the generation and ends the task,
   and it writes nothing to the channel. `ClearOnEnd` then finds a generation
   that does not match, and it writes nothing either.

## Consequences

**The first rule.** Both programs of this repository call
`Builder::new_multi_thread`. They are `crates/thunderbox/src/main.rs` and
`crates/thunderbox-dashboard/examples/full_embed.rs`. Neither one states the
reason, so a reader of those two files learns nothing about this rule.
[ADR 0019](0019-the-runtime-holds-one-query-to-a-limit.md) states the timer
that the runtime needs as well.

**The second rule.** A reader who deletes the comparison sees a simpler method,
and every test still passes.

**The third rule.** A dashboard that declares no `[source]` table is the
ordinary shape of an embedded dashboard. The program builds its own `Registry`,
and the dashboard then names no source of its own. A wrong name then reaches
the screen as a sentence in every panel, and not as one line before the program
opens the terminal. `full_embed.rs` holds the name in two places: in each
target of its dashboard, and in its registry. Only the writer of that file
keeps the two places the same, and no check compares them.

**The fourth rule.** A program reads a dashboard file again in the ordinary way
when it drops one dashboard and makes another one. A set of queries can run at
the moment of that drop. A program that then reads `is_refreshing()` on a
receiver of the old dashboard reads `true` for the rest of the process. A program that awaits `changed()` meets `Err` at once, because
the sender drops with the runtime.

A program that draws a message about a small terminal holds a further rule.
[ADR 0031](0031-a-snapshot-waits-in-a-third-case.md) states it.

## Measurements

**The first rule.** The same loop against the testdata data source answers
under `Builder::new_multi_thread`. It never leaves `PanelState::Loading` under
`Builder::new_current_thread`.

**The second rule.** Without the comparison, ten frames over 200 milliseconds
start ten sets of queries, and no panel leaves `PanelState::Loading`. With the
comparison the same ten frames start one set, and the panel answers.
