# The findings that no other file records

A sweep of this repository read the code and every document, and it looked for
facts that no file states. Fourteen readers proposed 103 candidates, and 84 of
those passed
two readers who tried to break them. The first of the two tried to prove that a
document already held the candidate. The second tried to prove the candidate
untrue, or too small to record. About fifteen of the 84 then reached a file of
this repository. **This file holds the rest.**

Each item carries a `file:line` citation. An item that a reader measured also
carries the measurement. The number of each item, in the form `N.N`, points back
at the synthesis of that sweep. Those numbers are not in the order of this file.

Section 6.2 of `CLAUDE.md` asks that a reason of this kind reaches this
repository, and not the description of a pull request alone. This file answers
that rule for the sweep above.

## How to read an item

Each item carries one of three marks:

| mark | meaning |
|---|---|
| defect | The code is wrong, and a person can reach the fault. |
| doc wrong | A file states something that the code contradicts. |
| doc silent | The item is true, it carries weight, and no file states it. |

Inside each part, the first item is the one most likely to cost a reader time.

## What this file is not

No item here is a decision of the project. Section 12 of `docs/ARCHITECTURE.md`
holds the decisions. An item of this file becomes a decision, a repair, or a
line of section 13, and it then leaves this file.

**Read each item as a hypothesis until you check the code.** Two readers
verified each item, and a reader can still be wrong. A citation can also age,
because the line numbers name an earlier tree of this code. Search for the
named item when a line number does not hold it.

## The items that reached a file already

Do not record these again. `CLAUDE.md` now holds nine of them:

- the four lints for documentation coverage;
- the standard of section 1 as a gate of the build;
- the list of commands that check the code;
- the count of 22 doctests;
- `clap`;
- the licence;
- the sizes of the pictures;
- the scene `too_small` in the table of scenes;
- the second `dropped_text`.

`docs/ARCHITECTURE.md` now holds the four rules that a program which embeds
thunderbox must keep, the corrected `Snapshot::is_loading`, and the items 5 to 7
of section 13. Those three items are the licence, the
overflow of `refresh`, and the field `downsamples` that no code reads.

The code now holds two more duties of a data source. It also holds the three
helpers that use `unsafe`, and a comment on the two copies of the list of the
keys. Item 6.3 below holds the facts about that list that the comment does not
state.

---

# The defects that a person can reach

## 4.7 The first command of README.md panics when a program reads the output

**defect**

`main.rs:156-160` states this promise: "It sends no control sequence to a
program that reads the output of this one".
`crates/thunderbox/tests/cli.rs:19-24` and `:44-48` hold that promise. Each of the
two tests names a dashboard that fails to load, so each one ends before
`main.rs:174`.

The measurement, with a dashboard that loads and with no terminal:
`./target/debug/thunderbox > out.bin` writes **8 bytes** into `out.bin`. Those
bytes are `033 [ ? 1 0 4 9 l`, which is the sequence that leaves the alternate
screen. The program writes a panic on the error output, and it returns 101.

Those bytes come from the hook of the panic, and not from a start that
succeeded. `try_init` installs the hook first, at `src/init.rs:397-402` of
ratatui 0.30.2. `enable_raw_mode()` then returns the error `ENXIO`, the `expect`
at `:366` panics, and `restore()` of the hook writes the sequence. The sentence
that a person reads is the sentence of ratatui, and section 1 of `CLAUDE.md`
governs no sentence of another project.

No code of this project asks whether the output is a terminal. A search for
`is_terminal`, `IsTty`, `isatty` and `atty` over `crates/` and over `scripts/`
finds nothing. No document names a code that the program returns.
`ratatui::try_init()` exists, and it returns an error in place of a panic.

## 4.3 `validate` reads no field of the time range

**defect**

Item 6 of section 13 of `docs/ARCHITECTURE.md` names this fault, and it gives
no detail. The detail below reaches no file.

`MIN_SPAN_MS` and `MAX_SPAN_MS` sit at `crates/thunderbox-core/src/time.rs:15` and
`:22`. One place reads them, which is `TimeRange::with_span_ms` at `:222`. The
keys `+` and `-` reach that method. The `[time]` table of a dashboard file
reaches neither value. `validate`, at `dashboard.rs:89-137`, reads `title`,
`refresh`, `panels` and `sources`, and it reads `time` nowhere.

A dashboard of `from = "now-9223372036854775807ms"` loads with no error, and
`QueryRequest::new` gives every data source a `span_ms()` of `i64::MAX`. The
measurement: the task of the testdata panel panicked at
`crates/thunderbox-source-testdata/src/lib.rs:200:33`, with the words `attempt to
multiply with overflow`.

The doc comment of `MAX_SPAN_MS` states that "the arithmetic stays correct".
That sentence holds for the path of the keys alone. The message that a person
reads names a data source, and it never names the dashboard.

## 5.1 The stat panel cuts a number, and three other panels forbid that

**defect**

A value that does not fit its tile reaches plain text at `stat.rs:511-514`.
`draw_centred`, at `stat.rs:421-425`, then calls the shared `clip`, which writes
a full stop at the end of the text (`panel.rs:331-352`). A reader takes that
full stop for a decimal point.

Two measurements. The value 999999, with `unit = "bytes"` and `decimals = 2`, in
an ordinary panel of 38 cells by 8 rows that holds eight series: the tile draws
`1000.00.` where the value is `1000.00KB`. At 8 cells, the value 1234.5678 with
four decimals: the tile draws `1.234.` where the value is `1.2346K`, which is
wrong by a factor of 1000.

Such a tile is not rare. `MIN_TILE_WIDTH` is 8, so a stat panel of several
series lays out tiles of 7 usable cells at every size of the panel.

Three other panels state the opposite rule, and each one holds a test.
`gauge.rs:244-246` reads "A number that lost its last digits would mislead a
reader, so the panel draws no number at all". Its test sits at `:830`.
`bargauge.rs:126-127` and `table.rs:397-401` hold the same rule, and the test of
the table sits at `:957`. No test of `stat.rs` names a number that the panel
cut, and no document states the exception.

## 5.2 The formatter picks the letter of the scale before it rounds

**defect**

`si` divides while `n >= 1000.0`, and it then applies the count of the decimals
(`crates/thunderbox-widgets/src/value.rs:218-231`). `places` rounds after that
step. A value that rounds up across a step of the scale therefore keeps the
smaller letter, and it gains a digit.

The measurements: 999.9 with `bytes` draws `1000B`; 999999 with `bytes` and
`decimals = 2` draws `1000.00KB`; 999999 with no count of decimals draws
`1000KB`; 999.6 with no unit and `decimals = 0` draws `1000`.

Every panel plans a width from that text. The widest text is therefore four
digits and the unit, and not three digits and the unit. The test at
`value.rs:266-271` holds the values 999.0, 1500.0, 2000000.0 and 3000000000.0,
and no value of that test crosses a step.

## 2.6 One panel alone panics on an area outside the buffer

**defect**

`crates/thunderbox-widgets/src/timeseries.rs:424-427` reads
`draws_a_panel(area.as_size())`, and it then gives the area of the caller
straight to the `Chart` of ratatui at `:500`. That value reads the buffer with
no check.

Three other panels hold a guard. `GaugePanel` at `gauge.rs:459-461` and
`BarGaugePanel` at `bargauge.rs:279-281` each write
`let area = area.intersection(buf.area);`, beside a comment that reads "A cell
outside the buffer would stop the program". `StatPanel` guards inside `put`, at
`stat.rs:406-417`. `TablePanel` holds no such line, and it survives all the
same.

The measurements, against a buffer of 40 cells by 10 rows: the area
`Rect::new(20,5,40,10)` panics with `index outside of buffer ... index is (24,
12)`. A buffer at `Rect::new(10,4,40,10)`, with an area at the origin, panics at
`(0,0)`.

The crate states no rule about the area against `buf.area`. The crate comment
at `crates/thunderbox-widgets/src/lib.rs:3` names the crate as the smaller of the
two ways to put thunderbox inside another program.

## 3.4 The testdata source answers a variable, and its descriptor denies it

**defect**

`crates/thunderbox-source-testdata/src/lib.rs:270-272` returns one choice for each
series. The descriptor at `:242` reads
`SourceDescriptor::new("testdata", "Test data").downsampling()`, and it holds no
call to `.with_label_values()`. `supports_label_values` therefore stays `false`,
and `crates/thunderbox-dashboard/src/query.rs:361-366` refuses before it calls the
source. A person then reads `The data source "test" returns no choice for a
variable.` The two other sources hold that call, at
`prometheus/src/lib.rs:285` and at `sysinfo/src/lib.rs:140`.

testdata is the source that this project names for a person who holds no server.
`dashboards/variables.toml` works today, because it names `options = [...]`. The
fault arrives when a person copies the form `query = {...}` of `README.md` onto
that file. One call at `testdata/src/lib.rs:242` repairs it.

## 2.8 The example of a panel builds a time column in the wrong unit

**defect**

`thunderbox_core::Timestamp` is "A count of milliseconds after the Unix epoch"
(`crates/thunderbox-core/src/time.rs:7`).
`crates/thunderbox-widgets/examples/panel_embed.rs:40` builds
`self.started.elapsed().as_millis() as i64`, and it gives that value to
`Field::time` at `:57`.

