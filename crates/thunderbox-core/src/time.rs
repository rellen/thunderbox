//! Time values for a dashboard.

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// A count of milliseconds after the Unix epoch.
pub type Timestamp = i64;

/// The narrowest range that [`TimeRange::with_span_ms`] returns.
///
/// A range of one second is about as wide as the distance between two samples
/// of a data source of this project. A narrower range would often hold no
/// point at all.
pub const MIN_SPAN_MS: i64 = 1_000;

/// The widest range that [`TimeRange::with_span_ms`] returns.
///
/// The value is ten years. A person who makes the range wider many times
/// therefore never reaches the limit of the number, and the arithmetic stays
/// correct.
pub const MAX_SPAN_MS: i64 = 10 * 365 * 86_400_000;

/// A window is the pair of moments that a time range names, and the moment
/// that named them.
///
/// A person who moves the range makes the end of the window and the present
/// moment two different moments, and a panel needs both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeWindow {
    /// The start of the window, in milliseconds after the Unix epoch.
    pub from: Timestamp,
    /// The end of the window, in milliseconds after the Unix epoch.
    pub to: Timestamp,
    /// The present moment, in milliseconds after the Unix epoch.
    pub now: Timestamp,
}

impl TimeWindow {
    /// Returns the width of the window in milliseconds.
    ///
    /// The result is always 1 or more, so that a caller can divide by it.
    pub fn span_ms(&self) -> i64 {
        (self.to - self.from).max(1)
    }
}

/// The time range that every panel on a dashboard shares.
///
/// This type holds a relative value when it can. A saved dashboard therefore
/// means "the last five minutes". It does not mean a fixed five minutes on the
/// day when a person saved the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeRange {
    /// The start of the range. The default value is five minutes in the past.
    #[serde(default = "TimeRange::default_from")]
    pub from: RelativeTime,
    /// The end of the range. The default value is the present moment.
    #[serde(default)]
    pub to: RelativeTime,
}

impl TimeRange {
    fn default_from() -> RelativeTime {
        RelativeTime::Offset(-300_000)
    }

    /// Returns the window that this range names at one moment.
    ///
    /// The `now` parameter supplies the present moment. The method puts the
    /// smaller value first, even if the caller supplies the range in the
    /// opposite order.
    ///
    /// # Examples
    ///
    /// ```
    /// use thunderbox_core::{RelativeTime, TimeRange};
    /// let range = TimeRange { from: RelativeTime::Offset(-60_000), to: RelativeTime::Now };
    /// let window = range.window(1_000_000);
    /// assert_eq!((window.from, window.to, window.now), (940_000, 1_000_000, 1_000_000));
    /// ```
    pub fn window(&self, now: Timestamp) -> TimeWindow {
        let from = self.from.resolve(now);
        let to = self.to.resolve(now);
        let (from, to) = if from <= to { (from, to) } else { (to, from) };
        TimeWindow { from, to, now }
    }

    /// Returns the start and the end as absolute milliseconds.
    ///
    /// The `now` parameter supplies the reference time. The method puts the
    /// smaller value first, even if the caller supplies the range in the
    /// opposite order. It reads [`TimeRange::window`], which holds that rule.
    pub fn resolve(&self, now: Timestamp) -> (Timestamp, Timestamp) {
        let window = self.window(now);
        (window.from, window.to)
    }

    /// Returns the width of the range in milliseconds.
    ///
    /// The method resolves the range first. The result is always 1 or more, so
    /// that a caller can divide by it.
    pub fn span_ms(&self, now: Timestamp) -> i64 {
        self.window(now).span_ms()
    }

    /// Returns `true` if this range follows the clock.
    ///
    /// A range that holds a fixed moment at each end names one window for
    /// ever. Every other range names a new window as the clock moves. A caller
    /// reads this method, and it can then tell a person that the window stands
    /// still.
    pub fn moves_with_the_clock(&self) -> bool {
        self.from.moves_with_the_clock() || self.to.moves_with_the_clock()
    }

