//! The step that turns a column of numbers into one number.

use serde::{Deserialize, Serialize};

/// The step that turns a column of numbers into one number.
///
/// A panel that shows a single number needs this step. The stat panel, the
/// gauge panel, and the bar gauge panel each show one number for each series.
/// The dashboard names the step in the `reduce` field of a panel.
///
/// `docs/ARCHITECTURE.md` section 3 gives a whole crate to the transform
/// pipeline. This type is the one step that a panel cannot do without, so it
/// sits here until that crate arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Reduce {
    /// The most recent value. This step is the default one.
    #[default]
    Last,
    /// The oldest value.
    First,
    /// The smallest value.
    Min,
    /// The largest value.
    Max,
    /// The mean of every value.
    Mean,
    /// The sum of every value.
    Sum,
    /// The number of values that the column holds.
    Count,
}

impl Reduce {
    /// Returns the one number for a column.
    ///
    /// The method ignores a hole in the data, and it ignores a value that is
    /// not finite. It returns `None` when the column holds no value that the
    /// step can use. The `Count` step is the one exception: it returns 0 for a
    /// column that holds no value.
    ///
    /// # Examples
    ///
    /// ```
    /// use thunderbox_core::Reduce;
    /// let column = [Some(1.0), None, Some(3.0)];
    /// assert_eq!(Reduce::Last.apply(&column), Some(3.0));
    /// assert_eq!(Reduce::Mean.apply(&column), Some(2.0));
    /// assert_eq!(Reduce::Count.apply(&column), Some(2.0));
    /// ```
    pub fn apply(self, values: &[Option<f64>]) -> Option<f64> {
        let usable = || values.iter().flatten().copied().filter(|v| v.is_finite());
        match self {
            Self::Last => usable().next_back(),
            Self::First => usable().next(),
            Self::Min => usable().reduce(f64::min),
            Self::Max => usable().reduce(f64::max),
            Self::Sum => {
                let mut count = 0_usize;
                let total = usable().inspect(|_| count += 1).sum::<f64>();
                (count > 0).then_some(total)
            }
            Self::Mean => {
                let mut count = 0_usize;
                let total = usable().inspect(|_| count += 1).sum::<f64>();
                (count > 0).then(|| total / count as f64)
            }
            Self::Count => Some(usable().count() as f64),
        }
    }

    /// Returns `true` if this step is the default step.
    ///
    /// The serializer reads this method. A dashboard that does not name a step
    /// therefore stays as its writer wrote it.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Returns the name that the dashboard file uses for this step.
    pub fn name(self) -> &'static str {
        match self {
            Self::Last => "last",
            Self::First => "first",
            Self::Min => "min",
            Self::Max => "max",
            Self::Mean => "mean",
            Self::Sum => "sum",
            Self::Count => "count",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLUMN: [Option<f64>; 5] = [Some(4.0), None, Some(1.0), Some(7.0), Some(2.0)];

    #[test]
    fn each_step_returns_the_number_that_its_name_promises() {
        assert_eq!(Reduce::Last.apply(&COLUMN), Some(2.0));
        assert_eq!(Reduce::First.apply(&COLUMN), Some(4.0));
        assert_eq!(Reduce::Min.apply(&COLUMN), Some(1.0));
        assert_eq!(Reduce::Max.apply(&COLUMN), Some(7.0));
        assert_eq!(Reduce::Sum.apply(&COLUMN), Some(14.0));
        assert_eq!(Reduce::Mean.apply(&COLUMN), Some(3.5));
        assert_eq!(Reduce::Count.apply(&COLUMN), Some(4.0));
    }

    #[test]
    fn each_step_ignores_a_value_that_is_not_finite() {
        let column = [Some(1.0), Some(f64::NAN), Some(f64::INFINITY), Some(3.0)];
        assert_eq!(Reduce::Last.apply(&column), Some(3.0));
        assert_eq!(Reduce::Max.apply(&column), Some(3.0));
        assert_eq!(Reduce::Count.apply(&column), Some(2.0));
    }

    #[test]
    fn a_column_of_holes_returns_no_number() {
        let column = [None, None];
        for step in [
            Reduce::Last,
            Reduce::First,
            Reduce::Min,
            Reduce::Max,
            Reduce::Sum,
            Reduce::Mean,
        ] {
            assert_eq!(
                step.apply(&column),
                None,
                "The step {} is wrong.",
                step.name()
            );
        }
    }

    #[test]
    fn the_count_step_returns_zero_for_a_column_of_holes() {
        assert_eq!(Reduce::Count.apply(&[None, None]), Some(0.0));
        assert_eq!(Reduce::Count.apply(&[]), Some(0.0));
    }

    #[test]
    fn the_default_step_is_the_most_recent_value() {
        assert_eq!(Reduce::default(), Reduce::Last);
        assert_eq!(Reduce::default().name(), "last");
        assert!(Reduce::Last.is_default());
        assert!(!Reduce::Mean.is_default());
    }

    #[test]
    fn serde_reads_the_name_that_a_dashboard_writes() {
        #[derive(serde::Deserialize)]
        struct Holder {
            reduce: Reduce,
        }
        let holder: Holder = toml::from_str("reduce = \"mean\"").unwrap();
        assert_eq!(holder.reduce, Reduce::Mean);
    }
}
