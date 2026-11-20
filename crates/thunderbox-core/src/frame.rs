//! The data frame. A data source returns data in this form.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Error, Result, Timestamp};

/// A set of columns that a data source returns.
///
/// Each data source keeps its own query language. The sources agree only on the
/// form of the result. A panel states the shape of the data that it needs. See
/// [`Frame::time_series`]. A panel never states the data source.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// The name of the result set. A legend can show this name.
    pub name: Option<String>,
    /// The columns, in the order that the data source supplied them.
    pub fields: Vec<Field>,
}

impl Frame {
    /// Returns a frame that has no name.
    pub fn new(fields: Vec<Field>) -> Self {
        Self { name: None, fields }
    }

    /// Returns a frame that has a name.
    pub fn named(name: impl Into<String>, fields: Vec<Field>) -> Self {
        Self {
            name: Some(name.into()),
            fields,
        }
    }

    /// Returns the number of rows. The longest column supplies this number.
    pub fn len(&self) -> usize {
        self.fields.iter().map(Field::len).max().unwrap_or(0)
    }

    /// Returns `true` if the frame holds no rows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the first column that holds time values.
    pub fn time_field(&self) -> Option<&Field> {
        self.fields
            .iter()
            .find(|f| matches!(f.values, Values::Time(_)))
    }

    /// Returns every column that holds numbers, in the original order.
    pub fn numeric_fields(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter().filter(|f| f.is_numeric())
    }

    /// Returns the columns that a time series panel needs.
    ///
    /// The frame must hold one column of time values and one or more columns of
    /// numbers. Every number column must hold the same number of rows as the
    /// time column. The method ignores the length of a column of text, and the
    /// length of a column of true and false values. Such a column can hold
    /// a different number of rows. The method returns the time values together
    /// with the number columns. A caller therefore does not read the frame a
    /// second time.
    ///
    /// # Errors
    ///
    /// This method returns an error in each of these conditions:
    ///
    /// - the frame holds no time column;
    /// - the frame holds no number column;
    /// - a number column and the time column hold a different number of rows.
    pub fn time_series(&self) -> Result<(&[Timestamp], Vec<&Field>)> {
        let time = self
            .time_field()
            .ok_or_else(|| Error::FrameShape("no time field".into()))?;
        let Values::Time(stamps) = &time.values else {
            unreachable!("The time_field method returns Values::Time only.")
        };

        let numeric: Vec<&Field> = self.numeric_fields().collect();
        if numeric.is_empty() {
            return Err(Error::FrameShape("no numeric fields".into()));
        }
        if let Some(bad) = numeric.iter().find(|f| f.len() != stamps.len()) {
            return Err(Error::FrameShape(format!(
                "field {:?} has {} values but the time field has {}",
                bad.name,
                bad.len(),
                stamps.len()
            )));
        }
        Ok((stamps, numeric))
    }
}

/// One column of a [`Frame`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Field {
    /// The name of the column.
    pub name: String,
    /// The dimensions that the data source supplied. A Prometheus label is an
    /// example. A legend uses these values.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,
    /// The hints that control how a panel shows this column.
    #[serde(default)]
    pub config: FieldConfig,
    /// The values in the column.
    pub values: Values,
}

impl Field {
    /// Returns a column of time values.
    pub fn time(name: impl Into<String>, values: Vec<Timestamp>) -> Self {
        Self {
            name: name.into(),
            values: Values::Time(values),
            ..Default::default()
        }
    }

    /// Returns a column of numbers. A `None` value is a hole in the data.
    pub fn f64(name: impl Into<String>, values: Vec<Option<f64>>) -> Self {
        Self {
            name: name.into(),
            values: Values::F64(values),
            ..Default::default()
        }
    }

    /// Attaches presentation hints to the column and returns the column.
    pub fn with_config(mut self, config: FieldConfig) -> Self {
        self.config = config;
        self
    }

    /// Adds one label to the column and returns the column.
    pub fn with_label(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.labels.insert(k.into(), v.into());
        self
    }