    /// Returns the same range, with a fixed moment at each end.
    ///
    /// The window that the range names then stands still while a person
    /// studies it. The method returns the two ends in order, as
    /// [`TimeRange::resolve`] does.
    ///
    /// # Examples
    ///
    /// ```
    /// use thunderbox_core::{RelativeTime, TimeRange};
    /// let fixed = TimeRange::default().to_absolute(1_000_000);
    /// assert_eq!(fixed.to, RelativeTime::Absolute(1_000_000));
    /// assert!(!fixed.moves_with_the_clock());
    /// ```
    pub fn to_absolute(&self, now: Timestamp) -> Self {
        let window = self.window(now);
        Self {
            from: RelativeTime::Absolute(window.from),
            to: RelativeTime::Absolute(window.to),
        }
    }

    /// Returns the same range, moved by a count of milliseconds.
    ///
    /// A negative value moves the range into the past. The method keeps the
    /// behaviour of each end, so a range that follows the clock still follows
    /// the clock. It needs no present moment, because each end moves itself.
    ///
    /// # Examples
    ///
    /// ```
    /// use thunderbox_core::{RelativeTime, TimeRange};
    /// let range = TimeRange {
    ///     from: RelativeTime::Absolute(0),
    ///     to: RelativeTime::Absolute(60_000),
    /// };
    /// let back = range.moved(-1_000);
    /// assert_eq!(back.from, RelativeTime::Absolute(-1_000));
    /// assert_eq!(back.to, RelativeTime::Absolute(59_000));
    /// ```
    pub fn moved(&self, by_ms: i64) -> Self {
        Self {
            from: self.from.moved(by_ms),
            to: self.to.moved(by_ms),
        }
    }

    /// Returns this range, moved by a count of milliseconds.
    ///
    /// The method fixes a moment at each end, so the window stands still and
    /// the panels stop at it. A move forward is the one exception. It returns
    /// a range of the same width that follows the clock, in each of these two
    /// conditions:
    ///
    /// - the move carries the end of the window to the present moment, or past
    ///   it. No data source of this project answers a question about the
    ///   future;
    /// - the move stops less than one step before the present moment. The
    ///   clock moves on between two key presses. A move back and then a move
    ///   forward would otherwise leave a window a few milliseconds behind the
    ///   present moment. That window would look live, and it would never move
    ///   again.
    ///
    /// A move back always holds the window still, even for a range that ends
    /// after the present moment.
    ///
    /// # Examples
    ///
    /// ```
    /// use thunderbox_core::TimeRange;
    /// let live = TimeRange::default();
    /// let back = live.panned(1_000_000, -60_000);
    /// assert!(!back.moves_with_the_clock());
    /// assert_eq!(back.span_ms(1_000_000), live.span_ms(1_000_000));
    /// assert_eq!(back.panned(1_000_000, 60_000), live);
    /// // The clock moved on between the two presses. The range still follows
    /// // the clock again.
    /// assert_eq!(back.panned(1_000_001, 60_000), live);
    /// ```
    pub fn panned(&self, now: Timestamp, by_ms: i64) -> Self {
        let span = self.span_ms(now);
        let moved = self.to_absolute(now).moved(by_ms);
        let end = moved.to.resolve(now);
        if by_ms > 0 && now.saturating_sub(end) < by_ms {
            return Self {
                from: RelativeTime::Offset(-span),
                to: RelativeTime::Now,
            };
        }
        moved
    }

    /// Returns a range of a stated width that ends at the same moment.
    ///
    /// The method holds the later end still, and it moves the earlier end. The
    /// later end keeps its kind, so a range that follows the clock still
    /// follows the clock. The method raises a width below [`MIN_SPAN_MS`], and
    /// it lowers a width above [`MAX_SPAN_MS`].
    ///
    /// # Examples
    ///
    /// ```
    /// use thunderbox_core::{RelativeTime, TimeRange};
    /// let range = TimeRange::default();
    /// let wider = range.with_span_ms(1_000_000, range.span_ms(1_000_000) * 2);
    /// assert_eq!(wider.from, RelativeTime::Offset(-600_000));
    /// assert_eq!(wider.to, RelativeTime::Now);
    /// ```
    pub fn with_span_ms(&self, now: Timestamp, span_ms: i64) -> Self {
        let span = span_ms.clamp(MIN_SPAN_MS, MAX_SPAN_MS);
        let later = if self.to.resolve(now) >= self.from.resolve(now) {
            self.to
        } else {
            self.from
        };
        Self {
            from: later.moved(-span),
            to: later,
        }
    }
}

