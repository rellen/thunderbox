//! A data source that makes test data. A developer can build a panel with this
//! data source, and needs no other backend.
//!
//! This data source has two properties.
//!
//! - The data does not change between two runs. A test that compares a picture
//!   of a panel therefore gives the same result each time.
//! - Each value depends only on its own time. A value never depends on its
//!   position in the result. One moment in time therefore gives one value, for
//!   every number of points that a caller asks for. A person who makes a panel
//!   wider sees the same shape at a higher resolution. The shape does not
//!   change.
//!
//! The second property matters for a test. A caller reduces the number of
//! points to the width of the panel. A test of that step needs a signal that
//! does not change when the number of points changes.
//!
//! The query `processes` returns a list of invented processes, and not a
//! signal over time. The list has the shape of the list of the sysinfo data
//! source. A top panel can therefore draw it, and the picture then shows
//! nothing of the machine that draws it. The values of the list depend only on
//! the end of the window, so the list keeps both properties above.
//!
//! `docs/contributor/architecture.md` states that this data source comes before
//! any data source that reads real values.

use async_trait::async_trait;
use thunderbox_core::{
    Error, Field, FieldConfig, Frame, QueryRequest, Result, SourceDecl, Timestamp, Values,
};
use thunderbox_source::{DataSource, SourceDescriptor};

/// The type name that a dashboard writes in the `kind` field.
pub const KIND: &str = "testdata";

/// The signals that this data source can make.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// A signal that moves slowly between 0 and 100.
    RandomWalk,
    /// A smooth wave. This signal shows the resolution of a marker clearly.
    Sine,
    /// A straight rise from 0 to 100 across the time range.
    Ramp,
    /// A signal that moves slowly, and that has holes in the data. This signal
    /// tests how a panel shows a hole.
    Gappy,
}

impl Shape {
    /// Reads the name of a signal.
    ///
    /// # Errors
    ///
    /// This function returns an error if the name is not in the list above.
    fn parse(expr: &str) -> Result<Self> {
        match expr.trim() {
            "random_walk" => Ok(Self::RandomWalk),
            "sine" => Ok(Self::Sine),
            "ramp" => Ok(Self::Ramp),
            "gappy" => Ok(Self::Gappy),
            other => Err(Error::Query(format!(
                "The data source testdata knows no query {other:?}. The known queries are \
                 random_walk, sine, ramp, gappy, and {PROCESSES}."
            ))),
        }
    }
}

/// The largest number of points that one query returns.
///
/// The request carries a number of points that a caller chooses, and that
/// number can be as large as a `u32` holds. This limit is far above the width
/// of any terminal.
const MAX_POINTS: usize = 100_000;

/// The text of the query that returns a list of invented processes.
const PROCESSES: &str = "processes";

/// The invented processes of the query `processes`.
///
/// Each row holds three values:
///
/// - the name of the process;
/// - the largest share of the processor that the process reaches;
/// - the memory that the process holds on average, in millions of bytes.
///
/// A share of 100 is one whole core, as the sysinfo data source reports it. The
/// process `compiler` therefore reaches almost two cores.
const PROCESS_TABLE: [(&str, f64, f64); 12] = [
    ("init", 2.0, 12.0),
    ("shell", 4.0, 6.0),
    ("editor", 25.0, 180.0),
    ("compiler", 190.0, 900.0),
    ("browser", 60.0, 2100.0),
    ("database", 45.0, 1400.0),
    ("server", 35.0, 350.0),
    ("indexer", 80.0, 260.0),
    ("backup", 30.0, 120.0),
    ("logger", 6.0, 40.0),
    ("player", 15.0, 210.0),
    ("terminal", 8.0, 90.0),
];

