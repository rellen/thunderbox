//! Time values for a dashboard.

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// A count of milliseconds after the Unix epoch.
pub type Timestamp = i64;

/// The number of milliseconds in one day.
const MS_PER_DAY: i64 = 86_400_000;

/// The units of a duration, from the smallest to the largest.
///
/// Each entry holds the name of a unit and the number of milliseconds in that
/// unit. [`parse_duration_ms`] reads the names, and the `Display`
/// implementation of [`RelativeTime`] writes them. One table serves both, so
/// the two cannot differ.
const UNITS: [(&str, i64); 6] = [
    ("ms", 1),
    ("s", 1_000),
    ("m", 60_000),
    ("h", 3_600_000),
    ("d", MS_PER_DAY),
    ("w", 7 * MS_PER_DAY),
];

/// The narrowest range that [`TimeRange::with_span_ms`] returns, and the
/// narrowest range that [`TimeRange::validate`] accepts.
///
/// A range of one second is about as wide as the distance between two samples
/// of a data source of this project. A narrower range would often hold no
/// point at all.
pub const MIN_SPAN_MS: i64 = 1_000;

/// The widest range that [`TimeRange::with_span_ms`] returns, and the widest
/// range that [`TimeRange::validate`] accepts.
///
/// The value is ten years, which is 3650 days. A person who makes the range
/// wider many times with the keys of the program reaches this limit long
/// before the limit of the number. The method [`TimeRange::with_span_ms`]
/// holds the width at this limit. A range that a dashboard file or a program
/// builds can be wider, so [`TimeRange::validate`] checks it.
pub const MAX_SPAN_MS: i64 = 10 * 365 * MS_PER_DAY;

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
    /// The result is always 1 or more, so that a caller can divide by it. A
    /// window that is wider than the number can hold gives the largest number.
    pub fn span_ms(&self) -> i64 {
        self.to.saturating_sub(self.from).max(1)
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

    /// Checks that this range names a window that the program can draw.
    ///
    /// The `now` parameter supplies the present moment. The method resolves
    /// both ends against that moment, and it puts the smaller value first. The
    /// window must then be at least [`MIN_SPAN_MS`] wide, and at most
    /// [`MAX_SPAN_MS`] wide. A window of exactly one of those widths passes.
    ///
    /// # Errors
    ///
    /// This method returns an error in each of these conditions:
    ///
    /// - an end is an offset, and the sum of the offset and `now` does not fit
    ///   in a count of milliseconds. The error names the field of that end;
    /// - the window is wider than [`MAX_SPAN_MS`];
    /// - the window is narrower than [`MIN_SPAN_MS`].
    ///
    /// # Examples
    ///
    /// ```
    /// use thunderbox_core::{RelativeTime, TimeRange};
    /// let now = 1_000_000;
    /// assert!(TimeRange::default().validate(now).is_ok());
    /// let too_narrow = TimeRange {
    ///     from: RelativeTime::Offset(-500),
    ///     to: RelativeTime::Now,
    /// };
    /// assert!(too_narrow.validate(now).is_err());
    /// ```
    pub fn validate(&self, now: Timestamp) -> Result<()> {
        let from = self
            .from
            .checked_resolve(now)
            .ok_or_else(|| self.end_error("from", self.from))?;
        let to = self
            .to
            .checked_resolve(now)
            .ok_or_else(|| self.end_error("to", self.to))?;
        // Two ends that each fit in an `i64` can lie further apart than an
        // `i64` holds, so the width uses a wider number.
        let width = i128::from(from.max(to)) - i128::from(from.min(to));
        if width > i128::from(MAX_SPAN_MS) {
            return Err(self.width_error(format!("wider than {} days", MAX_SPAN_MS / MS_PER_DAY)));
        }
        if width < i128::from(MIN_SPAN_MS) {
            return Err(self.width_error(format!("narrower than {MIN_SPAN_MS} milliseconds")));
        }
        Ok(())
    }

    /// Returns the error for an end that does not fit in a count of
    /// milliseconds.
    fn end_error(&self, field: &str, value: RelativeTime) -> Error {
        Error::Dashboard(format!(
            "The `[time]` table holds a `{field}` field of {value}. The sum of the present \
             moment and that value does not fit in a count of milliseconds."
        ))
    }

    /// Returns the error for a range that is too wide or too narrow.
    ///
    /// The `limit` parameter holds the words that state the fault, such as
    /// `wider than 3650 days`.
    fn width_error(&self, limit: String) -> Error {
        Error::Dashboard(format!(
            "The `[time]` table holds a `from` field of {} and a `to` field of {}. That \
             range is {limit}.",
            self.from, self.to
        ))
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

    /// Returns `true` if the later end of this range is at the present moment
    /// or after it.
    ///
    /// A range of this kind is live. A later end is [`RelativeTime::Now`], or
    /// an offset of 0 or more. A range can list its two ends in either order,
    /// because [`TimeRange::window`] puts the smaller value first. The method
    /// therefore reads both ends. A range that ends at a fixed moment, or at an
    /// offset before the present moment, is not live.
    ///
    /// # Examples
    ///
    /// ```
    /// use thunderbox_core::TimeRange;
    /// let live = TimeRange::default();
    /// assert!(live.ends_at_the_present());
    /// assert!(!live.to_absolute(1_000_000).ends_at_the_present());
    /// ```
    pub fn ends_at_the_present(&self) -> bool {
        let reaches = |end: RelativeTime| match end {
            RelativeTime::Now => true,
            RelativeTime::Offset(d) => d >= 0,
            RelativeTime::Absolute(_) => false,
        };
        reaches(self.from) || reaches(self.to)
    }

    /// Returns the same window at `now`, as a range that follows the clock.
    ///
    /// Each end of the new range is an offset from the present moment, so the
    /// window moves with the clock and keeps its width. A person who studied a
    /// held window can then watch it move on. A range whose end reaches the
    /// present moment, or passes it, returns the live form. The live form ends
    /// at [`RelativeTime::Now`] and starts one width earlier.
    ///
    /// The method returns the range as it was when an offset does not fit in
    /// a count of milliseconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use thunderbox_core::{RelativeTime, TimeRange};
    /// let now = 1_000_000;
    /// let held = TimeRange {
    ///     from: RelativeTime::Absolute(now - 180_000),
    ///     to: RelativeTime::Absolute(now - 60_000),
    /// };
    /// let following = held.following(now);
    /// assert_eq!(following.from, RelativeTime::Offset(-180_000));
    /// assert_eq!(following.to, RelativeTime::Offset(-60_000));
    /// // One minute later, the window has moved one minute.
    /// assert_eq!(following.resolve(now + 60_000), (now - 120_000, now));
    /// ```
    pub fn following(&self, now: Timestamp) -> Self {
        let window = self.window(now);
        if window.to >= now {
            // The width is 1 or more, so the negation cannot overflow.
            let span = window.span_ms();
            if now.checked_sub(span).is_none() {
                return self.clone();
            }
            return Self {
                from: RelativeTime::Offset(-span),
                to: RelativeTime::Now,
            };
        }
        let (Some(from), Some(to)) = (window.from.checked_sub(now), window.to.checked_sub(now))
        else {
            return self.clone();
        };
        Self {
            from: RelativeTime::Offset(from),
            to: RelativeTime::Offset(to),
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
    /// The method returns the range as it was when the move would carry an end
    /// past the limit of the number. It also returns the range as it was when
    /// the range is wider than the number can hold. The width of the range
    /// therefore never changes because an end reached that limit.
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
        let window = self.window(now);
        let span = window.span_ms();
        let (Some(from), Some(to), Some(_)) = (
            window.from.checked_add(by_ms),
            window.to.checked_add(by_ms),
            window.to.checked_sub(window.from),
        ) else {
            return self.clone();
        };
        if by_ms > 0 && now.saturating_sub(to) < by_ms {
            // The method moves the end to the present moment. The start of the
            // new window is the present moment less the width. That value
            // must fit, or the new window would have another width.
            if now.checked_sub(span).is_none() {
                return self.clone();
            }
            // The width is 1 or more, so the negation cannot overflow.
            return Self {
                from: RelativeTime::Offset(-span),
                to: RelativeTime::Now,
            };
        }
        Self {
            from: RelativeTime::Absolute(from),
            to: RelativeTime::Absolute(to),
        }
    }

    /// Returns a range of a stated width that ends at the same moment.
    ///
    /// The method holds the later end still, and it moves the earlier end. The
    /// later end keeps its kind, so a range that follows the clock still
    /// follows the clock. The method raises a width below [`MIN_SPAN_MS`], and
    /// it lowers a width above [`MAX_SPAN_MS`].
    ///
    /// The method returns the range as it was when the earlier end would pass
    /// the limit of the number. The new range would then have another width
    /// than the caller asked for.
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
        // The width is at least `MIN_SPAN_MS`, so the negation cannot overflow.
        let wider = Self {
            from: later.moved(-span),
            to: later,
        };
        if wider.span_ms(now) != span {
            return self.clone();
        }
        wider
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
    ///
    /// The method holds the value at the limit of the number when the sum of
    /// `now` and an offset would pass that limit.
    pub fn resolve(&self, now: Timestamp) -> Timestamp {
        match self {
            Self::Now => now,
            Self::Offset(d) => now.saturating_add(*d),
            Self::Absolute(t) => *t,
        }
    }

    /// Returns this point as absolute milliseconds, or `None` when the sum of
    /// `now` and an offset does not fit in a count of milliseconds.
    fn checked_resolve(self, now: Timestamp) -> Option<Timestamp> {
        match self {
            Self::Now => Some(now),
            Self::Offset(d) => now.checked_add(d),
            Self::Absolute(t) => Some(t),
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
/// - the unit is not in the list above;
/// - the duration in milliseconds does not fit in an `i64`.
pub fn parse_duration_ms(s: &str) -> Result<i64> {
    let s = s.trim();
    let split = s
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| Error::Time {
            input: s.to_string(),
            reason: "The text holds no unit. The known units are ms, s, m, h, d, and w.".into(),
        })?;
    let (digits, unit) = s.split_at(split);
    let too_large = || Error::Time {
        input: s.to_string(),
        reason: format!(
            "A duration cannot be larger than {} milliseconds.",
            i64::MAX
        ),
    };
    let n: i64 = match digits.parse() {
        Ok(n) => n,
        // The text before the unit holds digits alone. A text of one digit or
        // more that an `i64` cannot hold is therefore a number that is too
        // large.
        Err(_) if !digits.is_empty() => return Err(too_large()),
        Err(_) => {
            return Err(Error::Time {
                input: s.to_string(),
                reason: "The text does not start with a whole number.".into(),
            });
        }
    };
    let Some(&(_, mult)) = UNITS.iter().find(|(name, _)| *name == unit) else {
        return Err(Error::Time {
            input: s.to_string(),
            reason: format!(
                "The unit {unit:?} is not known. The known units are ms, s, m, h, d, and w."
            ),
        });
    };
    n.checked_mul(mult).ok_or_else(too_large)
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
        // The parser reads digits alone, so the duration is 0 or more and the
        // negation cannot overflow.
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

/// Returns a count of milliseconds as a whole number of the largest unit that
/// divides the count exactly, and the name of that unit.
///
/// The count 0 takes the unit `ms`.
fn largest_exact_unit(ms: u64) -> (u64, &'static str) {
    UNITS
        .iter()
        .rev()
        .find(|(_, mult)| ms.is_multiple_of(mult.unsigned_abs()))
        .filter(|_| ms != 0)
        .map_or((ms, "ms"), |(name, mult)| (ms / mult.unsigned_abs(), name))
}

impl std::fmt::Display for RelativeTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Now => write!(f, "now"),
            Self::Offset(d) => {
                let sign = if *d < 0 { '-' } else { '+' };
                let (count, unit) = largest_exact_unit(d.unsigned_abs());
                write!(f, "now{sign}{count}{unit}")
            }
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
    fn following_keeps_the_window_at_the_present_moment() {
        let held = fixed_in_the_past();
        let following = held.following(NOW);
        assert_eq!(following.resolve(NOW), held.resolve(NOW));
        assert_eq!(following.from, RelativeTime::Offset(-3_720_000));
        assert_eq!(following.to, RelativeTime::Offset(-3_600_000));
    }

    #[test]
    fn a_range_from_following_moves_with_the_clock() {
        let following = fixed_in_the_past().following(NOW);
        assert!(following.moves_with_the_clock());
        let (from, to) = following.resolve(NOW + 60_000);
        assert_eq!(from, NOW - 3_720_000 + 60_000);
        assert_eq!(to, NOW - 3_600_000 + 60_000);
        assert_eq!(to - from, 120_000, "The width must stay the same.");
    }

    #[test]
    fn following_returns_the_live_form_when_the_end_reaches_the_present_moment() {
        let expected = TimeRange {
            from: RelativeTime::Offset(-300_000),
            to: RelativeTime::Now,
        };
        assert_eq!(live().following(NOW), expected);
        let at_now = TimeRange {
            from: RelativeTime::Absolute(NOW - 300_000),
            to: RelativeTime::Absolute(NOW),
        };
        assert_eq!(at_now.following(NOW), expected);
        let in_the_future = TimeRange {
            from: RelativeTime::Absolute(NOW - 100_000),
            to: RelativeTime::Absolute(NOW + 200_000),
        };
        assert_eq!(
            in_the_future.following(NOW),
            TimeRange {
                from: RelativeTime::Offset(-300_000),
                to: RelativeTime::Now,
            },
            "The live form keeps the width."
        );
    }

    #[test]
    fn following_returns_the_range_as_it_was_when_an_offset_would_not_fit() {
        let range = TimeRange {
            from: RelativeTime::Absolute(i64::MIN + 10),
            to: RelativeTime::Absolute(i64::MIN + 20),
        };
        assert_eq!(range.following(i64::MAX), range);
    }

    #[test]
    fn ends_at_the_present_reports_the_range_that_ends_now_or_later() {
        assert!(live().ends_at_the_present());
        let later = TimeRange {
            from: RelativeTime::Offset(-1_000),
            to: RelativeTime::Offset(5_000),
        };
        assert!(later.ends_at_the_present());
        assert!(!fixed_in_the_past().ends_at_the_present());
        assert!(!fixed_in_the_past().following(NOW).ends_at_the_present());
    }

    #[test]
    fn ends_at_the_present_reads_a_range_whose_ends_stand_in_reverse_order() {
        let reversed = TimeRange {
            from: RelativeTime::Now,
            to: RelativeTime::Offset(-300_000),
        };
        assert!(reversed.ends_at_the_present());
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

    /// The two ends of a range that lie `width_ms` apart and stand still.
    fn fixed_of_width(width_ms: i64) -> TimeRange {
        TimeRange {
            from: RelativeTime::Absolute(0),
            to: RelativeTime::Absolute(width_ms),
        }
    }

    #[test]
    fn parse_duration_ms_returns_an_error_for_a_duration_that_does_not_fit() {
        assert!(parse_duration_ms("20000000000w").is_err());
        assert!(parse_duration_ms("9223372036854776s").is_err());
        assert!(parse_duration_ms("9223372036854775808ms").is_err());
    }

    #[test]
    fn parse_duration_ms_reads_a_duration_at_the_limit_of_the_number() {
        assert_eq!(
            parse_duration_ms("9223372036854775s").unwrap(),
            9_223_372_036_854_775_000
        );
        assert_eq!(
            parse_duration_ms("9223372036854775807ms").unwrap(),
            i64::MAX
        );
    }

    #[test]
    fn the_error_for_a_large_duration_names_the_input_and_the_limit() {
        // The second number and the third number do not fit before the
        // function reads the unit.
        for input in [
            "20000000000w",
            "9223372036854775808ms",
            "99999999999999999999s",
        ] {
            let text = parse_duration_ms(input).unwrap_err().to_string();
            assert!(text.contains(input), "The text must name the input. {text}");
            assert!(
                text.contains("9223372036854775807 milliseconds"),
                "The text must state the limit. {text}"
            );
        }
        let text = parse_duration_ms("s").unwrap_err().to_string();
        assert!(
            text.contains("does not start with a whole number"),
            "A text that holds no digit must name that fault. {text}"
        );
    }

    #[test]
    fn parse_relative_returns_an_error_for_an_offset_that_does_not_fit() {
        assert!(parse_relative("now-20000000000w").is_err());
        assert!(parse_relative("now+20000000000w").is_err());
    }

    #[test]
    fn parse_relative_reads_the_largest_offset() {
        assert_eq!(
            parse_relative("now-9223372036854775807ms").unwrap(),
            RelativeTime::Offset(-i64::MAX)
        );
        assert_eq!(
            parse_relative("now+9223372036854775807ms").unwrap(),
            RelativeTime::Offset(i64::MAX)
        );
    }

    #[test]
    fn display_writes_the_largest_unit_that_divides_an_offset() {
        let cases = [
            (-300_000, "now-5m"),
            (-90_000, "now-90s"),
            (604_800_000, "now+1w"),
            (-1_500, "now-1500ms"),
            (-86_400_000, "now-1d"),
            (7_200_000, "now+2h"),
            (-1_209_600_000, "now-2w"),
            (0, "now+0ms"),
        ];
        for (offset, text) in cases {
            assert_eq!(
                RelativeTime::Offset(offset).to_string(),
                text,
                "The offset {offset} must have this text form."
            );
        }
        assert_eq!(RelativeTime::Now.to_string(), "now");
        assert_eq!(RelativeTime::Absolute(-5).to_string(), "-5");
    }

    #[test]
    fn display_does_not_panic_for_the_limits_of_the_number() {
        assert_eq!(
            RelativeTime::Offset(i64::MIN).to_string(),
            "now-9223372036854775808ms"
        );
        assert_eq!(
            RelativeTime::Offset(i64::MAX).to_string(),
            "now+9223372036854775807ms"
        );
        assert_eq!(
            RelativeTime::Absolute(i64::MIN).to_string(),
            "-9223372036854775808"
        );
    }

    #[test]
    fn parse_relative_reads_each_text_that_display_writes() {
        let offsets = [
            0,
            1,
            -1,
            999,
            1_000,
            -1_000,
            1_500,
            -90_000,
            -300_000,
            3_600_000,
            -86_400_000,
            604_800_000,
            -1_209_600_000,
            3_650 * 86_400_000,
            i64::MAX,
            -i64::MAX,
            i64::MAX - 1,
            -(i64::MAX - 1),
        ];
        for offset in offsets {
            let point = RelativeTime::Offset(offset);
            let text = point.to_string();
            assert_eq!(
                parse_relative(&text).unwrap(),
                point,
                "The text {text} must return the same offset."
            );
        }
        for point in [RelativeTime::Now, RelativeTime::Absolute(i64::MIN)] {
            assert_eq!(parse_relative(&point.to_string()).unwrap(), point);
        }
    }

    #[test]
    fn serialize_writes_the_text_that_display_writes() {
        let range = TimeRange::default();
        let text = toml::to_string(&range).unwrap();
        assert!(text.contains("from = \"now-5m\""), "The text is {text}");
        assert_eq!(toml::from_str::<TimeRange>(&text).unwrap(), range);
    }

    #[test]
    fn resolve_holds_the_value_at_the_limit_of_the_number() {
        assert_eq!(RelativeTime::Offset(i64::MAX).resolve(1), i64::MAX);
        assert_eq!(RelativeTime::Offset(i64::MIN).resolve(-1), i64::MIN);
        assert_eq!(RelativeTime::Offset(-5).resolve(i64::MAX), i64::MAX - 5);
        assert_eq!(RelativeTime::Now.resolve(i64::MAX), i64::MAX);
        assert_eq!(RelativeTime::Absolute(i64::MIN).resolve(i64::MAX), i64::MIN);
    }

    #[test]
    fn a_window_that_is_wider_than_the_number_reports_the_largest_width() {
        let wide = TimeWindow {
            from: i64::MIN,
            to: i64::MAX,
            now: 0,
        };
        assert_eq!(wide.span_ms(), i64::MAX);
        let reversed = TimeWindow {
            from: i64::MAX,
            to: i64::MIN,
            now: 0,
        };
        assert_eq!(reversed.span_ms(), 1, "A caller divides by this number.");
    }

    #[test]
    fn validate_accepts_a_width_of_exactly_one_second() {
        assert!(fixed_of_width(MIN_SPAN_MS).validate(NOW).is_ok());
    }

    #[test]
    fn validate_returns_an_error_for_a_width_below_one_second() {
        let err = fixed_of_width(MIN_SPAN_MS - 1).validate(NOW).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("`from` field of 0"), "The text is {text}");
        assert!(text.contains("`to` field of 999"), "The text is {text}");
        assert!(
            text.contains("narrower than 1000 milliseconds"),
            "The text must state the limit. {text}"
        );
    }

    #[test]
    fn validate_accepts_a_width_of_exactly_ten_years() {
        assert!(fixed_of_width(MAX_SPAN_MS).validate(NOW).is_ok());
    }

    #[test]
    fn validate_returns_an_error_for_a_width_above_ten_years() {
        let err = fixed_of_width(MAX_SPAN_MS + 1).validate(NOW).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("`from` field of 0"), "The text is {text}");
        assert!(
            text.contains(&format!("`to` field of {}", MAX_SPAN_MS + 1)),
            "The text is {text}"
        );
        assert!(
            text.contains("wider than 3650 days"),
            "The text must state the limit. {text}"
        );
    }

    #[test]
    fn validate_measures_the_width_of_a_range_that_names_its_later_end_first() {
        let reversed = TimeRange {
            from: RelativeTime::Absolute(MAX_SPAN_MS),
            to: RelativeTime::Absolute(0),
        };
        assert!(reversed.validate(NOW).is_ok());
        let too_wide = TimeRange {
            from: RelativeTime::Absolute(MAX_SPAN_MS + 1),
            to: RelativeTime::Absolute(0),
        };
        assert!(too_wide.validate(NOW).is_err());
    }

    #[test]
    fn validate_returns_an_error_for_a_start_that_does_not_fit() {
        let range = TimeRange {
            from: RelativeTime::Offset(i64::MAX),
            to: RelativeTime::Now,
        };
        let text = range.validate(NOW).unwrap_err().to_string();
        assert!(
            text.contains("`from` field of now+9223372036854775807ms"),
            "{text}"
        );
    }

    #[test]
    fn validate_returns_an_error_for_an_end_that_does_not_fit() {
        let range = TimeRange {
            from: RelativeTime::Offset(-300_000),
            to: RelativeTime::Offset(i64::MIN),
        };
        let text = range.validate(-1).unwrap_err().to_string();
        assert!(
            text.contains("`to` field of now-9223372036854775808ms"),
            "{text}"
        );
    }

    #[test]
    fn validate_returns_an_error_for_two_ends_that_lie_further_apart_than_a_number_holds() {
        let range = TimeRange {
            from: RelativeTime::Absolute(i64::MIN),
            to: RelativeTime::Absolute(i64::MAX),
        };
        let text = range.validate(NOW).unwrap_err().to_string();
        assert!(text.contains("-9223372036854775808"), "The text is {text}");
        assert!(text.contains("9223372036854775807"), "The text is {text}");
        assert!(text.contains("`from`") && text.contains("`to`"), "{text}");
    }

    #[test]
    fn validate_accepts_the_default_range() {
        assert!(TimeRange::default().validate(NOW).is_ok());
        assert!(TimeRange::default().validate(0).is_ok());
    }

    #[test]
    fn panned_returns_the_range_as_it_was_when_an_end_would_pass_the_limit() {
        let high = TimeRange {
            from: RelativeTime::Absolute(i64::MAX - 1_000),
            to: RelativeTime::Absolute(i64::MAX),
        };
        assert_eq!(
            high.panned(NOW, 1),
            high,
            "A move forward passes the limit."
        );
        let low = TimeRange {
            from: RelativeTime::Absolute(i64::MIN),
            to: RelativeTime::Absolute(i64::MIN + 1_000),
        };
        assert_eq!(low.panned(NOW, -1), low, "A move back passes the limit.");
        assert_eq!(
            high.panned(NOW, -1).span_ms(NOW),
            1_000,
            "A move that stays inside the number must keep the width."
        );
    }

    #[test]
    fn panned_returns_the_range_as_it_was_when_the_range_is_wider_than_a_number() {
        let wide = TimeRange {
            from: RelativeTime::Absolute(i64::MIN + 1),
            to: RelativeTime::Absolute(i64::MAX - 1),
        };
        assert_eq!(wide.panned(NOW, 1), wide);
        assert_eq!(wide.panned(NOW, -1), wide);
    }

    #[test]
    fn panned_returns_the_range_as_it_was_when_the_present_moment_is_at_the_limit() {
        let range = TimeRange {
            from: RelativeTime::Absolute(i64::MIN),
            to: RelativeTime::Absolute(i64::MIN + 1_000),
        };
        assert_eq!(range.panned(i64::MIN, 500), range);
    }

    #[test]
    fn with_span_ms_returns_the_range_as_it_was_when_the_earlier_end_would_pass_the_limit() {
        let low = TimeRange {
            from: RelativeTime::Absolute(i64::MIN),
            to: RelativeTime::Absolute(i64::MIN + 1_000),
        };
        assert_eq!(low.with_span_ms(NOW, 2_000), low);
        let offset = TimeRange {
            from: RelativeTime::Offset(i64::MIN + 1_000),
            to: RelativeTime::Offset(i64::MIN + 2_000),
        };
        assert_eq!(offset.with_span_ms(0, 5_000), offset);
        let near = TimeRange {
            from: RelativeTime::Absolute(i64::MAX - 1_000),
            to: RelativeTime::Absolute(i64::MAX),
        };
        assert_eq!(near.with_span_ms(NOW, 2_000).span_ms(NOW), 2_000);
    }

    #[test]
    fn no_method_of_a_time_value_panics_at_the_limit_of_the_number() {
        let numbers = [i64::MIN, i64::MIN + 1, -1, 0, 1, i64::MAX - 1, i64::MAX];
        let mut points = vec![RelativeTime::Now];
        for n in numbers {
            points.push(RelativeTime::Offset(n));
            points.push(RelativeTime::Absolute(n));
        }
        for &now in &numbers {
            for &by_ms in &numbers {
                for &from in &points {
                    let _ = from.resolve(now);
                    let _ = from.moved(by_ms);
                    let _ = from.to_string();
                    for &to in &points {
                        let range = TimeRange { from, to };
                        let window = range.window(now);
                        assert!(window.from <= window.to, "{range:?} {now}");
                        assert!(window.span_ms() >= 1);
                        assert_eq!(range.resolve(now), (window.from, window.to));
                        assert!(range.span_ms(now) >= 1);
                        let following = range.following(now);
                        assert!(
                            following == range || following.span_ms(now) == range.span_ms(now),
                            "The method must keep the width. {range:?} {now}"
                        );
                        let _ = range.ends_at_the_present();
                        let fixed = range.to_absolute(now);
                        assert_eq!(fixed.resolve(0), (window.from, window.to));
                        let _ = range.moved(by_ms);
                        let _ = range.validate(now);
                        let panned = range.panned(now, by_ms);
                        assert!(
                            panned == range || panned.span_ms(now) == range.span_ms(now),
                            "A move must keep the width. {range:?} {now} {by_ms}"
                        );
                        let resized = range.with_span_ms(now, by_ms);
                        assert!(
                            resized == range
                                || resized.span_ms(now) == by_ms.clamp(MIN_SPAN_MS, MAX_SPAN_MS),
                            "A change of the width must give the width. {range:?} {now} {by_ms}"
                        );
                    }
                }
            }
        }
    }
}
