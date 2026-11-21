# The command line

This page lists the arguments and the options of the program `thunderbox`. It
also lists each key, and each text of the status row.

The tutorial [Watch this machine](../tutorials/watch-this-machine.md) runs the
program for the first time. The guide
[Show an earlier window](../how-to/show-an-earlier-window.md) uses the keys of
the time range.

## The usage

```
thunderbox [FILE]
thunderbox --help
thunderbox --version
```

| argument | what it is |
|---|---|
| `FILE` | The path of a dashboard file. [The dashboard file](dashboard-file.md) states its form. The program shows its own dashboard when a person names no file. That dashboard reads this machine through the data source `sysinfo`. |

The program reads its arguments with these rules:

- The program reads each argument in order. The first option that it finds
  decides what the program does.
- When no argument is an option, the program takes the first argument as
  `FILE`. It ignores every other argument.
- The program knows two options alone. It takes any other first argument as the
  path of a file. The argument `--foo` therefore gives the error
  `The program cannot read --foo.`

## The options

| option | what the program does |
|---|---|
| `-h`, `--help` | The program writes the help text to the standard output, and it stops. It opens no terminal screen. |
| `-V`, `--version` | The program writes `thunderbox` and its version to the standard output, and it stops. This workspace holds the version `0.0.0`. |

The help text lists the usage, the argument `FILE`, the options, and the keys.

## The exit code

| code | condition |
|---|---|
| `0` | A person stopped the program with a key. The options `--help` and `--version` also return this code. |
| `1` | The program stopped with an error. The program also returns 1 when its standard output is not a terminal. The next section lists the errors that a person meets most often. |
| `101` | The program panicked. A panic is a fault that the program cannot report as an error, and the program stops at once. |

## The errors before the terminal opens

The program reads the dashboard and makes each data source before it takes the
terminal. It then checks that its standard output is a terminal. A fault in one
of these steps stops the program. The program writes the message of the fault
to the standard error. It writes nothing to the standard output, so it sends no
control sequence to another program. A control sequence is a text that moves
the cursor of a terminal or changes a colour.

