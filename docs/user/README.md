# The user documentation of thunderbox

This folder holds the documentation for a person who uses thunderbox. The
documentation has four kinds, and each kind answers a different need:

| kind | the need that it answers | folder |
|---|---|---|
| tutorial | "Teach me thunderbox, one step after another." | [tutorials](tutorials) |
| how-to guide | "I have a task. Show me the steps." | [how-to](how-to) |
| reference | "Give me the exact facts of one part." | [reference](reference) |
| explanation | "Tell me why thunderbox works in this way." | [explanation](explanation) |

This division follows Diátaxis, which is a method that divides documentation by
the need of the reader. A person who contributes to thunderbox reads
[the contributor documentation](../contributor/README.md) as well.

## The tutorials

| page | what you learn |
|---|---|
| [Watch this machine](tutorials/watch-this-machine.md) | Run the program, read each panel, and find the busiest process. |
| [Write your first dashboard](tutorials/write-your-first-dashboard.md) | Write a dashboard file over the test data, and run it. |
| [Read a Prometheus server](tutorials/read-a-prometheus-server.md) | Connect a dashboard to a Prometheus server. |
| [Put a panel inside your program](tutorials/put-a-panel-inside-your-program.md) | Draw one thunderbox panel in a program that holds its own data. |
| [Put a dashboard inside your program](tutorials/put-a-dashboard-inside-your-program.md) | Run a whole dashboard inside a program that owns its terminal. |

## The how-to guides

| page | the task |
|---|---|
| [Install thunderbox](how-to/install-thunderbox.md) | Install the program with Cargo or with Nix. |
| [Show the busiest processes](how-to/show-the-busiest-processes.md) | Add a top panel that lists the processes of a machine. |
| [Group panels under row panels](how-to/group-panels-under-row-panels.md) | Add row panels, and collapse them. |
| [Let a person select a value](how-to/let-a-person-select-a-value.md) | Add a variable, and refer to it in a query. |
| [Name each series](how-to/name-each-series.md) | Write a pattern in the `legend` field. |
| [Colour a value that passes a bound](how-to/colour-a-value-that-passes-a-bound.md) | Add thresholds to a panel. |
| [Show an earlier window](how-to/show-an-earlier-window.md) | Move and scale the time range with the keys. |
| [Start each chart of a stack at the same cell](how-to/start-each-chart-of-a-stack-at-the-same-cell.md) | Use the rule `align = "column"`. |
| [Show two data sources on one dashboard](how-to/show-two-data-sources-on-one-dashboard.md) | Declare two data sources, and read both. |
| [Give the Prometheus source a CA file](how-to/give-the-prometheus-source-a-ca-file.md) | Repair the error `No CA certificates were loaded from the system`. |
| [Change the glyph of a chart](how-to/change-the-glyph-of-a-chart.md) | Select the marker that draws a point. |
| [Read a dashboard on a small terminal](how-to/read-a-dashboard-on-a-small-terminal.md) | Understand the panels that thunderbox drops. |

## The reference

| page | the facts |
|---|---|
| [The command line](reference/command-line.md) | The arguments, the options, the keys, and the status row. |
| [The dashboard file](reference/dashboard-file.md) | Every table and every field of a dashboard file. |
| [The panels](reference/panels/README.md) | Each kind of panel, and the fields that it reads. |
| [The data sources](reference/data-sources/README.md) | Each data source, its queries, and its options. |
| [Numbers and units](reference/numbers-and-units.md) | The units, the decimal places, the steps that reduce a column, and the thresholds. |
| [Time ranges](reference/time-ranges.md) | The form of a moment and of a duration. |
| [Variables](reference/variables.md) | The fields of a variable, and the form of a reference. |
| [Legend patterns](reference/legend-patterns.md) | The form of a pattern that names a series. |
| [The crates](reference/crates.md) | The crates that a program uses to hold thunderbox. |
| [The Nix flake](reference/nix-flake.md) | Each output of the flake. |

## The explanations

| page | the question that it answers |
|---|---|
| [How thunderbox works](explanation/how-thunderbox-works.md) | How do a dashboard, a data source, a frame, and a panel fit together? |
| [Why each panel asks for its own points](explanation/why-each-panel-asks-for-its-own-points.md) | Why does a narrow panel ask for fewer points? |
| [The time range and the window](explanation/the-time-range-and-the-window.md) | What does a label of the axis of time state? |
| [Panels that the terminal cannot hold](explanation/panels-that-the-terminal-cannot-hold.md) | Why does a panel disappear on a small terminal? |
| [Row panels and the place of a panel](explanation/row-panels-and-the-place-of-a-panel.md) | Why does a row panel hold panels by their place? |
| [Two ways into another program](explanation/two-ways-into-another-program.md) | When does a program use the widgets alone, and when the whole dashboard? |
