//! Text for a number, and text for a moment in time.

use thunderbox_core::{TimeWindow, Timestamp};

/// A count of milliseconds in one second.
const SECOND_MS: f64 = 1_000.0;

/// A count of milliseconds in one minute.
const MINUTE_MS: f64 = 60_000.0;

/// A count of milliseconds in one hour.
const HOUR_MS: f64 = 3_600_000.0;

/// A count of milliseconds in one day.
const DAY_MS: f64 = 86_400_000.0;

/// The text that a panel shows in place of a number that it cannot show.
///
/// CLAUDE.md section 1.5 keeps this string. The em dash here is a symbol, and
/// it is not punctuation.
pub const MISSING: &str = "—";

/// Returns the text of a number, in the form that the unit needs.
///
/// A terminal has few columns. The function therefore keeps the text short. It
/// returns `1.2G` in place of `1234567890`. It returns [`MISSING`] for a value
/// that is not finite.
///
/// The `decimals` argument states the number of decimal places. The function
/// selects a number of places itself when the argument is `None`. It also
/// removes a zero at the end of the text in that case.
///
/// # Examples
///
/// ```
/// use thunderbox_widgets::format_value;
/// assert_eq!(format_value(1_500.0, Some("bytes"), None), "1.5KB");
/// assert_eq!(format_value(42.7, Some("percent"), Some(1)), "42.7%");
/// ```
pub fn format_value(v: f64, unit: Option<&str>, decimals: Option<u8>) -> String {
    if !v.is_finite() {
        return MISSING.into();
    }
    match unit {
        Some("percent") => format!("{}%", places(v, decimals, 0)),
        Some("bytes") => format!("{}B", si(v, decimals)),
        Some("bytes/s") => format!("{}B/s", si(v, decimals)),
        Some("seconds") => format!("{}s", places(v, decimals, 2)),
        _ => si(v, decimals),
    }
}

/// Returns the text of a moment in time, as a time of day.
///
/// The function reads the clock of Coordinated Universal Time. It reads no
/// other clock, because this crate holds no time zone database. A table panel
/// uses this function.
///
/// # Examples
///
/// ```
/// use thunderbox_widgets::format_time;
/// assert_eq!(format_time(0), "00:00:00");
/// assert_eq!(format_time(3_661_000), "01:01:01");
/// ```
pub fn format_time(t: Timestamp) -> String {
    let seconds = t.div_euclid(1000).rem_euclid(86_400);
    let (h, m, s) = (seconds / 3600, (seconds / 60) % 60, seconds % 60);
    format!("{h:02}:{m:02}:{s:02}")
}

/// The units that [`format_from_now`] writes, from the shortest to the
/// longest.
///
/// Each row holds the count of milliseconds in one unit, and the letter that
/// names it. The list stops at the day, as `duration_text` does. The axis of a
/// panel and the status row of a program then name one unit for one width.
const UNITS: [(f64, &str); 5] = [
    (1.0, "ms"),
    (SECOND_MS, "s"),
    (MINUTE_MS, "m"),
    (HOUR_MS, "h"),
    (DAY_MS, "d"),
];

/// The count that a label must not reach. A label of four digits is too wide
/// for a narrow panel, so the function takes a longer unit instead.
const TOO_MANY: f64 = 1_000.0;