| the first line of the message | condition |
|---|---|
| `Error: The program cannot read FILE.` | The program cannot open or read the file. |
| `Error: The dashboard in FILE is not correct.` | The file is not valid TOML, or the content holds a fault. [The dashboard file](dashboard-file.md#the-faults-that-the-program-rejects) lists each fault. |
| `Error: The program cannot make the data sources of the dashboard.` | A `[source.NAME]` table names a kind that this build does not hold. A table that holds an option which its data source rejects gives this error as well. |
| `Error: The program draws a dashboard only on a terminal. The standard output of the program is not a terminal.` | The standard output is a file or a pipe, for example after `thunderbox > out.txt`. The program returns 1 and writes nothing to the standard output. |
| `Error: The program cannot start the terminal.` | The program found a terminal, and the terminal refused the start. The program returns the terminal to its first state, and it then returns 1. |

The lines after the words `Caused by:` state the reason.

## The keys

The program reads a key when a person presses it. Two parts of the code bind the
keys. The program binds most keys itself. The crate `thunderbox-dashboard` binds
three keys, and it uses them inside the dashboard.

### The keys that the program binds

| key | what the key does |
|---|---|
| `q`, `Esc` | Stop the program. |
| `Ctrl+C` | Stop the program. |
| `r` | Run the queries again. The key stops a set of queries that still runs, and it starts a new set at once. |
| `Left` | Move the time range back by half of its width. |
| `Right` | Move the time range forward by half of its width. |
| `-` | Make the time range twice as wide. |
| `+` | Make the time range half as wide. |
| `0`, `=` | Return to the time range that the dashboard states. |
| `v` | Select the next variable of the dashboard. |
| `]` | Take the choice that follows, on the variable that you selected. |
| `[` | Take the choice before it, on the variable that you selected. |
| `Enter`, `Space` | Collapse or expand the row panel that you selected. |

The crate `thunderbox-dashboard` holds the rule of each key of this table from
`Left` to `Space`. The functions `time_key`, `variable_key`, and `row_key` hold
those rules. The program passes each key except `q`, `Esc`, `Ctrl+C`, and `r`
to those three functions, in that order. A key that no function takes then
reaches the type `DashboardView`. The page [The crates](crates.md) lists these
items.

### The keys that the crate thunderbox-dashboard binds

The type `DashboardView` of the crate `thunderbox-dashboard` uses these keys:

| key | what the key does |
|---|---|
| `Tab` | Select the next panel. The selection skips each panel that a collapsed row panel hides. |
| `Shift+Tab` | Select the panel before. This key also skips each hidden panel. |
| `m` | Change the glyph that a chart draws a point with. The key selects braille, octant, half block, and block, in that order. Braille then follows block. |

The border of the selected panel is cyan. The guide
[Change the glyph of a chart](../how-to/change-the-glyph-of-a-chart.md)
describes each glyph.

### The rules of the keys of the time range

- A move fixes a moment at each end of the time range. The window then stands
  still, and the panels show that window.
- A move forward that reaches the present moment returns a time range of the
  same width. That time range follows the clock again.
- The keys `-` and `+` hold the later end of the time range still. They move
  the earlier end.
- The keys `-` and `+` keep the width between 1 second and 3650 days. The page
  [Time ranges](time-ranges.md#the-limits-of-the-width) states these limits.
- A key that asks for the time range that the dashboard already shows does
  nothing. The key `Right` on a time range that follows the clock is an example.
- A key that changes the time range returns every panel to `loading…`. It also
  starts a new set of queries.

### The rules of the keys of a variable

- The key `v` does nothing on a dashboard that declares one variable or no
  variable.
- The key `]` takes the first choice after the last choice. The key `[` takes
  the last choice before the first choice.
- A key that changes a value returns every panel to `loading…`. It also starts
  a new set of queries.
- The key `v` changes no value, so it asks a data source for nothing.

The page [Variables](variables.md) states the fields of a variable.

### The rules of the keys of a row panel

- The keys `Enter` and `Space` act only while the selected panel is a row
  panel.
- A key that makes a panel appear starts a new set of queries. A key that only
  hides panels starts no set of queries.

The page [The row panel](panels/row.md) states the rules of a row panel.

## The parts of the screen

The program divides the terminal into three parts, from the top to the bottom:

1. The row of the variables takes one line. The program draws this row only
   for a dashboard that declares a variable.
2. The panels take every other line. The program asks for 3 lines or more for
   this part.
3. The status row takes the last line.

## The row of the variables

The row writes the label of each variable, and then the value that the variable
holds. It puts two spaces between two variables. It puts the selected variable
between two square brackets. A variable that holds no value shows `no choice`.

The label is the `label` field of the variable. A variable that states no label
shows its name.

The row then adds four spaces and the key hint `v next variable  [] value`. It
adds the hint only while the whole row fits the width of the terminal. This
example shows two variables and the hint:

```
[job: web]  region: eu    v next variable  [] value
```

## The status row

The status row writes one text for each fact, in the order of the table below.
It puts four spaces between two texts. The list of the keys comes last.

| order | text | condition |
|---|---|---|
| 1 | `1 panel holds an error.` or `N panels hold an error.` | One panel or more holds an error. |
| 2 | `The terminal is too small for 1 panel.` or `The terminal is too small for N panels.` | The renderer drops one panel or more. The count holds no panel that a collapsed row panel hides. |
| 3 | `The variable "NAME" has no choice.` and the reason | The data source of a variable did not return the choices. The row names the first such variable. |
| 4 | `The dashboard waits for an answer.` | A set of queries has waited for 1 second or more. |
| 5 | `The panels show WIDTH that ended DISTANCE ago.` | The window ends before the present moment. The text `The panels show 5 minutes that ended 1 hour ago.` is an example. |
| 6 | `The numbers are N seconds old.` | The answers on the screen come from a set of queries that started 5 seconds ago or more. |
| 7 | `q quit  tab next panel  m marker  r refresh  arrows move  -+ width  0 range` | The row always writes this list of the keys. |

The renderer is the part of thunderbox that draws the panels. It drops a panel
that is under 3 cells wide or under 3 cells high. It also drops a row panel that
takes no line. This documentation calls each such panel a panel that the
renderer drops. Text 2 counts it.

Text 5 writes each length of time in one unit. The unit is seconds, minutes,
hours, or days. The row selects the unit from the length, and it rounds the
count to a whole number.

### The rules of the width of the status row

- The row drops the last text while the row is wider than the terminal. It
  never drops the first text.
- The list of the keys holds 75 characters, so it fits a terminal of 80
  columns.
- A row that names a window that stands still keeps the hint `0 range` after it
  drops the list of the keys. It adds the hint only while the hint fits.
- For a dashboard that holds a row panel, the row adds the hint `enter fold`
  after the list of the keys. Two spaces stand before the hint.
- The row adds the hint `enter fold` only while the row ends with the list of
  the keys. The longer row must also fit. The list and the hint need 87
  columns.

## The pages that give the reason

- [Panels that the terminal cannot hold](../explanation/panels-that-the-terminal-cannot-hold.md)
  explains the panel that the renderer drops.
- [The time range and the window](../explanation/the-time-range-and-the-window.md)
  explains the text that names a window.
- [Architecture decision record 0035](../../contributor/adr/0035-the-program-binds-each-key-of-the-time-range.md)
  states why the program binds each key of the time range.
- [Architecture decision record 0043](../../contributor/adr/0043-the-program-binds-each-key-of-a-variable.md)
  states why the program binds each key of a variable.
- [Architecture decision record 0054](../../contributor/adr/0054-the-program-binds-enter-and-space.md)
  states why the program binds `Enter` and `Space`.
