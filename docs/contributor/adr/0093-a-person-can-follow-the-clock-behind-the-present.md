# ADR 0093: A person can follow the clock behind the present

| field | value |
|---|---|
| Status | Accepted |
| Recorded | 2026-11-27 |
| Source | new |
| Code | `crates/thunderbox-core/src/time.rs`, `crates/thunderbox-dashboard/src/time.rs`, `crates/thunderbox/src/main.rs`, `crates/thunderbox-widgets/src/value.rs`, `scripts/record-demo.py` |
| Tests | `the_left_key_moves_the_range_back_by_half_of_its_width`, `the_right_key_moves_the_range_forward_by_half_of_its_width`, `a_move_that_reaches_the_present_follows_the_clock_and_keeps_the_width`, `the_minus_key_makes_the_range_twice_as_wide`, `the_plus_key_makes_the_range_half_as_wide`, `the_zero_key_returns_the_range_that_the_dashboard_states`, `a_key_that_changes_nothing_asks_for_no_range`, `a_move_forward_that_passes_the_present_moment_stops_at_it`, `a_move_forward_follows_the_clock_again_after_the_clock_moved_on`, `a_move_back_holds_still_a_range_that_ends_in_the_future`, `a_move_back_and_a_move_forward_differ_for_a_range_that_ends_in_the_future`, `the_view_uses_no_key_of_the_time_range`, `a_key_that_the_program_binds_moves_the_time_range`, `a_key_that_changes_nothing_starts_no_set_of_queries`, `each_key_of_the_time_range_moves_the_range`, `the_f_key_makes_a_held_range_follow_the_clock`, `the_f_key_holds_a_range_that_follows_the_clock_behind`, `the_f_key_asks_for_nothing_on_a_live_range`, `the_f_key_twice_returns_the_range_that_a_person_held`, `a_move_back_from_a_live_range_holds_the_window`, `a_move_from_a_held_range_keeps_the_range_held`, `a_move_forward_from_a_held_range_that_reaches_the_present_returns_the_live_range`, `a_move_back_from_a_following_range_keeps_it_following`, `a_move_forward_from_a_following_range_keeps_it_following`, `a_move_forward_from_a_following_range_that_reaches_the_present_returns_the_live_range`, `the_minus_and_plus_keys_keep_a_range_that_follows_the_clock_behind`, `the_zero_key_returns_the_range_that_the_dashboard_states_from_a_following_range`, `following_keeps_the_window_at_the_present_moment`, `a_range_from_following_moves_with_the_clock`, `following_returns_the_live_form_when_the_end_reaches_the_present_moment`, `following_returns_the_range_as_it_was_when_an_offset_would_not_fit`, `ends_at_the_present_reports_the_range_that_ends_now_or_later`, `the_status_row_names_the_keys_of_a_window_that_follows_the_clock`, `the_status_row_writes_the_short_keys_when_the_following_sentence_does_not_fit`, `the_following_sentence_does_not_change_when_only_the_present_moment_changes`, `the_status_row_writes_the_short_form_when_the_held_sentence_does_not_fit`, `a_move_forward_that_stops_less_than_one_step_before_the_present_returns_the_live_range`, `a_range_that_lists_its_ends_in_reverse_order_follows_the_rule_of_a_live_range`, `ends_at_the_present_reads_a_range_whose_ends_stand_in_reverse_order`, `the_status_row_writes_the_short_form_beside_the_sentence_of_an_error_for_a_following_window` |
| Related | [ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md), [ADR 0036](0036-each-label-of-the-time-axis-states-its-distance-from-now.md), [ADR 0037](0037-the-refresh-continues-while-the-window-stands-still.md), [ADR 0092](0092-the-status-row-names-a-held-window-by-its-clock-times.md) |

## Context

[ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md) makes every
move back hold the window at two fixed moments. A person who studies an event
needs that. A person who wants to watch a period of the past move on has no
key for it. [ADR 0092](0092-the-status-row-names-a-held-window-by-its-clock-times.md)
gives the status row a sentence for a held window, and that sentence names one
key only.

This record uses three terms:

- A live range ends at the present moment. Its end is `RelativeTime::Now`, or
  an offset of 0 or more.
- A held range has a fixed moment at each end. Its window stands still.
- A following range follows the clock, but it ends before the present moment.
  Its ends are offsets, and the offset at the end is negative.

## Decision