The example draws a correct picture for one reason alone. It never calls
`TimeSeriesPanel::window`, and `timeseries.rs:176-182` then reads the present
moment from the newest point. A window makes the chart empty, because a window
holds about 1.7 times 10 to the power 12, and each point holds less than 10 to
the power 6. This example is the one worked example of the way that holds one
panel.

## 2.5 The example of a dashboard does not stop on `Ctrl+C`

**defect**

thunderbox stops on `Ctrl+C` because `main.rs:390` reads that pair of keys itself.
The program holds the terminal in raw mode, so the terminal raises no signal.
`main.rs:305` records that the program keeps `q`, `Esc`, `Ctrl+C` and `r`.

`full_embed.rs:190-198` reads `q`, `Esc` and `r`, and it holds no arm for a
modifier at all. `README.md:494-497` names that example as the way to hold a
whole dashboard.

One more fact reaches no file. `Ctrl+Q` also stops the program, because
`main.rs:389` reads `Char('q')` and tests no modifier.

## 6.6 One back quote can turn the lint script off for a whole file

**defect**

Section 1.6 of `CLAUDE.md` now holds the scope of the script, which is a file
that ends in `.rs` and a file that ends in `.md`. Two lines of the script hold
that scope. The test of the suffix at `scripts/ste-lint.py:188` sits inside
`files_under`. The branch at `:99-107` of `prose_lines` ends in a bare
`else: continue` at `:106-107`, and that line drops every line that is not a
comment.
The text that opens the script states the scope at `:14-16` and `:20`, and the
text that opens `files_under` states it again at `:180`. The line `:114` drops a
title line and a table row of a Markdown file. The two facts below reach no
file.

**A back quote that no second back quote closes turns the checks off.**
`strip_spans`, at `scripts/ste-lint.py:70-85`, changes `in_span` at every back
quote, and it drops each character while that value holds. `prose_lines`, at
`:97-122`, carries that value from one line to the next. It resets the value at
no point: not at a line that holds no comment, not at the end of a block of
comments, and not at the end of a fence.

The measurement: a file whose first line holds one stray back quote reports zero
faults. The same file, without that back quote, reports three. The self test at
`:221-235` holds a span that closes alone, so the workflow cannot see the hole.
A run that reports nothing looks like a clean run, so no reader looks again.
No file of the tree ends inside a span today, so this fault waits.

**One fault of rule 3 stands in the tree today.**
`crates/thunderbox-dashboard/src/runtime.rs:1187` reads
`async fn taking_an_area_leaves_the_wait_of_that_area_open()`. The word
`taking` is an `-ing` form. Line 64 of `CLAUDE.md` covers the name of a test,
and a run over the whole project reports zero faults.

---

# The documents that state something the code contradicts

## 1.4 The Prometheus crate states that the build needs no C library

**doc wrong**

`crates/thunderbox-source-prometheus/Cargo.toml:14` reads: "The TLS library is
rustls. A build of thunderbox then needs no C library."

The default provider of rustls 0.23 is `aws-lc-rs`, which holds `aws-lc-sys`,
which builds C source with `cc` and with `cmake`. The path is thunderbox, with the
default feature `prometheus`, then `thunderbox-source-prometheus`, then `reqwest`,
then rustls 0.23.43, then `aws-lc-rs` 1.18.0, then `aws-lc-sys` 0.44.0.
`Cargo.lock` names `cc`, `cmake` and `pkg-config` as the build dependencies of
that last crate. `cargo run -p thunderbox`, which is the first command of
`README.md`, therefore needs a C compiler and `cmake` on the machine.

One line of this repository states that fact, and it is a comment about the time
limit of a job. `checks.yml:47-49` reads "the prometheus feature pulls in a
dependency that builds C source". `docs/RESEARCH.md:204-206` rejects `rusqlite`
and `duckdb` for exactly that reason. It states that each one would "add a C
compiler or a C++ compiler to the build of every program below them". The
default build already breaks that rule. No file joins the two statements, and no
file names the tool that a person needs on a new machine.

## 3.7 The limit of an answer binds one target, and not one panel

**doc wrong**

`run_panel` reads the targets in a plain loop, and the time limit sits inside
that loop (`crates/thunderbox-dashboard/src/query.rs:281`, `:291`). A panel of N
targets can therefore hold a whole set of queries for the sum of the limits of
its targets. It can also pull N times the budget of points.

Two documents deny that. `query.rs:128-129` reads "One set of queries can
therefore take the longest of the limits of its panels", and
`docs/ARCHITECTURE.md:645-646` holds the same words. Each one is correct for the
example that it names, which is a data source that never answers. `run_panel`
returns at the first limit that passes.

The measurement, against a data source that answers correctly in 150
milliseconds: one panel of three targets took 453.7 milliseconds. The same three
targets, across three panels, took 151.8 milliseconds.

`README.md:428` names the time limit of the Prometheus source, and it drops the
same multiplier. The default dashboard already holds such a panel, which
is the network panel of `crates/thunderbox/src/default.toml` and its two targets.

The panels do run at the same time (`query.rs:167-190`), and `query.rs:250-253`
does state that the limit binds one target. The arithmetic of the whole set is
the part that no file holds. `docs/ARCHITECTURE.md:636-654` argues at length
that the project needs no limit for a whole set. That argument rests on a bound
that the code does not hold.

## 3.5 The sysinfo store holds a floor of 250 milliseconds, and its swap signal can be empty

**doc wrong**, and **defect**

Two facts, and no file holds either one.

**`MIN_SAMPLE_GAP` is a floor of 250 milliseconds** (`lib.rs:65`). Its doc
comment, at `:60-64`, presents that value as a way to take one sample for the
panels of one refresh. It does not present the value as a floor of the
resolution. A dashboard of `refresh = "50ms"` loads, because `ms` is a unit that
`time.rs:327-328` permits. The measurement: 24 queries, 50 milliseconds apart,
across 1200 milliseconds, produce 5 samples.

A second fact belongs to a person who writes a test. `sample_if_due` measures
the distance on the clock of the machine (`lib.rs:120-129`, through
`at.elapsed()` and `Instant::now()`), while every other moment comes from
`req.now`. A test that moves `req.now` forward therefore takes exactly one
sample. The three tests of that file call
`std::thread::sleep(MIN_SAMPLE_GAP)` (`lib.rs:251`, `:337`, `:480`), and no
comment states why.

**The signal `swap` holds no series on a machine that reports no swap.**
`crates/thunderbox-source-sysinfo/src/sample.rs:229-233` pushes the sample only
while `total_swap > 0.0`. That is the ordinary state of a container. The panel
then reads no data for ever. The source returns a frame that holds nothing, and
it returns no error. `README.md:375` names the signal, and it states no
condition. The first sample of `network.rx` and of `network.tx` holds nothing
for a different reason, and `sample.rs:238-244` states that one.

## 1.9 Two sections of ARCHITECTURE.md describe things that do not exist

**doc wrong**

Four statements of sections 3 and 4 name a thing that the code does not hold,
and each one stands in the present tense. Section 8, at `:339-342`, correctly
marks `Heatmap`, `Logs` and `thunderbox-transform` as later work, so those three
are not at fault.

- `:137` reads "Each way to run thunderbox is a subcommand behind a feature".
  `crates/thunderbox/src/main.rs:113-124` reads `-h`, `--help`, `-V`, `--version`
  and one path. `--help` writes `Usage: thunderbox [FILE]`.
- `:163` holds
  `ratatui = { version = "0.30.2", features = ["crossterm", "layout-cache", "unstable-backend-writer"] }`.
  The real `Cargo.toml:38` sets `default-features = false`, and it turns
  `unstable-backend-writer` off. A reader can copy that block, because the file
  holds it as TOML.
- `:164` names `crossterm = "0.29"` as a dependency of the top level. No crate
  of this workspace names `crossterm` itself.
- The map of the crates of section 3 marks no crate as a plan.

## 1.7 README.md states that the recorder starts one program for each core

**doc wrong**

`README.md:592` holds that sentence. The script reads no count of the cores. It
holds no call to `os.cpu_count`, to `nproc`, or to `sched_getaffinity`. The
number is the value 4, which `scripts/record-demo.py` writes at `:192`, at
`:265` and at `:273`, and reads at `:470`. The script offers no option
`--load N`. The parser at `:449-459` offers `--cols`, `--rows`, `--program` and
`--no-load` alone.

Two results follow. On a machine of 16 cores the panel of the processor reaches
about 25 percent, and the picture then looks still. The load also runs for the
scenes `stack_none` and `stack_column`, and the sentence names neither. The cost
in memory is four programs of `BLOCK_BYTES` each. That is 500000000 bytes for
each program, and 2000000000 bytes in total. `record-demo.py:341-343` does state
that cost.

## 1.8 Two panels hold a copy of `clip`, and each states a false reason

**doc wrong**

`crates/thunderbox-widgets/src/panel.rs:322-323` correctly states that `clip`
"counts cells, and it does not count characters".
`crates/thunderbox-widgets/src/bargauge.rs:222-223` states that "the clip function
of the panel module counts characters".
`crates/thunderbox-widgets/src/table.rs:140-143` states that "The shared function
clip counts one character as one cell." Both of those sentences are wrong. Each
one is also the only stated reason that its copy exists.

