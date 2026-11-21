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

/// The largest number of times that [`Grid::for_request`] raises the step by
/// one round value.
///
/// A range of ten years and a limit of two points need fewer than 100 turns. A
/// range of the width of the whole number would need a billion turns. After
/// this number of turns the function takes a step that it calculates.
const MAX_TURNS: u32 = 2_000;

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
/// `step_ms` passes that value. The result is never larger than `i64::MAX`.
fn round_up(step_ms: i64) -> i64 {
    let step = step_ms.max(MIN_STEP_MS);
    if let Some(round) = ROUND_STEPS.iter().find(|s| **s >= step) {
        return *round;
    }
    let largest = ROUND_STEPS[ROUND_STEPS.len() - 1];
    // The step is 1000 or more, so the subtraction cannot overflow. The count
    // is small. The product stops at the largest number.
    let count = (step - 1) / largest + 1;
    largest.saturating_mul(count)
}

/// Returns the largest multiple of `step_ms` that is not larger than `at`.
///
/// The multiple can lie below the smallest number that an `i64` holds, for
/// example for `at` of `i64::MIN`. The function then returns the next multiple
/// above `at`, which always fits. The `step_ms` parameter must be 1 or more.
fn align_down(at: Timestamp, step_ms: i64) -> Timestamp {
    let cells = at.div_euclid(step_ms);
    // The count of cells is 0 or less when the product can overflow. The
    // count plus 1 is then 0 or less as well, and its product with the step
    // lies above `at`, so that product fits.
    cells
        .checked_mul(step_ms)
        .unwrap_or_else(|| (cells + 1) * step_ms)
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
        let mut turns = 0;
        loop {
            let grid = Self::aligned(request.from, request.to, step);
            if grid.len() <= limit {
                return grid;
            }
            let next = round_up(step.saturating_add(1));
            // The step has reached the largest value that this type holds,
            // so it cannot grow again. The loop must stop here.
            if next <= step {
                return grid;
            }
            turns += 1;
            // A request of ordinary width never reaches this limit, so its
            // grid holds the smallest round step that fits.
            step = if turns < MAX_TURNS {
                next
            } else {
                next.max(Self::step_that_fits(request, limit))
            };
        }
    }

    /// Returns a step that always gives a grid of `limit` points or fewer.
    ///
    /// The number of steps between the two ends of a grid is the difference of
    /// two floors. That difference is at most the floor of the width divided
    /// by the step, plus 1. A step `s` therefore gives few enough points when
    /// `s * (limit - 1)` is larger than the width.
    ///
    /// A limit of 1 has no such step. The function then returns the largest
    /// step, which gives the fewest points that a grid can hold.
    fn step_that_fits(request: &QueryRequest, limit: usize) -> i64 {
        // Two ends that each fit in an `i64` can lie further apart than an
        // `i64` holds, so the width uses a wider number.
        let width = i128::from(request.to) - i128::from(request.from);
        let gaps = i128::try_from(limit).unwrap_or(i128::MAX) - 1;
        if gaps < 1 {
            return i64::MAX;
        }
        round_up(i64::try_from(width / gaps + 1).unwrap_or(i64::MAX))
    }

    /// Returns a grid of this step that covers the range.
    ///
    /// The function moves both ends back to the point of the grid at or before
    /// them. The times of the points therefore do not move while the range
    /// moves, and a chart that refreshes does not shake.
    fn aligned(from: Timestamp, to: Timestamp, step_ms: i64) -> Self {
        let step_ms = step_ms.max(MIN_STEP_MS);
        let start = align_down(from, step_ms);
        let end = align_down(to, step_ms).max(start);
        Self {
            start,
            end,
            step_ms,
        }
    }

    /// Returns the number of points in the grid. The result is 1 or more.
    pub(crate) fn len(&self) -> usize {
        // The two ends can lie further apart than an `i64` holds, but never
        // further apart than a `u64` holds.
        let steps = self.end.abs_diff(self.start) / self.step_ms.unsigned_abs().max(1);
        usize::try_from(steps)
            .unwrap_or(usize::MAX)
            .saturating_add(1)
    }

    /// Returns the time of every point, in order.
    pub(crate) fn timestamps(&self) -> Vec<Timestamp> {
        (0..self.len())
            .map(|i| {
                let offset = i128::from(self.step_ms) * i as i128;
                let at = i128::from(self.start) + offset;
                // The time of a point never passes the end, so the value
                // fits. The clamp guards a grid that a caller built by hand.
                at.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
            })
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
        // The time is not before the start, so the distance is 0 or more. It
        // can be larger than an `i64` holds, so the subtraction uses `u64`.
        let offset = at.abs_diff(self.start);
        let step = self.step_ms.unsigned_abs().max(1);
        if !offset.is_multiple_of(step) {
            return None;
        }
        usize::try_from(offset / step).ok()
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

    /// Runs a function on a thread of its own, and returns its result.
    ///
    /// A function that takes too long then gives a test that fails, and not a
    /// test that never stops.
    fn in_a_short_time<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = send.send(work());
        });
        receive
            .recv_timeout(std::time::Duration::from_secs(20))
            .expect("The function must return a result in a short time, with no panic.")
    }

    /// A grid of the widest range must not need a billion turns of the loop.
    #[test]
    fn for_request_returns_quickly_for_the_widest_range() {
        for max_data_points in [1, 2, 160, 11_000] {
            let req = QueryRequest {
                query: Query {
                    source: "p".into(),
                    expr: "up".into(),
                },
                from: i64::MIN,
                to: i64::MAX,
                now: i64::MAX,
                step_ms: 1,
                max_data_points,
            };
            let grid = in_a_short_time(move || Grid::for_request(&req, MIN_STEP_MS));
            assert!(grid.start <= grid.end);
            assert_eq!(grid.start % grid.step_ms, 0);
            // The largest step is half of the width of the whole number, so
            // no grid of this range holds fewer than 3 points.
            assert!(
                grid.len() <= (max_data_points as usize).max(3),
                "The grid holds {} points for a limit of {max_data_points}.",
                grid.len()
            );
        }
    }

    /// A request of one point and of an ordinary width keeps the smallest
    /// round step that fits. The function must not take the largest step.
    #[test]
    fn for_request_keeps_a_small_step_for_a_request_of_one_point() {
        let req = QueryRequest {
            query: Query {
                source: "p".into(),
                expr: "up".into(),
            },
            from: 1_700_000_010_000,
            to: 1_700_000_050_000,
            now: 1_700_000_050_000,
            step_ms: 1,
            max_data_points: 1,
        };
        let grid = Grid::for_request(&req, MIN_STEP_MS);
        assert_eq!(grid.len(), 1);
        assert_eq!(
            (grid.start, grid.step_ms),
            (1_699_999_800_000, 300_000),
            "The grid must hold the smallest round step that covers both ends."
        );
    }

    #[test]
    fn for_request_does_not_panic_for_the_limits_of_the_number() {
        let ends = [i64::MIN, i64::MIN + 1, -1, 0, 1, i64::MAX - 1, i64::MAX];
        let steps = [1, MIN_STEP_MS, i64::MAX];
        let limits = [0, 1, 2, 11_000, u32::MAX];
        for from in ends {
            for to in ends {
                for step_ms in steps {
                    for max_data_points in limits {
                        let req = QueryRequest {
                            query: Query {
                                source: "p".into(),
                                expr: "up".into(),
                            },
                            from,
                            to,
                            now: i64::MAX,
                            step_ms,
                            max_data_points,
                        };
                        for min_step_ms in [i64::MIN, MIN_STEP_MS, i64::MAX] {
                            let asked = req.clone();
                            let grid =
                                in_a_short_time(move || Grid::for_request(&asked, min_step_ms));
                            assert!(grid.start <= grid.end);
                            assert!(grid.step_ms >= MIN_STEP_MS);
                            // A grid of the widest step holds at most 3 points,
                            // so this call allocates a small vector.
                            if grid.len() <= 3 {
                                assert_eq!(grid.timestamps().len(), grid.len());
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn for_request_keeps_the_limit_of_the_panel_for_a_range_from_the_minimum() {
        let req = QueryRequest {
            query: Query {
                source: "p".into(),
                expr: "up".into(),
            },
            from: i64::MIN,
            to: 0,
            now: 0,
            step_ms: 1,
            max_data_points: 160,
        };
        let grid = Grid::for_request(&req, MIN_STEP_MS);
        assert!(grid.len() <= 160, "The grid holds {} points.", grid.len());
    }

    #[test]
    fn aligned_does_not_panic_at_the_limits_of_the_number() {
        for step in [MIN_STEP_MS, 3_000, i64::MAX - 1, i64::MAX] {
            let grid = Grid::aligned(i64::MIN, i64::MAX, step);
            assert!(grid.start <= grid.end);
            assert!(grid.len() >= 2, "The range of the grid is not empty.");
        }
        let grid = Grid::aligned(i64::MIN, i64::MIN, 3_000);
        assert_eq!(grid.len(), 1);
        assert_eq!(grid.start, grid.end);
        let grid = Grid::aligned(i64::MAX, i64::MAX, i64::MAX);
        assert_eq!((grid.start, grid.end), (i64::MAX, i64::MAX));
    }

    #[test]
    fn align_down_returns_the_largest_multiple_that_fits() {
        assert_eq!(align_down(1_699_999_980_123, 60_000), 1_699_999_980_000);
        assert_eq!(align_down(-1, 1_000), -1_000);
        assert_eq!(align_down(0, 1_000), 0);
        assert_eq!(align_down(i64::MAX, i64::MAX), i64::MAX);
        let low = align_down(i64::MIN, 3_000);
        assert_eq!(low % 3_000, 0);
        assert!(low - i64::MIN < 3_000, "The multiple lies within one step.");
    }

    #[test]
    fn len_does_not_panic_for_a_grid_across_the_whole_number() {
        let grid = Grid {
            start: i64::MIN,
            end: i64::MAX,
            step_ms: 1,
        };
        assert_eq!(
            grid.len(),
            usize::MAX,
            "The count stops at the largest number."
        );
        let grid = Grid {
            start: i64::MIN,
            end: i64::MAX,
            step_ms: i64::MAX,
        };
        assert_eq!(grid.len(), 3);
    }

    #[test]
    fn timestamps_does_not_panic_for_a_grid_at_the_limits_of_the_number() {
        let grid = Grid {
            start: i64::MIN,
            end: i64::MAX - 1,
            step_ms: i64::MAX,
        };
        assert_eq!(grid.timestamps(), vec![i64::MIN, -1, i64::MAX - 1]);
        let grid = Grid {
            start: i64::MAX,
            end: i64::MAX,
            step_ms: i64::MAX,
        };
        assert_eq!(grid.timestamps(), vec![i64::MAX]);
    }

    #[test]
    fn slot_does_not_panic_for_a_grid_across_the_whole_number() {
        let grid = Grid {
            start: i64::MIN,
            end: i64::MAX - 1,
            step_ms: i64::MAX,
        };
        assert_eq!(grid.slot(i64::MIN), Some(0));
        assert_eq!(grid.slot(-1), Some(1));
        assert_eq!(grid.slot(i64::MAX - 1), Some(2));
        assert_eq!(grid.slot(i64::MAX), None);
        assert_eq!(grid.slot(0), None);
    }

    #[test]
    fn round_up_does_not_panic_at_the_limit_of_the_number() {
        assert_eq!(round_up(i64::MAX), i64::MAX);
        assert_eq!(round_up(i64::MIN), MIN_STEP_MS);
        assert!(round_up(i64::MAX - 1) >= i64::MAX - 1);
    }
}