    /// Returns the number of values in the column.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns `true` if the column holds no values.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the column holds numbers.
    ///
    /// A series is a column of numbers. [`Frame::numeric_fields`] and
    /// [`crate::legend::apply`] both call this method, so one method decides
    /// what a series is.
    pub fn is_numeric(&self) -> bool {
        matches!(self.values, Values::F64(_))
    }

    /// Returns the name that a legend shows.
    ///
    /// The method uses the first of these that is available:
    ///
    /// - the display name in the column configuration;
    /// - the column name and the labels, in the Prometheus form;
    /// - the column name alone.
    ///
    /// [`crate::legend::apply`] writes the display name from the pattern of a
    /// target. That step runs before a panel calls this method. This method
    /// reads the display name, and [`crate::legend::apply`] writes it.
    pub fn display_name(&self) -> String {
        if let Some(n) = &self.config.display_name {
            return n.clone();
        }
        if self.labels.is_empty() {
            return self.name.clone();
        }
        let inner = self
            .labels
            .iter()
            .map(|(k, v)| format!("{k}={v:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}{{{}}}", self.name, inner)
    }

    /// Returns the smallest and the largest number in the column.
    ///
    /// The method ignores a hole in the data. It also ignores a value that is
    /// not finite. The method returns `None` if the column holds no numbers, or
    /// if the column holds a different type.
    pub fn numeric_bounds(&self) -> Option<(f64, f64)> {
        let Values::F64(vs) = &self.values else {
            return None;
        };
        let mut bounds: Option<(f64, f64)> = None;
        for v in vs.iter().flatten() {
            if !v.is_finite() {
                continue;
            }
            bounds = Some(match bounds {
                None => (*v, *v),
                Some((lo, hi)) => (lo.min(*v), hi.max(*v)),
            });
        }
        bounds
    }
}

/// The hints that a data source can attach to a column.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FieldConfig {
    /// The name that a legend shows in place of the column name.
    pub display_name: Option<String>,
    /// The name of the unit. The values `percent`, `bytes`, and `bytes/s` are
    /// examples.
    pub unit: Option<String>,
    /// The number of decimal places that a panel shows.
    pub decimals: Option<u8>,
    /// The smallest value that an axis shows.
    pub min: Option<f64>,
    /// The largest value that an axis shows.
    pub max: Option<f64>,
    /// The thresholds that change the colour of a value.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub thresholds: Vec<Threshold>,
}