The three functions agree on ordinary text. A reader compared them at each width
from 0 to 13. The inputs were text of the ASCII set, the wide text `中央処理装置`,
mixed text, and empty text.
Both copies can therefore go. `table.rs:144` already calls the shared `clip` for
the ordinary case. `bargauge.rs:229` holds a whole implementation of its own,
and its own `cells_of` scores a control character of the ASCII set as 0 cells.
`panel::cells_of` reads `unicode-width` for that character, so a control
character is the one input where the two can differ.
`docs/ARCHITECTURE.md:244-247` forbids exactly this duplication.

## 1.10 Two files overstate how often the sysinfo source takes a sample

**doc wrong**

`README.md:405-409` and `crates/thunderbox-source-sysinfo/src/lib.rs:15-17` each
state that "the distance between two samples equals the refresh interval of the
dashboard". `MIN_SAMPLE_GAP` is a floor of 250 milliseconds (`lib.rs:65`), and a
dashboard of `refresh = "50ms"` loads. Item 3.5 above holds the measurement.

---

# The rules of a panel and of a program that embeds thunderbox

## 2.4 `ratatui::init()` installs a hook for a panic, and that hook reaches the whole program

**doc silent**

`docs/ARCHITECTURE.md:501-502` states that `thunderbox-dashboard` "installs no
logger, no panic handler, and no signal handler". `query.rs:202-203` states that
"A panic inside one job therefore stops no other job." Both hold for the
library.

The program breaks that chain. `crates/thunderbox/src/main.rs:174` calls
`ratatui::init()`, which calls `set_panic_hook()` at `src/init.rs:398` of
ratatui 0.30.2. The body of that hook is `restore(); hook(info);` (`:566-573`),
and `restore()` turns raw mode off and leaves the alternate screen
(`:524-530`).

A panic inside a task of a query runs that hook before tokio catches the unwind.
A reader measured that with a hook that counts and a `JoinSet` task that panics.
The join returned an error, so the loop went on, and the counter of the hook
read 1. The terminal therefore leaves raw mode while `draw_loop` still draws.

No document states that the program installs such a hook. `DataSource` is the
extension point that this project offers, and the runtime survives a data source
that panics. A person who writes a new data source will test that rule, and will
then take the wrong lesson.

## 2.7 `measure_label_width` reads the panel that a caller finished

**doc silent**

`TimeSeriesPanel::measure_label_width` reads `self.options` and `self.window`
through `self.plot()` (`timeseries.rs:144-151`, `:168-183`).

The measurements, over the same frames, which hold the values 0 to 95. A panel
that holds no option reports **3**. The same panel, with `unit = "bytes/s"` and
`decimals = 2`, reports **8**. The same panel, with `max = 1000000.0`, reports
**4**.

The example at `timeseries.rs:118-127` is correct, and it teaches the wrong
shape. It measures, and it then writes `let top = top.label_width(width)` again.
A caller who adds `.options(...)` or `.window(...)` at that point takes an error
of five cells. That error is the fault that this feature removes, and it arrives
with no error of the compiler and no test that fails. `README.md:186` names the
method, and it states no order. `crates/thunderbox-dashboard/src/view.rs:229-255`
is correct for one reason alone: it builds the whole panel in one expression.

## 5.4 The marker reaches two panels of five, and no setting reaches the border

**doc silent**

`pub fn marker` sits on `TimeSeriesPanel` (`timeseries.rs:64`) and on
`StatPanel` (`stat.rs:129`), and on no other panel. `marker.rs:1-31` does state
that a marker belongs to a chart. The result for `Marker::Block` reaches no
file, and `README.md:514` names that marker "For a plain terminal".

- Every panel draws `Block::bordered().border_type(BorderType::Rounded)`
  (`panel.rs:241-243`), and `panel_block` is `pub(crate)`. No field of a
  dashboard file and no theme reaches it. `Theme` (`theme.rs:10-31`) holds styles and colours alone.
- The large digits of the stat panel come from the table `FONT`, which holds
  characters of the block kind (`stat.rs:64-77`). The bar gauge holds `EIGHTHS`
  and `TRACK` (`bargauge.rs:20`, `:27`). The gauge holds two characters of its
  own, which are `FULL`, `█`, and `TRACK`, `░` (`gauge.rs:514`, `:517`).

A person whose font holds none of those characters can press `m` to the end of
the cycle. That person still reads a screen of boxes. The example of the
crate shows that difference: `panel_embed.rs:90` draws the block of the host
with `Block::bordered()`, which is the plain border, beside two panels of
thunderbox that draw a round border.

Two smaller facts reach no file. `Marker::from_str` accepts `tty` as a second
name for `block` (`marker.rs:74`), and no document lists that name.
`docs/ARCHITECTURE.md:190-194` drops `halfblock`, which `README.md:513` lists.

## 5.3 Each name of a unit holds a rule, and an unknown name takes another rule

**doc silent**

`README.md:137` states that "The formatter knows `percent`, `bytes`, `bytes/s`,
and `seconds`." `value.rs:44-51` holds three rules that no file states.

- **`percent` multiplies by nothing, and it takes 0 decimals.** A value must
  already run from 0 to 100. A ratio of 0.42 from Prometheus therefore draws
  `0%`. `value.rs:214-217` does state the rule that divides by 1000 for
  `bytes`, and the doctest at `value.rs:38` does show the behaviour of
  `percent`. Neither states the requirement.
- **`seconds` changes no scale, and it takes 2 decimals.** `places` then drops a
  zero at the end (`value.rs:241-253`). The value 0.002 draws `0s`, the value
  0.0001 draws `0s`, and the value 86400 draws `86400s`.
- **A name that this list does not hold takes `si`, with no letter of a unit**
  (`value.rs:49`). A panel of `unit = "furlongs"` therefore draws `1.5K`.

Each of the three looks like a data source that answers wrongly.
`dashboards/prometheus.toml` and `README.md:436-445` make the first two easy to
reach, and they do so by design.

## 2.2 The guard of `set_marker` carries every frame, and no test holds it

**doc silent**

The doc comment of `set_marker` now states the guard. The two results below
reach no file.

`crates/thunderbox-dashboard/src/runtime.rs:264-269` holds this method:

```rust
pub fn set_marker(&mut self, marker: Marker) {
    if self.marker != marker { self.marker = marker; self.refresh(); }
}
```

Both programs that embed thunderbox call `dash.set_marker(view.marker())` on
**every frame** (`crates/thunderbox/src/main.rs:380`, `full_embed.rs:180`). A
reader who takes the guard out reads that step as a simplification. The
dashboard then stops and starts its queries about ten times each second, and it
finishes no refresh. The whole suite stays green, at 740 tests.

The measurement: with no guard, 10 frames across 200 milliseconds started 10
queries, and the panel never left the state that holds no answer. With the
guard, the same drive started 1 query, and the panel answered.

Three tests call `set_marker` (`tests/runtime.rs:692`, `:1114`, `:1616`), and
each one calls it once with a value that differs. None of the three therefore
holds the guard.

The doc comment at `runtime.rs:257-263` does not state that a caller may call
this method on every frame. It also does not state that the guard is the reason
that such a caller is safe. The doc comment of `set_time_range`, at
`runtime.rs:494-498`, does state that hazard for its own method.

Two more facts reach no file.

- **The key `m` buys nothing on two steps of four.** The marker reaches a query
  through `Marker::horizontal_subcells()` alone
  (`crates/thunderbox-widgets/src/marker.rs:39-44`). That method returns 2 for
  Braille and for Octant, and 1 for HalfBlock and for Block. The cycle of
  `view.rs:136-144` runs Braille, Octant, HalfBlock, Block. Two steps of the
  four therefore ask every data source again, for the same budget of points.
  `set_panel_area` guards against that, at `runtime.rs:328`.
- **A key `m` that repeats starts one set of queries for each repeat, and it
  finishes none.** The measurement: 50 repeats, 30 milliseconds apart, against a
  data source that answers in 60 milliseconds, started 51 queries and finished
  0. `docs/ARCHITECTURE.md:1261-1264` records that hazard for `time_key` alone.

---

# The behaviour of a dashboard file that surprises a reader

## 4.4 A `refresh` of `"0s"` means that the dashboard never refreshes

**doc silent**

`DashboardRuntime::new` reads
`u64::try_from(millis).ok().filter(|m| *m > 0).map(Duration::from_millis)`
(`runtime.rs:211-218`). A value of zero therefore becomes `None`. That value is
the value of a dashboard that holds no field `refresh`, and
`docs/ARCHITECTURE.md:642` states that such a dashboard "never ticks for the
clock". `validate` accepts the value.

The comment at `runtime.rs:213` gives the reason for the filter, which is that
"A period of zero would ask the data source without a pause". It states no
result for the person. A person who writes `0s` asks for the fastest refresh.

## 4.5 `thunderbox-core` accepts a `kind` of a source that no crate implements

**doc silent**