/// Returns the text of a distance from the present moment.
///
/// A negative `offset_ms` is a moment in the past, and the text then starts
/// with a minus sign. A positive value is a moment in the future, and the text
/// starts with a plus sign. A value of 0 returns the text `now`.
///
/// The `step_ms` argument is the distance between two labels of an axis. The
/// function takes the largest unit that is not longer than that distance, so
/// that two labels of one axis differ. A chart of one hour that ended one hour
/// ago would otherwise read `-2h` twice.
///
/// The function then corrects that unit against the value:
///
/// - it takes a shorter unit while the value rounds to zero. The text `-0m`
///   would otherwise state both a distance and no distance;
/// - it takes a longer unit while the value needs four digits. The text
///   `-61000ms` would otherwise fill a narrow panel.
///
/// A unit that is longer than the distance between two labels keeps one or two
/// decimal places, so the two labels still differ. Every other text holds a
/// whole number of units.
///
/// # Examples
///
/// ```
/// use thunderbox_widgets::format_from_now;
/// assert_eq!(format_from_now(0, 30_000), "now");
/// assert_eq!(format_from_now(-60_000, 30_000), "-60s");
/// assert_eq!(format_from_now(300_000, 300_000), "+5m");
/// assert_eq!(format_from_now(-1_000, 60_000), "-1s");
/// ```
pub fn format_from_now(offset_ms: i64, step_ms: i64) -> String {
    if offset_ms == 0 {
        return "now".to_string();
    }
    let sign = if offset_ms < 0 { '-' } else { '+' };
    let size = offset_ms.unsigned_abs() as f64;
    // Two labels stand at least one millisecond apart, because a moment holds
    // a whole number of milliseconds.
    let step = step_ms.unsigned_abs().max(1) as f64;
    let mut unit = UNITS
        .iter()
        .rposition(|(length, _)| *length <= step)
        .unwrap_or(0);
    while unit > 0 && (size / UNITS[unit].0).round() == 0.0 {
        unit -= 1;
    }
    while unit + 1 < UNITS.len() && (size / UNITS[unit].0).round() >= TOO_MANY {
        unit += 1;
    }
    let (length, name) = UNITS[unit];
    // A unit that is longer than the distance between two labels needs a
    // decimal place. Two labels of one axis would otherwise round to one text.
    let mut decimals = 0;
    let mut resolution = length;
    while decimals < 2 && resolution > step {
        decimals += 1;
        resolution /= 10.0;
    }
    format!("{sign}{:.*}{name}", decimals, size / length)
}

/// Returns the sentence that describes a window.
///
/// A window is the pair of moments that a time range names, and the moment
/// that named them. The sentence names the width of the window, and it names
/// the distance from the present moment back to the end of the window. A
/// program writes this sentence while the window stands still in the past.
///
/// # Examples
///
/// ```
/// use thunderbox_core::TimeWindow;
/// use thunderbox_widgets::format_window;
/// let window = TimeWindow { from: 0, to: 300_000, now: 3_900_000 };
/// assert_eq!(
///     format_window(window),
///     "The panels show 5 minutes that ended 1 hour ago."
/// );
/// ```
pub fn format_window(window: TimeWindow) -> String {
    let width = duration_text(window.span_ms());
    let behind = window.now.saturating_sub(window.to);
    if behind <= 0 {
        return format!("The panels show the last {width}.");
    }
    format!(
        "The panels show {width} that ended {} ago.",
        duration_text(behind)
    )
}

/// Returns the text of a length of time, in words.
///
/// The function selects the unit from the length itself, and it writes the
/// name of that unit in full. The text `5 minutes` is an example.
fn duration_text(ms: i64) -> String {
    let size = ms.max(0) as f64;
    if size < MINUTE_MS {
        return units((size / SECOND_MS).round() as i64, "second");
    }
    if size < HOUR_MS {
        return units((size / MINUTE_MS).round() as i64, "minute");
    }
    if size < DAY_MS {
        return units((size / HOUR_MS).round() as i64, "hour");
    }
    units((size / DAY_MS).round() as i64, "day")
}

/// Returns a count and the name of a unit.
///
/// The function adds the letter `s` to the name for every count except 1. It
/// raises a count of 0 to 1, because a length of time that a person reads is
/// never nothing.
fn units(count: i64, name: &str) -> String {
    let count = count.max(1);
    if count == 1 {
        format!("1 {name}")
    } else {
        format!("{count} {name}s")
    }
}

