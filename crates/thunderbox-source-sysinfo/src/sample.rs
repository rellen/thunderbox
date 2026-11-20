//! The code that reads the machine and adds samples to the store.

use sysinfo::{MemoryRefreshKind, Networks, RefreshKind, System};
use thunderbox_core::{Error, Result, Timestamp};

use crate::history::History;
use crate::process::PROCESSES;

/// The name of the label that separates one core from another.
///
/// The signal `cpu.cores` returns one series for each core that the machine
/// reports. This label holds the position of that core in the list that the
/// machine returned. A person writes `{{core}}` in the `legend` field of a
/// target, and each series then takes the name of its own core.
///
/// The machine decides what that list holds. It reports one entry for each
/// stream of instructions that it can run. A machine that runs two streams on
/// one piece of hardware therefore reports two entries for that piece.
pub const CORE_LABEL: &str = "core";

/// The first part of the name that the store gives to one core.
const CORE_PREFIX: &str = "cpu.core.";

/// The signals that this data source can draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// The share of the processor in use, across every core.
    Cpu,
    /// The share of each core in use. One series for each core.
    CpuCores,
    /// The share of the memory in use.
    Memory,
    /// The memory in use, in bytes.
    MemoryBytes,
    /// The share of the swap area in use.
    Swap,
    /// The bytes that the network cards received in each second.
    NetworkRx,
    /// The bytes that the network cards sent in each second.
    NetworkTx,
}

impl Signal {
    /// Returns the name of every signal, for a message and for a variable.
    pub fn names() -> &'static [&'static str] {
        &[
            "cpu",
            "cpu.cores",
            "memory",
            "memory.bytes",
            "swap",
            "network.rx",
            "network.tx",
        ]
    }

    /// Reads the name of a signal.
    ///
    /// # Errors
    ///
    /// This function returns an error when the name is not in the list above.
    pub fn parse(expr: &str) -> Result<Self> {
        match expr.trim() {
            "cpu" => Ok(Self::Cpu),
            "cpu.cores" => Ok(Self::CpuCores),
            "memory" => Ok(Self::Memory),
            "memory.bytes" => Ok(Self::MemoryBytes),
            "swap" => Ok(Self::Swap),
            "network.rx" => Ok(Self::NetworkRx),
            "network.tx" => Ok(Self::NetworkTx),
            other => Err(Error::Query(format!(
                "The sysinfo data source does not hold a signal named {other:?}. \
                 It holds {}.",
                query_names().join(", ")
            ))),
        }
    }

    /// Returns the name of each series that this signal draws.
    ///
    /// The store orders its names by the text. The name `cpu.core.10` comes
    /// before `cpu.core.2` in that order. A person reads the cores in the order
    /// of their position, so this method orders them by that number.
    pub fn series_names(self, history: &History) -> Vec<String> {
        match self {
            Self::CpuCores => {
                let mut names = history.names_with_prefix(CORE_PREFIX);
                names.sort_by_key(|name| core_position(name));
                names
            }
            other => vec![other.key().to_string()],
        }
    }

    /// Returns the labels of one series of this signal.
    ///
    /// A label states one property that separates one series of a signal from
    /// another series of the same signal. The signal `cpu.cores` is the only
    /// signal that returns more than one series, so every other signal returns
    /// an empty list.
    ///
    /// The value of the label [`CORE_LABEL`] is the position of the core in the
    /// list that the machine reported.
    pub fn series_labels(self, series: &str) -> Vec<(&'static str, String)> {
        match self {
            Self::CpuCores => vec![(CORE_LABEL, self.field_name(series))],
            _ => Vec::new(),
        }
    }

    /// Returns the key that the store uses for this signal.
    fn key(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::CpuCores => CORE_PREFIX,
            Self::Memory => "memory",
            Self::MemoryBytes => "memory.bytes",
            Self::Swap => "swap",
            Self::NetworkRx => "network.rx",
            Self::NetworkTx => "network.tx",
        }
    }

    /// Returns the name of the column that this signal draws for one series.
    pub fn field_name(self, series: &str) -> String {
        match self {
            Self::CpuCores => series.trim_start_matches(CORE_PREFIX).to_string(),
            _ => series.to_string(),
        }
    }

    /// Returns the name of the unit that a panel uses to format a value.
    pub fn unit(self) -> &'static str {
        match self {
            Self::Cpu | Self::CpuCores | Self::Memory | Self::Swap => "percent",
            Self::MemoryBytes => "bytes",
            Self::NetworkRx | Self::NetworkTx => "bytes/s",
        }
    }

    /// Returns the smallest value that an axis shows.
    pub fn min(self) -> Option<f64> {
        Some(0.0)
    }

    /// Returns the largest value that an axis shows.
    ///
    /// A share of use always reaches 100. A count of bytes has no fixed limit,
    /// so the panel takes its limit from the data.
    pub fn max(self) -> Option<f64> {
        match self {
            Self::Cpu | Self::CpuCores | Self::Memory | Self::Swap => Some(100.0),
            _ => None,
        }
    }
}

/// Returns the text of every query that this data source answers.
///
/// The list holds each signal of the store, and then the query that returns
/// the processes. That query returns one row for each process, and the store
/// holds no series of it, so [`Signal`] does not name it.
pub fn query_names() -> Vec<&'static str> {
    let mut names = Signal::names().to_vec();
    names.push(PROCESSES);
    names
}

