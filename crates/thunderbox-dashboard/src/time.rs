//! The rule that maps a key to a time range.

use thunderbox_core::{TimeRange, Timestamp};

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
/// another map ignores this function. ADR 0035 in `docs/contributor/adr` states that
/// rule.
///
/// The function reads these keys:
///
/// - `Left` moves the range back by half of its width;
/// - `Right` moves the range forward by half of its width;
/// - `-` makes the range twice as wide;
/// - `+` makes the range half as wide;
/// - `0` and `=` return the range that the dashboard states.
///
/// A move fixes a moment at each end, so the window stands still and the
/// panels stop at it. A move forward that reaches the present moment returns a
/// range of the same width that follows the clock. The `declared` parameter
/// takes the range of the dashboard, which is the range that `0` and `=`
/// return.
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
    let asked = match key {
        Key::Left => range.panned(now, -step),
        Key::Right => range.panned(now, step),
        Key::Char('-') => range.with_span_ms(now, span.saturating_mul(WIDTH_FACTOR)),
        Key::Char('+') => range.with_span_ms(now, span / WIDTH_FACTOR),
        Key::Char('0') | Key::Char('=') => declared.clone(),
        _ => return None,
    };
    (asked != *range).then_some(asked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use thunderbox_core::RelativeTime;

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

    /// The six keys of the range, in the order of the list in the comment of
    /// `time_key`.
    const RANGE_KEYS: [Key; 6] = [
        Key::Left,
        Key::Right,
        Key::Char('-'),
        Key::Char('+'),
        Key::Char('0'),
        Key::Char('='),
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
