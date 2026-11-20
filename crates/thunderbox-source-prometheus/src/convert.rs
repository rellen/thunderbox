//! The part that turns the answer of a Prometheus server into frames.

use std::collections::BTreeMap;

use prometheus_http_query::response::{Data, RangeVector};
use thunderbox_core::legend::RESERVED_NAME;
use thunderbox_core::{Error, Field, Frame, Result};

use crate::step::Grid;

/// Returns the frames of one answer.
///
/// The function returns one frame. That frame holds one time column and one
/// number column for each series. Every column holds one value for each point
/// of the grid, so a panel can draw the frame at once.
///
/// The function returns an empty list when the server matched no series. A
/// panel then shows that it has no data.
///
/// # Errors
///
/// This function returns an error if the answer is not a range of values. A
/// range query returns such a range, so this fault means that the server did
/// not answer the question that this data source asked.
pub(crate) fn frames(expr: &str, grid: &Grid, data: Data) -> Result<Vec<Frame>> {
    let series = match data {
        Data::Matrix(series) => series,
        Data::Vector(_) => {
            return Err(Error::Query(
                "the server returned a single value for each series, and not a \
                 range of values"
                    .into(),
            ));
        }
        Data::Scalar(_) => {
            return Err(Error::Query(
                "the server returned one number, and not a range of values".into(),
            ));
        }
    };
    if series.is_empty() {
        return Ok(Vec::new());
    }

    let mut fields = Vec::with_capacity(series.len() + 1);
    fields.push(Field::time("time", grid.timestamps()));
    for one in series {
        fields.push(field(expr, grid, one));
    }
    Ok(vec![Frame::named(expr.to_string(), fields)])
}

