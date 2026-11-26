//! A data source that reads this machine.
//!
//! The `sysinfo` crate reports the state of the machine at one moment. A panel
//! draws values against time, so this crate keeps its own store of samples.
//! The query `processes` is the one exception, and the section below states
//! it.
//!
//! The store fills when a caller runs a query. This crate starts no task of its
//! own, and it needs no async runtime. The trait gives this crate no place to
//! stop such a task. A dashboard that refreshes once a second therefore adds
//! one sample a second to each series.
//!
//! Two effects follow from that choice, and a reader should know both:
//!
//! - The store holds nothing from before the program started. A chart of the
//!   last hour draws only the part of that hour that the program saw.
//! - The distance between two samples equals the refresh interval of the
//!   dashboard. A dashboard that refreshes every ten seconds cannot draw a
//!   change that lasted one second.
//!
//! # The labels of this data source
//!
//! The signal `cpu.cores` returns one series for each core that the machine
//! reports. Each of those series carries the label `core`. The value of that
//! label is the position of the core in the list that the machine returned. A
//! person writes `{{core}}` in the `legend` field of a target, and each series
//! then takes the name of its own core.
//!
//! The machine decides what that list holds. It reports one entry for each
//! stream of instructions that it can run. A machine that runs two streams on
//! one piece of hardware therefore reports two entries for that piece.
//!
//! The series come in the order of that position, and not in the order of the
//! text of their names.
//!
//! Every other signal returns one series, so no property separates one series
//! of that signal from another. Those signals therefore carry no label.
//!
//! # The list of processes
//!
//! The query `processes` returns one frame, and that frame holds one row for
//! each process of the machine. The four columns are these:
//!
//! - `pid`, the number of the process. This column holds numbers and is an
//!   identifier, so a panel writes every digit of 1234 and never writes
//!   `1.2K`. A top panel never puts the rows in order by this column;
//! - `name`, the short name of the program that the process runs;
//! - `cpu`, the share of the processor since the sample before. A value of 100
//!   is one whole core, as in the program `top`;
//! - `memory`, the memory that the process holds in physical memory, in bytes.
//!
//! The rows come in the order of the number of each process. A top panel puts
//! them in another order. The frame holds no time column, because a row
//! describes a process and not a moment.
//!
//! This crate keeps no list of processes from the past. A window that does not
//! hold the moment of the last sample therefore returns the four columns and no
//! row. The first sample reports a share of 0 for each process, because the
//! `sysinfo` crate needs two samples before it can report a share.
//!
//! A variable of this data source takes the name of a signal, and not the value
//! of a label. [`DataSource::label_values`] returns the names of the signals for
//! every query. ADR 0042 in `docs/contributor/adr` states when the runtime asks for those
//! names.

mod history;
mod process;
mod sample;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use thunderbox_core::{
    Error, Field, FieldConfig, Frame, QueryRequest, Result, SourceDecl, Timestamp,
};
use thunderbox_source::{DataSource, SourceDescriptor};

/// The type name that a dashboard writes in the `kind` field.
pub const KIND: &str = "sysinfo";

use crate::history::History;
use crate::process::{PROCESSES, ProcessSampler};
use crate::sample::{Sampler, Signal, query_names};

/// The shortest time between two samples.
///
/// A dashboard runs the query of each panel, and every panel reads the same
/// machine. This limit stops a dashboard of six panels from taking six samples
/// for one refresh.
const MIN_SAMPLE_GAP: Duration = Duration::from_millis(250);

/// A data source that reads the processor, the memory, the network, and the
/// processes of this machine.
pub struct SysinfoSource {
    inner: Mutex<Inner>,
}

struct Inner {
    sampler: Sampler,
    history: History,
    last_sample: Option<Instant>,
    processes: ProcessSampler,
}

impl Default for SysinfoSource {
    fn default() -> Self {
        Self::new()
    }
}

