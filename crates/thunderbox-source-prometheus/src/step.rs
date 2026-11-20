//! The step of a range query, and the grid of times that the step makes.
//!
//! A panel states how many points it can draw. This module turns that number
//! into a step, which is the distance between two points. It then aligns the
//! range to that step. ADR 0009 in `docs/contributor/adr` states the rule that this module
//! follows.
//!
//! The module does three things to the step that the panel suggests:
//!
//! - it raises the step to the smallest step that the data source permits;
//! - it raises the step to the next round value, so that the times of the
//!   points stay the same while a person watches the dashboard;
//! - it raises the step again until the number of points fits the limit.
//!
//! The module never lowers the step. A step that only grows keeps the number of
//! points at or below the limit of the panel.

use thunderbox_core::{QueryRequest, Timestamp};

/// The smallest step that this data source asks for, in milliseconds.
///
/// The Prometheus HTTP API takes the start and the end of a range query as a
/// whole number of seconds. Every value in [`ROUND_STEPS`] is therefore a whole
/// number of seconds, and every grid falls on a whole second.
pub(crate) const MIN_STEP_MS: i64 = 1_000;

/// The largest number of points that a Prometheus server returns for one range
/// query. The server refuses a query that asks for more.
pub(crate) const MAX_POINTS: usize = 11_000;

/// The steps that this module selects from, in milliseconds.
///
/// A person reads a chart of a round step more easily than a chart of a step of
/// 1873 milliseconds. A round step also holds the times of the points still
/// while the range moves, because the module aligns the range to the step.
const ROUND_STEPS: [i64; 21] = [
    1_000,         // 1 second
    2_000,         // 2 seconds
    5_000,         // 5 seconds
    10_000,        // 10 seconds
    15_000,        // 15 seconds
    30_000,        // 30 seconds
    60_000,        // 1 minute
    120_000,       // 2 minutes
    300_000,       // 5 minutes
    600_000,       // 10 minutes
    900_000,       // 15 minutes
    1_800_000,     // 30 minutes
    3_600_000,     // 1 hour
    7_200_000,     // 2 hours
    10_800_000,    // 3 hours
    21_600_000,    // 6 hours
    43_200_000,    // 12 hours
    86_400_000,    // 1 day
    172_800_000,   // 2 days
    604_800_000,   // 7 days
    2_592_000_000, // 30 days
];

/// Returns the smallest round step that is `step_ms` or more.
///
/// The function returns a whole multiple of the largest round step when
/// `step_ms` passes that value.
fn round_up(step_ms: i64) -> i64 {
    let step = step_ms.max(MIN_STEP_MS);
    if let Some(round) = ROUND_STEPS.iter().find(|s| **s >= step) {
        return *round;
    }
    let largest = ROUND_STEPS[ROUND_STEPS.len() - 1];
    let count = (step - 1) / largest + 1;
    largest.saturating_mul(count)
}

/// The times that a range query returns.
///
/// A Prometheus range query returns one point at the start, and then one point
/// after each step, up to the end. Both ends belong to the grid. The data
/// source builds the time column of a frame from this grid. It then puts each
/// sample that the server returned into its own place. A place that holds no
/// sample keeps a hole in the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Grid {
    /// The time of the first point, in milliseconds after the Unix epoch.
    pub(crate) start: Timestamp,
    /// The time of the last point, in milliseconds after the Unix epoch.
    pub(crate) end: Timestamp,
    /// The distance between two points, in milliseconds.
    pub(crate) step_ms: i64,
}

impl Grid {
    /// Returns the grid for one request.
    ///
    /// The `min_step_ms` parameter states the smallest step that the person who
    /// wrote the dashboard permits. The function raises a value below
    /// [`MIN_STEP_MS`] to that constant.
    pub(crate) fn for_request(request: &QueryRequest, min_step_ms: i64) -> Self {
        let limit = usize::try_from(request.max_data_points)
            .unwrap_or(MAX_POINTS)
            .clamp(1, MAX_POINTS);
        let mut step = round_up(request.step_ms.max(min_step_ms));
        loop {
            let grid = Self::aligned(request.from, request.to, step);
            if grid.len() <= limit {
                return grid;
            }
            let next = round_up(step + 1);
            // The step has reached the largest value that this type holds,
            // so it cannot grow again. The loop must stop here.
            if next <= step {
                return grid;
            }
            step = next;
        }
    }

    /// Returns a grid of this step that covers the range.
    ///
    /// The function moves both ends back to the point of the grid at or before
    /// them. The times of the points therefore do not move while the range
    /// moves, and a chart that refreshes does not shake.
    fn aligned(from: Timestamp, to: Timestamp, step_ms: i64) -> Self {
        let step_ms = step_ms.max(MIN_STEP_MS);
        let start = from.div_euclid(step_ms) * step_ms;
        let end = (to.div_euclid(step_ms) * step_ms).max(start);
        Self {
            start,
            end,
            step_ms,
        }
    }

    /// Returns the number of points in the grid. The result is 1 or more.
    pub(crate) fn len(&self) -> usize {
        let steps = (self.end - self.start) / self.step_ms;
        usize::try_from(steps).unwrap_or(0) + 1
    }

    /// Returns the time of every point, in order.
    pub(crate) fn timestamps(&self) -> Vec<Timestamp> {
        (0..self.len())
            .map(|i| self.start + self.step_ms * i as i64)
            .collect()
    }

    /// Returns the place of a time in the grid.
    ///
    /// The method returns `None` for a time that no point of the grid holds.
    /// The Prometheus server returns a sample for a point of the grid only. A
    /// `None` result therefore means that the answer did not match the
    /// request.
    pub(crate) fn slot(&self, at: Timestamp) -> Option<usize> {
        if at < self.start || at > self.end {
            return None;
        }
        let offset = at - self.start;
        if offset % self.step_ms != 0 {
            return None;
        }
        usize::try_from(offset / self.step_ms).ok()
    }