impl Default for TimeRange {
    fn default() -> Self {
        Self {
            from: Self::default_from(),
            to: RelativeTime::Now,
        }
    }
}

/// A point in time. The value is either fixed, or relative to the present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RelativeTime {
    #[default]
    /// The present moment.
    Now,
    /// A count of milliseconds away from the present. A negative value is in
    /// the past.
    Offset(i64),
    /// A count of milliseconds after the Unix epoch.
    Absolute(Timestamp),
}

impl RelativeTime {
    /// Returns this point as absolute milliseconds.
    ///
    /// The `now` parameter supplies the reference time.
    pub fn resolve(&self, now: Timestamp) -> Timestamp {
        match self {
            Self::Now => now,
            Self::Offset(d) => now + d,
            Self::Absolute(t) => *t,
        }
    }

    /// Returns this point, moved by a count of milliseconds.
    ///
    /// A negative value moves the point into the past. The method keeps the
    /// behaviour of the point. A point that follows the clock still follows
    /// the clock, and [`RelativeTime::Now`] becomes an offset from the present
    /// moment. A move of no milliseconds returns the point itself, so a range
    /// that follows the clock keeps the text form that it had.
    ///
    /// The method holds a value at the limit of the number when the move would
    /// carry that value past the limit.
    ///
    /// # Examples
    ///
    /// ```
    /// use thunderbox_core::RelativeTime;
    /// assert_eq!(RelativeTime::Now.moved(-1_000), RelativeTime::Offset(-1_000));
    /// assert_eq!(RelativeTime::Now.moved(0), RelativeTime::Now);
    /// ```
    pub fn moved(self, by_ms: i64) -> Self {
        if by_ms == 0 {
            return self;
        }
        match self {
            Self::Now => Self::Offset(by_ms),
            Self::Offset(d) => Self::Offset(d.saturating_add(by_ms)),
            Self::Absolute(t) => Self::Absolute(t.saturating_add(by_ms)),
        }
    }

    /// Returns `true` if this point follows the clock.
    ///
    /// A point that holds a fixed moment names one moment for ever. Every
    /// other point names a new moment as the clock moves.
    pub fn moves_with_the_clock(self) -> bool {
        !matches!(self, Self::Absolute(_))
    }
}

/// Returns the present moment, in milliseconds after the Unix epoch.
///
/// The function reads the clock of the machine. A function of this module that
/// needs the present moment takes it as a parameter, so a test can hold the
/// clock still. The function returns 0 when the machine reports a moment
/// before the Unix epoch.
pub fn now_ms() -> Timestamp {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as Timestamp)
        .unwrap_or(0)
}

/// Reads a duration and returns a count of milliseconds.
///
/// The text is a whole number and then a unit. The permitted units are `ms`,
/// `s`, `m`, `h`, `d`, and `w`. For example, `90s` and `5m` are both correct.
///
/// # Errors
///
/// This function returns an error in each of these conditions:
///
/// - the text has no unit;
/// - the first part of the text is not a whole number;
/// - the unit is not in the list above.
pub fn parse_duration_ms(s: &str) -> Result<i64> {
    let s = s.trim();
    let split = s
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| Error::Time {
            input: s.to_string(),
            reason: "expected a unit suffix, one of s/m/h/d/w".into(),
        })?;
    let (digits, unit) = s.split_at(split);
    let n: i64 = digits.parse().map_err(|_| Error::Time {
        input: s.to_string(),
        reason: "leading value is not a number".into(),
    })?;
    let mult = match unit {
        "ms" => 1,
        "s" => 1_000,
        "m" => 60_000,
        "h" => 3_600_000,
        "d" => 86_400_000,
        "w" => 604_800_000,
        other => {
            return Err(Error::Time {
                input: s.to_string(),
                reason: format!("unknown unit {other:?}, expected one of ms/s/m/h/d/w"),
            });
        }
    };
    Ok(n * mult)
}