/// Returns the number column of one series.
///
/// The column holds one value for each point of the grid. A point that holds no
/// sample from the server keeps a hole in the data. A value that is not finite
/// also becomes a hole, because a chart cannot draw such a value.
///
/// This function moves the name of the metric out of the labels and into the
/// name of the column. `thunderbox_core::legend` depends on that move, because
/// the reserved word `{{__name__}}` reads the name of the column. A query such
/// as `rate(x[5m])` returns a series that holds no metric name. The name of
/// the column is then the text of the query, and every column of that answer
/// carries the same name.
fn field(expr: &str, grid: &Grid, series: RangeVector) -> Field {
    let (metric, samples) = series.into_inner();
    let mut labels: BTreeMap<String, String> = metric.into_iter().collect();
    let name = labels
        .remove(RESERVED_NAME)
        .unwrap_or_else(|| expr.to_string());

    let mut values: Vec<Option<f64>> = vec![None; grid.len()];
    for sample in samples {
        let at = (sample.timestamp() * 1_000.0).round() as i64;
        let value = sample.value();
        if let Some(slot) = grid.slot(at)
            && value.is_finite()
        {
            values[slot] = Some(value);
        }
    }

    let mut field = Field::f64(name, values);
    field.labels = labels;
    field
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The grid that every test in this module uses.
    fn grid() -> Grid {
        Grid {
            start: 1_700_000_000_000,
            end: 1_700_000_030_000,
            step_ms: 10_000,
        }
    }

    /// Reads the `data` part of an answer of the Prometheus HTTP API.
    fn data(json: &str) -> Data {
        serde_json::from_str(json).expect("The test must hold a correct answer.")
    }

    /// Returns the frames of one answer, and then applies the pattern of a
    /// target.
    ///
    /// `thunderbox-dashboard` runs these two steps in this order. This data
    /// source applies no pattern. A test of a pattern therefore runs both
    /// steps, and it keeps this order.
    fn frames_and_legend(expr: &str, pattern: &str, data: Data) -> Vec<Frame> {
        let mut out = frames(expr, &grid(), data).expect("The answer must convert.");
        thunderbox_core::legend::apply(Some(pattern), &mut out);
        out
    }

    const TWO_SERIES: &str = r#"{
        "resultType": "matrix",
        "result": [
            {
                "metric": {"__name__": "up", "instance": "web-1", "job": "node"},
                "values": [
                    [1700000000, "1"],
                    [1700000010, "1"],
                    [1700000020, "0"],
                    [1700000030, "1"]
                ]
            },
            {
                "metric": {"__name__": "up", "instance": "web-2", "job": "node"},
                "values": [
                    [1700000000, "1"],
                    [1700000030, "1"]
                ]
            }
        ]
    }"#;

    #[test]
    fn frames_returns_one_column_for_each_series() {
        let out = frames("up", &grid(), data(TWO_SERIES)).unwrap();
        assert_eq!(out.len(), 1);
        let (times, numeric) = out[0].time_series().unwrap();
        assert_eq!(times.len(), 4);
        assert_eq!(numeric.len(), 2);
    }

    #[test]
    fn frames_puts_each_sample_at_its_own_time() {
        let out = frames("up", &grid(), data(TWO_SERIES)).unwrap();
        let (_, numeric) = out[0].time_series().unwrap();
        assert_eq!(
            numeric[0].values.as_f64().unwrap(),
            &[Some(1.0), Some(1.0), Some(0.0), Some(1.0)]
        );
    }

    #[test]
    fn frames_leaves_a_hole_where_a_series_has_no_sample() {
        let out = frames("up", &grid(), data(TWO_SERIES)).unwrap();
        let (_, numeric) = out[0].time_series().unwrap();
        assert_eq!(
            numeric[1].values.as_f64().unwrap(),
            &[Some(1.0), None, None, Some(1.0)],
            "A series that stops must show a hole, and not a straight line."
        );
    }

    /// The legend rule depends on this test. The name of the metric becomes
    /// the name of the column, and not a label.
    #[test]
    fn frames_reads_the_metric_name_and_the_labels() {
        let out = frames("up", &grid(), data(TWO_SERIES)).unwrap();
        let (_, numeric) = out[0].time_series().unwrap();
        assert_eq!(numeric[0].name, "up");
        assert_eq!(numeric[0].labels.get("instance").unwrap(), "web-1");
        assert!(
            !numeric[0].labels.contains_key("__name__"),
            "The metric name becomes the name of the column, and not a label."
        );
        assert_eq!(
            numeric[0].display_name(),
            r#"up{instance="web-1", job="node"}"#
        );
    }

    #[test]
    fn frames_builds_the_name_of_a_series_from_the_pattern() {
        let out = frames_and_legend("up", "{{instance}}", data(TWO_SERIES));
        let (_, numeric) = out[0].time_series().unwrap();
        assert_eq!(numeric[0].display_name(), "web-1");
        assert_eq!(numeric[1].display_name(), "web-2");
    }

    #[test]
    fn frames_reads_the_metric_name_in_a_pattern() {
        // This test guards the order of the two steps. This data source moves
        // the metric name into the name of the column, and the legend rule
        // reads the name of the column. The result must still be `up node`.
        let out = frames_and_legend("up", "{{__name__}} {{job}}", data(TWO_SERIES));
        let (_, numeric) = out[0].time_series().unwrap();
        assert_eq!(numeric[0].display_name(), "up node");
    }

    #[test]
    fn frames_keeps_the_default_name_when_a_pattern_gives_nothing() {
        // A query that counts or that takes a rate returns a series that holds
        // no metric name. The name of the column is then the text of the
        // query, and `{{__name__}}` reads that text.
        let json = r#"{
            "resultType": "matrix",
            "result": [
                {"metric": {}, "values": [[1700000000, "5"]]}
            ]
        }"#;
        let out = frames_and_legend("sum(up)", "{{__name__}}", data(json));
        let (_, numeric) = out[0].time_series().unwrap();
        assert_eq!(numeric[0].display_name(), "sum(up)");
    }

    #[test]
    fn a_pattern_of_the_reserved_word_gives_one_name_to_a_query_that_drops_the_metric_name() {
        // This test records a change of behaviour. The old rule read the label
        // `__name__`, which this series does not hold, so each series kept the
        // form `sum by (instance) (up){instance="web-1"}`. The new rule reads
        // the name of the column, which is the same text for every series. A
        // person who needs one name for each series writes
        // `{{__name__}} {{instance}}` or `{{instance}}`.
        let json = r#"{
            "resultType": "matrix",
            "result": [
                {"metric": {"instance": "web-1"}, "values": [[1700000000, "5"]]},
                {"metric": {"instance": "web-2"}, "values": [[1700000000, "6"]]}
            ]
        }"#;
        let expr = "sum by (instance) (up)";
        let out = frames_and_legend(expr, "{{__name__}}", data(json));
        let (_, numeric) = out[0].time_series().unwrap();
        assert_eq!(numeric[0].display_name(), expr);
        assert_eq!(numeric[1].display_name(), expr);

        let out = frames_and_legend(expr, "{{__name__}} {{instance}}", data(json));
        let (_, numeric) = out[0].time_series().unwrap();
        assert_eq!(numeric[0].display_name(), "sum by (instance) (up) web-1");
        assert_eq!(numeric[1].display_name(), "sum by (instance) (up) web-2");
    }

    #[test]
    fn frames_names_a_series_after_the_query_when_it_has_no_metric_name() {
        let json = r#"{
            "resultType": "matrix",
            "result": [
                {"metric": {}, "values": [[1700000000, "5"]]}
            ]
        }"#;
        let out = frames("sum(up)", &grid(), data(json)).unwrap();
        let (_, numeric) = out[0].time_series().unwrap();
        assert_eq!(numeric[0].name, "sum(up)");
    }

    #[test]
    fn frames_returns_a_hole_for_a_value_that_is_not_finite() {
        let json = r#"{
            "resultType": "matrix",
            "result": [
                {"metric": {}, "values": [[1700000000, "NaN"], [1700000010, "2"]]}
            ]
        }"#;
        let out = frames("x", &grid(), data(json)).unwrap();
        let (_, numeric) = out[0].time_series().unwrap();
        assert_eq!(numeric[0].values.as_f64().unwrap()[0], None);
        assert_eq!(numeric[0].values.as_f64().unwrap()[1], Some(2.0));
    }

    #[test]
    fn frames_ignores_a_sample_that_the_grid_does_not_hold() {
        let json = r#"{
            "resultType": "matrix",
            "result": [
                {"metric": {}, "values": [[1699999999, "7"], [1700000010, "2"]]}
            ]
        }"#;
        let out = frames("x", &grid(), data(json)).unwrap();
        let (_, numeric) = out[0].time_series().unwrap();
        assert_eq!(
            numeric[0].values.as_f64().unwrap(),
            &[None, Some(2.0), None, None]
        );
    }

    #[test]
    fn frames_returns_no_frame_when_the_query_matched_no_series() {
        let json = r#"{"resultType": "matrix", "result": []}"#;
        let out = frames("up", &grid(), data(json)).unwrap();
        assert!(
            out.is_empty(),
            "A panel must then show that it has no data."
        );
    }

    #[test]
    fn frames_returns_an_error_for_an_answer_that_holds_no_range() {
        let json = r#"{
            "resultType": "vector",
            "result": [{"metric": {}, "value": [1700000000, "1"]}]
        }"#;
        let error = frames("up", &grid(), data(json)).unwrap_err().to_string();
        assert!(
            error.contains("range of values"),
            "The message is wrong. {error}"
        );
    }
}