A `kind` of a panel that this project does not hold is a fault of the parser. A
`kind` of a source is a plain `String`, and `thunderbox-core` reads it nowhere. A
reader confirmed that: `Dashboard::from_toml` returns `Ok` for
`kind = "no-such-kind-at-all"`. One place checks that field, which is
`sources::build` of the program (`crates/thunderbox/src/sources.rs:29-37`).
`docs/ARCHITECTURE.md:1750-1752` does name that place.

No file states the result for a program that builds a `Registry` of its own.
Such a registry holds a **name** as its key, and not a kind
(`docs/ARCHITECTURE.md:1210-1214`), so that program reads no error at all.
`README.md:344-351` carries the title "What thunderbox rejects", and it names the
variables alone.

The tests of this project rest on that behaviour.
`crates/thunderbox-dashboard/tests/runtime.rs:52-55` names `kind = "test"`, which
no crate implements. The same file names `kind = "testdata"` at `:30-31`, while
it registers `Counter`.

## 4.6 An option that the program does not know becomes the name of a file

**doc silent**

`crates/thunderbox/src/main.rs:113-124` reads every argument for `-h`, `--help`,
`-V` and `--version`. It then takes the first argument as the path, whatever
that argument holds. The measurements, against the program:

- `thunderbox --colour` writes `Error: The program cannot read --colour.`, and it
  returns 1;
- `thunderbox -- my-dashboard.toml` writes `Error: The program cannot read --.`
  `README.md:20-22` teaches `cargo run -p thunderbox -- my-dashboard.toml`. A
  person who installs the program, and who keeps the `--`, therefore meets that
  message;
- `thunderbox a.toml b.toml` reads `a.toml`, and it drops `b.toml` with no word;
- `thunderbox board.toml --help` writes the help, and it draws no dashboard.

The block `Options:` at `main.rs:40-42` names two options. A reader takes that
block for a promise that the program rejects every other option.

## 3.6 The value of a variable reaches the text of a query with no quotation mark

**doc silent**

`Target::to_query` calls `variable::apply`
(`crates/thunderbox-core/src/dashboard.rs:328-333`), which reads
`out.push_str(value)` (`crates/thunderbox-core/src/variable.rs:79-81`). No step
adds a quotation mark, no step adds an escape, and no step rejects a value. The
example of `README.md:307` puts the reference inside a matcher of PromQL that
holds two quotation marks, which is `up{job="$job"}`.

The measurements: the value `a"b` builds `up{job="a"b"}`. The value
`web" or on() vector(1) or up{a=` builds an expression of another shape.

A variable of the kind `query` takes its choices from `DataSource::label_values`,
and the Prometheus source returns the values of the server with no filter
(`prometheus/src/lib.rs:315-327`).

This fault sits inside the language of the query, and it crosses no boundary of
a privilege. The expression still reaches the data source that the target names.
The lesson for a maintainer: **no step between `label_values` and
`DataSource::query` adds a quotation mark.** A data source of a later version,
whose language holds a rule for an escape, must therefore hold that rule itself.
`README.md:303-317` and `docs/ARCHITECTURE.md:1382-1397` each stop at the
grammar of the `$` sign.

---

# The tests, and the seam between this machine and the workflow

## 6.1 The clock of tokio cannot drive the path of the area

**doc silent**

`DashboardRuntime` measures `AREA_QUIET_TIME`, which is 250 milliseconds, and
`AREA_MAX_HOLD`, which is 1000 milliseconds, with **`std::time::Instant`**. Ten lines hold that type. `runtime.rs:5` is the import. The fields sit at
`:122`, `:140` and `:171`. The code writes a value at `:232`, `:242`, `:336`
and `:840`. Two unit tests read the type at `:1191` and `:1226`.

`start_paused = true` moves the clock of tokio alone. `area_changed_at.elapsed()`
therefore never grows, and `tick()` never takes an area that waits. The feature
`test-util` of tokio would not help, because this path reads no clock of tokio.
`crates/thunderbox-dashboard/Cargo.toml:26` holds no such feature today, so
`#[tokio::test(start_paused = true)]` does not compile.

Two results belong to a maintainer.

- **The deterministic tests of the wait sit inside `src/runtime.rs`.** They
  write the private fields `area`, `pending_area`, `area_changed_at` and
  `area_wait_since` themselves (`runtime.rs:1187-1195`, `:1221-1231`).
  `tests/runtime.rs` holds the other tests of the area, and it cannot hold one
  of these. It reads about 30 real pauses in their place.
- **A clock that a test pauses hides a fault.** Each negative test of the area then
  passes while it holds nothing.
  `an_area_that_returns_to_the_area_in_use_starts_no_set_of_queries`
  (`tests/runtime.rs:1131`) reads `!runtime.tick()` in a loop. Under a clock
  that a test paused, that loop passes for ever, and it tests nothing. Section
  4.3 of `CLAUDE.md` exists because this project shipped three tests that passed
  while each held nothing. This is the fourth such fault, and a person who wants
  a faster suite reaches for it first.

## 6.5 Three facts of the suite of tests

**doc silent**

- **No command compiles the build that holds no default feature.**
  `README.md:461` states this promise: "A build without the `prometheus` feature
  reads this machine and the test data alone". Every step of the workflow reads the
  default features or `--all-features` (`checks.yml:81`, `:84`, `:87`, `:89`,
  `:125`). No step reads `--no-default-features`. That build passes today, and
  it holds 35 unit tests in place of 38. Three tests sit behind
  `#[cfg(feature = "prometheus")]`, at `sources.rs:119`, `:148` and `:187`, so
  every run of the promised build drops those three. A run of `--all-features`
  can never find a fault that the absence of a feature makes.
- **Two crates hold a copy of `settle`, and six files hold a copy of one
  helper.** `crates/thunderbox-dashboard/tests/common/mod.rs:16-27` and
  `crates/thunderbox/tests/default_dashboard.rs:23-34` differ by the word `pub`
  alone. One crate cannot read the directory `tests/common` of another crate.
  Six files hold their own `fn text(buf: &Buffer)`. The helpers of the tests sit at
  `tests/runtime.rs:273-570`. `Gate`, at `:527-533`, is the one data source of
  those helpers that lets a test choose the moment when an answer arrives. Such
  a test needs no pause of a fixed length.
- **The count of the panels of `default.toml` binds the tour of the recorder.**
  `scripts/record-demo.py:195-200` sends six presses of `Tab`, and its comments
  name each panel by its place. `crates/thunderbox/tests/default_dashboard.rs`
  reads the panels 0, 3, 4 and 5. A seventh panel therefore reaches one
  assertion alone, which is `crates/thunderbox/src/main.rs:436`. That line reads
  `assert_eq!(dash.panels.len(), 6);` and it carries **no message**. A reader
  meets `left: 7 / right: 6`, and the repair that reads as obvious is a change
  of the 6. No file then states that the tour of the recorder is wrong. Section
  1 of `CLAUDE.md` asks for a message on each `assert_eq!`. A sentence that
  named `scripts/record-demo.py` would have cost nothing here.

---

# The facts of the repository and of its process

**doc silent**, for each item of this part.

- **No tool updates the crates that this workspace depends on, and no tool
  audits them.** `.github/dependabot.yml` names `rust-toolchain` (`:19`) and
  `github-actions` (`:32`), and it names no entry for `cargo`. No command runs
  `cargo audit` or `cargo deny`. The whole chain of TLS sits in a lock file
  that no tool reads again. That chain is `reqwest` 0.13.4, rustls 0.23.43 and
  `aws-lc-sys` 0.44.0, inside a program that reads a network and that accepts a
  password inside an address. `crates/thunderbox-source-sysinfo/Cargo.toml:14-16`
  records an upgrade that waits. It reads "Move to 0.39 when the workspace
  raises its own limit". No tool, no calendar and no document tracks that
  upgrade. The rest of that file gives a reason for each choice, so the absence
  looks like an accident and not a choice.

- **A rule repairs a wrong picture, and no rule repairs a wrong sentence.**
  Section 6 of `CLAUDE.md` obliges a writer to repair a picture that the
  change of that writer did not break. Step 2 of section 1.6 asks about the
  sentences that a change adds or alters, and about no other sentence. It
  never asks about a sentence elsewhere that the change made false. The
  section "The pictures that README.md holds" covers the sentences of
  `README.md` beside a picture that a person recorded again, and nothing more.
  `docs/ARCHITECTURE.md` holds 1826 lines of decisions, and the project has
  twice found a batch of wrong statements in it. One batch held a `Query` type
  of three fields in section 2, and a type `SourceParams` that does not exist.
  Section 2 is the section that a new author of a data source reads first. The
  other batch held seven doc comments and four items that stated a rule that
  the code no longer held. **Treat a
  statement of section 2 and of section 12 of `docs/ARCHITECTURE.md` as a
  hypothesis until you check the code.**

