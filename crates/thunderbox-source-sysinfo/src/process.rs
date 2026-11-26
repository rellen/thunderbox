//! The code that reads the processes of this machine.
//!
//! The query `processes` returns one row for each process. A row describes one
//! thing, and not one moment, so this code keeps no store over time. It keeps
//! the list of the last sample alone.

use std::time::{Duration, Instant};

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use thunderbox_core::{Field, FieldConfig, Frame, Timestamp, Values};

/// The text of the query that returns the processes of this machine.
pub const PROCESSES: &str = "processes";

/// One process, at the moment of the last sample.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessRow {
    /// The number that the operating system gives the process.
    pub pid: u32,
    /// The short name of the program that the process runs.
    pub name: String,
    /// The share of the processor that the process used since the sample
    /// before. A value of 100 is one whole core, so a process that runs on two
    /// cores reports 200.
    pub cpu: f64,
    /// The memory that the process holds in physical memory, in bytes.
    pub memory: f64,
}

/// The reader that takes one sample of the processes of this machine.
///
/// The reader holds a `System` of its own, and it shares no `System` with the
/// signal `cpu`. The `sysinfo` crate measures the share of a process against
/// the time of the processor between two refreshes. Each refresh of the
/// processes therefore reads the time of the processor again. A shared
/// `System` would then measure the signal `cpu` over a shorter period than the
/// time between two of its own samples.
pub struct ProcessSampler {
    system: System,
    /// The processes of the last sample, in the order of their number.
    rows: Vec<ProcessRow>,
    /// The moment when the machine produced the last sample.
    taken_at: Option<Timestamp>,
    /// The moment of the last sample, on the clock that measures a gap.
    last: Option<Instant>,
}

impl Default for ProcessSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessSampler {
    /// Returns a reader that has taken no sample.
    ///
    /// The reader reads no process until a query asks for the list. A dashboard
    /// that shows no list of processes therefore pays nothing for this
    /// reader.
    pub fn new() -> Self {
        Self {
            system: System::new(),
            rows: Vec::new(),
            taken_at: None,
            last: None,
        }
    }

