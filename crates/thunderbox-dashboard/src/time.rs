//! The rule that maps a key to a time range.

use thunderbox_core::{RelativeTime, TimeRange, Timestamp};

use crate::view::Key;

/// One key press moves the range by the width divided by this number.
///
/// A move of half the width keeps half of the picture, so a person still
/// recognises what they looked at. It also halves the number of presses that a
/// person needs. A press that changes the range puts every panel back to the
/// state that has no answer.
const MOVE_DIVISOR: i64 = 2;

/// One key press makes the range this many times wider, or this many times
/// narrower.
const WIDTH_FACTOR: i64 = 2;

/// Returns the time range that a key asks for, or `None` for another key.
///
/// thunderbox consumes no key of the time range.
/// [`crate::DashboardView::handle_key`] reports `Consumed::No` for every key
/// below, so a program that wants those keys for itself keeps them. This
/// function offers a default map from a key to a range, and it holds the rule
/// alone. The program decides which key reaches it, and a program that wants
/// another map ignores this function. ADR 0035 and ADR 0093 in
/// `docs/contributor/adr` state that rule.
///
/// The function reads these keys:
///
/// - `Left` moves the range back by half of its width;
/// - `Right` moves the range forward by half of its width;
/// - `-` makes the range twice as wide;
/// - `+` makes the range half as wide;
/// - `0` and `=` return the range that the dashboard states;
/// - `f` switches a range between a held mode and a following mode.
///
/// A range is in one of three modes. A live range ends at the present moment.
/// A held range has a fixed moment at each end, so its window stands still. A
/// following range ends before the present moment, and it moves with the
/// clock. A move back from a live range holds the window. A move from a held
/// range keeps it held. A move from a following range keeps that mode. A
/// change of the width keeps the mode in each case. A move forward that reaches
/// the present moment returns a live range of the same width.
///
/// The key `f` asks for a following range when the range is held. It asks for
/// a held range when the range follows the clock behind. A live range already
/// follows the clock, so the key asks for nothing there. The `declared`
/// parameter takes the range of the dashboard, which is the range that `0` and
/// `=` return.
///
/// The function returns `None` for a key that asks for the range that the
/// caller already holds. [`crate::DashboardRuntime::set_time_range`] starts a
/// new set of queries on every call, so a key that changes nothing would
/// otherwise stop the set of queries that runs. A person who holds the key
/// `Right` down on a live range would then meet a dashboard that never
/// completes a refresh.
///
/// The caller passes the result to [`crate::DashboardRuntime::set_time_range`].
/// A program therefore calls this function from a key press, and never on every
/// frame.
///
/// # Examples
///
/// ```
/// use thunderbox_core::TimeRange;
/// use thunderbox_dashboard::{Key, time_key};
/// let live = TimeRange::default();
/// let back = time_key(Key::Left, &live, &live, 1_000_000).expect("The key moves the range.");
/// assert!(!back.moves_with_the_clock());
/// assert_eq!(time_key(Key::Char('0'), &back, &live, 1_000_000), Some(live.clone()));
/// assert_eq!(time_key(Key::Tab, &back, &live, 1_000_000), None);
/// // The key `f` makes the held window follow the clock behind.
/// let following = time_key(Key::Char('f'), &back, &live, 1_000_000).expect("The key follows.");
/// assert!(following.moves_with_the_clock());
/// assert_eq!(following.resolve(1_000_000), back.resolve(1_000_000));
/// // The range already follows the clock, so this key asks for nothing.
/// assert_eq!(time_key(Key::Right, &live, &live, 1_000_000), None);
/// ```
pub fn time_key(
    key: Key,
    range: &TimeRange,
    declared: &TimeRange,
    now: Timestamp,
) -> Option<TimeRange> {
    let span = range.span_ms(now);
    let step = span / MOVE_DIVISOR;
    let following = range.moves_with_the_clock() && !range.ends_at_the_present();
    let asked = match key {
        Key::Left if following => move_following(range, now, -step),
        Key::Right if following => move_following(range, now, step),
        Key::Left => range.panned(now, -step),
        Key::Right => range.panned(now, step),
        Key::Char('-') => range.with_span_ms(now, span.saturating_mul(WIDTH_FACTOR)),
        Key::Char('+') => range.with_span_ms(now, span / WIDTH_FACTOR),
        Key::Char('0') | Key::Char('=') => declared.clone(),
        Key::Char('f') if following => range.to_absolute(now),
        Key::Char('f') if !range.moves_with_the_clock() => range.following(now),
        _ => return None,
    };
    (asked != *range).then_some(asked)
}