impl SysinfoSource {
    /// Returns a data source that holds no sample yet.
    ///
    /// The first query takes the first sample. The processor needs two samples
    /// before it can report a share of use, so the first chart of the processor
    /// holds one point at zero.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                sampler: Sampler::new(),
                history: History::new(),
                last_sample: None,
                processes: ProcessSampler::new(),
            }),
        }
    }

    /// Returns a data source from the declaration in a dashboard.
    ///
    /// This data source reads the machine that the program runs on, so it takes
    /// no address and no option.
    ///
    /// # Errors
    ///
    /// This function returns an error if the declaration holds an option.
    pub fn from_declaration(declaration: &SourceDecl) -> Result<Self> {
        if let Some(key) = declaration.options.keys().next() {
            return Err(Error::Dashboard(format!(
                "a sysinfo source has no option {key:?}; it reads the machine \
                 that the program runs on"
            )));
        }
        Ok(Self::new())
    }

    /// Takes a sample when enough time has passed since the last one.
    fn sample_if_due(inner: &mut Inner, now: Timestamp) {
        let due = match inner.last_sample {
            None => true,
            Some(at) => at.elapsed() >= MIN_SAMPLE_GAP,
        };
        if !due {
            return;
        }
        inner.last_sample = Some(Instant::now());
        inner.sampler.sample(&mut inner.history, now);
    }
}

#[async_trait]
impl DataSource for SysinfoSource {
    fn descriptor(&self) -> SourceDescriptor {
        // The store returns the number of points that the caller asks for, so
        // the caller does not reduce them again.
        SourceDescriptor::new("sysinfo", "This machine")
            .downsampling()
            .with_label_values()
    }

    async fn query(&self, req: QueryRequest) -> Result<Vec<Frame>> {
        let processes = req.query.expr.trim() == PROCESSES;
        let signal = if processes {
            None
        } else {
            Some(Signal::parse(&req.query.expr)?)
        };
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| Error::Query("The store of samples is not usable.".into()))?;

        // The list of processes reads no store. A sample of the processes
        // costs more than a sample of the other signals. A dashboard that shows
        // no list of processes therefore takes no such sample.
        let Some(signal) = signal else {
            inner.processes.sample_if_due(req.now, MIN_SAMPLE_GAP);
            return Ok(vec![inner.processes.frame(req.from, req.to)]);
        };

        // The sample carries the moment when the machine produced it, and
        // never the end of the range. This data source would otherwise stamp
        // every sample with an old moment while a person studies the past. The
        // store would then hold its samples out of order.
        Self::sample_if_due(&mut inner, req.now);

        let names = signal.series_names(&inner.history);
        let mut fields = Vec::with_capacity(names.len() + 1);
        let mut times: Vec<Timestamp> = Vec::new();

        for (index, name) in names.iter().enumerate() {
            let points = inner
                .history
                .range(name, req.from, req.to, req.max_data_points);
            if index == 0 {
                times = points.iter().map(|(at, _)| *at).collect();
            }
            // Every series of one signal shares one sample time, so the values
            // line up with the times of the first series.
            let values = points.iter().map(|(_, value)| Some(*value)).collect();
            let mut column = Field::f64(signal.field_name(name), values).with_config(FieldConfig {
                unit: Some(signal.unit().to_string()),
                min: signal.min(),
                max: signal.max(),
                ..Default::default()
            });
            let labels = signal.series_labels(name);
            if !labels.is_empty() {
                // `Field::display_name` writes the labels of a column beside
                // the name of that column, when the configuration holds no
                // name. This data source therefore states the name that a
                // panel draws. A target that holds no pattern then shows the
                // same short name as before.
                column.config.display_name = Some(column.name.clone());
            }
            for (key, value) in labels {
                column = column.with_label(key, value);
            }
            fields.push(column);
        }