- **The list of the scenes lives in three places, and the text that opens the
  script holds one scene fewer.** `scripts/record-demo.py:18-23` lists six
  scenes, and it omits `variables`. `SCENES` and `README.md:567-575` each hold
  seven. `argparse` takes its choices from `SCENES` (`:450`), so `--help` is
  right while the text above it is wrong. Section 6 of `CLAUDE.md` names
  `SCENES` and two changes of `README.md`, and it never names that text. That
  is why nobody changed the text. One more pair of files that must change
  together reaches no rule. `dashboards/variables.toml` is both the input of a
  test, through `include_str!` at `crates/thunderbox/src/sources.rs:172`, and
  the input of the scene `variables` (`record-demo.py:109-111`, `:243`). The
  comments of the script state that pair (`:106-108`, `:238-240`). The
  question that decides, in section 6 of `CLAUDE.md`, asks about "a person who
  runs the program", and the program with no argument draws `default.toml`. A
  change to a dashboard that this project ships therefore answers "no" to that
  question, and it still makes a recorded picture wrong.

---

# The detail of an item that a file records in its headline alone

A commit of the sweep above carried the headline of each item below into a
file. The detail did not follow it. Each item names the file that holds the
headline, and it then states the facts that no file holds. Those facts are a
measurement, a citation of a line, a result of a mutation, and a consequence
for a reader.

A mutation is a change that takes one rule out of the code. Section 4.3 of
`CLAUDE.md` states the step that a reader takes with it.

## 2.1 No test holds the rule of the async runtime that runs on two threads

**doc silent**

Four places now state that rule. `docs/ARCHITECTURE.md:533-549` states it in
full, with a measurement, and `:685-694` states it again.
`crates/thunderbox-dashboard/src/runtime.rs:186-192` and
`crates/thunderbox-dashboard/src/lib.rs:22-24` repeat it. One of the four names a
file of the code, which is `docs/ARCHITECTURE.md:546-548`. None of the four
names a line. No file states that no
test of this repository holds the rule. The three parts below hold both.

**No test holds the rule.** A search for `new_current_thread` over `crates/`
finds nothing. No test calls `event::poll`, so no test holds a loop that parks
its thread. 92 tests carry a plain `#[tokio::test]`, and each one runs on a
`current_thread` async runtime. 86 of the 92 hold an `.await`. The other six
hold none, and none of the six waits for the answer of a query. 21 tests carry
`#[tokio::test(flavor = "multi_thread", worker_threads = 4)]`, and all 21 sit in
`crates/thunderbox-dashboard/tests/runtime.rs`. One test of that file builds an
async runtime with a `Builder` of tokio, at `:1982`, and it calls
`new_multi_thread` with one worker thread.

**Three tests rest on the flavour that the rule forbids.** Each of the three
states in a comment that `#[tokio::test]` builds "a runtime of one thread", at
`crates/thunderbox-dashboard/tests/runtime.rs:1530-1531`, `:1592-1593`, and
`:2119-2120`. Each comment then states that the task of a set of queries cannot
run before the read below it. The test at `:1595` reads that value at
`:1605` and awaits `settle` at `:1614`. The test at `:2115` reads at `:2121` and
awaits `settle` at `:2126`. The test at `:1526` awaits `settle` at `:1540`
before it reads, at `:1549`. A reader gave every plain `#[tokio::test]` of that file
the flavour `multi_thread`, with 4 worker threads. Three of 15 runs of that
target then failed, and one test failed in each of the three. That test is
`a_dashboard_whose_panel_holds_no_target_reports_its_set_of_queries`, which is
the test of the comment at `:2119-2120`. The flavour that stops the program is
therefore a guarantee that three tests hold. A change of that flavour does not
make the three fail at every run. Each one then passes sometimes, and fails
sometimes.

**The lines of the two loops.** `draw_loop` is the loop that draws, at
`crates/thunderbox/src/main.rs:321-425`. It is an `async fn`, and its body holds
no `.await`. It parks the thread at `:384`, inside
`event::poll(Duration::from_millis(100))`. `run`, at `:161-178`, holds one
`.await`, at `:175`, and that line awaits `draw_loop`. The loop of the example
`full_embed` sits inside `run`, at
`crates/thunderbox-dashboard/examples/full_embed.rs:117-236`, and that function
holds no `.await` either. It polls at `:184` with the same 100 milliseconds. The
doc comment of `AREA_QUIET_TIME`, at
`crates/thunderbox-dashboard/src/runtime.rs:43-44`, states that duration for both
programs.

## 4.1 Two more functions reach the overflow of a duration

**defect**

`docs/ARCHITECTURE.md` names this fault twice. Item 6 of section 13, at
`:1731-1754`, holds the measurement for `refresh = "20000000000w"`, and it names
the `from` field in one sentence. Item 3 of the list of section 12, at
`:1210-1221`, repeats that measurement. An overflow is a result that does not
fit in its type. That file also holds `Ok(n * mult)` and the call from
`Dashboard::validate`. The line numbers, the two calls below that sit outside
`thunderbox-core`, and the smallest number of each unit reach no file.

The program writes `panicked at crates/thunderbox-core/src/time.rs:364:8`, and
then `attempt to multiply with overflow`. Each measurement below reads
`target/debug/thunderbox`. The root `Cargo.toml` declares no `[profile]` table, so
a build for release turns that check off, and `docs/ARCHITECTURE.md:1743-1745`
states that rule. The measurement: `target/release/thunderbox` wrapped the number,
passed `validate`, and panicked inside ratatui 0.30.2 at `init.rs:366`.

**Five units of six reach the fault.** `parse_duration_ms` reads the number into
an `i64` at `crates/thunderbox-core/src/time.rs:346`, and `:364` multiplies it.
The multiplier of `ms` is 1, so that unit cannot reach the fault. The smallest
number that reaches the fault is 9223372036854776 for `s`, and it is
15250284453 for `w`. A dashboard of `refresh = "9223372036854776s"` gives the
same panic and the code 101. A `refresh` of `9223372036854775808ms` does not fit
an `i64`, so `:346` returns an error and the code 1.

**Four functions call `parse_duration_ms`. A dashboard file reaches three of
those calls, and one call cannot reach the fault at all.**

- `Dashboard::validate` calls it at `crates/thunderbox-core/src/dashboard.rs:103`.
  `Dashboard::from_toml` (`dashboard.rs:53-57`) calls `validate` at `:55`, and
  `crates/thunderbox/src/main.rs:145` and `:150` call `from_toml`.
- `parse_relative` calls it at `time.rs:381` and at `:384`, and `RelativeTime`
  reads its TOML text through `parse_relative` (`time.rs:414`). The `from` field
  and the `to` field reach the fault inside `toml::from_str`, at
  `dashboard.rs:54`, before `validate` runs. A `from` of `now-20000000000w` and
  a `to` of `now+20000000000w` each give the same panic.
- `DashboardRuntime::new` reads `refresh` again at
  `crates/thunderbox-dashboard/src/runtime.rs:212`, and that call never reaches
  the fault. `:207` reads the same text through `validate`, so
  `dashboard.rs:103` panics first. A program that embeds thunderbox can build a
  `Dashboard` in code, and it then reaches the fault at `:207`.
- `Settings::from_options` of `crates/thunderbox-source-prometheus/src/lib.rs`
  calls `duration_ms` for `min_step` at `:114` and for `timeout` at `:116`.
  `duration_ms` sits at `:153-155`, and it calls `parse_duration_ms` at `:154`.
  A `min_step` of `20000000000w` gives the same panic, and the report under
  `stack backtrace` names `:154`, `:114:54` and `:258:24`.

No test names a number that does not fit. The two tests of `parse_duration_ms`
at `time.rs:422-436` hold `7d` as their largest value. The test of `validate`
for `refresh`, at `dashboard.rs:675-678`, holds `soon` alone.

## 4.2 No test reads the sentence that a panel shows for a data source that the registry does not hold

**doc silent**

Section 12 of `docs/ARCHITECTURE.md` (`:562-570`) states the rule and its result
on the screen, and it takes that rule as a decision. This item records what that
decision leaves untested. One paragraph of the doc comment of
`Dashboard::validate` (`crates/thunderbox-core/src/dashboard.rs:92-97`) states the
same rule. The `# Errors` list of that comment states it twice more, at `:80-81`
for a panel and at `:87-88` for a variable. The line numbers, the two files that
a reader used, and the tests below reach no file. `docs/ARCHITECTURE.md:567-568`
already states both outcomes in prose.

A guard is the clause that skips the check for a dashboard with no `[source]`
table. Both guards hold `!self.sources.is_empty() &&`, at
`crates/thunderbox-core/src/dashboard.rs:132` for the target of a panel and at
`:205` for the query of a variable. No comment stands at either line.

`DashboardRuntime::new` (`crates/thunderbox-dashboard/src/runtime.rs:206`) calls
`dashboard.validate()` at `:207`, and it checks no name against the registry.
`run_panel` reads the name of a data source only when a set of queries runs. It
calls `sources.get(&target.source)` at
`crates/thunderbox-dashboard/src/query.rs:285`, and it returns `PanelState::Error`
at `:287`. That error is `UnknownSource`
(`crates/thunderbox-core/src/error.rs:34`), and its attribute at `:33` holds `The
registry does not hold the data source {0}.`

`sources::build` (`crates/thunderbox/src/sources.rs:38-44`) reads
`dashboard.sources` alone, so such a dashboard gives the program an empty
registry. The function `run` (`crates/thunderbox/src/main.rs:161-178`) then calls
`ratatui::init()` at `:174`. A reader measured that path with two files. One
wrong name in a file that declares one data source stops the program before it
opens the terminal. The same name reaches `ratatui::init()` in a file that
declares none, and every panel then shows the sentence above.

