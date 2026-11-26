# Sysinfo data source

The data source `sysinfo` reads the machine that the program runs on. It reads
the processor, the memory, the swap area, the network, and the processes. The
swap area is a part of a disk that the operating system uses as more memory. The
dashboard inside the program reads this data source.

The example below is a part of a dashboard file, and it does not load alone.
The file also needs a `title`.

```toml
[source.machine]
kind = "sysinfo"
```

## Options

The data source takes no option. It ignores the field `url`. Any other field
gives this error. The program writes it under the line
`The program cannot make the data sources of the dashboard.`

```
The dashboard is not correct. a sysinfo source has no option "NAME"; it reads the machine that the program runs on
```

## Queries

The `expr` field of a target holds the name of one query. The data source
ignores the spaces around the name.

| query | what it reports | unit | range | label |
|---|---|---|---|---|
| `cpu` | The share of the processor in use, across every core. | `percent` | 0 to 100 | none |
| `cpu.cores` | The share of each core in use. The data source returns one series for each core that the machine reports. | `percent` | 0 to 100 | `core` |
| `memory` | The share of the memory in use. | `percent` | 0 to 100 | none |
| `memory.bytes` | The memory in use. | `bytes` | 0 and more | none |
| `swap` | The share of the swap area in use. | `percent` | 0 to 100 | none |
| `network.rx` | The sum of the bytes that each network card received in one second. | `bytes/s` | 0 and more | none |
| `network.tx` | The sum of the bytes that each network card sent in one second. | `bytes/s` | 0 and more | none |
| `processes` | One row for each process of the machine. The section below states the columns. | `percent` and `bytes` | 0 and more | none |

A query of another name gives this error in the panel:

```
The data source returned an error. The sysinfo data source does not hold a signal named "NAME". It holds cpu, cpu.cores, memory, memory.bytes, swap, network.rx, network.tx, processes.
```

## Answer of a signal

A frame is a table of columns, and each column holds one value for each row.
Each query except `processes` is a signal. The answer of a signal holds one
frame:

- The name of the frame is the text of the query.
- The first column holds the time of each sample, and its name is `time`.
- Each other column holds one series. The name of the column is the name of the
  signal. The signal `cpu.cores` is the one exception.
- The signal `cpu.cores` names each column after the position of its core: `0`,
  `1`, `2`, and so on. Each such column carries the label `core`, and the label
  holds the same position.
- The series of `cpu.cores` come in the order of that position.

The machine decides what the list of the cores holds. It reports one entry for
each stream of instructions that it can run. A machine that runs two streams on
one piece of hardware therefore reports two entries for that piece.

## Store of samples

The machine reports its state at one moment. A panel draws values against time,
so this data source keeps a store of samples.

- The data source takes a sample when a query runs. It starts no timer of its
  own.
- The distance between two samples therefore equals the field `refresh` of the
  dashboard.
- The data source takes no more than one sample in 250 milliseconds. The panels
  of one set of queries therefore share one sample.
- The store holds nothing from before the program started. A window that ends
  before the first sample shows `no data`.
- Each series holds 4096 samples or fewer. A full series drops its oldest
  sample for each new sample. A dashboard that refreshes each second fills a
  series in about one hour.
- A query returns the point limit of the panel or fewer points. The points
  stand at an even distance apart, and the most recent sample is always one of
  them.

Some signals need two samples:

- The first sample of `cpu` and of `cpu.cores` reports a share of 0. The
  machine needs two samples to measure a share.
- The first sample adds no point to `network.rx` and `network.tx`. A rate needs
  two totals.
- A machine that holds no swap area adds no point to `swap`. A panel of that
  signal then shows `no data`.

## List of the processes

The query `processes` reads no store. Its answer holds one frame, named
`processes`, with one row for each process and these four columns:

| column | what it holds | the type of the column | unit |
|---|---|---|---|
| `pid` | The number of the process. | numbers, and an identifier | none |
| `name` | The short name of the program that the process runs. | text | none |
| `cpu` | The share of the processor since the sample before. A value of 100 is one whole core, as in the program `top`. | numbers | `percent` |
| `memory` | The memory that the process holds in physical memory. | numbers | `bytes` |

- The frame holds no time column, because a row describes a process and not a
  moment.
- The column `pid` holds numbers, and it is an identifier. A panel writes the
  process 1234 as `1234`, and never as `1.2K`. A top panel never puts the rows
  in order by this column. The page [Numbers and units](../numbers-and-units.md)
  states the rules for an identifier.
- The rows come in the order of the number of each process. A top panel puts
  them in its own order.
- The list holds each process one time. It holds no thread of a process as a row
  of its own.
- The first sample reports a share of 0 for each process.
- The data source keeps no list from the past. A window that does not hold the
  moment of the last sample returns the four columns and no row. The panel then
  shows `no data`.
- The data source takes no sample of the processes for a dashboard that asks
  for no list of the processes.

## A variable of this data source

A variable that states a `query` on this data source takes the name of each
query as its choices. The list ends with `processes`. The data source ignores
the text of the query of the variable.

The example below is a part of a dashboard file, and it does not load alone.
The file also needs a `title`, and a table `[source.machine]` of the kind
`sysinfo`.

```toml
[[variable]]
name = "signal"
query = { source = "machine", expr = "any text" }
```

## The longest wait

The data source states no longest wait. thunderbox therefore waits 60 seconds.

## Pages that give the reason

- [Watch this machine](../../tutorials/watch-this-machine.md) reads this data
  source for the first time.
- [Architecture decision record 0038](../../../contributor/adr/0038-a-sysinfo-sample-carries-the-moment-of-the-machine.md)
  states why a sample carries the moment of the machine.
- [Architecture decision record 0047](../../../contributor/adr/0047-the-process-list-of-sysinfo-keeps-no-past.md)
  states why the list of the processes keeps no past.
- [Architecture decision record 0049](../../../contributor/adr/0049-the-number-of-a-process-is-text.md)
  states why the number of a process was text. Record 0083 replaced it.
- [Architecture decision record 0083](../../../contributor/adr/0083-a-column-can-hold-identifiers.md)
  states why the number of a process is an identifier.