/// Returns the text of a number, with a letter that states the scale.
///
/// The letters are `K`, `M`, `G`, `T`, and `P`. Each letter is 1000 times the
/// letter before it. A system monitor shows a number in this form.
///
/// The function rounds the number before it keeps a letter. The value 999.9
/// rounds to 1000 with no decimal place, so the function takes the next letter
/// and writes `1K`. The whole part of the text therefore holds three digits at
/// most. The letter `P` is the one exception, because no larger letter exists.
fn si(v: f64, decimals: Option<u8>) -> String {
    const UNITS: [&str; 6] = ["", "K", "M", "G", "T", "P"];
    let sign = if v < 0.0 { "-" } else { "" };
    let mut n = v.abs();
    let mut i = 0;
    while n >= 1000.0 && i < UNITS.len() - 1 {
        n /= 1000.0;
        i += 1;
    }
    let mut text = scaled(n, i, decimals);
    // The rounded number can reach 1000. The function then takes the next
    // letter, and the number needs fewer digits.
    if whole_digits(&text) > 3 && i < UNITS.len() - 1 {
        n /= 1000.0;
        i += 1;
        text = scaled(n, i, decimals);
    }
    format!("{sign}{text}{}", UNITS[i])
}

/// Returns the text of a number that the caller divided by its scale.
///
/// The `step` argument is the position of the letter in the list of `si`. The
/// function selects no decimal place by itself for a number with no letter, or
/// for a number of 100 or more.
fn scaled(n: f64, step: usize, decimals: Option<u8>) -> String {
    let automatic = if step == 0 || n >= 100.0 { 0 } else { 1 };
    places(n, decimals, automatic)
}

/// Returns the number of digits before the decimal point of a text.
///
/// The text holds no sign, because `si` formats the absolute value.
fn whole_digits(text: &str) -> usize {
    text.find('.').unwrap_or(text.len())
}