        let mut all = vec![Field::time("time", times)];
        all.append(&mut fields);
        Ok(vec![Frame::named(req.query.expr.clone(), all)])
    }

    /// Returns the name of every signal, whatever the query asks for.
    ///
    /// A variable of this data source takes the name of a signal. It does not
    /// take the value of a label, so this method reads no query. The module
    /// comment states that limit.
    ///
    /// The list ends with the query `processes`. A variable can select that
    /// query for a top panel.
    async fn label_values(&self, _query: &str) -> Result<Vec<String>> {
        Ok(query_names().iter().map(|s| (*s).to_string()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thunderbox_core::{Query, RelativeTime, TimeRange};

    fn request(expr: &str, now: Timestamp) -> QueryRequest {
        QueryRequest::new(
            Query {
                source: "machine".into(),
                expr: expr.into(),
            },
            &TimeRange {
                from: RelativeTime::Offset(-300_000),
                to: RelativeTime::Now,
            },
            now,
            200,
        )
    }

    fn ask(source: &SysinfoSource, expr: &str, now: Timestamp) -> Result<Vec<Frame>> {
        block_on(source.query(request(expr, now)))
    }

    /// Returns the time of every sample of one series.
    fn stamps_of(source: &SysinfoSource, name: &str) -> Vec<Timestamp> {
        source
            .inner
            .lock()
            .unwrap()
            .history
            .range(name, Timestamp::MIN, Timestamp::MAX, u32::MAX)
            .iter()
            .map(|(at, _)| *at)
            .collect()
    }

    /// The fields of a request are public, so a caller can state any number.
    /// The data source must answer, and it must not panic.
    #[test]
    fn a_request_at_the_limits_of_the_number_gets_an_answer() {
        let source = SysinfoSource::new();
        for expr in ["cpu", "memory", "network.rx", PROCESSES] {
            for now in [i64::MAX, i64::MIN, 0] {
                for (from, to) in [(i64::MIN, i64::MAX), (i64::MAX, i64::MIN), (0, 0)] {
                    let request = QueryRequest {
                        query: Query {
                            source: "machine".into(),
                            expr: expr.into(),
                        },
                        from,
                        to,
                        now,
                        step_ms: i64::MAX,
                        max_data_points: u32::MAX,
                    };
                    block_on(source.query(request)).expect("The query must answer.");
                }
            }
        }
    }

    /// A person can move the range one hour into the past. A new sample must
    /// still carry the moment when the machine produced it. The store would
    /// otherwise hold its samples out of order, and a chart would draw a line
    /// that jumps backwards.
    #[test]
    fn a_sample_carries_the_present_moment_and_not_the_end_of_the_range() {
        let source = SysinfoSource::new();
        ask(&source, "cpu", 1_000_000).expect("The query must answer.");
        std::thread::sleep(MIN_SAMPLE_GAP);

        let past = TimeRange {
            from: RelativeTime::Absolute(1_000_000 - 3_600_000),
            to: RelativeTime::Absolute(1_000_000 - 3_540_000),
        };
        let request = QueryRequest::new(
            Query {
                source: "machine".into(),
                expr: "cpu".into(),
            },
            &past,
            1_000_300,
            200,
        );
        block_on(source.query(request)).expect("The query must answer.");

        let stamps = stamps_of(&source, "cpu");
        assert_eq!(stamps.len(), 2, "The store must hold both samples.");
        assert!(
            stamps.windows(2).all(|pair| pair[0] < pair[1]),
            "The store must stay in order. {stamps:?}"
        );
        assert!(
            stamps.iter().all(|at| *at >= 1_000_000),
            "Each sample must carry the moment of the machine. {stamps:?}"
        );
    }

    /// The store holds nothing from before the program started. A window in
    /// the past therefore returns no point, and the panel then reports that it
    /// has no data.
    #[test]
    fn a_window_that_the_store_does_not_cover_returns_no_point() {
        let source = SysinfoSource::new();
        ask(&source, "cpu", 1_000_000).expect("The query must answer.");
        let past = TimeRange {
            from: RelativeTime::Absolute(0),
            to: RelativeTime::Absolute(60_000),
        };
        let request = QueryRequest::new(
            Query {
                source: "machine".into(),
                expr: "cpu".into(),
            },
            &past,
            1_000_000,
            200,
        );
        let frames = block_on(source.query(request)).expect("The query must answer.");
        assert!(
            frames[0].is_empty(),
            "This data source holds nothing from that window."
        );
    }

    #[test]
    fn an_unknown_signal_returns_an_error_that_lists_the_signals() {
        let error = ask(&SysinfoSource::new(), "cheese", 1_000_000)
            .unwrap_err()
            .to_string();
        assert!(error.contains("cheese"), "The message is wrong. {error}");
        assert!(
            error.contains("cpu.cores"),
            "The message must list the signals. {error}"
        );
        assert!(
            error.contains("processes"),
            "The message must list the query of the processes. {error}"
        );
    }

    #[test]
    fn a_query_of_the_processes_returns_one_row_for_each_process() {
        let source = SysinfoSource::new();
        let frames = ask(&source, " processes ", 1_000_000).expect("The query must answer.");
        assert_eq!(frames.len(), 1, "The list of processes is one frame.");
        let frame = &frames[0];
        assert!(!frame.is_empty(), "A machine runs at least one process.");
        assert!(
            frame.time_field().is_none(),
            "A row describes a process and not a moment."
        );
        assert_eq!(
            frame.numeric_fields().next().map(|f| f.name.as_str()),
            Some("cpu"),
            "A top panel that names no column takes the first column of numbers."
        );
    }

    /// A list of processes reads no store. A query of that list must therefore
    /// leave the store as it was. A dashboard that shows only that list then
    /// pays for no other sample.
    #[test]
    fn a_query_of_the_processes_takes_no_sample_of_the_store() {
        let source = SysinfoSource::new();
        ask(&source, "processes", 1_000_000).expect("The query must answer.");
        assert!(
            stamps_of(&source, "cpu").is_empty(),
            "The query of the processes must take no sample of the processor."
        );
    }

    /// The window of the default dashboard ends at the present moment. A window
    /// that a person moved one hour into the past holds no list of processes.
    #[test]
    fn a_window_in_the_past_returns_no_process() {
        let source = SysinfoSource::new();
        let past = TimeRange {
            from: RelativeTime::Absolute(1_000_000 - 3_600_000),
            to: RelativeTime::Absolute(1_000_000 - 3_540_000),
        };
        let request = QueryRequest::new(
            Query {
                source: "machine".into(),
                expr: "processes".into(),
            },
            &past,
            1_000_000,
            200,
        );
        let frames = block_on(source.query(request)).expect("The query must answer.");
        assert!(
            frames[0].is_empty(),
            "This data source keeps no list of processes from the past."
        );
    }

    #[test]
    fn a_query_of_the_processor_returns_a_frame_that_a_panel_can_draw() {
        let source = SysinfoSource::new();
        let frames = ask(&source, "cpu", 1_000_000).expect("The query must answer.");
        let frame = &frames[0];
        let (times, numbers) = frame
            .time_series()
            .expect("A panel needs one time column and one number column.");
        assert_eq!(times.len(), 1, "The first query takes the first sample.");
        assert_eq!(numbers.len(), 1);
        assert_eq!(numbers[0].config.unit.as_deref(), Some("percent"));
    }

    #[test]
    fn the_share_of_the_processor_stays_between_zero_and_one_hundred() {
        let source = SysinfoSource::new();
        let mut now = 1_000_000;
        for _ in 0..4 {
            std::thread::sleep(MIN_SAMPLE_GAP);
            now += 300;
            let frames = ask(&source, "cpu", now).expect("The query must answer.");
            let (_, numbers) = frames[0].time_series().expect("The frame must hold data.");
            for value in numbers[0].values.as_f64().unwrap().iter().flatten() {
                assert!(
                    (0.0..=100.0).contains(value),
                    "The value is outside the range. {value}"
                );
            }
        }
    }

    #[test]
    fn two_queries_close_together_take_one_sample_only() {
        let source = SysinfoSource::new();
        for _ in 0..6 {
            ask(&source, "cpu", 1_000_000).expect("The query must answer.");
        }
        let frames = ask(&source, "cpu", 1_000_000).expect("The query must answer.");
        let (times, _) = frames[0].time_series().expect("The frame must hold data.");
        assert_eq!(
            times.len(),
            1,
            "A dashboard of six panels must not take six samples."
        );
    }

    #[test]
    fn the_cores_signal_returns_one_series_for_each_core() {
        let source = SysinfoSource::new();
        let frames = ask(&source, "cpu.cores", 1_000_000).expect("The query must answer.");
        let count = frames[0].numeric_fields().count();
        assert!(count >= 1, "The machine must report at least one core.");
        assert_eq!(
            count,
            cores_reported(&source),
            "Each core must have its own series."
        );
    }

    /// The second duty of a data source is one label for each property that
    /// separates one series from another. The position of the core is that
    /// property here.
    #[test]
    fn each_column_of_the_cores_signal_carries_the_position_of_its_core() {
        let source = SysinfoSource::new();
        let frames = ask(&source, "cpu.cores", 1_000_000).expect("The query must answer.");
        for column in frames[0].numeric_fields() {
            assert_eq!(
                column.labels.get("core").map(String::as_str),
                Some(column.name.as_str()),
                "Each column must carry the position of its own core."
            );
        }
    }

    /// A label would otherwise reach the name that a panel draws.
    /// `Field::display_name` writes `0{core="0"}` for a column that carries a
    /// label and no display name. This data source states the short name, so a
    /// target that holds no pattern draws the same name as before.
    #[test]
    fn a_column_of_a_core_shows_the_short_name_that_it_showed_before() {
        let source = SysinfoSource::new();
        let frames = ask(&source, "cpu.cores", 1_000_000).expect("The query must answer.");
        for column in frames[0].numeric_fields() {
            assert_eq!(
                column.display_name(),
                column.name,
                "A person who states no pattern must read the short name."
            );
        }
    }

    #[test]
    fn a_column_of_a_signal_that_returns_one_series_carries_no_label() {
        let source = SysinfoSource::new();
        for expr in [
            "cpu",
            "memory",
            "memory.bytes",
            "swap",
            "network.rx",
            "network.tx",
        ] {
            let frames = ask(&source, expr, 1_000_000).expect("The query must answer.");
            for column in frames[0].numeric_fields() {
                assert!(
                    column.labels.is_empty(),
                    "A label must separate two series. This signal returns one. {expr}"
                );
                assert!(
                    column.config.display_name.is_none(),
                    "A column that carries no label needs no display name. {expr}"
                );
            }
        }
    }

    /// Returns the count of the cores that the machine reported.
    fn cores_reported(source: &SysinfoSource) -> usize {
        source
            .inner
            .lock()
            .unwrap()
            .history
            .names_with_prefix("cpu.core.")
            .len()
    }

    #[test]
    fn the_memory_signal_reports_a_share_and_a_count_of_bytes() {
        let source = SysinfoSource::new();
        let share = ask(&source, "memory", 1_000_000).expect("The query must answer.");
        let (_, numbers) = share[0].time_series().expect("The frame must hold data.");
        let value = numbers[0].values.as_f64().unwrap()[0].unwrap();
        assert!(
            (0.0..=100.0).contains(&value),
            "The share is outside the range. {value}"
        );

        let bytes = ask(&source, "memory.bytes", 1_000_000).expect("The query must answer.");
        let (_, numbers) = bytes[0].time_series().expect("The frame must hold data.");
        assert_eq!(numbers[0].config.unit.as_deref(), Some("bytes"));
        assert!(numbers[0].values.as_f64().unwrap()[0].unwrap() > 0.0);
    }

    #[test]
    fn the_first_sample_of_the_network_reports_no_rate() {
        let source = SysinfoSource::new();
        let frames = ask(&source, "network.rx", 1_000_000).expect("The query must answer.");
        // One total gives no rate, so the store holds nothing yet. The frame
        // therefore holds a time column and an empty number column.
        assert!(
            frames[0].time_series().is_err() || frames[0].is_empty(),
            "One total cannot give a rate."
        );
    }

    #[test]
    fn a_later_sample_of_the_network_reports_a_rate_that_is_not_negative() {
        let source = SysinfoSource::new();
        ask(&source, "network.rx", 1_000_000).expect("The query must answer.");
        std::thread::sleep(MIN_SAMPLE_GAP);
        let frames = ask(&source, "network.rx", 1_000_300).expect("The query must answer.");
        let (_, numbers) = frames[0]
            .time_series()
            .expect("The second sample must give a rate.");
        for value in numbers[0].values.as_f64().unwrap().iter().flatten() {
            assert!(*value >= 0.0, "A rate must not be negative. {value}");
        }
        assert_eq!(numbers[0].config.unit.as_deref(), Some("bytes/s"));
    }

    #[test]
    fn the_descriptor_states_that_this_source_reduces_the_points_itself() {
        let descriptor = SysinfoSource::new().descriptor();
        assert_eq!(descriptor.kind, "sysinfo");
        assert!(descriptor.downsamples);
        assert!(descriptor.supports_label_values);
    }

    #[test]
    fn label_values_lists_every_signal() {
        let names = block_on(SysinfoSource::new().label_values("")).unwrap();
        assert!(names.contains(&"cpu".to_string()));
        assert!(names.contains(&"network.tx".to_string()));
        assert!(names.contains(&"processes".to_string()));
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
    /// `crates/thunderbox-source-testdata/src/lib.rs` each hold a copy of this
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
