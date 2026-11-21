# Libraries that this project examined

This document records the evidence for the choices of [the
architecture](architecture.md) and of [the decision records](adr/README.md).
Each download count is the count for 90 days, from crates.io, on 26 August
2026.

---

## Terminal user interface library: ratatui

| crate | latest version | release date | downloads in 90 days |
|---|---|---|---|
| **ratatui** | 0.30.2 | 2026-06-19 | 16,749,238 |
| cursive | 0.21.1 | 2024-08-03 | 433,409 |
| iocraft | 0.8.5 | 2026-08-20 | 51,728 |
| tuie | 0.2.4 | 2026-06-25 | 444 |

The choice is clear, and the reason belongs to this project. ratatui is the only
Rust library of this type that supplies charts. It has a `Chart` type, a
`Dataset` type, an `Axis` type, a `Sparkline`, a `BarChart`, and a `Canvas` that
can draw inside one terminal cell. Every other library supplies boxes and text,
and a project must then write its own charts.

- **cursive** supplies no chart, and it made no release in two years.
- **iocraft** has a pleasant design, but it writes text to a command line. It
  supplies no chart, and it cannot draw inside one terminal cell.
- **tuie** appears in several lists of good libraries for 2026, and it offers a
  flexible layout, pictures, and a window outside the terminal. At 444 downloads
  in 90 days it is the work of one person. It is not a base for a project.

### What version 0.30 added for this project

Version 0.30.0 arrived on 26 December 2025. It is the largest release in the
history of the project, and it added the parts that this project needs.

- `GraphType::Area` and `Dataset::fill_to_y(f64)` fill the area below a line.
- The markers `Octant`, `Sextant`, `Quadrant`, and `Custom(char)` join
  `Braille`. An octant character divides a cell in the same way as a braille
  character, but no space separates one cell from the next. That space makes a
  chart of braille characters look rougher.
- A braille character now draws above a block character. A chart can therefore
  put one series above another, and it can put text above a chart.
- The project divided itself into `ratatui-core`, `ratatui-widgets`, and
  `ratatui-macros`, and it moved each terminal backend into its own crate.
  **This change is the reason why a program can hold a thunderbox panel.**
- The `Canvas` type now rounds a position correctly. It also cuts a line that
  starts outside the visible area, and it accepts a grid that is larger than
  `u16::MAX`.

### Which part of ratatui each panel uses

The project built the five panels of version 1 against ratatui alone. No panel
needed another library, and no panel needed a copy of a ratatui file.

| panel | the part that it uses | the part that it draws itself |
|---|---|---|
| TimeSeries | `Chart`, with `GraphType::Area` and `Dataset::fill_to_y` | nothing |
| Stat | `Sparkline`, for the small chart under the number | the large digits, as a table of block characters |
| Gauge | `Gauge`, for the bar and its label | the shade that marks the whole range |
| BarGauge | nothing | the bar, from the characters that divide a cell into eighths |
| Table | `Table`, with `Row` and `Cell` | nothing |

Two notes come from that work.

**`Gauge::ratio` stops the program for an argument outside 0.0 to 1.0.** A
value that a data source returns can sit outside the range of a meter. A data
source can also return a value that is not a number. The gauge panel therefore
computes the ratio itself and clamps it. Read `ratio_of` in
`thunderbox-widgets/src/gauge.rs`.

**A `BarChart` does not fit a bar gauge.** `BarChart` draws a bar for each
column of a group, and it scales every bar against the largest value. A bar
gauge scales each bar against a range that a person states, and it draws one
horizontal bar for each series. The panel therefore draws its own bars, and it
uses a part of a cell to separate two values that lie close together.

### What ratatui cannot do

- A `Dataset` holds one `Style` for a whole series. A point therefore cannot
  have its own colour, and a gradient needs a `Canvas` or a second pass over the
  `Buffer`.
- Unicode added the octant and sextant characters recently, and the ratatui
  documentation warns that a terminal may show a replacement character in their
  place. Braille stays the default value.
- A chart can need four parts that `Chart` does not hold:
  - a time axis that ends at the right;
  - a type for the bound of an axis;
  - a legend in a chosen place;
  - a logarithmic axis.

  A program that needs them writes its own chart.
- The features `unstable-rendered-line-info` and `layout-cache` reach past the
  stable interface, and serious work on a chart can need them.

### Chart crates that this project rejected

| crate | latest version | downloads in 90 days |
|---|---|---|
| tui-piechart | 1.0.2 | 4,439 |
| chandelier (charts for money) | 0.4.0 | 210 |
| ratatui-plt | 0.0.2 | 179 |
| malevich | 1.17.0 | 596 in total. Its first release was in August 2026. |
| ratatui-stacked-bar, waveformchart, trazo, tuika-charts | — | 20 to 110 |

None of these is worth a dependency here. `malevich` deserves attention later.
It draws a box plot, a violin plot, a density, and a two-dimensional histogram.
It holds real statistics code, it works with `ratatui-core`, and the
awesome-ratatui list already holds it. It is also a few weeks old.

Note that `ratatui-chart`, `ratatui-barchart`, `ratatui-sparkline`, and
`ratatui-treemap` all sit at version `0.0.0`. A person reserved each name. None
of them holds any code.