/// Returns the text of a number with a number of decimal places.
///
/// The function keeps every place that `decimals` asks for. It uses
/// `automatic` places when `decimals` is `None`, and it then removes a zero at
/// the end of the text.
fn places(v: f64, decimals: Option<u8>, automatic: usize) -> String {
    match decimals {
        Some(d) => format!("{v:.*}", usize::from(d)),
        None => {
            let s = format!("{v:.automatic$}");
            if s.contains('.') {
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            } else {
                s
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_value_returns_a_whole_number_for_a_percent() {
        assert_eq!(format_value(42.7, Some("percent"), None), "43%");
        assert_eq!(format_value(0.0, Some("percent"), None), "0%");
    }

    #[test]
    fn format_value_adds_a_letter_for_the_scale_of_a_byte_count() {
        assert_eq!(format_value(999.0, Some("bytes"), None), "999B");
        assert_eq!(format_value(1_500.0, Some("bytes"), None), "1.5KB");
        assert_eq!(format_value(2_000_000.0, Some("bytes"), None), "2MB");
        assert_eq!(format_value(3_000_000_000.0, Some("bytes"), None), "3GB");
    }

    #[test]
    fn format_value_removes_the_decimal_from_a_large_number() {
        assert_eq!(format_value(150_000.0, None, None), "150K");
    }

    #[test]
    fn format_value_keeps_the_sign_of_a_negative_number() {
        assert_eq!(format_value(-1_500.0, None, None), "-1.5K");
    }

    #[test]
    fn format_value_returns_a_symbol_for_a_value_that_is_not_finite() {
        assert_eq!(format_value(f64::NAN, None, None), MISSING);
        assert_eq!(format_value(f64::INFINITY, Some("bytes"), None), MISSING);
    }

    #[test]
    fn format_value_uses_the_plain_scale_for_an_unknown_unit() {
        assert_eq!(format_value(1_500.0, Some("furlongs"), None), "1.5K");
    }

    #[test]
    fn format_value_keeps_every_place_that_the_caller_asks_for() {
        assert_eq!(format_value(42.0, Some("percent"), Some(2)), "42.00%");
        assert_eq!(format_value(1_500.0, Some("bytes"), Some(2)), "1.50KB");
        assert_eq!(format_value(2.6, Some("seconds"), Some(0)), "3s");
        assert_eq!(format_value(7.0, None, Some(1)), "7.0");
    }

    /// A number that rounds to 1000 takes the next letter of the scale.
    /// Each row holds a value, a unit, a count of decimal places, and the text.
    #[test]
    fn format_value_takes_the_next_letter_when_the_rounded_number_reaches_1000() {
        for (value, unit, decimals, expected) in [
            (999.9, Some("bytes"), None, "1KB"),
            (999_999.0, Some("bytes"), Some(2), "1.00MB"),
            (999_999.0, Some("bytes"), None, "1MB"),
            (999.6, None, Some(0), "1K"),
            (-999.9, Some("bytes"), None, "-1KB"),
            (999_600.0, None, None, "1M"),
            (999_600_000.0, None, None, "1G"),
            (999_600_000_000.0, None, None, "1T"),
            (999_600_000_000_000.0, None, None, "1P"),
        ] {
            assert_eq!(
                format_value(value, unit, decimals),
                expected,
                "The value {value} must take the next letter."
            );
        }
    }

    /// A number that rounds to less than 1000 keeps its letter. Each row holds
    /// a value just below one step of the scale.
    #[test]
    fn format_value_keeps_the_letter_of_a_number_that_rounds_below_1000() {
        for (value, unit, decimals, expected) in [
            (999.4, None, None, "999"),
            (999.4, Some("bytes"), Some(0), "999B"),
            (999_400.0, None, None, "999K"),
            (999_400_000.0, None, None, "999M"),
            (999_400_000_000.0, None, None, "999G"),
            (999_400_000_000_000.0, None, None, "999T"),
            (999_994.0, Some("bytes"), Some(2), "999.99KB"),
            (1234.5678, None, Some(4), "1.2346K"),
        ] {
            assert_eq!(
                format_value(value, unit, decimals),
                expected,
                "The value {value} must keep its letter."
            );
        }
    }

    /// No letter is larger than `P`. A number of 1000 `P` or more therefore
    /// keeps the letter `P`, and its whole part holds four digits or more.
    #[test]
    fn format_value_keeps_the_largest_letter_for_a_number_of_1000_or_more() {
        assert_eq!(format_value(1.5e18, None, Some(0)), "1500P");
        assert_eq!(format_value(999.6e15, None, None), "1000P");
    }

    #[test]
    fn format_time_returns_the_time_of_day() {
        assert_eq!(format_time(0), "00:00:00");
        assert_eq!(format_time(3_661_000), "01:01:01");
        assert_eq!(format_time(86_399_000), "23:59:59");
        assert_eq!(format_time(86_400_000), "00:00:00");
    }

    #[test]
    fn format_time_reads_a_moment_before_the_epoch() {
        assert_eq!(format_time(-1000), "23:59:59");
    }

    #[test]
    fn format_from_now_names_the_present_moment() {
        assert_eq!(format_from_now(0, 1_000), "now");
    }

    #[test]
    fn format_from_now_marks_a_moment_in_the_past_with_a_minus_sign() {
        assert_eq!(format_from_now(-500, 100), "-500ms");
        assert_eq!(format_from_now(-30_000, 15_000), "-30s");
        assert_eq!(format_from_now(-600_000, 300_000), "-10m");
        assert_eq!(format_from_now(-7_200_000, 10_800_000), "-2h");
    }

    #[test]
    fn format_from_now_marks_a_moment_in_the_future_with_a_plus_sign() {
        assert_eq!(format_from_now(300_000, 150_000), "+5m");
        assert_eq!(format_from_now(5_000, 1_000), "+5s");
    }

    /// The unit comes from the distance between two labels, and not from the
    /// value. An axis of one hour that ended one hour ago would otherwise read
    /// the same text twice.
    #[test]
    fn format_from_now_selects_the_unit_from_the_distance_between_two_labels() {
        let step = 1_800_000;
        let labels = [
            format_from_now(-7_200_000, step),
            format_from_now(-5_400_000, step),
            format_from_now(-3_600_000, step),
        ];
        assert_eq!(labels, ["-120m", "-90m", "-60m"]);
    }

    /// A value that rounds to zero in the unit of the axis takes a shorter
    /// unit. The text `-0m` states both a distance and no distance, and the
    /// function keeps the word `now` for a distance of nothing.
    #[test]
    fn format_from_now_writes_no_count_of_zero() {
        assert_eq!(format_from_now(-1_000, 60_000), "-1s");
        assert_eq!(format_from_now(1_000, 60_000), "+1s");
        assert_eq!(format_from_now(-30, 60_000), "-30ms");
    }

    /// A value that needs four digits in the unit of the axis takes a longer
    /// unit. A label of eight characters does not fit a narrow panel. The
    /// three labels must still differ, so the longer unit keeps one decimal
    /// place.
    #[test]
    fn format_from_now_writes_no_count_of_four_digits() {
        let step = 500;
        let labels = [
            format_from_now(-61_000, step),
            format_from_now(-60_500, step),
            format_from_now(-60_000, step),
        ];
        assert_eq!(labels, ["-61.0s", "-60.5s", "-60.0s"]);
        assert!(
            labels[0] != labels[1] && labels[1] != labels[2],
            "Two labels of one axis must differ. {labels:?}"
        );
    }

    /// The axis of a panel and the status row of a program must name one unit
    /// for one width. The widest range of this project is ten years, and both
    /// texts then name a count of days.
    #[test]
    fn format_from_now_names_a_count_of_days() {
        let ten_years = thunderbox_core::MAX_SPAN_MS;
        assert_eq!(format_from_now(-ten_years, ten_years / 2), "-3650d");
        assert_eq!(format_from_now(-172_800_000, 86_400_000), "-2d");
    }

    #[test]
    fn format_window_names_the_width_and_the_distance_back_in_time() {
        let window = TimeWindow {
            from: 0,
            to: 300_000,
            now: 3_900_000,
        };
        assert_eq!(
            format_window(window),
            "The panels show 5 minutes that ended 1 hour ago."
        );
    }

    #[test]
    fn format_window_names_a_window_that_ends_at_the_present_moment() {
        let window = TimeWindow {
            from: 0,
            to: 300_000,
            now: 300_000,
        };
        assert_eq!(format_window(window), "The panels show the last 5 minutes.");
    }

    #[test]
    fn format_window_writes_the_name_of_one_unit_without_the_letter_s() {
        let window = TimeWindow {
            from: 0,
            to: 60_000,
            now: 120_000,
        };
        assert_eq!(
            format_window(window),
            "The panels show 1 minute that ended 1 minute ago."
        );
    }

    #[test]
    fn format_window_names_a_window_of_days() {
        let window = TimeWindow {
            from: 0,
            to: 172_800_000,
            now: 172_800_000,
        };
        assert_eq!(format_window(window), "The panels show the last 2 days.");
    }

    #[test]
    fn format_window_does_not_panic_for_the_limits_of_the_number() {
        let ends = [i64::MIN, -1, 0, 1, i64::MAX];
        for from in ends {
            for to in ends {
                for now in ends {
                    let text = format_window(TimeWindow { from, to, now });
                    assert!(text.starts_with("The panels show"), "The text is {text}");
                }
            }
        }
    }

    #[test]
    fn format_window_names_a_window_that_ended_a_very_long_time_ago() {
        let window = TimeWindow {
            from: i64::MIN,
            to: i64::MIN,
            now: i64::MAX,
        };
        let text = format_window(window);
        assert!(text.contains("that ended"), "The text is {text}");
        assert!(text.ends_with(" days ago."), "The text is {text}");
    }

    #[test]
    fn format_from_now_does_not_panic_for_the_limits_of_the_number() {
        for offset in [i64::MIN, i64::MIN + 1, -1, 1, i64::MAX] {
            for step in [i64::MIN, -1, 0, 1, i64::MAX] {
                let text = format_from_now(offset, step);
                assert!(!text.is_empty());
            }
        }
    }
}