/// Reads a point in time from the text form that a dashboard file uses.
///
/// The permitted forms are `now`, `now-<duration>`, `now+<duration>`, and a
/// count of milliseconds after the Unix epoch.
///
/// # Errors
///
/// This function returns an error if the text matches none of these forms.
fn parse_relative(s: &str) -> Result<RelativeTime> {
    let s = s.trim();
    if s == "now" {
        return Ok(RelativeTime::Now);
    }
    if let Some(rest) = s.strip_prefix("now-") {
        return Ok(RelativeTime::Offset(-parse_duration_ms(rest)?));
    }
    if let Some(rest) = s.strip_prefix("now+") {
        return Ok(RelativeTime::Offset(parse_duration_ms(rest)?));
    }
    s.parse::<i64>()
        .map(RelativeTime::Absolute)
        .map_err(|_| Error::Time {
            input: s.to_string(),
            reason: "expected \"now\", \"now-<duration>\", or epoch milliseconds".into(),
        })
}

impl std::fmt::Display for RelativeTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Now => write!(f, "now"),
            Self::Offset(d) if *d < 0 => write!(f, "now-{}ms", -d),
            Self::Offset(d) => write!(f, "now+{d}ms"),
            Self::Absolute(t) => write!(f, "{t}"),
        }
    }
}

impl Serialize for RelativeTime {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for RelativeTime {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        parse_relative(&raw).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_ms_reads_each_unit() {
        assert_eq!(parse_duration_ms("500ms").unwrap(), 500);
        assert_eq!(parse_duration_ms("30s").unwrap(), 30_000);
        assert_eq!(parse_duration_ms("5m").unwrap(), 300_000);
        assert_eq!(parse_duration_ms("2h").unwrap(), 7_200_000);
        assert_eq!(parse_duration_ms("7d").unwrap(), 604_800_000);
    }

    #[test]
    fn parse_duration_ms_returns_an_error_for_a_wrong_unit() {
        assert!(parse_duration_ms("5").is_err());
        assert!(parse_duration_ms("5y").is_err());
        assert!(parse_duration_ms("banana").is_err());
    }

    #[test]
    fn parse_relative_reads_each_form() {
        assert_eq!(parse_relative("now").unwrap(), RelativeTime::Now);
        assert_eq!(
            parse_relative("now-5m").unwrap(),
            RelativeTime::Offset(-300_000)
        );
        assert_eq!(
            parse_relative("1700000000000").unwrap(),
            RelativeTime::Absolute(1_700_000_000_000)
        );
    }

    #[test]
    fn resolve_returns_the_start_and_the_end() {
        let r = TimeRange {
            from: RelativeTime::Offset(-60_000),
            to: RelativeTime::Now,
        };
        assert_eq!(r.resolve(1_000_000), (940_000, 1_000_000));
        assert_eq!(r.span_ms(1_000_000), 60_000);
    }

    #[test]
    fn resolve_puts_the_smaller_value_first() {
        let r = TimeRange {
            from: RelativeTime::Now,
            to: RelativeTime::Offset(-60_000),
        };
        assert_eq!(r.resolve(1_000_000), (940_000, 1_000_000));
    }

    /// The moment that every test in this module calls the present moment.
    const NOW: Timestamp = 1_000_000;

    /// Returns a range of five minutes that ends at the present moment.
    fn live() -> TimeRange {
        TimeRange::default()
    }

    /// Returns a range of two minutes that ended one hour before the present
    /// moment.
    fn fixed_in_the_past() -> TimeRange {
        TimeRange {
            from: RelativeTime::Absolute(NOW - 3_720_000),
            to: RelativeTime::Absolute(NOW - 3_600_000),
        }
    }

    #[test]
    fn window_holds_the_two_ends_and_the_present_moment() {
        let window = live().window(NOW);
        assert_eq!(window.from, NOW - 300_000);
        assert_eq!(window.to, NOW);
        assert_eq!(window.now, NOW);
        assert_eq!(window.span_ms(), 300_000);
    }

    #[test]
    fn a_window_of_no_width_still_reports_a_width_of_one() {
        let window = TimeWindow {
            from: NOW,
            to: NOW,
            now: NOW,
        };
        assert_eq!(window.span_ms(), 1, "A caller divides by this number.");
    }

