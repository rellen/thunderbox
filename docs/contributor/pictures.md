# Make the pictures again

thunderbox draws on a terminal, so each change that alters the screen holds a
picture. Section 6 of [CLAUDE.md](../../CLAUDE.md) states when a change needs a
picture, and how a pull request shows it. This page states how to make one.

## The tools

`scripts/record-demo.py` starts a program inside a pseudo-terminal, sends a
list of keys, and writes a cast file in the asciinema format. The `agg` program
then turns that file into a GIF. Install `agg` with this command:

```
cargo install --git https://github.com/asciinema/agg --locked agg
```

The shell of the flake already holds `agg`, so a person inside that shell does
not run that command. That `agg` reads Unifont from the directory where Nix
keeps each package.

## Record a scene

Run these commands from the root of the workspace. The first one builds every
program that a scene names:

```
cargo build --workspace --bins --examples
python3 scripts/record-demo.py app /tmp/demo.cast
agg --font-family "Unifont" --theme asciinema --fps-cap 10 \
    /tmp/demo.cast docs/media/demo.gif
```

The first argument names the scene. Each scene names its own program and its
own list of keys. Put the name of another scene in place of `app`, and put the
picture of that scene in place of `docs/media/demo.gif`.

## The scenes

| scene | what it records | the page that shows the picture |
|---|---|---|
| `app` | The whole program, over the layout of the dashboard that it holds, with the data source testdata. | [README](../../README.md), [Watch this machine](../user/tutorials/watch-this-machine.md) |
| `panel_embed` | The example that draws one panel. | [Put a panel inside your program](../user/tutorials/put-a-panel-inside-your-program.md) |
| `full_embed` | The example that holds a whole dashboard. | [Put a dashboard inside your program](../user/tutorials/put-a-dashboard-inside-your-program.md) |
| `stack_none` | Two charts in one column, under no rule. | [Start each chart of a stack at the same cell](../user/how-to/start-each-chart-of-a-stack-at-the-same-cell.md) |
| `stack_column` | The same two charts, under the rule `align = "column"`. | [Start each chart of a stack at the same cell](../user/how-to/start-each-chart-of-a-stack-at-the-same-cell.md) |
| `variables` | The row of the variables, and the panels that a choice changes. | [Let a person select a value](../user/how-to/let-a-person-select-a-value.md) |
| `too_small` | A terminal that is too small for one panel of its dashboard. | [Read a dashboard on a small terminal](../user/how-to/read-a-dashboard-on-a-small-terminal.md) |
| `rows` | The row panels, which collapse and expand. | [Group panels under row panels](../user/how-to/group-panels-under-row-panels.md) |

The two scenes of the stack hold a dashboard of their own. The script writes
that dashboard to a file, and it gives the path of that file to the program.
The two dashboards differ in one line, which is the line that states the rule.

## The rules of a picture

- **No picture shows data of the machine that records it.** Each scene reads
  the data source testdata, or data that its program makes itself. The script
  stops before it records a scene that runs `thunderbox` with no dashboard, or
  a scene whose dashboard declares the data source `sysinfo`. The `app` scene
  therefore draws the layout of the dashboard inside the program, with the data
  source testdata in place of `sysinfo`.
- **The font matters.** A recorder draws a box in place of each character that
  its font does not hold. Unifont and FreeMono both hold every braille
  character. DejaVu Sans Mono and JetBrains Mono hold none of them.
- **Each picture stays under 200 kilobytes.** `--fps-cap 10` holds the size
  down, and so does a short list of steps. No command measures a picture, so
  measure it yourself after you record a scene.
- **Each picture appears in one page of the user documentation.** The table
  above names that page. A new scene adds a row to that table, and its picture
  to the page that matches it.
