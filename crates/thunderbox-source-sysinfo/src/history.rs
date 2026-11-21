//! The store that holds the samples of this machine over time.

use std::collections::{BTreeMap, VecDeque};

use thunderbox_core::Timestamp;

/// The largest number of samples that one series holds.
///
/// A dashboard that refreshes once a second fills this store in about one hour.
/// The store then drops its oldest sample for each new one.
pub const CAPACITY: usize = 4096;

/// One series of samples, in the order that the machine produced them.
type Series = VecDeque<(Timestamp, f64)>;

/// The samples of this machine, by the name of the series.
#[derive(Debug, Default)]
pub struct History {
    series: BTreeMap<String, Series>,
}

impl History {
    /// Returns a store that holds no sample.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one sample to one series.
    ///
    /// The method drops the oldest sample when the series is full.
    pub fn push(&mut self, name: &str, at: Timestamp, value: f64) {
        let series = self.series.entry(name.to_string()).or_default();
        if series.len() >= CAPACITY {
            series.pop_front();
        }
        series.push_back((at, value));
    }

    /// Returns the name of every series that starts with a prefix.
    ///
    /// The name `cpu.core.` matches every core, and it matches no other series.
    pub fn names_with_prefix(&self, prefix: &str) -> Vec<String> {
        self.series
            .keys()
            .filter(|name| name.starts_with(prefix))
            .cloned()
            .collect()
    }

    /// Returns the samples of one series that sit inside a range of time.
    ///
    /// The method returns at most `budget` samples, at an even distance apart.
    /// It always takes the most recent sample, because a person reads the right
    /// edge of a chart first.
    pub fn range(
        &self,
        name: &str,
        from: Timestamp,
        to: Timestamp,
        budget: u32,
    ) -> Vec<(Timestamp, f64)> {
        let Some(series) = self.series.get(name) else {
            return Vec::new();
        };
        let inside: Vec<(Timestamp, f64)> = series
            .iter()
            .filter(|(at, _)| *at >= from && *at <= to)
            .copied()
            .collect();

        let budget = budget.max(1) as usize;
        if inside.len() <= budget {
            return inside;
        }
        // Walk backwards from the most recent sample, so that the right edge of
        // the chart always holds the newest value.
        let step = inside.len() as f64 / budget as f64;
        let mut out: Vec<(Timestamp, f64)> = (0..budget)
            .map(|i| {
                let index = inside.len() - 1 - (i as f64 * step) as usize;
                inside[index]
            })
            .collect();
        out.reverse();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled(count: usize) -> History {
        let mut history = History::new();
        for i in 0..count {
            history.push("cpu", i as Timestamp * 1000, i as f64);
        }
        history
    }

    #[test]
    fn a_full_series_drops_its_oldest_sample() {
        let history = filled(CAPACITY + 10);
        let all = history.range("cpu", 0, Timestamp::MAX, u32::MAX);
        assert_eq!(all.len(), CAPACITY);
        assert_eq!(all[0].1, 10.0, "The store must drop the oldest sample.");
    }

    #[test]
    fn range_returns_only_the_samples_inside_the_range() {
        let history = filled(100);
        let out = history.range("cpu", 10_000, 20_000, u32::MAX);
        assert_eq!(out.len(), 11);
        assert_eq!(out.first().unwrap().0, 10_000);
        assert_eq!(out.last().unwrap().0, 20_000);
    }

    #[test]
    fn range_holds_the_number_of_points_that_the_caller_asks_for() {
        let history = filled(1000);
        let out = history.range("cpu", 0, Timestamp::MAX, 50);
        assert_eq!(out.len(), 50);
    }

    #[test]
    fn range_keeps_the_most_recent_sample() {
        let history = filled(1000);
        let out = history.range("cpu", 0, Timestamp::MAX, 50);
        assert_eq!(
            out.last().unwrap().1,
            999.0,
            "A person reads the right edge of a chart first."
        );
    }

    #[test]
    fn range_returns_the_samples_in_order() {
        let history = filled(1000);
        let out = history.range("cpu", 0, Timestamp::MAX, 50);
        assert!(
            out.windows(2).all(|w| w[0].0 < w[1].0),
            "The times must rise."
        );
    }

    #[test]
    fn range_of_a_series_that_is_not_there_is_empty() {
        assert!(History::new().range("nothing", 0, 1, 10).is_empty());
    }

    #[test]
    fn a_budget_of_zero_still_returns_a_sample() {
        let history = filled(100);
        assert_eq!(history.range("cpu", 0, Timestamp::MAX, 0).len(), 1);
    }

    #[test]
    fn names_with_prefix_finds_each_core_and_nothing_else() {
        let mut history = History::new();
        history.push("cpu", 0, 1.0);
        history.push("cpu.core.0", 0, 1.0);
        history.push("cpu.core.1", 0, 1.0);
        assert_eq!(history.names_with_prefix("cpu.core.").len(), 2);
        assert!(history.names_with_prefix("memory").is_empty());
    }

    #[test]
    fn range_does_not_panic_for_ends_at_the_limits_of_the_number() {
        let history = filled(100);
        for from in [i64::MIN, 0, i64::MAX] {
            for to in [i64::MIN, 0, i64::MAX] {
                for budget in [0, 1, 7, u32::MAX] {
                    let out = history.range("cpu", from, to, budget);
                    assert!(out.len() <= 100);
                }
            }
        }
        assert_eq!(
            history.range("cpu", i64::MIN, i64::MAX, u32::MAX).len(),
            100
        );
        assert!(
            history
                .range("cpu", i64::MAX, i64::MIN, u32::MAX)
                .is_empty()
        );
    }
}
