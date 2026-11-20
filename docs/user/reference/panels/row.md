# The row panel

The kind `row` draws one line across the whole grid. The row panel holds the
panels below it on the grid, up to the next row panel. A person collapses a row
panel to hide those panels.

The example below is a part of a dashboard file, and it does not load alone.
The file also needs a `title`, and a table `[source.machine]` of the kind
`sysinfo`.

```toml
[[panel]]
title = "network"
kind = "row"
collapsed = true
grid = { x = 0, y = 9, w = 24, h = 1 }

# The row panel above holds this panel, because the panel starts below it.
[[panel]]
title = "network in"
kind = "timeseries"
grid = { x = 0, y = 10, w = 24, h = 7 }

[[panel.target]]
source = "machine"
expr = "network.rx"
```

The guide
[Group panels under row panels](../../how-to/group-panels-under-row-panels.md)
adds row panels to a dashboard.

## The panels that a row panel holds

- A row panel holds each panel that starts below its line, up to the line of
  the next row panel.
- A panel above the first row panel belongs to no row panel.
- The file lists each panel beside the row panel, and never inside it.
- No panel can cross the line of a row panel. Such a panel would belong to two
  places, and the program rejects it.
- Two row panels cannot stand on one line.

## What the line holds

The line holds these parts, from the left to the right:

1. A mark: `▾` for a row panel that shows its panels, and `▸` for a row panel
   that hides them.
2. One space, and the title.
3. One space, and a rule of the character `─` to the right end of the line.
4. A collapsed row panel adds a note at the right end. The note counts the
   panels that the row panel holds. A row panel that holds no panel writes no
   note.

The mark and the rule are cyan when a person selects the row panel. They are
dark grey at every other time.

The line of a collapsed row panel looks like this:

```
▸ network ────── The row hides 3 panels.
```

## A collapse

- A collapsed row panel hides the panels that it holds.
- The renderer is the part of thunderbox that draws the panels. It then gives
  no line to the rows of the grid that those panels held. The panels below the
  row panel move higher on the screen, and every panel that remains grows.
- thunderbox asks no data source about a hidden panel.
- A panel that appears starts a new set of queries at once.
- The key `Tab` skips each hidden panel.
- The field `collapsed` states the first state alone. The program never writes
  the file.

## The keys

| key | what the key does |
|---|---|
| `Tab`, `Shift+Tab` | Select a row panel, as any other panel. |
| `Enter`, `Space` | Collapse or expand the row panel that you selected. |

The page [The command line](../command-line.md#the-keys) lists every key.

## The fields that the panel reads

| field | what the panel does with it |
|---|---|
| `title` | The line shows this title. |
| `grid` | The line stands at the row `y` of the grid. The field must state `x = 0`, `w = 24`, and `h = 1`. |
| `collapsed` | The row panel starts collapsed when this field holds `true`. The default value is `false`. |

- The program rejects a target on a row panel, because a row panel shows no
  data.
- The program rejects the field `collapsed` on a panel of every other kind.
- A row panel ignores `unit`, `decimals`, `min`, `max`, `reduce`, and the
  thresholds.

## The texts that the panel can draw

| text | condition |
|---|---|
| `The row hides 1 panel.` or `The row hides N panels.` | The row panel is collapsed, and the sentence fits the line. |
| `+N` | The row panel is collapsed, and the sentence does not fit the line. |

A row panel draws no `loading…`, no error, and no `no data`, because it shows no
data.

## The panel at a small size

- A row panel takes one line of the terminal, whatever the height of the
  terminal.
- A row panel needs one line and one cell. It draws nothing in a smaller place.
- A terminal can hold fewer lines than the dashboard holds row panels. The last
  row panels then take no line, and every other panel takes no height.
- The status row counts a row panel that takes no line as a panel that the
  renderer drops.
- The line cuts a title that is too long. A full stop marks the cut.

## The pages that give the reason

- [Row panels and the place of a panel](../../explanation/row-panels-and-the-place-of-a-panel.md)
  explains why a row panel holds panels by their place.
- [Architecture decision record 0050](../../../contributor/adr/0050-a-row-panel-holds-panels-by-their-place.md)
  states the decision of the place.
- [Architecture decision record 0051](../../../contributor/adr/0051-a-row-panel-takes-one-line.md)
  states why a row panel takes one line.