/// Returns a range that follows the clock behind, moved by `by_ms`.
///
/// The offsets of the two ends move, so the range keeps its mode. A move
/// forward can stop less than one step before the present moment. The function
/// then returns the live range of the same width, as [`TimeRange::panned`]
/// does for a held range. The function returns the range as it was when the
/// move would change the width.
fn move_following(range: &TimeRange, now: Timestamp, by_ms: i64) -> TimeRange {
    let span = range.span_ms(now);
    let moved = range.moved(by_ms);
    if moved.span_ms(now) != span {
        return range.clone();
    }
    if by_ms > 0 && now.saturating_sub(moved.resolve(now).1) < by_ms {
        // The width is 1 or more, so the negation cannot overflow.
        if now.checked_sub(span).is_none() {
            return range.clone();
        }
        return TimeRange {
            from: RelativeTime::Offset(-span),
            to: RelativeTime::Now,
        };
    }
    moved
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The moment that every test in this module calls the present moment.
    const NOW: Timestamp = 1_000_000;

    /// Returns the range of five minutes that the dashboard states.
    fn declared() -> TimeRange {
        TimeRange::default()
    }

    /// Returns the range that the key `key` asks for.
    fn press(key: Key, range: &TimeRange) -> TimeRange {
        time_key(key, range, &declared(), NOW).expect("The key must move the range.")
    }

    #[test]
    fn the_left_key_moves_the_range_back_by_half_of_its_width() {
        let back = press(Key::Left, &declared());
        assert_eq!(back.resolve(NOW), (NOW - 450_000, NOW - 150_000));
        assert!(
            !back.moves_with_the_clock(),
            "The window must stand still while a person studies it."
        );
    }

    #[test]
    fn the_right_key_moves_the_range_forward_by_half_of_its_width() {
        let back = press(Key::Left, &press(Key::Left, &declared()));
        let ahead = press(Key::Right, &back);
        assert_eq!(ahead.resolve(NOW), (NOW - 450_000, NOW - 150_000));
    }

    #[test]
    fn a_move_that_reaches_the_present_follows_the_clock_and_keeps_the_width() {
        let wide = press(Key::Char('-'), &declared());
        let back = press(Key::Left, &wide);
        let ahead = press(Key::Right, &back);
        assert!(ahead.moves_with_the_clock(), "{ahead:?}");
        assert_eq!(
            ahead.span_ms(NOW),
            600_000,
            "The move must keep the width that a person chose."
        );
    }

    #[test]
    fn the_minus_key_makes_the_range_twice_as_wide() {
        let wider = press(Key::Char('-'), &declared());
        assert_eq!(wider.from, RelativeTime::Offset(-600_000));
        assert_eq!(wider.to, RelativeTime::Now);
    }

    #[test]
    fn the_plus_key_makes_the_range_half_as_wide() {
        let narrower = press(Key::Char('+'), &declared());
        assert_eq!(narrower.from, RelativeTime::Offset(-150_000));
        assert_eq!(narrower.to, RelativeTime::Now);
    }

    /// The key stops when the range is as narrow as this project permits. It
    /// then asks for nothing, and the loop below ends.
    #[test]
    fn many_presses_never_make_the_range_narrower_than_a_moment() {
        let mut range = declared();
        let mut presses = 0;
        while let Some(narrower) = time_key(Key::Char('+'), &range, &declared(), NOW) {
            range = narrower;
            presses += 1;
            assert!(presses < 20, "The key must stop at the narrowest range.");
        }
        assert_eq!(range.span_ms(NOW), thunderbox_core::MIN_SPAN_MS);
    }

    #[test]
    fn the_zero_key_returns_the_range_that_the_dashboard_states() {
        let moved = press(Key::Char('-'), &press(Key::Left, &declared()));
        assert_eq!(press(Key::Char('0'), &moved), declared());
        assert_eq!(press(Key::Char('='), &moved), declared());
    }

    /// Returns a range that follows the clock and ends five minutes before the
    /// present moment. The width is two minutes.
    fn following() -> TimeRange {
        TimeRange {
            from: RelativeTime::Offset(-420_000),
            to: RelativeTime::Offset(-300_000),
        }
    }

    /// Returns a range that holds the window of `following` still.
    fn held() -> TimeRange {
        following().to_absolute(NOW)
    }

    #[test]
    fn the_f_key_makes_a_held_range_follow_the_clock() {
        let asked = press(Key::Char('f'), &held());
        assert_eq!(asked, following());
        assert!(asked.moves_with_the_clock());
        assert_eq!(asked.resolve(NOW), held().resolve(NOW));
    }

    #[test]
    fn the_f_key_holds_a_range_that_follows_the_clock_behind() {
        let asked = press(Key::Char('f'), &following());
        assert_eq!(asked, held());
        assert!(!asked.moves_with_the_clock());
    }

    #[test]
    fn the_f_key_asks_for_nothing_on_a_live_range() {
        assert_eq!(
            time_key(Key::Char('f'), &declared(), &declared(), NOW),
            None
        );
    }

    #[test]
    fn the_f_key_twice_returns_the_range_that_a_person_held() {
        let again = press(Key::Char('f'), &press(Key::Char('f'), &held()));
        assert_eq!(again, held());
    }

    #[test]
    fn a_move_back_from_a_live_range_holds_the_window() {
        let back = press(Key::Left, &declared());
        assert!(!back.moves_with_the_clock());
    }

    #[test]
    fn a_move_from_a_held_range_keeps_the_range_held() {
        let back = press(Key::Left, &held());
        assert!(!back.moves_with_the_clock(), "{back:?}");
        assert_eq!(back.resolve(NOW), (NOW - 480_000, NOW - 360_000));
        let ahead = press(Key::Right, &held());
        assert!(!ahead.moves_with_the_clock(), "{ahead:?}");
        assert_eq!(ahead.resolve(NOW), (NOW - 360_000, NOW - 240_000));
    }

    #[test]
    fn a_move_forward_from_a_held_range_that_reaches_the_present_returns_the_live_range() {
        let near = TimeRange {
            from: RelativeTime::Absolute(NOW - 150_000),
            to: RelativeTime::Absolute(NOW - 30_000),
        };
        let ahead = press(Key::Right, &near);
        assert_eq!(ahead.to, RelativeTime::Now);
        assert_eq!(ahead.span_ms(NOW), 120_000);
    }

    #[test]
    fn a_move_back_from_a_following_range_keeps_it_following() {
        let back = press(Key::Left, &following());
        assert!(back.moves_with_the_clock(), "{back:?}");
        assert_eq!(back.from, RelativeTime::Offset(-480_000));
        assert_eq!(back.to, RelativeTime::Offset(-360_000));
        assert_eq!(back.span_ms(NOW), 120_000);
    }

    #[test]
    fn a_move_forward_from_a_following_range_keeps_it_following() {
        let ahead = press(Key::Right, &following());
        assert_eq!(ahead.from, RelativeTime::Offset(-360_000));
        assert_eq!(ahead.to, RelativeTime::Offset(-240_000));
    }

    #[test]
    fn a_move_forward_from_a_following_range_that_reaches_the_present_returns_the_live_range() {
        let near = TimeRange {
            from: RelativeTime::Offset(-150_000),
            to: RelativeTime::Offset(-30_000),
        };
        let ahead = press(Key::Right, &near);
        assert_eq!(ahead.to, RelativeTime::Now, "{ahead:?}");
        assert_eq!(ahead.from, RelativeTime::Offset(-120_000));
        let past = TimeRange {
            from: RelativeTime::Offset(-30_000),
            to: RelativeTime::Offset(-10_000),
        };
        assert_eq!(press(Key::Right, &past).to, RelativeTime::Now);
    }

    #[test]
    fn a_move_forward_that_stops_less_than_one_step_before_the_present_returns_the_live_range() {
        // The width is 60 seconds, so the step is 30 seconds. The move puts the
        // end 20 seconds before the present moment.
        let near = TimeRange {
            from: RelativeTime::Offset(-110_000),
            to: RelativeTime::Offset(-50_000),
        };
        let ahead = press(Key::Right, &near);
        assert_eq!(ahead.to, RelativeTime::Now, "{ahead:?}");
        assert_eq!(ahead.from, RelativeTime::Offset(-60_000));
        // A move that stops one step before the present moment keeps the mode.
        let far = TimeRange {
            from: RelativeTime::Offset(-130_000),
            to: RelativeTime::Offset(-70_000),
        };
        let kept = press(Key::Right, &far);
        assert_eq!(kept.to, RelativeTime::Offset(-40_000), "{kept:?}");
    }

    #[test]
    fn a_range_that_lists_its_ends_in_reverse_order_follows_the_rule_of_a_live_range() {
        let reversed = TimeRange {
            from: RelativeTime::Now,
            to: RelativeTime::Offset(-300_000),
        };
        assert_eq!(time_key(Key::Char('f'), &reversed, &declared(), NOW), None);
        let back = press(Key::Left, &reversed);
        assert!(
            !back.moves_with_the_clock(),
            "A move back from a live range must hold the window. {back:?}"
        );
    }

    #[test]
    fn the_minus_and_plus_keys_keep_a_range_that_follows_the_clock_behind() {
        let wider = press(Key::Char('-'), &following());
        assert!(wider.moves_with_the_clock(), "{wider:?}");
        assert_eq!(wider.to, RelativeTime::Offset(-300_000));
        assert_eq!(wider.span_ms(NOW), 240_000);
        let narrower = press(Key::Char('+'), &following());
        assert!(narrower.moves_with_the_clock(), "{narrower:?}");
        assert_eq!(narrower.to, RelativeTime::Offset(-300_000));
        assert_eq!(narrower.span_ms(NOW), 60_000);
    }

    #[test]
    fn the_zero_key_returns_the_range_that_the_dashboard_states_from_a_following_range() {
        assert_eq!(press(Key::Char('0'), &following()), declared());
    }

    /// A key that asks for the range that the caller already holds must ask
    /// for nothing. `DashboardRuntime::set_time_range` starts a new set of
    /// queries on every call. A person who holds such a key down would
    /// otherwise stop each set of queries before it answers.
    #[test]
    fn a_key_that_changes_nothing_asks_for_no_range() {
        let live = declared();
        assert_eq!(
            time_key(Key::Right, &live, &live, NOW),
            None,
            "A range that follows the clock is already at the present moment."
        );
        assert_eq!(time_key(Key::Char('0'), &live, &live, NOW), None);
        assert_eq!(time_key(Key::Char('='), &live, &live, NOW), None);
        let narrowest = live.with_span_ms(NOW, thunderbox_core::MIN_SPAN_MS);
        assert_eq!(
            time_key(Key::Char('+'), &narrowest, &live, NOW),
            None,
            "The range is as narrow as this project permits."
        );
        let widest = live.with_span_ms(NOW, thunderbox_core::MAX_SPAN_MS);
        assert_eq!(
            time_key(Key::Char('-'), &widest, &live, NOW),
            None,
            "The range is as wide as this project permits."
        );
    }

    /// The program keeps every key that this function does not read. The view
    /// keeps the three keys that it binds.
    #[test]
    fn another_key_moves_no_range() {
        for key in [Key::Tab, Key::BackTab, Key::Other, Key::Char('m')] {
            assert_eq!(time_key(key, &declared(), &declared(), NOW), None);
        }
    }

    /// The seven keys of the range, in the order of the list in the comment of
    /// `time_key`.
    const RANGE_KEYS: [Key; 7] = [
        Key::Left,
        Key::Right,
        Key::Char('-'),
        Key::Char('+'),
        Key::Char('0'),
        Key::Char('='),
        Key::Char('f'),
    ];

    /// A person who holds a key down presses it many times. No press may
    /// carry the range outside the limits of this project, and no press may
    /// panic.
    #[test]
    fn many_presses_of_the_keys_keep_the_range_inside_the_limits() {
        let mut range = declared();
        let press_key = |key: Key, times: usize, range: &mut TimeRange| {
            for press_number in 0..times {
                // A press that returns `None` leaves the range as it was.
                if let Some(asked) = time_key(key, range, &declared(), NOW) {
                    *range = asked;
                }
                assert!(
                    range.validate(NOW).is_ok(),
                    "The press {press_number} of {key:?} gave {range:?}."
                );
            }
        };
        press_key(Key::Char('-'), 200, &mut range);
        assert_eq!(range.span_ms(NOW), thunderbox_core::MAX_SPAN_MS);
        press_key(Key::Left, 10_000, &mut range);
        press_key(Key::Right, 10_000, &mut range);
        press_key(Key::Char('+'), 200, &mut range);
        assert_eq!(range.span_ms(NOW), thunderbox_core::MIN_SPAN_MS);
        // The same presses on a range that follows the clock behind.
        let mut range = following();
        press_key(Key::Char('-'), 200, &mut range);
        press_key(Key::Left, 10_000, &mut range);
        press_key(Key::Right, 10_000, &mut range);
        press_key(Key::Char('f'), 3, &mut range);
    }

    /// The caller of `time_key` states the present moment and the range. Both
    /// can be any number, because the fields of a range are public.
    #[test]
    fn no_key_panics_for_a_range_at_the_limits_of_the_number() {
        let ends = [
            RelativeTime::Absolute(i64::MIN),
            RelativeTime::Absolute(i64::MAX),
            RelativeTime::Offset(i64::MIN),
            RelativeTime::Offset(i64::MAX),
            RelativeTime::Now,
        ];
        let moments = [i64::MAX, i64::MIN, 0];
        for from in ends {
            for to in ends {
                let range = TimeRange { from, to };
                for now in moments {
                    for key in RANGE_KEYS {
                        // A call that does not panic is the whole test.
                        let asked = time_key(key, &range, &declared(), now);
                        if let Some(asked) = asked {
                            assert_ne!(asked, range, "A key that changes nothing returns None.");
                        }
                    }
                }
            }
        }
    }
}