    /// Reads the processes again when `gap` has passed since the last sample.
    ///
    /// Two panels can show the processes, and both panels ask in one refresh.
    /// The gap stops the second query from a second sample.
    ///
    /// The first sample reports a share of 0 for every process. The `sysinfo`
    /// crate needs two samples before it can report a share, as it does for the
    /// signal `cpu`.
    pub fn sample_if_due(&mut self, now: Timestamp, gap: Duration) {
        if self.last.is_some_and(|at| at.elapsed() < gap) {
            return;
        }
        self.last = Some(Instant::now());
        self.taken_at = Some(now);
        // The reader asks for no task. A task is a thread of a process, and
        // `sysinfo` otherwise lists each thread as a process of its own. A
        // thread of the kernel stays in the list, because the kernel runs it as
        // a process of its own.
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .without_tasks(),
        );
        let mut rows: Vec<ProcessRow> = self
            .system
            .processes()
            .values()
            .map(|process| ProcessRow {
                pid: process.pid().as_u32(),
                name: process.name().to_string_lossy().into_owned(),
                cpu: f64::from(process.cpu_usage()),
                memory: process.memory() as f64,
            })
            .collect();
        // The map of `sysinfo` holds no order. A panel keeps the order of the
        // data for two rows that hold the same number. An order that changed
        // on each sample would move those rows on each refresh.
        rows.sort_by_key(|row| row.pid);
        self.rows = rows;
    }

    /// Returns the frame of the last sample, for a window from `from` to `to`.
    ///
    /// The frame holds the rows only when the window holds the moment of the
    /// sample. This data source keeps no list of processes from the past. A
    /// window that a person moved into the past therefore holds the columns
    /// and no row. The panel then reports that it has no data.
    pub fn frame(&self, from: Timestamp, to: Timestamp) -> Frame {
        let inside = self.taken_at.is_some_and(|at| from <= at && at <= to);
        let rows: &[ProcessRow] = if inside { &self.rows } else { &[] };
        let text = |values: Vec<String>, name: &str| Field {
            name: name.to_string(),
            values: Values::Str(values.into_iter().map(Some).collect()),
            ..Default::default()
        };
        let number = |values: Vec<f64>, name: &str, unit: &str| {
            Field::f64(name, values.into_iter().map(Some).collect()).with_config(FieldConfig {
                unit: Some(unit.to_string()),
                min: Some(0.0),
                ..Default::default()
            })
        };
        // The number of a process is a name, and not a quantity. The column
        // is an identifier, so a panel writes every digit of 1234 and never
        // writes `1.2K`. A top panel never puts the rows in order by it.
        let identifier = |values: Vec<f64>, name: &str| {
            Field::f64(name, values.into_iter().map(Some).collect()).with_config(FieldConfig {
                identifier: true,
                ..Default::default()
            })
        };
        Frame::named(
            PROCESSES,
            vec![
                identifier(rows.iter().map(|r| f64::from(r.pid)).collect(), "pid"),
                text(rows.iter().map(|r| r.name.clone()).collect(), "name"),
                number(rows.iter().map(|r| r.cpu).collect(), "cpu", "percent"),
                number(rows.iter().map(|r| r.memory).collect(), "memory", "bytes"),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gap that the tests of this module wait between two samples.
    const GAP: Duration = Duration::from_millis(250);

    /// Returns the number of each process of a frame, in the order of the rows.
    fn pids(frame: &Frame) -> Vec<u32> {
        let Values::F64(values) = &frame.fields[0].values else {
            panic!("The first column must hold numbers. {:?}", frame.fields[0]);
        };
        values
            .iter()
            .map(|v| {
                v.map(|number| number as u32)
                    .expect("Each row must hold the number of its process.")
            })
            .collect()
    }

    #[test]
    fn a_sample_lists_this_process() {
        let mut sampler = ProcessSampler::new();
        sampler.sample_if_due(1_000, GAP);
        let frame = sampler.frame(0, 2_000);
        assert!(
            pids(&frame).contains(&std::process::id()),
            "The list must hold the process that runs this test."
        );
    }

    #[test]
    fn the_column_pid_holds_numbers_and_is_an_identifier() {
        let mut sampler = ProcessSampler::new();
        sampler.sample_if_due(1_000, GAP);
        let frame = sampler.frame(0, 2_000);
        let pid = &frame.fields[0];
        assert_eq!(pid.name, "pid");
        assert!(matches!(pid.values, Values::F64(_)), "{pid:?}");
        assert!(pid.config.identifier, "{pid:?}");
        assert!(
            !pid.is_numeric(),
            "The number of a process is not a series, so it never gives the order."
        );
        assert!(
            frame.fields[1..].iter().all(|f| !f.config.identifier),
            "Only the column pid is an identifier."
        );
    }

    #[test]
    fn every_column_holds_one_value_for_each_process() {
        let mut sampler = ProcessSampler::new();
        sampler.sample_if_due(1_000, GAP);
        let frame = sampler.frame(0, 2_000);
        let names: Vec<&str> = frame.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["pid", "name", "cpu", "memory"]);
        let rows = frame.len();
        assert!(rows > 0, "A machine runs at least one process.");
        assert!(
            frame.fields.iter().all(|f| f.len() == rows),
            "The fifth duty of a data source asks for one length."
        );
        assert_eq!(frame.fields[2].config.unit.as_deref(), Some("percent"));
        assert_eq!(frame.fields[3].config.unit.as_deref(), Some("bytes"));
    }

    #[test]
    fn the_rows_come_in_the_order_of_the_number_of_each_process() {
        let mut sampler = ProcessSampler::new();
        sampler.sample_if_due(1_000, GAP);
        let pids = pids(&sampler.frame(0, 2_000));
        assert!(
            pids.windows(2).all(|pair| pair[0] < pair[1]),
            "A panel keeps the order of the data for a tie, so the order must not change. \
             {pids:?}"
        );
    }

    /// A process that runs two threads appears one time. `sysinfo` lists each
    /// thread as a process of its own when a refresh reads the tasks.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_thread_of_this_process_is_not_a_row_of_its_own() {
        let (send, receive) = std::sync::mpsc::channel();
        let (stop, wait) = std::sync::mpsc::channel::<()>();
        let worker = std::thread::spawn(move || {
            // The link names the thread as `process/task/thread`.
            let link = std::fs::read_link("/proc/thread-self").expect("Linux holds this link.");
            let thread: u32 = link
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.parse().ok())
                .expect("The link ends with the number of the thread.");
            send.send(thread)
                .expect("The test must receive the number.");
            let _ = wait.recv();
        });
        let thread = receive.recv().expect("The thread must report its number.");
        assert_ne!(
            thread,
            std::process::id(),
            "A new thread holds a number of its own."
        );

        let mut sampler = ProcessSampler::new();
        sampler.sample_if_due(1_000, GAP);
        let pids = pids(&sampler.frame(0, 2_000));
        stop.send(()).expect("The thread must still wait.");
        worker.join().expect("The thread must end.");

        assert!(pids.contains(&std::process::id()));
        assert!(
            !pids.contains(&thread),
            "A thread of this process must not appear as a process. {thread}"
        );
    }

    /// Holds one core of the processor for this long.
    fn spin(time: Duration) {
        let end = Instant::now() + time;
        let mut spins: u64 = 0;
        while Instant::now() < end {
            spins = std::hint::black_box(spins.wrapping_add(1));
        }
    }

    /// The process must use the processor before the first sample as well.
    /// Version 0.38.4 of `sysinfo` reports no share for a process that held no
    /// time of the processor at the sample before. `compute_cpu_usage` in
    /// `src/unix/linux/process.rs` returns early in that state. A test program
    /// that starts this test first holds less than one tick of 10 milliseconds
    /// at that sample. This test then failed 30 times of 30, and the sandbox
    /// of Nix ran it in that way.
    #[test]
    fn a_second_sample_reports_the_share_of_a_busy_process() {
        spin(Duration::from_millis(100));
        let mut sampler = ProcessSampler::new();
        sampler.sample_if_due(1_000, GAP);
        spin(Duration::from_millis(400));
        sampler.sample_if_due(1_400, GAP);
        let me = sampler
            .rows
            .iter()
            .find(|row| row.pid == std::process::id())
            .expect("The list must hold this process.");
        assert!(
            me.cpu > 0.0,
            "This process held a core for 400 milliseconds. It reports {}.",
            me.cpu
        );
        assert!(me.memory > 0.0, "This process holds memory.");
    }

    #[test]
    fn two_queries_close_together_take_one_sample_only() {
        let mut sampler = ProcessSampler::new();
        sampler.sample_if_due(1_000, GAP);
        sampler.sample_if_due(1_100, GAP);
        assert_eq!(
            sampler.taken_at,
            Some(1_000),
            "Two panels that ask in one refresh must share one sample."
        );
    }

    #[test]
    fn a_window_that_does_not_hold_the_sample_returns_the_columns_and_no_row() {
        let mut sampler = ProcessSampler::new();
        sampler.sample_if_due(1_000_000, GAP);
        for (from, to) in [(0, 60_000), (1_000_001, 2_000_000)] {
            let frame = sampler.frame(from, to);
            assert_eq!(frame.fields.len(), 4, "The columns must stay.");
            assert!(
                frame.is_empty(),
                "This data source keeps no list from the past. {from} {to}"
            );
        }
        assert!(
            !sampler.frame(1_000_000, 1_000_000).is_empty(),
            "A window that ends at the moment of the sample holds it."
        );
    }

    #[test]
    fn a_reader_that_took_no_sample_returns_no_row() {
        let frame = ProcessSampler::new().frame(Timestamp::MIN, Timestamp::MAX);
        assert_eq!(frame.fields.len(), 4);
        assert!(frame.is_empty());
    }
}
