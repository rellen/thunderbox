# Let a person select a value

This guide adds a variable to a dashboard file. A person selects a value of the
variable while the program runs, and each query that refers to the variable
then reads that value.

## Add a variable with fixed choices

1. Add a `[[variable]]` table to the dashboard file:

   ```toml
   [[variable]]
   name = "signal"
   label = "the signal"
   options = ["random_walk", "sine", "ramp", "gappy"]
   current = "sine"
   ```

2. Write `$` and the name of the variable in the field `expr` of a target:

   ```toml
   [[panel.target]]
   source = "test"
   expr = "$signal"
   ```

3. Run the program with your dashboard file.

A row at the top of the terminal names each variable and its value, for example
`[the signal: sine]`. The field `current` gives the first value. A variable that
states no `current` takes its first option.

The file `dashboards/variables.toml` of the repository holds a whole dashboard
of this form.

## Select a value while the program runs

1. Press `v` to select the next variable. The square brackets move to that
   variable.
2. Press `]` for the choice that follows.
3. Press `[` for the choice before.

Each panel that refers to the variable asks its data source again, and it draws
the answer for the new value.

![A row of variables stands above three panels. The key `]` takes the choice that follows, and every panel draws the new signal.](../../media/variables.gif)

In the picture, the value in the row at the top changes after each press of the
key `]`. Every panel then draws the signal that the row names.

## Take the choices from a data source

1. Replace the field `options` with the field `query`:

   ```toml
   [[variable]]
   name = "instance"
   label = "the machine"
   query = { source = "prom", expr = "instance" }
   ```

2. Refer to the variable in the field `expr` of a target:

   ```toml
   [[panel.target]]
   source = "prom"
   expr = 'node_load1{instance="$instance"}'
   ```

thunderbox asks the data source for the choices one time, when the program
starts. Each kind of data source reads the text of that query in its own way.
The data source `prometheus` reads the name of a label, and it returns each value
of that label. The file `dashboards/prometheus.toml` holds this variable.

## Write a correct reference

- A name holds letters, digits, and underscores. It starts with a letter or an
  underscore.
- A reference ends at the first character that a name cannot hold. Write
  `up{job="$job"}`, with no mark around the name.
- Write `$$` for one `$` that refers to nothing.
- Write a reference in the field `expr` alone. The field `legend` holds no
  reference.
- Give a variable either `options` or `query`, and never both.

## The pages for the facts and the reasons

- [Variables](../reference/variables.md) states each field of a variable, and
  each fault that the program reports.
- [The command line](../reference/command-line.md) states the keys of a
  variable, and the row of the variables.