One test compares against that sentence, and it drives the path of a variable.
That test is `load_choices_names_a_data_source_that_the_registry_does_not_hold`
(`query.rs:1138-1144`). No test compares against that sentence on the path of a
panel.

The same reader deleted both guards, and the whole suite stayed green, at 740
tests. Two tests name that fault, and each one declares `[source.a]`. They are
`validate_returns_an_error_for_an_undeclared_data_source` (`dashboard.rs:650`)
and `validate_returns_an_error_for_a_variable_that_names_an_undeclared_source`
(`:826`), which calls `variable_error` (`:704-709`). The reader then searched
every dashboard text that a test of this workspace reads. That search covered
each file under `dashboards`, `crates/thunderbox/src/default.toml`, and every
dashboard text inside a file of `crates` that ends in `.rs`. No text there names
a data source without a `[source]` table.

## 2.3 A receiver that outlives the runtime reports `refreshing` for ever

**doc silent**

Section 12 of `docs/ARCHITECTURE.md` holds this rule, in the list of the four
rules that a program which embeds thunderbox must hold. Its title is "A
`watch::Receiver` that outlives the runtime reports `refreshing` for ever". A
later commit moved that list, so this item cites the title alone. The code sits
at `crates/thunderbox-dashboard/src/runtime.rs:928` and at `:995`. The rule
states the `true` alone. The four facts below add what no file holds.

The measurement: a reader wrote `let rx = runtime.subscribe();`, and then
`drop(runtime);`. After 500 milliseconds the receiver reported
`is_refreshing()` as `true`, and every panel of its snapshot held
`PanelState::Loading`. Both values stayed the same for the rest of the process.
The rule names neither the 500 milliseconds nor the panels.

**No doc comment warns.** The doc comment of `Drop for DashboardRuntime`, at
`runtime.rs:919-922`, states that a program can drop one dashboard and make
another one. The comment at `:924-927` names the receiver that a caller kept.
It names only the error that the receiver does not read. It does not name the
wait that the receiver can never leave. The doc comment of `subscribe`, at
`:693-696`, holds no warning, and the doc comment of `is_refreshing`, at
`:673-678`, holds none. The doc comment of `Snapshot::is_refreshing` holds none
either, at `crates/thunderbox-dashboard/src/snapshot.rs:153-157`.

**No test holds the rule.** Three tests of
`crates/thunderbox-dashboard/tests/runtime.rs` drop the runtime, at `:1199`,
`:1227` and `:1282`. The first and the third count the tasks that stay alive
after the runtime drops. The second waits until no query of the runtime still
runs. None of the three reads a receiver after the runtime drops. The tests
call `subscribe` at three places: `tests/runtime.rs:2882`,
`tests/common/mod.rs:17` and `crates/thunderbox/tests/default_dashboard.rs:24`.
Each place reads the receiver while the runtime lives. The library test at
`crates/thunderbox-dashboard/src/runtime.rs:1266-1298` holds the guard of
`ClearOnEnd` itself. It asserts that a `ClearOnEnd` of an older set of queries
leaves `refreshing` at `true`. That test holds the guard for a refresh, and no
test holds it for a runtime that drops.

**No program of this repository meets the fault.** Neither
`crates/thunderbox/src/main.rs` nor
`crates/thunderbox-dashboard/examples/full_embed.rs` calls `subscribe`. Those two
programs are the only ones of this repository that embed a dashboard.
`crates/thunderbox/src/main.rs:340` reads `refresh_age()` on the runtime itself.
The fault therefore reaches a program that embeds thunderbox, keeps a receiver,
and reads `is_refreshing()` on it without a wait on `changed()`.

## 3.2 Four facts behind the fourth duty of a data source reach no file

**doc silent**

The fourth
duty of `crates/thunderbox-source/src/lib.rs:23-24` states the rule, and `:27-33`
gives the reason. `docs/ARCHITECTURE.md:395-396` and `:399-402` repeat both.
Both files state that `Reduce` reads a position in the column, and not a moment.
Neither file holds the measurement below, and neither holds the four facts after
it.

`Reduce::apply` reads that position
(`crates/thunderbox-core/src/reduce.rs:54-55`). The example at `:44-50` holds a
column whose values rise, so it shows no column out of order. The measurement:
`Reduce::Last.apply(&[Some(5.0), Some(1.0)])` returns `Some(1.0)`.
`reduce_frames` calls the method at `crates/thunderbox-widgets/src/panel.rs:209`,
for the three panels that show one number.

`Frame::time_series` checks no order. Its doc comment at
`crates/thunderbox-core/src/frame.rs:58-74` names three conditions for an error.
Two of them are a frame with no time column, and a frame with no number column.
The third is a number column that holds a different number of rows from the time
column. No code of `thunderbox-core`, of `thunderbox-widgets`, or of
`thunderbox-dashboard` checks the order of a time column.

`thunderbox_widgets::last_value` compares the moments of two columns, and it reads
a position inside one column. The loop at
`crates/thunderbox-widgets/src/timeseries.rs:517-524` walks each column from its
end, and it stops at the first finite value. The line at `:519` then keeps the
newer of two columns, so the function repeats the fault of `Reduce::Last`. The
measurement: `last_value` returns `Some((1, 1.0))` for a frame with the moments
`[5, 1]` and the values `[Some(5.0), Some(1.0)]`. The doc comment at `:504-508`
names the order nowhere. It reads "A panel that shows a single number uses this
function." at `:507-508`, and no panel calls it.
`crates/thunderbox-widgets/src/lib.rs:47` exports the function, and two tests at
`timeseries.rs:765-775` call it.

A time column that runs from the newest moment to the oldest draws the same line
in the time series panel. `collect` pushes each point in the order of the column
(`timeseries.rs:234-249`). ratatui-widgets 0.3.2 then joins each point to the
next one with `windows(2)` (`src/chart.rs:1059`, `:1081`), for the two graph
types of `timeseries.rs:453-455`. A column whose moments rise and then fall, or
fall and then rise, draws a line that jumps backwards.
`docs/ARCHITECTURE.md:1376-1381` records one such line for the sysinfo store.
The test
`a_sample_carries_the_present_moment_and_not_the_end_of_the_range`, at
`crates/thunderbox-source-sysinfo/src/lib.rs:247-278`, holds the order of that
store. `crates/thunderbox-source-sysinfo/src/history.rs:136-144` and
`crates/thunderbox-source-testdata/src/lib.rs:416-426` each hold the same rule for
their own store. No test of `thunderbox-core` holds it for `Reduce`.

`README.md:125` and `reduce.rs:17` name `last` as "The most recent value".
`README.md:126` and `reduce.rs:20` name `first` as "The oldest value". Neither
pair states that these words are true for an answer in order alone.

## 3.3 A frame of two lengths gives a chart `no data`, and a table draws it

**doc silent**

The fifth duty of a data source names this fault twice. The duty sits at
`crates/thunderbox-source/src/lib.rs:25` and `:35-43`, and at
`docs/ARCHITECTURE.md:397` and `:404-407`. Neither place names a caller, and
neither holds a measurement. The second place writes "The chart then draws no
line for that frame. The table draws the same frame, and it fills the short
column with the symbol of a value that is not there."

Three parts of the detail below reach no file. The first is the error that no
`match` arm reads, and the count that the status row misses. The second is the
three panels that the fault does not reach. The third is the measurement.

`Frame::time_series`, at `crates/thunderbox-core/src/frame.rs:75-96`, returns an
error for a frame of two lengths. `collect`, at
`crates/thunderbox-widgets/src/timeseries.rs:210-301`, calls it at `:222-224`, and
`last_value`, at `:509-528`, calls it at `:512-514`. Each call drops the error
with a `let ... else` block that holds `continue;`, and no comment states why.

The code builds `Error::FrameShape` (`crates/thunderbox-core/src/error.rs:30`) at
`frame.rs:78`, at `:85` and at `:88`. No `match` arm names it, and no code makes
a panel that holds it. `Snapshot::error_count`
(`crates/thunderbox-dashboard/src/snapshot.rs:176-178`) therefore counts no error.

`collect` returns `None` when `time_series` returns an error for every frame.
`plot`, at `timeseries.rs:168-184`, reads `collect` at `:172`, and the panel
writes `NO_DATA` at `:436`. The table panel reads the same frame at
`table.rs:265-303`, and its `collect` takes the number of rows from the longest
frame at `:271`. `entry`, at `:210-242`, returns the symbol of a value that is
not there for a short column. `table.rs:252` and `:209` state those two rules.

The fault does not reach the stat panel, the gauge panel and the bar gauge
panel. Those three call `reduce_frames` (`panel.rs:204-220`), and the stat panel
also calls `series_points` (`stat.rs:393-404`). Each function reads
`numeric_fields`, and neither one calls `time_series`.