impl FieldConfig {
    /// Returns the threshold that applies to `value`.
    ///
    /// The method selects the threshold that has the largest bound that
    /// `value` reaches.
    pub fn threshold_at(&self, value: f64) -> Option<&Threshold> {
        Threshold::at(&self.thresholds, value)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// A bound and the colour that a panel uses above that bound.
pub struct Threshold {
    /// The smallest value that this threshold applies to.
    pub value: f64,
    /// The name of the colour. The theme reads this name. The values `warning`
    /// and `critical` are examples.
    pub color: String,
}

impl Threshold {
    /// Returns the threshold in `list` that applies to `value`.
    ///
    /// The function selects the threshold that has the largest bound that
    /// `value` reaches. It returns `None` when `value` reaches no bound.
    ///
    /// # Examples
    ///
    /// ```
    /// use thunderbox_core::Threshold;
    /// let list = [Threshold { value: 80.0, color: "critical".into() }];
    /// assert_eq!(Threshold::at(&list, 90.0).unwrap().color, "critical");
    /// assert!(Threshold::at(&list, 10.0).is_none());
    /// ```
    pub fn at(list: &[Self], value: f64) -> Option<&Self> {
        list.iter()
            .filter(|t| value >= t.value)
            .max_by(|a, b| a.value.total_cmp(&b.value))
    }
}

/// A column of values. Each variant holds one type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "values", rename_all = "lowercase")]
pub enum Values {
    /// A column of time values.
    Time(Vec<Timestamp>),
    /// A column of numbers. A `None` value is a hole in the data.
    F64(Vec<Option<f64>>),
    /// A column of text. A `None` value is a hole in the data.
    Str(Vec<Option<String>>),
    /// A column of true and false values. A `None` value is a hole in the data.
    Bool(Vec<Option<bool>>),
}

impl Default for Values {
    fn default() -> Self {
        Self::F64(Vec::new())
    }
}

impl Values {
    /// Returns the number of values in the column.
    pub fn len(&self) -> usize {
        match self {
            Self::Time(v) => v.len(),
            Self::F64(v) => v.len(),
            Self::Str(v) => v.len(),
            Self::Bool(v) => v.len(),
        }
    }

    /// Returns `true` if the column holds no values.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the time values, or `None` if the column holds another type.
    pub fn as_time(&self) -> Option<&[Timestamp]> {
        match self {
            Self::Time(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the numbers, or `None` if the column holds another type.
    pub fn as_f64(&self) -> Option<&[Option<f64>]> {
        match self {
            Self::F64(v) => Some(v),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series() -> Frame {
        Frame::named(
            "cpu",
            vec![
                Field::time("time", vec![0, 1000, 2000]),
                Field::f64("usage", vec![Some(1.0), None, Some(3.0)]),
            ],
        )
    }

    #[test]
    fn time_series_returns_the_columns_of_a_correct_frame() {
        let f = series();
        let (stamps, numeric) = f.time_series().unwrap();
        assert_eq!(stamps, &[0, 1000, 2000]);
        assert_eq!(numeric.len(), 1);
        assert_eq!(f.len(), 3);
    }

    #[test]
    fn time_series_returns_an_error_without_a_time_column() {
        let f = Frame::new(vec![Field::f64("usage", vec![Some(1.0)])]);
        assert!(f.time_series().is_err());
    }

    #[test]
    fn time_series_returns_an_error_without_a_number_column() {
        let f = Frame::new(vec![Field::time("time", vec![0])]);
        assert!(f.time_series().is_err());
    }

    #[test]
    fn time_series_returns_an_error_for_columns_of_different_length() {
        let f = Frame::new(vec![
            Field::time("time", vec![0, 1000, 2000]),
            Field::f64("usage", vec![Some(1.0)]),
        ]);
        let err = f.time_series().unwrap_err().to_string();
        assert!(err.contains("1 values"), "The message is wrong. {err}");
    }

    #[test]
    fn numeric_bounds_ignores_a_hole_in_the_data() {
        let f = series();
        let usage = &f.fields[1];
        assert_eq!(usage.numeric_bounds(), Some((1.0, 3.0)));
    }

    #[test]
    fn numeric_bounds_returns_none_for_a_column_of_holes() {
        let f = Field::f64("empty", vec![None, None]);
        assert_eq!(f.numeric_bounds(), None);
    }

    #[test]
    fn numeric_bounds_ignores_a_value_that_is_not_finite() {
        let f = Field::f64("x", vec![Some(1.0), Some(f64::NAN), Some(f64::INFINITY)]);
        assert_eq!(f.numeric_bounds(), Some((1.0, 1.0)));
    }

    #[test]
    fn display_name_adds_the_labels_to_the_column_name() {
        let f = Field::f64("up", vec![]).with_label("job", "api");
        assert_eq!(f.display_name(), r#"up{job="api"}"#);
    }

    #[test]
    fn display_name_returns_the_name_from_the_configuration_first() {
        let f = Field::f64("up", vec![])
            .with_label("job", "api")
            .with_config(FieldConfig {
                display_name: Some("API".into()),
                ..Default::default()
            });
        assert_eq!(f.display_name(), "API");
    }

    #[test]
    fn threshold_at_returns_the_threshold_with_the_largest_bound() {
        let cfg = FieldConfig {
            thresholds: vec![
                Threshold {
                    value: 80.0,
                    color: "critical".into(),
                },
                Threshold {
                    value: 50.0,
                    color: "warning".into(),
                },
            ],
            ..Default::default()
        };
        assert!(cfg.threshold_at(10.0).is_none());
        assert_eq!(cfg.threshold_at(60.0).unwrap().color, "warning");
        assert_eq!(cfg.threshold_at(90.0).unwrap().color, "critical");
    }
}
