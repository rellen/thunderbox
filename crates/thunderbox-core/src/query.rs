//! A query, and the context that a data source needs to answer it.

use serde::{Deserialize, Serialize};

use crate::{TimeRange, Timestamp};

/// The name of a configured data source. The value `prometheus-prod` is an
/// example.
pub type SourceUid = String;

/// One query. This crate does not read the text of the query.
///
/// thunderbox has no common query language. The `expr` field holds PromQL for a
/// Prometheus data source, and SQL for a SQL data source. Each data source
/// reads its own form. PromQL is the query language of Prometheus.
///
/// A query holds no pattern. The pattern of the `legend` field stays on
/// [`crate::Target`], and [`crate::legend::apply`] applies it after the data
/// source answers. A data source therefore cannot apply a pattern, and no
/// pattern crosses the boundary to a data source in another process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Query {
    /// The data source that answers this query.
    pub source: SourceUid,
    /// The query text. Only the data source reads this text.
    pub expr: String,
}

/// A query, and the context that a data source needs to answer it well.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryRequest {
    /// The query to answer.
    pub query: Query,
    /// The start of the range, in milliseconds after the Unix epoch.
    pub from: Timestamp,
    /// The end of the range, in milliseconds after the Unix epoch.
    pub to: Timestamp,
    /// The present moment, in milliseconds after the Unix epoch.
    ///
    /// A person can move the range into the past, and the end of the range is
    /// then not the present moment. A data source that measures the machine
    /// stamps a new sample with this field, and never with `to`.
    pub now: Timestamp,
    /// The largest number of points that the data source returns. The usable
    /// width of the panel supplies this number. A data source reduces the
    /// number of points to this limit when it can.
    pub max_data_points: u32,
    /// The suggested distance between two points, in milliseconds.
    pub step_ms: i64,
}

impl QueryRequest {
    /// Returns a request. The range and the point limit supply `step_ms`.
    ///
    /// The method raises a `max_data_points` value of 0 to 1. It also raises a
    /// `step_ms` value of 0 to 1. A caller can therefore divide by either
    /// number.
    ///
    /// The method keeps `now` beside the two ends of the range. A person who
    /// moved the range into the past makes those two moments differ.
    pub fn new(query: Query, range: &TimeRange, now: Timestamp, max_data_points: u32) -> Self {
        let (from, to) = range.resolve(now);
        let budget = max_data_points.max(1);
        let step_ms = (to.saturating_sub(from) / i64::from(budget)).max(1);
        Self {
            query,
            from,
            to,
            now,
            max_data_points: budget,
            step_ms,
        }
    }

    /// Returns the width of the range in milliseconds. The result is 1 or more.
    ///
    /// The fields are public, so `from` can be larger than `to`. The result is
    /// then 1. A range that is wider than the number can hold gives the largest
    /// number.
    pub fn span_ms(&self) -> i64 {
        self.to.saturating_sub(self.from).max(1)
    }
}

/// Returns the number of points that a panel asks for.
///
/// A marker can address more positions than the terminal has cells. A braille
/// marker, for example, addresses two positions in each cell. The number of
/// points therefore follows the horizontal resolution of the marker. See
/// ADR 0009 in `docs/contributor/adr`.
pub fn max_data_points(panel_width_cols: u16, horizontal_subcells: u16) -> u32 {
    u32::from(panel_width_cols.max(1)) * u32::from(horizontal_subcells.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RelativeTime;

    fn range(span_ms: i64) -> TimeRange {
        TimeRange {
            from: RelativeTime::Offset(-span_ms),
            to: RelativeTime::Now,
        }
    }

    #[test]
    fn new_divides_the_range_by_the_number_of_points() {
        let q = Query {
            source: "s".into(),
            expr: "up".into(),
        };
        let req = QueryRequest::new(q, &range(60_000), 1_000_000, 200);
        assert_eq!(req.from, 940_000);
        assert_eq!(req.to, 1_000_000);
        assert_eq!(req.step_ms, 300);
        assert_eq!(req.span_ms(), 60_000);
    }

    /// A data source that measures the machine stamps a new sample with the
    /// present moment. The request must therefore carry that moment, and not
    /// the end of the range alone.
    #[test]
    fn new_keeps_the_present_moment_beside_the_range() {
        let q = Query {
            source: "s".into(),
            expr: "up".into(),
        };
        let past = TimeRange {
            from: RelativeTime::Absolute(100_000),
            to: RelativeTime::Absolute(160_000),
        };
        let req = QueryRequest::new(q, &past, 1_000_000, 200);
        assert_eq!(req.to, 160_000);
        assert_eq!(req.now, 1_000_000);
    }

    #[test]
    fn new_raises_a_step_of_zero_to_one() {
        let q = Query {
            source: "s".into(),
            expr: "up".into(),
        };
        let req = QueryRequest::new(q, &range(10), 1_000_000, 5_000);
        assert_eq!(req.step_ms, 1);
    }

    #[test]
    fn new_raises_a_point_limit_of_zero_to_one() {
        let q = Query {
            source: "s".into(),
            expr: "up".into(),
        };
        let req = QueryRequest::new(q, &range(60_000), 1_000_000, 0);
        assert_eq!(req.max_data_points, 1);
    }

    fn query() -> Query {
        Query {
            source: "s".into(),
            expr: "up".into(),
        }
    }

    #[test]
    fn new_does_not_panic_for_a_range_at_the_limit_of_the_number() {
        let wide = TimeRange {
            from: RelativeTime::Absolute(i64::MIN),
            to: RelativeTime::Absolute(i64::MAX),
        };
        let req = QueryRequest::new(query(), &wide, 0, 1);
        assert_eq!(req.span_ms(), i64::MAX);
        assert_eq!(req.step_ms, i64::MAX, "One point covers the whole range.");
        let far = TimeRange {
            from: RelativeTime::Offset(i64::MIN),
            to: RelativeTime::Offset(i64::MAX),
        };
        for now in [i64::MIN, -1, 0, 1, i64::MAX] {
            let req = QueryRequest::new(query(), &far, now, u32::MAX);
            assert!(req.step_ms >= 1, "The step must stay at 1 or more.");
            assert!(req.span_ms() >= 1, "The width must stay at 1 or more.");
        }
    }

    #[test]
    fn span_ms_does_not_panic_for_ends_in_the_wrong_order() {
        let mut req = QueryRequest::new(query(), &range(60_000), 1_000_000, 200);
        req.from = i64::MAX;
        req.to = i64::MIN;
        assert_eq!(
            req.span_ms(),
            1,
            "A start after the end gives a width of 1."
        );
        req.from = i64::MIN;
        req.to = i64::MAX;
        assert_eq!(req.span_ms(), i64::MAX);
        req.from = 5;
        req.to = 3;
        assert_eq!(req.span_ms(), 1);
    }

    #[test]
    fn max_data_points_multiplies_the_width_by_the_sub_cells() {
        assert_eq!(max_data_points(80, 2), 160);
        assert_eq!(max_data_points(80, 1), 80);
    }

    #[test]
    fn max_data_points_returns_one_point_for_a_width_of_zero() {
        assert_eq!(max_data_points(0, 2), 2);
    }
}