A reader drew each panel at 40 cells by 8 cells, against two frames. The first
frame holds a time column of four values, and `usage`, a number column, holds
10, 50 and 20. The error ends with `field "usage" has 3 values but the time
field has 4`. `Frame::len` (`frame.rs:37-39`) reads the longest column, so it
returns 4 for each frame. `last_value` returns `None`, and the chart draws
`no data`. The table draws four rows with `—` under `usage` in the last row, and
the stat panel draws `20`. The second frame holds a time column of three values,
and `usage` holds 10, 50, 20 and 80. The chart draws `no data` again, the last
row holds `—` under `time` and `80` under `usage`, and the stat panel draws
`80`. A person reads two panels that disagree, and no message names the cause.

## 1.5 A new time range holds `Snapshot::is_loading` at `true` for a terminal that shrank, and its doc comment denies that

**doc wrong**

The doc comment of `Snapshot::is_loading`, at
`crates/thunderbox-dashboard/src/snapshot.rs:121-148`, contradicts itself. Its
list of three cases, at `:126-129`, names the window that changed as a case that
returns `true`. Its prose at `:135-140` then states that the method returns
`false` for a terminal that shrank after the panels answered. A reader measured `true` for
such a terminal after a change of the time range.

A reader took the measurement below. Two panels answered in an area of 100 cells
by 30 rows. The area then held 2 cells by 2 rows for 20 ticks. A tick is one
call of `DashboardRuntime::tick`. The snapshot read `is_loading = false`, and
each panel read `Ready`. The reader then called
`DashboardRuntime::set_time_range` with a different range, and it ran 20 more
ticks. The snapshot then read `is_loading = true`, and each panel read
`Loading`. No file holds those numbers.

`docs/ARCHITECTURE.md:1122-1130` does hold the cause. The runtime puts every
panel back to the state that has no answer. That file states that the third case
then reaches a terminal which is too small. It names
`DashboardRuntime::set_time_range` and `DashboardRuntime::set_variables`. The
doc comment at `snapshot.rs:135-140` holds none of that text. It names the first
cause alone, and it then states that each panel stays at `PanelState::Ready`.
That doc comment also states that this method returns `false`.

The code holds that cause in three places. `kept_state` maps
`(Kept::Nothing, _)` to `PanelState::Loading` (`runtime.rs:911-916`). `limits_for` returns
`None` for a panel that the renderer drops (`runtime.rs:747`), so the runtime
asks no data source about it.
`panel_jobs` gives such a panel a `PanelJob::Skip` that carries the state of
`kept_state` (`runtime.rs:780`). The runtime therefore holds each such panel at
`PanelState::Loading` until the terminal grows.

**`take_choices` is a third method that starts a set with `Kept::Nothing`, and
no file names it.** The method does so for a choice that changed the value of a
variable (`runtime.rs:635-658`, `:653-656`). `DashboardRuntime::tick` calls it
(`runtime.rs:549`). A variable that states a query therefore starts such a set,
and no person presses a key. `docs/ARCHITECTURE.md:1125-1126` names the other
two methods alone.

No test holds this rule for the method.
`a_new_window_puts_a_panel_that_the_renderer_drops_back`
(`crates/thunderbox-dashboard/tests/runtime.rs:2243-2270`) reads `Snapshot::panel`
for each panel, and it never calls `Snapshot::is_loading`. No test of the
workspace calls `Snapshot::is_loading` after an area that drops every panel.

A program that embeds thunderbox can build its message for a small terminal on
this method. The doc comment gives it that permission for a terminal that
shrank. Such a program draws no message after the next key that changes the time
range. `crates/thunderbox/src/main.rs:363-371` avoids the fault. It reads
`DashboardView::dropped_panels` whatever the method returns.

## 3.1 Only a program that embeds thunderbox reaches the fault of `downsamples`

**doc silent**

Item 7 of section 13 of `docs/ARCHITECTURE.md`, at `:1755-1769`, holds this
fault: no code reduces the number of points when a descriptor holds `false`.
The comment on the field holds the same fault, at
`crates/thunderbox-source/src/lib.rs:100-106`. The list at
`docs/OPEN_FINDINGS.md:44-61` therefore forbids that fault as an item here. The
sentence at `:46` forbids a second record of such an item.
Neither text names a line of the code, and neither text names the program that
reaches the fault. This item holds those two facts, and it holds no other.

`crates/thunderbox-source/src/lib.rs:107` declares the field. Two lines write it.
The function `new` writes `false` at `:130`, and the function `downsampling`
writes `true` at `:147`. Five lines read it:

- `crates/thunderbox-source/src/lib.rs:307`, below the `#[cfg(test)]` line at
  `:250`;
- `crates/thunderbox/src/sources.rs:129`, below the `#[cfg(test)]` line at `:65`;
- `crates/thunderbox-source-sysinfo/src/lib.rs:495`, below the `#[cfg(test)]`
  line at `:206`;
- `crates/thunderbox-source-prometheus/src/lib.rs:411`, below the `#[cfg(test)]`
  line at `:330`;
- `crates/thunderbox-source-testdata/src/lib.rs:304`, below the `#[cfg(test)]`
  line at `:275`.

Each read sits inside a test module. The command
`grep -rn downsamples --include='*.rs' crates` finds those eight lines, and it
finds no other. `crates/thunderbox-dashboard/src/query.rs:361` reads
`supports_label_values`, and the function `answer_limit` at `:398-403` reads
the field of that name. No line of that crate reads `downsamples`.
`grep -c -i lttb Cargo.lock` writes 0.

Each of the three data sources calls `downsampling`, at
`crates/thunderbox-source-sysinfo/src/lib.rs:139`, at
`crates/thunderbox-source-prometheus/src/lib.rs:284`, and at
`crates/thunderbox-source-testdata/src/lib.rs:242`. Every descriptor outside a
test therefore holds `true`. The fault reaches none of the three data sources.
It reaches a program that embeds thunderbox, when the descriptor of the data
source of that program holds `false`.

## 6.3 Four texts name the keys of the program, and no test compares any pair

**doc silent**

The doc comment of `KEYS`, at `crates/thunderbox/src/main.rs:68-80`, states four
facts about the copies. `full_embed.rs` holds a copy. The `--help` text and
`README.md` hold the complete sentence of each key. `thunderbox-dashboard` cannot
depend on `thunderbox`. No test compares the two copies, and no test reads either
one against `DashboardView::handle_key`. The comment at `full_embed.rs:162-165`
repeats the copy, the reason, and the absence of a test. The facts about the
tests below, the two measurements, and the one repair reach no file.

**Four texts name the keys of the program, and a person keeps each one by
hand.** Two hold the same words, and two hold the same thirteen sentences.
`KEYS` sits at `main.rs:81`, and the text at `full_embed.rs:168-169` is equal
to it, character for character. The thirteen rows of `HELP`, at `main.rs:45-57`,
and the thirteen rows of the table of `README.md:29-41` hold the same thirteen sentences.
`crates/thunderbox-widgets/examples/panel_embed.rs:114` draws the two keys of a
different program, so it is not a fifth copy. Three of the thirteen sentences
name the keys of a variable, which `VARIABLE_KEYS` at `main.rs:88` holds, and
`KEYS` names none of those three.

**The code confirms the comment.** `crates/thunderbox-dashboard/Cargo.toml:13-26`
names ten crates, and `thunderbox` is not one of them. `view.rs:136` binds `m`
inside `DashboardView::handle_key`, and no test reads that arm against a copy.
The one test that reads `KEYS` alone is
`the_list_of_the_keys_fits_a_terminal_of_eighty_columns`, at `main.rs:617-628`.
It checks `KEYS.chars().count() <= 80` and `KEYS.ends_with(RANGE_KEY)`. Four
assertions compare the output of `status_text` against `KEYS`. Each one names
`KEYS`, at `:483`, `:506`, `:532` and `:585`, and both sides read the same
constant. Two tests read `HELP`, at `main.rs:786-798` and at `:803-817`, and
each one holds its own rows. No test reads `README.md`.

**A reader made two changes of the kind that section 4.3 of `CLAUDE.md` asks
for.** `cargo test --no-fail-fast` runs 740 tests here, and it reports no
failure. The reader wrote `0 rang` in place of `0 range` at
`full_embed.rs:169`, and no test failed. `scripts/ste-lint.py` also reported no
fault. The reader then deleted `m marker  ` from `KEYS`, while `view.rs:136`
still bound `m`. The shorter text holds 65 characters, so it passes both
assertions of the one test that reads `KEYS`. No test failed again, and the
script reported no fault. No test reads the text of an example, and the script
reads a comment line alone inside a file that ends in `.rs`.

**The one repair is a rule about a picture, and not a test.**
`CLAUDE.md:535-542` records that one change gave new words to `KEYS` and to
the text of `full_embed.rs`. Two pictures then drew a list that neither program
drew, and a later change repaired them. `CLAUDE.md:544-547` asks
the next person who works on a scene to record its picture again. That rule is
the whole repair, and the next difference between two copies reaches a reader
alone.

## 5.5 Each `dropped_text` names the other function nowhere

**doc silent**, and **doc wrong**

The section "The rule for a short label inside a panel" of `CLAUDE.md` states
the main fact. It names the second function, which sits in
`crates/thunderbox-widgets/src/table.rs`, the different words, and the different
unit of width. `CLAUDE.md:138-140` also asks a writer to change both functions.
The facts below add to that section.