    #[test]
    fn moved_keeps_a_point_that_follows_the_clock() {
        assert_eq!(RelativeTime::Now.moved(-500), RelativeTime::Offset(-500));
        assert_eq!(
            RelativeTime::Offset(-500).moved(-500),
            RelativeTime::Offset(-1_000)
        );
        assert_eq!(
            RelativeTime::Absolute(10).moved(-4),
            RelativeTime::Absolute(6)
        );
    }

    #[test]
    fn a_move_of_no_milliseconds_returns_the_range_itself() {
        assert_eq!(live().moved(0), live(), "The range must keep its form.");
    }

    #[test]
    fn a_move_holds_a_value_at_the_limit_of_the_number() {
        let far = RelativeTime::Absolute(Timestamp::MAX);
        assert_eq!(far.moved(1_000), far);
        assert_eq!(
            RelativeTime::Offset(i64::MIN).moved(-1_000),
            RelativeTime::Offset(i64::MIN)
        );
    }

    #[test]
    fn a_range_that_reaches_the_present_stands_still_after_a_move_back() {
        let back = live().panned(NOW, -150_000);
        assert!(
            !back.moves_with_the_clock(),
            "A person who studies the past needs a window that stands still."
        );
        assert_eq!(back.resolve(NOW), (NOW - 450_000, NOW - 150_000));
        assert_eq!(back.span_ms(NOW), live().span_ms(NOW));
    }

    #[test]
    fn a_move_that_reaches_the_present_follows_the_clock_again() {
        let back = fixed_in_the_past();
        let ahead = back.panned(NOW, 3_600_000);
        assert!(
            ahead.moves_with_the_clock(),
            "A range whose window reaches the present must follow the clock."
        );
        assert_eq!(
            ahead.span_ms(NOW),
            back.span_ms(NOW),
            "The move must keep the width that a person chose."
        );
        assert_eq!(ahead.to, RelativeTime::Now);
    }

    #[test]
    fn a_range_that_is_fixed_in_the_past_stays_fixed_after_a_move() {
        let further = fixed_in_the_past().panned(NOW, -60_000);
        assert!(!further.moves_with_the_clock());
        assert_eq!(further.resolve(NOW), (NOW - 3_780_000, NOW - 3_660_000));
    }

    /// The program reads the clock again for each key press. A move forward
    /// therefore never lands on the present moment exactly. A move that stops
    /// less than one step short must still follow the clock. The dashboard
    /// would otherwise stand still for ever, a moment behind the present.
    #[test]
    fn a_move_forward_follows_the_clock_again_after_the_clock_moved_on() {
        let back = live().panned(NOW, -150_000);
        let ahead = back.panned(NOW + 1, 150_000);
        assert!(
            ahead.moves_with_the_clock(),
            "The range must follow the clock again. {ahead:?}"
        );
        assert_eq!(
            ahead,
            live(),
            "One move back and one move forward must return the range itself."
        );
    }

    /// A move forward of many steps must still stop at the present moment.
    #[test]
    fn a_move_forward_that_passes_the_present_moment_stops_at_it() {
        let ahead = fixed_in_the_past().panned(NOW, 7_200_000);
        assert!(ahead.moves_with_the_clock());
        assert_eq!(ahead.resolve(NOW), (NOW - 120_000, NOW));
    }

    /// Returns a range of ten minutes that ends five minutes after the present
    /// moment. The text form `now+5m` names such an end.
    fn ends_in_the_future() -> TimeRange {
        TimeRange {
            from: RelativeTime::Offset(-300_000),
            to: RelativeTime::Offset(300_000),
        }
    }

    /// A move back holds the window still, whatever the range names. A range
    /// that ends after the present moment must not become a range that follows
    /// the clock.
    #[test]
    fn a_move_back_holds_still_a_range_that_ends_in_the_future() {
        let back = ends_in_the_future().panned(NOW, -300_000);
        assert!(
            !back.moves_with_the_clock(),
            "A move back must hold the window still. {back:?}"
        );
        assert_eq!(back.resolve(NOW), (NOW - 600_000, NOW));
    }

