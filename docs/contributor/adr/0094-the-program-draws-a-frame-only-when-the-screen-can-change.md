# ADR 0094: The program draws a frame only when the screen can change

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-27 |
| Source | new |
| Code | `crates/thunderbox/src/main.rs`, `crates/thunderbox-dashboard/examples/full_embed.rs`, `crates/thunderbox-dashboard/src/runtime.rs` |
| Tests | `the_first_wake_draws_a_frame`, `a_wake_with_no_change_draws_no_frame`, `a_new_snapshot_draws_a_frame`, `a_new_size_draws_a_frame`, `an_applied_event_draws_a_frame`, `a_change_of_the_variables_draws_a_frame`, `an_error_of_a_variable_draws_a_frame`, `a_set_of_queries_that_starts_or_stops_to_wait_draws_a_frame`, `a_new_age_text_draws_a_frame`, `an_age_that_grows_but_keeps_the_same_text_draws_no_frame`, `a_burst_of_three_tab_keys_moves_the_selection_three_panels_and_draws_one_frame`, `the_loop_waits_for_the_first_event_and_reads_the_rest_with_no_wait`, `the_loop_reads_an_empty_queue_after_one_wait_of_the_wake`, `one_wake_of_the_loop_reads_a_burst_of_three_keys`, `the_keys_esc_and_control_c_stop_the_loop_inside_a_burst`, `a_key_that_is_not_a_press_applies_nothing`, `the_status_row_shows_the_seconds_that_the_decision_compares`, `each_fact_alone_draws_a_frame` (in `full_embed`), `a_burst_of_three_keys_costs_one_frame`, `a_burst_that_holds_q_stops_the_loop_at_once`, `a_tab_after_an_enter_in_one_burst_skips_the_panels_that_the_enter_hid`, `a_collapse_keeps_the_address_of_the_snapshot_and_the_receiver_reports_it`, `an_idle_dashboard_that_refreshes_each_second_draws_about_two_frames_each_second`, `a_dashboard_that_never_refreshes_draws_no_frame_while_nobody_acts`, `a_key_reaches_the_screen_at_once`, `a_new_size_of_the_terminal_reaches_the_screen_at_once` |
| Related | [ADR 0016](0016-a-program-that-holds-a-dashboard-holds-four-rules.md), [ADR 0021](0021-a-new-area-waits.md), [ADR 0029](0029-the-runtime-asks-about-every-panel-until-an-area-arrives.md), [ADR 0052](0052-the-runtime-owns-the-collapse.md), [ADR 0053](0053-a-panel-that-appears-costs-a-set-of-queries.md) |

## Context

A frame is one complete draw of the cells of the screen. A wake is one pass of
the loop of the program. The loop of the program `thunderbox` waited at most
100 milliseconds for an event, so it woke at least ten times each second. It
drew a frame on each wake. On each wake it also copied the dashboard and the
variables, and it read one event.

A dashboard that refreshes each second changes its numbers once each second.
Nine frames of ten therefore repeated the frame before them. ratatui compares
each cell with the cell of the frame before it, and it writes only the cells
that changed. A frame that changes nothing still draws the whole dashboard into
a buffer. The example `full_embed` held a loop of the same shape, and a program
that embeds thunderbox copies that example.

A burst of keys cost one frame for each key, because the loop read one event on
each wake.

## Decision

The program draws a frame only when a fact differs from the same fact of the
last frame. The facts are these:

- the snapshot, which the receiver of `DashboardRuntime::subscribe` reports;
- the size of the terminal, which the loop reads on each wake;
- an event that the loop applied;
- the variables, the error of a variable, the fact that a set of queries waits,
  and the age of the numbers.

The loop holds the facts of the two rows in one value that implements
`PartialEq`. The age takes the form that the status row shows, which is whole
seconds after the numbers reach the age of 5 seconds. `must_draw` compares the
facts of the wake with the facts of the last frame. The first wake draws,
because no last frame exists.

The loop still wakes at least ten times each second, so a key reaches the
program at once. A wake that draws no frame still calls `set_panel_area`,
`set_marker`, and `tick`. It gives `set_panel_area` the area of the last frame.

The loop applies each event that waits before it reads the snapshot and draws.
A burst of keys therefore costs one frame. A key that stops the program stops
it at once, also inside a burst. `apply_key` reads the snapshot again for each
key, because an earlier key of the burst can change it.