**No doc comment of the code names the other function.** The doc comment at
`crates/thunderbox-widgets/src/panel.rs:290-299` covers the function at
`panel.rs:300-310`. It reads "Three panels show one number for each series, and
all three call this function. A reader therefore meets the same words
everywhere". It names no function of `table.rs`. The doc comment at
`table.rs:409-414` names no function of `panel.rs`. The change that added the
paragraph at `CLAUDE.md:133-140` changed that one file alone. A person
who reads `panel.rs` therefore changes one function, and leaves the other
function as it was.

**`stat.rs:440-441` promises more than the code holds.** It reads "The shared
function `dropped_text` supplies the text, so that every panel states the same
thing". The table panel states a different thing, because its note counts a
column (`table.rs:416-419`).

`CLAUDE.md` names no panel of the three that call the function of
`panel.rs:300-310`. Those three are the gauge panel, the bar gauge panel and
the stat panel. The calls sit at `gauge.rs:499`, at `bargauge.rs:375`, and at
`stat.rs:446`.

The function at `table.rs:415-425` holds no `pub(crate)`, so nothing outside
that module reaches it. `plan_widths`, at `table.rs:434-448`, is its one caller
outside the tests, and that call sits at `table.rs:446`. A search over
`CLAUDE.md`, `docs/` and `README.md` finds no mention of `plan_widths`.

The two functions share one signature, which is
`(dropped: usize, width: usize) -> String`. Each one builds the complete
sentence first, and each one then builds the short form with
`format!("+{dropped}")`, at `panel.rs:309` and at `table.rs:424`. Both
functions cut that short form with `clip` of `panel.rs:331-353`. `clip_cells`
of `table.rs:144-171` calls `clip` at `table.rs:147`, and the short form is a
text in which each character takes one cell. `table.rs:25` imports `clip` from
the module `panel`, and it imports no `dropped_text`.

The tests of each function sit at `panel.rs:527-540` and at
`table.rs:1286-1298`, and no test compares the two functions. `gauge.rs:1064`
and `stat.rs:930` also compare against the words of the function of `panel.rs`.

## 6.2 Three facts of the table of the doctests reach no file

**doc silent**

A doctest is a fence inside a doc comment. A fence is the code between two rows
of three back quotes. `cargo test --doc` compiles and runs the code of each
doctest. `CLAUDE.md:355-368` holds four statements about the doctests. It gives
the rule that the count must not fall, and it gives the count of 22. It holds a
table with the count of each crate. It also states that no command compares the
count against that number. The step at `.github/workflows/checks.yml:86-87` runs
`cargo test --doc`, and that command fails only when a doctest fails. The three
facts below reach no file.

**Two of the doctests never run.** A fence that carries `no_run` on its first
row is different. The tool rustdoc compiles that code, and it never runs that
code. The workspace holds two such fences, at
`crates/thunderbox-widgets/src/lib.rs:9-16` and at
`crates/thunderbox-source-prometheus/src/lib.rs:15-22`. Each fence sits inside a
crate comment. The first calls `TimeSeriesPanel::new`, and the second calls
`Prometheus::new`. Each test proves only that the call compiles. A change that
keeps the shape of a function, and that alters what the function does, passes
both tests. `cargo test --doc` writes the word `compile` on the line of each of
the two, and on no other line. The fence of `toml` at `:26-32` of the second
file is not a doctest, because rustdoc tests a fence of Rust code alone.

**`cargo test --doc` reads no part of the crate `thunderbox`.** Cargo collects the
doctests of a library alone. `crates/thunderbox/src` holds `main.rs`, `sources.rs`
and `default.toml`, and it holds no `lib.rs`. `crates/thunderbox/Cargo.toml`
declares no `[lib]` table. `cargo test --doc` therefore writes the header
`Doc-tests` seven times, one time for each library. It writes no such header for
the crate `thunderbox`. The instruction at `CLAUDE.md:366-367` reads
"Read the output of each crate yourself". `cargo test --doc` writes no output
for that crate. The row `each other crate | 0` at `CLAUDE.md:363` gives that
crate a count of zero. A count of zero says that Cargo read a library and found
no doctest. That crate holds no library. The measurement:
`cargo test --doc -p thunderbox` writes `no library targets found in package` and
returns 101.

**Three crates hold zero doctests, and no file names the three.** They are
`thunderbox-source`, `thunderbox-source-sysinfo` and `thunderbox-source-testdata`. No
file of the directory `src` of those three crates holds a fence. Each of the
three crates holds a library, so `cargo test --doc` writes a line for each one.
That line reports zero tests, and it reports `0 passed`. `cargo test --doc`
writes the same words for a crate that lost its last doctest. A person who reads
the output therefore needs the names of the three crates, and the table gives a
count and no name.

## 6.4 The status row of `too_small` holds no list of the keys

**doc wrong**

`CLAUDE.md:512` names that scene among the scenes that draw the status row. The
same cell describes that row as the row "which ends with the list of the keys".
The row of that scene ends with no such list. That cell holds one part of this
item, and `README.md:475` holds another. No file states that the row of this
scene drops the list of the keys, and no file states the reason.

The `too_small` scene runs on a terminal of 90 cells
(`scripts/record-demo.py:282`), and the call at
`crates/thunderbox/src/main.rs:363-371` gives the status row that width. The
sentence at `main.rs:244` holds 38 characters. `GAP` at `main.rs:91` holds 4
characters, and `KEYS` at `main.rs:81` holds 75 characters. The sum is 117,
which is more than 90. The loop at `main.rs:261-263` drops the last part while
the row is too wide, so it drops `KEYS`. A reader measured
`docs/media/too_small.gif`, and the picture draws the sentence alone. The row
holds 48 cells after the sentence and the gap. A change that gives `KEYS` more
than 48 characters therefore leaves this scene as it is.

Five scenes run `target/debug/thunderbox`. The key `program` of each one names
that program at `record-demo.py:187`, `:242`, `:261`, `:269` and `:280`. The
cell at `CLAUDE.md:512` names the same five scenes. No file states that count. The key
`program` does state the program of each scene, and `README.md:570` states that
rule.

`README.md:475` holds the sentence inside the alternative text of the picture,
which is the text between the two square brackets. No sentence beside that
picture quotes the sentence. `CLAUDE.md:526-528` state that `README.md` quotes
it "as the text beside the picture of that scene", and they name no alternative
text. A change to the sentence at `main.rs:244` therefore makes `too_small.gif`
and `README.md:475` wrong. Section 6 names the alternative text at
`CLAUDE.md:676-678` alone, and there it names a text that a writer copies into
a description. No sentence of section 6 names that text as a text that a writer
corrects.

The cell first named four scenes, and a later change added the fifth scene,
`too_small`, to it. The section "When
no scene reaches the change" asks two changes of `README.md` for a new scene
(`CLAUDE.md:624-626`). It asks no change of that cell. The cell can therefore
omit the next scene that runs `target/debug/thunderbox`.

## 1.10 No test reads 16 of the 25 lines of text of the constant `HELP`

**doc silent**

`CLAUDE.md:67-71` places the constant `HELP` of `crates/thunderbox/src/main.rs`
under the standard. `CLAUDE.md:216-219` states that `scripts/ste-lint.py` reads
no Rust expression. Neither passage counts the lines. Item 1.10 above holds a
different fault, so this item takes the letter `b`.

`const HELP: &str = "\` stands at `main.rs:27`, and the `";` that closes it
stands at `:58`. The 30 lines between them hold the text. Five of those lines
are empty, so 25 lines hold words. The block `Options:` at `:40-42` names two
options, and the block `Keys:` at `:44-57` holds thirteen rows. Item 4.6 of this
file reads the block `Options:`, and it states how `main.rs:113-124` reads each
argument.

Four tests read that text. `main.rs:786-798` and `:800-817` read the eight whole
rows of `:50-57`. `main.rs:819-826` and `crates/thunderbox/tests/cli.rs:52-65`
read the one word `Usage` of `main.rs:33`. `cli.rs:61-64` reads the whole text
for one control character, and it holds no word of the text. Nine lines
therefore reach a test. The other 16 lines of words reach no test. The rows of
`q`, `Tab`, `Shift+Tab`, `m`, and `r` at `:45-49` are five of the 16.

`scripts/ste-lint.py` reads none of the 30 lines. `scripts/ste-lint.py:99-103`
keeps a line of a `.rs` file only when that line starts with `//`, after any
space, and it drops every other line. A reader took this measurement. The reader
wrote four faults into line 29 of a copy of `main.rs`, inside `HELP`. Those
faults are an em dash, the abbreviation that rule 9 forbids, and two words in
the `-ing` form. The script wrote
`The script found none of the common faults.`, and it returned 0. The script
reported four faults for the same words on the `///` line at `:26` of a second
copy, and it returned 1. A reader is therefore the only check of that text.

`README.md:29-41` holds a table of the same thirteen keys, and each row of that
table repeats the sentence of a row of `main.rs:45-57`. `main.rs:72-73` and
`:86-87` state that relation, so the code names the pair. No file states that no
check compares the two texts, and no such check exists. Five of the sixteen
lines above therefore stand twice, and a change to one copy leaves every test
green.