    /// Returns the start as a whole number of seconds after the Unix epoch.
    pub(crate) fn start_seconds(&self) -> i64 {
        self.start.div_euclid(1_000)
    }

    /// Returns the end as a whole number of seconds after the Unix epoch.
    pub(crate) fn end_seconds(&self) -> i64 {
        self.end.div_euclid(1_000)
    }

    /// Returns the step as a number of seconds.
    pub(crate) fn step_seconds(&self) -> f64 {
        self.step_ms as f64 / 1_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thunderbox_core::{Query, RelativeTime, TimeRange};

    fn request(span_ms: i64, limit: u32) -> QueryRequest {
        QueryRequest::new(
            Query {
                source: "p".into(),
                expr: "up".into(),
            },
            &TimeRange {
                from: RelativeTime::Offset(-span_ms),
                to: RelativeTime::Now,
            },
            1_700_000_000_000,
            limit,
        )
    }

    #[test]
    fn round_up_returns_the_next_round_step() {
        assert_eq!(round_up(1), 1_000);
        assert_eq!(round_up(1_000), 1_000);
        assert_eq!(round_up(1_001), 2_000);
        assert_eq!(round_up(1_875), 2_000);
        assert_eq!(round_up(20_000), 30_000);
        assert_eq!(round_up(3_600_000), 3_600_000);
    }

    #[test]
    fn round_up_returns_a_multiple_of_the_largest_step() {
        let largest = ROUND_STEPS[ROUND_STEPS.len() - 1];
        assert_eq!(round_up(largest + 1), largest * 2);
        assert_eq!(round_up(largest * 3), largest * 3);
    }

    #[test]
    fn every_round_step_is_a_whole_number_of_seconds() {
        for step in ROUND_STEPS {
            assert_eq!(
                step % 1_000,
                0,
                "The Prometheus API takes whole seconds. The step {step} is not one."
            );
        }
    }

    #[test]
    fn for_request_raises_the_step_to_a_round_value() {
        // Five minutes across 160 points suggests 1875 milliseconds.
        let grid = Grid::for_request(&request(300_000, 160), MIN_STEP_MS);
        assert_eq!(grid.step_ms, 2_000);
    }

    #[test]
    fn for_request_raises_the_step_to_the_smallest_step_of_the_dashboard() {
        let grid = Grid::for_request(&request(300_000, 160), 15_000);
        assert_eq!(grid.step_ms, 15_000);
    }

    #[test]
    fn for_request_never_returns_more_points_than_the_panel_asked_for() {
        for span in [1_000, 60_000, 300_000, 3_600_000, 86_400_000, 604_800_000] {
            for limit in [1, 2, 3, 7, 40, 80, 160, 400, 1_920] {
                let grid = Grid::for_request(&request(span, limit), MIN_STEP_MS);
                assert!(
                    grid.len() <= limit as usize,
                    "A span of {span} and a limit of {limit} gave {} points at a \
                     step of {}.",
                    grid.len(),
                    grid.step_ms
                );
            }
        }
    }

    #[test]
    fn for_request_stays_inside_the_limit_of_the_server() {
        // The panel asks for far more points than a server returns.
        let grid = Grid::for_request(&request(604_800_000, u32::MAX), MIN_STEP_MS);
        assert!(
            grid.len() <= MAX_POINTS,
            "The grid holds {} points.",
            grid.len()
        );
    }

    #[test]
    fn for_request_puts_both_ends_on_the_step() {
        let grid = Grid::for_request(&request(300_000, 160), MIN_STEP_MS);
        assert_eq!(grid.start % grid.step_ms, 0);
        assert_eq!(grid.end % grid.step_ms, 0);
    }

    #[test]
    fn the_grid_does_not_move_when_the_range_moves_by_less_than_one_step() {
        let step = 60_000;
        let a = Grid::aligned(1_700_000_000_000, 1_700_000_300_000, step);
        let b = Grid::aligned(1_700_000_001_000, 1_700_000_301_000, step);
        assert_eq!(a, b, "A chart that refreshes must not shake.");
    }

    #[test]
    fn timestamps_returns_one_time_for_each_point() {
        let grid = Grid {
            start: 1_000,
            end: 4_000,
            step_ms: 1_000,
        };
        assert_eq!(grid.len(), 4);
        assert_eq!(grid.timestamps(), vec![1_000, 2_000, 3_000, 4_000]);
    }

    #[test]
    fn slot_returns_the_place_of_a_time_that_the_grid_holds() {
        let grid = Grid {
            start: 1_000,
            end: 4_000,
            step_ms: 1_000,
        };
        assert_eq!(grid.slot(1_000), Some(0));
        assert_eq!(grid.slot(4_000), Some(3));
        assert_eq!(grid.slot(999), None);
        assert_eq!(grid.slot(4_001), None);
        assert_eq!(grid.slot(1_500), None);
    }

    #[test]
    fn a_range_that_is_shorter_than_one_step_gives_one_point() {
        let grid = Grid::aligned(1_700_000_000_100, 1_700_000_000_200, 60_000);
        assert_eq!(grid.len(), 1);
        assert_eq!(grid.start, grid.end);
    }

    #[test]
    fn the_seconds_of_the_grid_match_the_milliseconds() {
        let grid = Grid {
            start: 1_700_000_000_000,
            end: 1_700_000_060_000,
            step_ms: 30_000,
        };
        assert_eq!(grid.start_seconds(), 1_700_000_000);
        assert_eq!(grid.end_seconds(), 1_700_000_060);
        assert_eq!(grid.step_seconds(), 30.0);
    }
}