The loop borrows the dashboard and the variables on a wake. It copies the
variables only for a frame that it draws, and it keeps that copy as the facts
of the last frame.

The example `full_embed` follows the same rule. Its line of text is one more
fact of its frame.

## Consequences

- A new input of a frame needs a fact of its own when it reads the clock. It
  also needs one when it changes with no event and no new snapshot. The doc
  comment of `Facts` lists the inputs that need none, and it gives the reason
  for each. The two inputs that read the clock are the age of the numbers and
  the wait of a set of queries.
- The program draws no frame to repair a screen that another program wrote
  into. The old loop did not repair such a screen either, because ratatui
  writes only the cells that differ from its own buffer.
- A refresh that answers at once publishes two snapshots between two wakes, and
  the receiver then reports one change. The loop draws one frame for that
  refresh. A slow refresh draws a frame when it starts and a frame when it
  ends.
- The loop reads the snapshot first and the age from that snapshot. The frame
  then shows the age of the numbers that it draws.
- A program that embeds thunderbox and keeps the older loop still works. That
  program draws a frame on each wake, and it uses more processor time.
- The documents that said "on every frame" now say "on each wake". A record
  keeps its text, so ADR 0016, ADR 0021, ADR 0022, ADR 0035, ADR 0052, and ADR
  0093 still say "on every frame" or "on each frame". Read "frame" there as
  "wake". The rules of those records stay the same.

## Measurements

The command is a script that the project does not hold. Section 5.1 of
`docs/contributor/architecture.md` states its steps. The script starts the
program in a pseudo-terminal, sends no key for 10 seconds, and counts the end of
each draw. The backend of ratatui writes `ESC [39m ESC [49m ESC [59m ESC [0m`
at the end of each draw, also when the draw wrote no cell. The script reads the
processor time with `getrusage(RUSAGE_CHILDREN)` after the program exits. Each
result is the median of three runs of the same debug build.

| input | measure | the loop before this record | the loop after this record |
|---|---|---|---|
| `dashboards/variables.toml`, 10 seconds, 110 by 32 cells, debug build | frames | 92 | 11 |
| the same input | frames that wrote no cell | 81 | 0 |
| the same input | processor time | 7.14 percent | 1.11 percent |
| the key `Tab` or a change of the window size, five times each | delay to the next frame | not measured | 3.4 to 17.4 milliseconds |

The test `an_idle_dashboard_that_refreshes_each_second_draws_about_two_frames_each_second`
failed on the older loop with the message `The program drew 47 frames in 5
seconds. The limit is 12.` It passes on the new loop. Section 5.1 of
`docs/contributor/architecture.md` holds the method of the processor
measurement and its result.

One more measurement shaped the design. `DashboardRuntime::toggle_row` edited
the snapshot in place, and the address of its `Arc` stayed the same. The test
`a_collapse_keeps_the_address_of_the_snapshot_and_the_receiver_reports_it`
holds that fact.

## Designs that the project rejected

- **Compare the `Arc` of the snapshot by address.** The runtime edits the
  snapshot in place at four places, with `Arc::make_mut`. The edit keeps the
  address when no other owner holds the `Arc`. The address of the snapshot after
  `toggle_row` was equal to the address before it, as the test above shows. A
  comparison by address misses such a change. The loop reads the receiver
  instead, which reports each change.
- **Keep the age as a `Duration` in the facts.** The age grows on each wake, so
  the facts differ on each wake and the loop draws on each wake again. The test
  `an_age_that_grows_but_keeps_the_same_text_draws_no_frame` fails for that
  design.
- **Read one event on each wake.** A burst of three keys then costs three
  frames. Two tests fail for that design: `one_wake_of_the_loop_reads_a_burst_of_three_keys`
  for the function that the loop calls, and `a_burst_of_three_keys_costs_one_frame`
  for the whole program. The test
  `the_loop_waits_for_the_first_event_and_reads_the_rest_with_no_wait` holds
  the function `read_burst` alone.
- **Read the snapshot once for the whole burst.** A key `Tab` after a key
  `Enter` that collapsed a row panel then skips the wrong panels. The test
  `a_tab_after_an_enter_in_one_burst_skips_the_panels_that_the_enter_hid`
  fails for that design.