### How to draw a chart in true pixels

`ratatui-image` 11.0.6 arrived on 25 June 2026 and has 318,349 downloads in 90
days. It draws a picture with the sixel format, the Kitty format, or the iTerm2
format. It asks the terminal which format it supports, and it falls back to half
block characters. Together with `plotters` it draws a chart in true pixels on
Kitty, WezTerm, and Ghostty. That is the only way past the limit of 8 sub-cells
in one terminal cell. It is also a second path through the code that draws, so this
work comes later.

---

## SSH server: russh

| crate | latest version | release date | downloads in 90 days |
|---|---|---|---|
| **russh** | 0.63.1 | 2026-08-23 | 2,480,749 |
| thrussh (the crate that came before) | 0.48.0 | 2026-08-21 | 6,563 |

`russh` is an SSH client and an SSH server, it is written in Rust alone, and it
runs on tokio. The `russh-keys` crate joined `russh` as `russh::keys` at version
0.50. The repository holds two examples that draw with ratatui:
`russh/examples/ratatui_app.rs` gives one dashboard to each person, and
`ratatui_shared_app.rs` gives one dashboard to everybody.

The method is simple. A ratatui backend writes to any object that implements
`io::Write`. The program supplies an object that collects the bytes and sends
them into the SSH channel. It sends one message for each picture, and not one
message for each control sequence. The size arrives in `pty_request` and in
`window_change_request`, and the program calls `terminal.resize()`. The viewport
is `Viewport::Fixed`, because no real terminal exists to ask.

**The parts that need care:**

- `crossterm::event::read()` reads standard input, and it is of no use here. The
  `data()` method delivers raw bytes. Read them with
  `termwiz::input::InputParser`. The example matches whole bytes, as in
  `match data { b"q" => ... }`, and that method fails at the first arrow key.
- The terminal of the person who connects holds the terminal mode. The program
  writes the control sequences for the second screen, for the cursor, and for
  the mouse into the channel itself.
- Turn on `unstable-backend-writer` in ratatui.
- Answer `shell_request`. The example does not, and a strict client waits for
  that answer.
- Keep one host key in a file. `PrivateKey::random()` makes a new key at each
  start, and every client then reports that the host key changed.

**A program that people use today:** `rebels-in-the-sky` sets
`ratatui = "=0.30.2"` with `unstable-backend-writer`, and it puts its SSH code
in `frittura-ssh-core`, which uses russh, crossterm, and tokio. The same person
wrote the ratatui examples in the russh repository, so the example is the method
that the program uses.

**The wrapper crates that this project rejected.** Each is the work of one
person, and few people use any of them: frittura-ssh-core 326, chai-framework
212, charmed-wish 79, sshui 74, sshdance 19, tuihost 14. The russh example is
about 270 lines, and that number includes its demonstration program. Copy it.

**The other method:** OpenSSH with `ForceCommand` supplies these parts:

- the key management;
- the password check;
- the record of each login;
- the limit on how often a person can try a password;
- PAM, which is the system that Linux uses to check a password.

The program then stays an ordinary terminal program. Choose russh in one of
these conditions:

- one process must serve many people from shared state;
- thunderbox must check the password itself;
- the project must ship one file.

A dashboard meets those conditions.

---

## Data sources

| crate | latest version | release date | downloads in 90 days |
|---|---|---|---|
| prometheus-http-query | 0.9.0 | 2026-05-09 | 751,285 |
| promql-parser | 0.10.0 | 2026-06-08 | 223,363 |
| influx_db_client | 0.7.0 | 2026-06-08 | 44,431 |
| influxdb3-client | 0.2.0 | 2026-06-16 | 1,111 |

---

## Storage: examined, then dropped

This project examined these crates while its aim was still a store for
measurements. This section records the result, so that nobody repeats the work.

`redb` 4.2.0 has 4,307,664 downloads in 90 days and is written in Rust alone.
That last point matters for a program that holds a thunderbox panel. Both
`rusqlite` with its bundled option and `duckdb` add a C compiler or a C++
compiler to the build of every program below them. `fjall` 3.1.9 has 437,264
downloads, and it suits a load of many writes. `pco` 1.0.3 has 714,194 downloads and makes a
column of numbers smaller. `sled` made its last release in 2024, so avoid it.

A dashboard platform stores no measurement. What remains is a cache of limited
size in memory, and the dashboard documents on disk.

---

## How to reduce the number of points

`minmaxlttb` 0.1.4 has 1,471 downloads in 90 days. It first selects the smallest
and the largest value in each group, and it then runs the LTTB algorithm. It is
much faster than `lttb` 0.2.0 at the sizes that matter here. That older crate
made its last release in 2020, and it has 10,595 downloads in 90 days. It is
small enough to copy.

No code of thunderbox reduces the number of points that a data source returns, and
`Cargo.lock` holds no crate that reduces a series. Every read of the field
`SourceDescriptor::downsamples` sits inside a test module.
[ADR 0062](adr/0062-downsamples-promises-a-step-that-no-code-takes.md) records
that absence. The design that takes the
reduction uses this code only when a data source cannot reduce the points
itself. The data source does that work first.
