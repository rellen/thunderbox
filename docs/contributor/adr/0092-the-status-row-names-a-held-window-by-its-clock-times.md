# ADR 0092: The status row names a held window by its clock times

| field | value |
|---|---|
| Status | Superseded by [ADR 0093](0093-a-person-can-follow-the-clock-behind-the-present.md) |
| Recorded | 2026-11-27 |
| Source | new |
| Code | `crates/thunderbox-widgets/src/value.rs`, `crates/thunderbox/src/main.rs` |
| Tests | `the_status_row_writes_the_held_sentence_for_a_held_window`, `the_status_row_writes_the_distance_for_a_window_that_follows_the_clock`, `the_status_row_writes_the_short_form_when_the_held_sentence_does_not_fit`, `the_held_sentence_does_not_change_when_only_the_present_moment_changes`, `the_status_row_picks_the_sentence_by_the_kind_of_the_range`, `format_held_window_names_both_ends_of_a_window_under_a_day`, `format_held_window_writes_the_date_for_a_window_of_two_days`, `format_held_window_does_not_change_when_only_the_present_moment_changes`, `format_held_window_short_names_both_ends_and_the_key`, `the_status_row_writes_the_short_form_beside_the_sentence_of_an_error`, `the_status_row_writes_the_short_form_beside_the_sentence_of_a_wait`, `the_status_row_keeps_the_complete_sentence_beside_the_sentence_of_an_error` |
| Related | [ADR 0035](0035-the-program-binds-each-key-of-the-time-range.md), [ADR 0037](0037-the-refresh-continues-while-the-window-stands-still.md) |

## Context

A move back holds the window at two fixed moments. The refresh continues
([ADR 0037](0037-the-refresh-continues-while-the-window-stands-still.md)), and
the trace stands still. The status row wrote the sentence `The panels show 1
minute that ended 1 minute ago.` The distance in that sentence grew on each
refresh while the picture did not change. The sentence also did not say how to
return to the present moment, unless the width of the terminal left room for the
hint `0 range`.

## Decision

The function `format_held_window` of `thunderbox-widgets` writes a sentence that
does not change while the window stands still:

```
The panels hold 14:00:00 to 14:02:00 UTC. Press 0 to follow the clock.
```

1. `format_clock` writes each end. It writes `HH:MM:SS` when the span is under
   one day, and `MM-DD HH:MM` otherwise. Both forms use Coordinated Universal
   Time.
2. The program writes that sentence for a held range, which is a range that
   does not move with the clock. It keeps `format_window` for each other range
   that ends before the present moment. A live range writes no sentence.
3. The row writes the complete sentence while it fits. It draws the short form
   `Held 14:00:00 to 14:02:00. 0 follows.` only when the sentence does not fit.
   The row tries that short form before it drops the sentence, so a window that
   stands still stays named beside the sentence of an error.
   `CLAUDE.md` section 1.5 lists the short form.
4. The example `full_embed.rs` writes no sentence of a window, so it needs no
   change.

## Consequences

- The sentence of a held window is the same on each refresh.
- The short form is a string that a test compares against. A change to it needs
  a change to the table of section 1.5.
- The short form names the key, so the row adds no hint `0 range` after it.

## Designs that the project rejected

- **Keep the distance from the present moment.** The distance changes on each
  refresh while the picture stands still.
- **A symbol, such as a pause sign.** A reader of the row learns nothing from a
  symbol without the documentation.