The project binds the key `f` to the mode of a time range. This record
supersedes ADR 0035 and ADR 0092. It restates each rule of those two records
that still holds, and it adds rules 1 to 4.

### Rules that stay from ADR 0092

- `format_clock` writes each end of a held window in Coordinated Universal
  Time. It writes `HH:MM:SS` when the span is under one day, and `MM-DD HH:MM`
  otherwise.
- The program writes the sentence of a held window for a held range. A live
  range writes no sentence.
- The sentence of a held window stays the same on each refresh.
- The row writes the complete sentence while it fits. It draws a short form
  only when the sentence does not fit, and it tries that form before it drops
  the sentence.

### Rules that stay from ADR 0035

- The program binds every key of the time range, and thunderbox consumes none
  of them. `DashboardView::handle_key` consumes `Tab`, `Shift+Tab`, and `m`
  alone.
- The keys are the left arrow, the right arrow, `-`, `+`, `0`, and `=`. A move
  is half of the width. A scale is a factor of two. `0` and `=` return the range
  that the dashboard states.
- `thunderbox_core::TimeRange` holds the arithmetic, and
  `thunderbox_dashboard::time_key` is the one policy. A program that wants
  another map ignores `time_key`. No program holds a copy of the policy.
- A move forward can carry the end of the window to the present moment, or past
  it. The move then returns a live range of the same width. A move forward that
  stops less than one step before the present moment returns that live range
  too.
- `time_key` returns `None` for a key that asks for the range that the runtime
  already holds. Each call to `DashboardRuntime::set_time_range` starts a new
  set of queries, so a key that changes nothing would stop each set before it
  answers.
- A program calls `time_key` from a key press, and never on every frame.

### New rules

1. `TimeRange::following(now)` returns the same window at `now`, with a
   `RelativeTime::Offset` at each end, so the window moves with the clock. A
   range whose end reaches `now` or later returns the live form. In the live
   form, `to` is `RelativeTime::Now`, and `from` is an offset of minus the
   width. `TimeRange::ends_at_the_present` tells a live range from the other
   two modes. It reads both ends, because a range can list its ends in reverse
   order.
2. `time_key` binds `Key::Char('f')`:
   - on a held range, it returns `range.following(now)`;
   - on a following range, it returns `range.to_absolute(now)`;
   - on a live range, it returns `None`, because a live range already follows
     the clock.
3. The arrows keep the mode that the person chose:
   - a move back from a live range holds the window, as ADR 0035 states;
   - a move from a held range keeps it held, and a move forward that reaches
     the present moment returns the live range;
   - a move back from a following range keeps that mode, with
     `range.moved(-step)`. A move forward calls `range.moved(step)`. The end can
     reach the present moment, or stop less than one step before it. The move
     then returns the live range of the same width;
   - the keys `-` and `+` keep the mode. `TimeRange::with_span_ms` holds the
     later end still and keeps its kind, so a following range keeps its mode.
4. The status row names both keys.
   - A held window writes this sentence: `The panels hold 14:00:00 to 14:02:00
     UTC. Press f to follow the clock behind, or 0 to return.` Its short form
     is `Held 14:00:00 to 14:02:00. f follows, 0 returns.`
   - A following window writes the text of `format_window`. Then it writes
     `Press f to hold, or 0 to return.` The text of `format_window` stays the
     same on each refresh, because the distance stays the same. The short form
     of the last sentence is `f holds, 0 returns.`
   - The row draws a short form only when the complete sentence does not fit.
     `CLAUDE.md` section 1.5 lists both short forms.

The example `full_embed.rs` binds no key `f` of its own, and it passes each key
to `time_key`. The key `f` therefore works there. The example writes no
sentence of a window.

## Consequences

- A person chooses the mode at the time of the choice, and the arrows keep
  that mode.
- The key `f` is a new key of the program. The list `KEYS` of the status row
  does not change, because the list has no room. The sentence of a window
  names the key.
- The sentence of ADR 0092 for a held window changes to the text of rule 4. ADR
  0092 keeps its record of the earlier text, and this record replaces it.
- The scene `app` of `scripts/record-demo.py` presses `f` after the move back.

## Designs that the project rejected

- **Make every move back follow the clock.** A person who studies an event
  would see the event slide out of the panel on each refresh.
- **A field of the dashboard that chooses the mode.** A person decides when the
  question arises, and the author of the dashboard cannot know that moment. A
  key fits.