/// Mixes the bits of a number. This function is the splitmix64 algorithm.
///
/// The function is small, and it gives the same result for the same input. It
/// is not a source of secure random numbers. A test signal does not need one.
fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Returns a number that is 0 or more and less than 1.
///
/// The result depends only on the seed, the series, and the cell.
fn unit(seed: u64, series: usize, cell: i64) -> f64 {
    let h = mix(seed ^ ((series as u64) << 32) ^ cell as u64);
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// Returns a smooth signal at one moment in time.
///
/// The function mixes the bits of the two grid points that enclose `t_ms`. It
/// then blends the two results with a smooth curve. The grid follows absolute
/// time. The grid does not follow the position of the value in the result. The
/// curve is therefore the same for every number of points.
fn value_noise(seed: u64, series: usize, t_ms: i64, grid_ms: i64) -> f64 {
    let cell = t_ms.div_euclid(grid_ms);
    let frac = t_ms.rem_euclid(grid_ms) as f64 / grid_ms as f64;
    let (a, b) = (unit(seed, series, cell), unit(seed, series, cell + 1));
    let s = frac * frac * (3.0 - 2.0 * frac);
    a + (b - a) * s
}

/// Returns a slow signal that is 0 or more and 1 or less.
///
/// The function adds three copies of the smooth signal. Each copy uses a
/// smaller grid and a smaller amplitude than the copy before it. The sum moves
/// like a walk, but it stays smooth at every resolution.
fn drift(seed: u64, series: usize, t_ms: i64) -> f64 {
    const OCTAVES: [(i64, f64); 3] = [(20_000, 0.6), (7_000, 0.3), (2_500, 0.1)];
    OCTAVES
        .iter()
        .map(|(grid, amp)| value_noise(seed, series, t_ms, *grid) * amp)
        .sum::<f64>()
        .clamp(0.0, 1.0)
}

/// Returns the value of an option as a whole number that is 0 or more.
///
/// # Errors
///
/// This function returns an error if the value is not such a number.
fn count(key: &str, value: &toml::Value) -> Result<i64> {
    value.as_integer().filter(|n| *n >= 0).ok_or_else(|| {
        Error::Dashboard(format!(
            "the {key} of a testdata source must be a whole number that is 0 or more"
        ))
    })
}

/// A data source that makes test data.
#[derive(Debug, Clone)]
pub struct TestData {
    seed: u64,
    /// The number of series that each query returns.
    series: usize,
}

impl Default for TestData {
    fn default() -> Self {
        Self {
            seed: 0x0BAD_CAFE,
            series: 1,
        }
    }
}

impl TestData {
    /// Returns a data source that uses the default seed and one series.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the seed and returns the data source.
    ///
    /// Two data sources that have a different seed make a different signal.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Sets the number of series and returns the data source.
    ///
    /// The method raises a value of 0 to 1.
    pub fn with_series(mut self, series: usize) -> Self {
        self.series = series.max(1);
        self
    }

    /// Returns a data source from the declaration in a dashboard.
    ///
    /// The function reads these options. It returns an error for any other
    /// option.
    ///
    /// - `seed`, which holds a whole number that is 0 or more;
    /// - `series`, which holds the number of series that a query returns.
    ///
    /// # Errors
    ///
    /// This function returns an error in each of these conditions:
    ///
    /// - the declaration holds an option that is not in the list above;
    /// - a value is not a whole number that is 0 or more.
    pub fn from_declaration(declaration: &SourceDecl) -> Result<Self> {
        let mut source = Self::new();
        for (key, value) in &declaration.options {
            match key.as_str() {
                "seed" => source = source.with_seed(count(key, value)? as u64),
                "series" => source = source.with_series(count(key, value)? as usize),
                other => {
                    return Err(Error::Dashboard(format!(
                        "a testdata source has no option {other:?}; it takes \
                         seed and series"
                    )));
                }
            }
        }
        Ok(source)
    }

    /// Returns the times of the points, spread evenly across the range.
    ///
    /// The result holds at most [`MAX_POINTS`] times. A caller that asks for
    /// more gets that number, so that a request cannot use all the memory.
    fn timestamps(req: &QueryRequest) -> Vec<Timestamp> {
        let n = (req.max_data_points.max(1) as usize).min(MAX_POINTS);
        let span = i128::from(req.span_ms());
        let from = i128::from(req.from);
        (0..n)
            .map(|i| {
                // The product of a wide span and an index does not fit in an
                // `i64`, so the sum uses a wider number. The result lies
                // between `from` and `from + span`. The clamp guards the case
                // of a range in the wrong order, which gives a span of 1.
                let at = from + span * i as i128 / n as i128;
                at.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
            })
            .collect()
    }

    /// Returns the list of invented processes at one moment.
    ///
    /// The frame holds one row for each process, and the four columns of the
    /// list of the sysinfo data source: `pid` as an identifier, `name`, `cpu` as a
    /// share, and `MemB` in bytes with no unit. The rows come in the order of the number
    /// of each process, as the rows of that list do.
    ///
    /// Each value depends only on the seed and on `at`. The share of each
    /// process moves slowly, so the order of a top panel changes over a few
    /// seconds and not on every refresh.
    fn processes(&self, at: Timestamp) -> Frame {
        let count = PROCESS_TABLE.len();
        let mut pids = Vec::with_capacity(count);
        let mut names = Vec::with_capacity(count);
        let mut cpu = Vec::with_capacity(count);
        let mut memory = Vec::with_capacity(count);
        for (index, (name, peak, megabytes)) in PROCESS_TABLE.iter().enumerate() {
            pids.push(Some((1 + 211 * index) as f64));
            names.push(Some((*name).to_string()));
            cpu.push(Some(peak * drift(self.seed, index, at)));
            // The memory reads a series of its own, so its value does not
            // follow the share of the processor.
            let swing = 0.8 + 0.4 * drift(self.seed, count + index, at);
            memory.push(Some(megabytes * 1_000_000.0 * swing));
        }
        let text = |name: &str, values: Vec<Option<String>>| Field {
            name: name.into(),
            values: Values::Str(values),
            ..Default::default()
        };
        let cpu = Field::f64("cpu", cpu).with_config(FieldConfig {
            unit: Some("percent".into()),
            min: Some(0.0),
            width: Some(4),
            ..Default::default()
        });
        // The column holds bytes and states no unit, as the list of the sysinfo
        // data source does.
        let memory = Field::f64("MemB", memory).with_config(FieldConfig {
            min: Some(0.0),
            width: Some(4),
            ..Default::default()
        });
        let identifier = Field::f64("pid", pids).with_config(FieldConfig {
            identifier: true,
            width: Some(6),
            ..Default::default()
        });
        Frame::named(
            PROCESSES,
            vec![identifier, text("name", names), cpu, memory],
        )
    }

    /// Returns the value at one moment in time.
    ///
    /// The result depends only on `t_ms`. It never depends on the position of
    /// the value in the result. The method returns `None` at a hole in the
    /// data.
    fn value_at(&self, shape: Shape, series: usize, t_ms: i64, range: (i64, i64)) -> Option<f64> {
        // The hole lasts ten seconds in each minute. The hole follows absolute
        // time. It therefore stays in place when a person moves or scales the
        // time range.
        if matches!(shape, Shape::Gappy) && t_ms.div_euclid(10_000).rem_euclid(6) == 3 {
            return None;
        }
        let v = match shape {
            Shape::RandomWalk | Shape::Gappy => 100.0 * drift(self.seed, series, t_ms),
            Shape::Sine => {
                // The period is a number of seconds. The wave therefore keeps
                // its frequency for every number of points. Each series starts
                // at a different point in the wave, so that a person can tell
                // the series apart.
                let phase = series as f64 * 0.7;
                let secs = t_ms as f64 / 1000.0;
                50.0 + 45.0 * (secs * std::f64::consts::TAU / 30.0 + phase).sin()
            }
            Shape::Ramp => {
                let (from, to) = range;
                // A range of the widest span would overflow a plain
                // subtraction, so each difference stops at the limit of the number.
                let span = to.saturating_sub(from).max(1) as f64;
                100.0 * (t_ms.saturating_sub(from) as f64 / span).clamp(0.0, 1.0)
            }
        };
        Some(v)
    }
}

#[async_trait]
impl DataSource for TestData {
    fn descriptor(&self) -> SourceDescriptor {
        // This data source returns exactly the number of points that the
        // caller asks for. The caller therefore does not reduce them again.
        SourceDescriptor::new("testdata", "Test data").downsampling()
    }

    async fn query(&self, req: QueryRequest) -> Result<Vec<Frame>> {
        if req.query.expr.trim() == PROCESSES {
            // The list describes one moment, and the end of the window is that
            // moment. A window in the past therefore shows the list of that
            // moment.
            return Ok(vec![self.processes(req.to)]);
        }
        let shape = Shape::parse(&req.query.expr)?;
        let stamps = Self::timestamps(&req);

        let range = (req.from, req.to);
        let mut fields = vec![Field::time("time", stamps.clone())];
        for s in 0..self.series {
            let values: Vec<Option<f64>> = stamps
                .iter()
                .map(|t| self.value_at(shape, s, *t, range))
                .collect();
            fields.push(
                Field::f64(req.query.expr.clone(), values)
                    .with_label("series", s.to_string())
                    .with_config(FieldConfig {
                        unit: Some("percent".into()),
                        min: Some(0.0),
                        max: Some(100.0),
                        ..Default::default()
                    }),
            );
        }
        Ok(vec![Frame::named(req.query.expr.clone(), fields)])
    }

    async fn label_values(&self, _query: &str) -> Result<Vec<String>> {
        Ok((0..self.series).map(|s| s.to_string()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thunderbox_core::{Query, RelativeTime, TimeRange};

    fn req(expr: &str, points: u32) -> QueryRequest {
        QueryRequest::new(
            Query {
                source: "t".into(),
                expr: expr.into(),
            },
            &TimeRange {
                from: RelativeTime::Offset(-60_000),
                to: RelativeTime::Now,
            },
            1_000_000,
            points,
        )
    }

    fn run(src: &TestData, expr: &str, points: u32) -> Result<Vec<Frame>> {
        block_on(src.query(req(expr, points)))
    }

    #[test]
    fn query_returns_the_number_of_points_that_the_caller_asks_for() {
        let src = TestData::new();
        let frames = run(&src, "random_walk", 37).unwrap();
        assert_eq!(frames[0].len(), 37);
        assert!(src.descriptor().downsamples);
    }

    #[test]
    fn query_returns_the_same_data_for_the_same_seed() {
        let a = run(&TestData::new(), "random_walk", 20).unwrap();
        let b = run(&TestData::new(), "random_walk", 20).unwrap();
        assert_eq!(a, b, "A test that compares a picture depends on this.");
    }

    #[test]
    fn query_returns_different_data_for_a_different_seed() {
        let a = run(&TestData::new().with_seed(1), "random_walk", 20).unwrap();
        let b = run(&TestData::new().with_seed(2), "random_walk", 20).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn random_walk_stays_between_zero_and_one_hundred() {
        let frames = run(&TestData::new(), "random_walk", 500).unwrap();
        let (_, numeric) = frames[0].time_series().unwrap();
        for v in numeric[0].values.as_f64().unwrap().iter().flatten() {
            assert!(
                (0.0..=100.0).contains(v),
                "The value is outside the range. {v}"
            );
        }
    }

    /// The property the old index-based generator silently violated.
    #[test]
    fn query_returns_the_same_value_for_one_moment_at_any_resolution() {
        let src = TestData::new();
        for expr in ["random_walk", "sine", "ramp", "gappy"] {
            let coarse = run(&src, expr, 20).unwrap();
            let fine = run(&src, expr, 40).unwrap();
            let (ct, cv) = coarse[0].time_series().unwrap();
            let (ft, fv) = fine[0].time_series().unwrap();
            let (cv, fv) = (
                cv[0].values.as_f64().unwrap(),
                fv[0].values.as_f64().unwrap(),
            );

            let mut compared = 0;
            for (i, t) in ct.iter().enumerate() {
                let Some(j) = ft.iter().position(|u| u == t) else {
                    continue;
                };
                assert_eq!(
                    cv[i], fv[j],
                    "{expr}: the value changed at time {t} when the number of points changed."
                );
                compared += 1;
            }
            assert!(compared > 1, "{expr}: the test compared no value.");
        }
    }

    #[test]
    fn sine_keeps_its_period_in_seconds() {
        let src = TestData::new();
        // The wave has a period of 30 seconds, and the window is 60 seconds
        // wide. The wave therefore crosses the middle value twice on the way
        // up. That count holds for every number of points.
        let peaks = |pts: u32| {
            let f = run(&src, "sine", pts).unwrap();
            let (_, num) = f[0].time_series().unwrap();
            let v: Vec<f64> = num[0]
                .values
                .as_f64()
                .unwrap()
                .iter()
                .flatten()
                .copied()
                .collect();
            v.windows(2).filter(|w| w[0] < 50.0 && w[1] >= 50.0).count()
        };
        assert_eq!(
            peaks(60),
            peaks(240),
            "The period must not depend on the number of points."
        );
    }

    #[test]
    fn gappy_returns_holes_and_values() {
        let frames = run(&TestData::new(), "gappy", 100).unwrap();
        let (_, numeric) = frames[0].time_series().unwrap();
        let values = numeric[0].values.as_f64().unwrap();
        assert!(
            values.iter().any(Option::is_none),
            "The result must hold at least one hole."
        );
        assert!(
            values.iter().any(Option::is_some),
            "The result must hold at least one value."
        );
    }

    #[test]
    fn query_gives_each_series_a_different_name() {
        let frames = run(&TestData::new().with_series(3), "sine", 20).unwrap();
        let (_, numeric) = frames[0].time_series().unwrap();
        assert_eq!(numeric.len(), 3);
        let names: Vec<String> = numeric.iter().map(|f| f.display_name()).collect();
        assert_eq!(names.len(), 3);
        assert_ne!(
            names[0], names[1],
            "A legend needs a different name for each series."
        );
    }

    #[test]
    fn query_returns_times_inside_the_range_and_in_order() {
        let r = req("ramp", 50);
        let frames = block_on(TestData::new().query(r.clone())).unwrap();
        let (stamps, _) = frames[0].time_series().unwrap();
        assert!(stamps.iter().all(|t| *t >= r.from && *t <= r.to));
        assert!(
            stamps.windows(2).all(|w| w[0] < w[1]),
            "The times must rise."
        );
    }

    /// Returns a request whose fields hold the numbers that a test names.
    fn request_at(expr: &str, from: i64, to: i64, points: u32) -> QueryRequest {
        QueryRequest {
            query: Query {
                source: "t".into(),
                expr: expr.into(),
            },
            from,
            to,
            now: i64::MAX,
            step_ms: i64::MAX,
            max_data_points: points,
        }
    }

    #[test]
    fn timestamps_does_not_panic_for_the_widest_range() {
        let stamps = TestData::timestamps(&request_at("ramp", i64::MIN, i64::MAX, 1_000));
        assert_eq!(stamps.len(), 1_000);
        assert_eq!(stamps[0], i64::MIN);
        assert!(
            stamps.windows(2).all(|w| w[0] < w[1]),
            "The times must rise."
        );
        assert!(stamps[999] < i64::MAX);
    }

    #[test]
    fn timestamps_does_not_panic_for_a_range_at_one_limit_of_the_number() {
        let at_top = TestData::timestamps(&request_at("ramp", i64::MAX - 10, i64::MAX, 7));
        assert!(at_top.iter().all(|t| *t >= i64::MAX - 10));
        let at_bottom = TestData::timestamps(&request_at("ramp", i64::MIN, i64::MIN + 10, 7));
        assert!(at_bottom.iter().all(|t| *t <= i64::MIN + 10));
    }

    #[test]
    fn timestamps_does_not_panic_for_a_range_in_the_wrong_order() {
        let stamps = TestData::timestamps(&request_at("ramp", i64::MAX, i64::MIN, 5));
        assert_eq!(stamps.len(), 5);
        assert!(stamps.iter().all(|t| *t == i64::MAX));
    }

    #[test]
    fn timestamps_returns_no_more_than_the_largest_number_of_points() {
        let stamps = TestData::timestamps(&request_at("ramp", i64::MIN, i64::MAX, u32::MAX));
        assert_eq!(stamps.len(), MAX_POINTS);
    }

    #[test]
    fn query_does_not_panic_for_a_range_at_the_limits_of_the_number() {
        for expr in ["random_walk", "sine", "ramp", "gappy", PROCESSES] {
            for (from, to) in [
                (i64::MIN, i64::MAX),
                (i64::MAX, i64::MIN),
                (i64::MIN, i64::MIN),
                (i64::MAX, i64::MAX),
            ] {
                let frames = block_on(TestData::new().query(request_at(expr, from, to, 50)))
                    .expect("The query must answer.");
                assert_eq!(frames.len(), 1);
            }
        }
    }

    #[test]
    fn a_ramp_over_the_widest_range_stays_between_0_and_100() {
        let frames =
            block_on(TestData::new().query(request_at("ramp", i64::MIN, i64::MAX, 50))).unwrap();
        let field = frames[0].numeric_fields().next().expect("A column.");
        let (low, high) = field.numeric_bounds().expect("The column holds numbers.");
        assert!(low >= 0.0 && high <= 100.0, "The bounds are {low} {high}.");
    }

    #[test]
    fn query_returns_an_error_for_an_unknown_signal() {
        let err = run(&TestData::new(), "nonsense", 10)
            .unwrap_err()
            .to_string();
        assert!(err.contains("nonsense"), "The message is wrong. {err}");
        assert!(
            err.contains("random_walk"),
            "The message must list the correct names. {err}"
        );
        assert!(
            err.contains("processes"),
            "The message must list the query of the processes. {err}"
        );
    }

    /// Returns the list of invented processes for a window that ends at `at`.
    fn processes_at(src: &TestData, at: Timestamp) -> Frame {
        processes_for(src, at, at)
    }

    /// Returns the list of invented processes for a window that ends at `at`,
    /// asked at the present moment `now`.
    fn processes_for(src: &TestData, at: Timestamp, now: Timestamp) -> Frame {
        let request = QueryRequest::new(
            Query {
                source: "t".into(),
                expr: "processes".into(),
            },
            &TimeRange {
                from: RelativeTime::Absolute(at - 60_000),
                to: RelativeTime::Absolute(at),
            },
            now,
            10,
        );
        let mut frames = block_on(src.query(request)).expect("The query must answer.");
        assert_eq!(frames.len(), 1, "The list of processes is one frame.");
        frames.remove(0)
    }

    /// Returns the names of the processes, from the largest share of the
    /// processor to the smallest.
    fn busiest_first(frame: &Frame) -> Vec<String> {
        let Values::Str(names) = &frame.fields[1].values else {
            panic!("The second column must hold the names.");
        };
        let cpu = frame.fields[2].values.as_f64().expect("cpu holds numbers.");
        let mut rows: Vec<(f64, String)> = cpu
            .iter()
            .zip(names)
            .map(|(c, n)| (c.unwrap_or(0.0), n.clone().unwrap_or_default()))
            .collect();
        rows.sort_by(|a, b| b.0.total_cmp(&a.0));
        rows.into_iter().map(|(_, n)| n).collect()
    }

    /// The list has the shape of the list of the sysinfo data source. A
    /// dashboard can then read either data source, and the scene of the program
    /// reads this one.
    #[test]
    fn processes_returns_the_columns_of_the_sysinfo_list() {
        let frame = processes_at(&TestData::new(), 1_000_000);
        let names: Vec<&str> = frame.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["pid", "name", "cpu", "MemB"]);
        assert_eq!(frame.len(), PROCESS_TABLE.len());
        assert!(frame.fields.iter().all(|f| f.len() == frame.len()));
        assert!(frame.time_field().is_none(), "A row describes a process.");
        assert_eq!(frame.fields[2].config.unit.as_deref(), Some("percent"));
        assert_eq!(
            frame.fields[3].config.unit, None,
            "The column MemB holds no unit, so a panel writes the letter of the scale alone."
        );
        let widths: Vec<Option<u16>> = frame.fields.iter().map(|f| f.config.width).collect();
        assert_eq!(widths, vec![Some(6), None, Some(4), Some(4)]);
        assert!(frame.fields[0].config.identifier);
        let pids = frame.fields[0]
            .values
            .as_f64()
            .expect("The number of a process is a number.");
        let pids: Vec<f64> = pids.iter().map(|p| p.unwrap()).collect();
        assert!(
            pids.windows(2).all(|pair| pair[0] < pair[1]),
            "The rows come in the order of the number of each process. {pids:?}"
        );
    }

    #[test]
    fn processes_returns_the_same_list_for_the_same_moment() {
        let a = processes_at(&TestData::new(), 1_000_000);
        let b = processes_at(&TestData::new(), 1_000_000);
        assert_eq!(a, b, "A test that compares a picture depends on this.");
    }

    /// The list describes the end of the window, and not the moment of the
    /// question. A person who moves the window one hour back then reads the
    /// list of that moment, as every other query of this data source does.
    #[test]
    fn processes_describes_the_end_of_the_window() {
        let src = TestData::new();
        let asked_then = processes_for(&src, 1_000_000, 1_000_000);
        let asked_later = processes_for(&src, 1_000_000, 1_000_000 + 3_600_000);
        assert_eq!(
            asked_then, asked_later,
            "The present moment must change nothing."
        );
        assert_ne!(
            asked_then,
            processes_at(&src, 1_000_000 + 3_600_000),
            "Another end of the window must give another list."
        );
    }

    #[test]
    fn each_process_stays_inside_its_own_range() {
        for at in (0..50).map(|i| i * 7_000) {
            let frame = processes_at(&TestData::new(), at);
            let cpu = frame.fields[2].values.as_f64().unwrap();
            let memory = frame.fields[3].values.as_f64().unwrap();
            for (index, (_, peak, megabytes)) in PROCESS_TABLE.iter().enumerate() {
                let share = cpu[index].unwrap();
                assert!((0.0..=*peak).contains(&share), "{share} {peak}");
                let bytes = memory[index].unwrap();
                assert!(
                    (0.8 * megabytes * 1e6..=1.2 * megabytes * 1e6).contains(&bytes),
                    "{bytes} {megabytes}"
                );
            }
        }
    }

    /// The scene of the program records about seventeen seconds. A list whose
    /// order never changed would show a reader no movement at all.
    #[test]
    fn the_busiest_process_changes_over_one_minute() {
        let src = TestData::new();
        let leaders: std::collections::BTreeSet<String> = (0..60)
            .map(|second| busiest_first(&processes_at(&src, 1_000_000 + second * 1_000))[1].clone())
            .collect();
        assert!(
            leaders.len() > 1,
            "The second row of a top panel must change in one minute. {leaders:?}"
        );
    }

    #[test]
    fn from_declaration_reads_the_seed_and_the_number_of_series() {
        let declaration: SourceDecl =
            toml::from_str("kind = \"testdata\"\nseed = 7\nseries = 3").unwrap();
        let source = TestData::from_declaration(&declaration).unwrap();
        let frames = block_on(source.query(req("sine", 10))).unwrap();
        assert_eq!(frames[0].numeric_fields().count(), 3);
        assert_eq!(source.seed, 7);
    }

    #[test]
    fn from_declaration_returns_an_error_for_an_option_that_it_does_not_know() {
        let declaration: SourceDecl =
            toml::from_str("kind = \"testdata\"\nseries = 3\nsereis = 2").unwrap();
        let error = TestData::from_declaration(&declaration)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("sereis"),
            "The message must name the option. {error}"
        );
    }

    #[test]
    fn from_declaration_returns_an_error_for_a_value_that_is_not_a_number() {
        let declaration: SourceDecl = toml::from_str("kind = \"testdata\"\nseed = \"x\"").unwrap();
        assert!(TestData::from_declaration(&declaration).is_err());
    }

    #[test]
    fn with_series_raises_a_count_of_zero_to_one() {
        let frames = run(&TestData::new().with_series(0), "sine", 10).unwrap();
        assert_eq!(frames[0].numeric_fields().count(), 1);
    }

    /// Runs a future to completion. This function lets the tests of this crate
    /// run without a dependency on an async runtime.
    ///
    /// The loop polls without a pause and without a limit. The waker that it
    /// builds wakes nothing. The function therefore holds one rule: the future
    /// must reach its end after a limited number of polls, and it must never
    /// wait for a wake. Every data source of this crate awaits nothing, so each
    /// future here reaches its end on the first poll.
    ///
    /// A future that returns `Poll::Pending` even once turns this loop into a
    /// spin that never ends. Such a test reports no failure. It holds a
    /// processor at full speed until the job reaches its limit of time, and the
    /// report then names a timeout. Use an async runtime for a future that
    /// awaits.
    ///
    /// `crates/thunderbox-source/src/lib.rs` and
    /// `crates/thunderbox-source-sysinfo/src/lib.rs` each hold a copy of this
    /// function.
    ///
    /// # Safety
    ///
    /// The vtable of the waker holds no state, and the pointer of the waker is
    /// null. No function of that vtable reads the pointer, so no code ever
    /// dereferences it. `Pin::new_unchecked` is sound because `fut` stays on
    /// this stack frame, and this function never moves it after the pin.
    fn block_on<F: std::future::Future>(mut fut: F) -> F::Output {
        use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
        fn vt() -> &'static RawWakerVTable {
            &RawWakerVTable::new(
                |_| RawWaker::new(std::ptr::null(), vt()),
                |_| {},
                |_| {},
                |_| {},
            )
        }
        let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), vt())) };
        let mut cx = Context::from_waker(&waker);
        let mut fut = unsafe { std::pin::Pin::new_unchecked(&mut fut) };
        loop {
            if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }
}