/// Returns the position of one core, from the name of its series.
///
/// The store names a series `cpu.core.7`, and this function then returns 7.
/// The function returns `u32::MAX` when the name holds no position, so such a
/// name comes last.
fn core_position(series: &str) -> u32 {
    series
        .trim_start_matches(CORE_PREFIX)
        .parse()
        .unwrap_or(u32::MAX)
}

/// The reader that takes one sample of the machine.
pub struct Sampler {
    system: System,
    networks: Networks,
    /// The totals of the last sample, so that the next one can report a rate.
    last: Option<NetworkTotals>,
    last_at: Option<Timestamp>,
}

/// The bytes that every network card had carried at one moment.
#[derive(Debug, Clone, Copy)]
struct NetworkTotals {
    received: u64,
    transmitted: u64,
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}

impl Sampler {
    /// Returns a reader that has taken no sample.
    pub fn new() -> Self {
        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(sysinfo::CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(MemoryRefreshKind::everything()),
        );
        Self {
            system,
            networks: Networks::new_with_refreshed_list(),
            last: None,
            last_at: None,
        }
    }

    /// Reads the machine once and adds a sample to each series.
    pub fn sample(&mut self, history: &mut History, at: Timestamp) {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.networks.refresh(true);

        history.push("cpu", at, f64::from(self.system.global_cpu_usage()));
        for (index, cpu) in self.system.cpus().iter().enumerate() {
            history.push(
                &format!("{CORE_PREFIX}{index}"),
                at,
                f64::from(cpu.cpu_usage()),
            );
        }

        let total = self.system.total_memory() as f64;
        let used = self.system.used_memory() as f64;
        history.push("memory.bytes", at, used);
        if total > 0.0 {
            history.push("memory", at, used / total * 100.0);
        }

        let total_swap = self.system.total_swap() as f64;
        if total_swap > 0.0 {
            let used_swap = self.system.used_swap() as f64;
            history.push("swap", at, used_swap / total_swap * 100.0);
        }

        self.sample_network(history, at);
    }

    /// Adds the rate of the network to the store.
    ///
    /// A network card reports a total, and a panel draws a rate. The reader
    /// therefore keeps the total of the last sample, and it divides the
    /// difference by the time between the two samples. The first sample adds
    /// nothing, because one total gives no rate.
    fn sample_network(&mut self, history: &mut History, at: Timestamp) {
        let mut totals = NetworkTotals {
            received: 0,
            transmitted: 0,
        };
        for data in self.networks.values() {
            totals.received = totals.received.saturating_add(data.total_received());
            totals.transmitted = totals.transmitted.saturating_add(data.total_transmitted());
        }

        if let (Some(last), Some(last_at)) = (self.last, self.last_at) {
            let seconds = (at - last_at) as f64 / 1000.0;
            if seconds > 0.0 {
                let rx = totals.received.saturating_sub(last.received) as f64 / seconds;
                let tx = totals.transmitted.saturating_sub(last.transmitted) as f64 / seconds;
                history.push("network.rx", at, rx);
                history.push("network.tx", at, tx);
            }
        }
        self.last = Some(totals);
        self.last_at = Some(at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns a store that holds one sample for each of twelve cores.
    fn twelve_cores() -> History {
        let mut history = History::new();
        for index in 0..12 {
            history.push(&format!("{CORE_PREFIX}{index}"), 1_000, 1.0);
        }
        history
    }

    /// A machine of twelve cores gives the store the names `cpu.core.0` to
    /// `cpu.core.11`. The store orders those names by the text. A person
    /// therefore read `core 0, core 1, core 10, core 11, core 2` before this
    /// rule.
    #[test]
    fn the_cores_come_in_the_order_of_their_position() {
        let names = Signal::CpuCores.series_names(&twelve_cores());
        let positions: Vec<String> = names
            .iter()
            .map(|name| Signal::CpuCores.field_name(name))
            .collect();
        assert_eq!(
            positions,
            vec!["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11"],
            "A person reads the cores in the order of their position."
        );
    }

    #[test]
    fn each_series_of_the_cores_signal_carries_the_position_of_its_core() {
        let labels = Signal::CpuCores.series_labels("cpu.core.7");
        assert_eq!(
            labels,
            vec![(CORE_LABEL, "7".to_string())],
            "The label must separate one core from another."
        );
    }

    #[test]
    fn a_signal_that_returns_one_series_carries_no_label() {
        for signal in [
            Signal::Cpu,
            Signal::Memory,
            Signal::MemoryBytes,
            Signal::Swap,
            Signal::NetworkRx,
            Signal::NetworkTx,
        ] {
            assert!(
                signal.series_labels("anything").is_empty(),
                "A label must separate two series. This signal returns one. {signal:?}"
            );
        }
    }

    /// The store can hold a name that carries no position. The order must then
    /// stay stable, and the name must come last.
    #[test]
    fn a_name_that_holds_no_position_comes_last() {
        let mut history = twelve_cores();
        history.push("cpu.core.spare", 1_000, 1.0);
        let names = Signal::CpuCores.series_names(&history);
        assert_eq!(
            names.last().map(String::as_str),
            Some("cpu.core.spare"),
            "A name that holds no position must come last. {names:?}"
        );
    }
}