    /// The two moves must not return the same range. A move forward stops at
    /// the present moment, and a move back holds the window still.
    #[test]
    fn a_move_back_and_a_move_forward_differ_for_a_range_that_ends_in_the_future() {
        let range = ends_in_the_future();
        let back = range.panned(NOW, -300_000);
        let ahead = range.panned(NOW, 300_000);
        assert_ne!(back, ahead, "The two moves must differ. {back:?} {ahead:?}");
        assert!(
            ahead.moves_with_the_clock(),
            "A move forward stops at the present moment. {ahead:?}"
        );
        assert_eq!(ahead.resolve(NOW), (NOW - 600_000, NOW));
    }

    /// A range can name its later moment first. A move must not cross the two
    /// ends, and it must keep the width.
    #[test]
    fn a_move_keeps_the_two_ends_in_order() {
        let crossed = TimeRange {
            from: RelativeTime::Now,
            to: RelativeTime::Offset(-60_000),
        };
        let back = crossed.panned(NOW, -600_000);
        let (from, to) = back.resolve(NOW);
        assert!(from < to, "The start must stay before the end. {back:?}");
        assert_eq!(to - from, 60_000, "The move must keep the width.");
    }

    #[test]
    fn to_absolute_returns_a_range_that_names_one_window_for_ever() {
        let fixed = live().to_absolute(NOW);
        assert_eq!(fixed.from, RelativeTime::Absolute(NOW - 300_000));
        assert_eq!(fixed.to, RelativeTime::Absolute(NOW));
        assert_eq!(
            fixed.resolve(NOW + 60_000),
            (NOW - 300_000, NOW),
            "A window that stands still names the same two moments later."
        );
    }

    #[test]
    fn moves_with_the_clock_reports_the_range_that_follows_the_clock() {
        assert!(live().moves_with_the_clock());
        assert!(!fixed_in_the_past().moves_with_the_clock());
        assert!(
            TimeRange {
                from: RelativeTime::Absolute(0),
                to: RelativeTime::Now,
            }
            .moves_with_the_clock(),
            "One end that follows the clock moves the whole range."
        );
    }

    #[test]
    fn a_wider_range_holds_the_later_end_still() {
        let wider = live().with_span_ms(NOW, 600_000);
        assert_eq!(wider.from, RelativeTime::Offset(-600_000));
        assert_eq!(
            wider.to,
            RelativeTime::Now,
            "A range that follows the clock must still follow it."
        );
    }

    #[test]
    fn a_narrower_range_holds_the_later_end_of_a_fixed_window_still() {
        let narrower = fixed_in_the_past().with_span_ms(NOW, 60_000);
        assert_eq!(narrower.to, RelativeTime::Absolute(NOW - 3_600_000));
        assert_eq!(narrower.resolve(NOW), (NOW - 3_660_000, NOW - 3_600_000));
    }

    #[test]
    fn a_range_never_becomes_narrower_than_the_narrowest_range() {
        let narrower = live().with_span_ms(NOW, 0);
        assert_eq!(narrower.span_ms(NOW), MIN_SPAN_MS);
        assert_eq!(live().with_span_ms(NOW, -5).span_ms(NOW), MIN_SPAN_MS);
    }

    #[test]
    fn a_range_never_becomes_wider_than_the_widest_range() {
        let wider = live().with_span_ms(NOW, i64::MAX);
        assert_eq!(wider.span_ms(NOW), MAX_SPAN_MS);
    }

    /// A person who makes the range wider many times must not reach the limit
    /// of the number. [`MAX_SPAN_MS`] holds each step inside the values that
    /// the arithmetic can carry.
    #[test]
    fn many_steps_that_widen_the_range_stay_inside_the_number() {
        let mut range = live();
        for _ in 0..80 {
            let span = range.span_ms(NOW);
            range = range.with_span_ms(NOW, span.saturating_mul(2));
        }
        assert_eq!(range.span_ms(NOW), MAX_SPAN_MS);
    }

    #[test]
    fn a_range_that_names_its_later_moment_first_still_holds_that_moment_still() {
        let crossed = TimeRange {
            from: RelativeTime::Now,
            to: RelativeTime::Offset(-60_000),
        };
        let wider = crossed.with_span_ms(NOW, 120_000);
        assert_eq!(wider.resolve(NOW), (NOW - 120_000, NOW));
    }

    #[test]
    fn now_ms_returns_a_moment_after_the_year_2020() {
        assert!(
            now_ms() > 1_577_836_800_000,
            "The clock of the machine must report a moment of this century."
        );
    }
}
