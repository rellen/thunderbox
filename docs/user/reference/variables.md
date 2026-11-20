# Variables

A variable holds a value that a person changes while the program runs. A target
refers to that value, and the value reaches the data source in the place of the
reference.

The guide [Let a person select a value](../how-to/let-a-person-select-a-value.md)
adds a variable to a dashboard. This dashboard file lets a person select the
query of the data source `testdata`:

```toml
title = "a variable"

[source.test]
kind = "testdata"

[[variable]]
name = "signal"
label = "the signal"
options = ["random_walk", "sine", "ramp", "gappy"]
current = "random_walk"

[[panel]]
title = "the signal that you selected"
kind = "timeseries"
grid = { x = 0, y = 0, w = 24, h = 10 }

[[panel.target]]
source = "test"
expr = "$signal"
```

## The fields of a variable

A dashboard declares each variable in a `[[variable]]` table.

| field | type | default | what it states |
|---|---|---|---|
| `name` | text | required | The name that a target refers to. A target writes `$name`. |
| `label` | text | the name | The name that the row of the variables shows. |
| `options` | list of texts | none | The fixed choices. |
| `query` | inline table | none | The query that holds the choices. |
| `current` | text | none | The choice that is active when the program starts. |

The inline table `query` holds two fields:

| field | type | what it states |
|---|---|---|
| `source` | text | The name of a data source of the dashboard. |
| `expr` | text | The text of the query. The data source reads this text in its own way. |

The program ignores any other field of the table `query`.

## The rules of the fields

- A name starts with a letter or an underscore. It then holds letters, digits,
  and underscores. The letters are the letters of the English alphabet.
- Two variables cannot carry one name.
- A variable states `options` or `query`, and never both.
- A variable that states `options` can state a `current` value. That value must
  be one of the options.
- A variable that states `query` can state any `current` value.
- The `expr` field of the `query` must hold no reference to a variable.
- The `source` of the `query` must name a data source that the dashboard
  declares. The program makes this check only when the dashboard declares one
  data source or more.

The page [The dashboard file](dashboard-file.md#the-second-step-dashboardvalidate)
lists the text of each error.

## The value of a variable

- A variable that states `options` takes its `current` value when the program
  starts. It takes the first option when it states no `current` value.
- A variable that states `query` holds its `current` value until the data
  source answers. It holds no value until then when it states no `current`
  value.
- thunderbox asks the data source for the choices one time, when the program
  starts.
- The answer then sets the value. The variable keeps its value when the choices
  hold that value. It takes the first choice in every other condition.
- A value that the answer changes starts a new set of queries.
- A variable whose data source returns no choice holds no value. The row of the
  variables then shows `no choice` for it.
- A variable whose data source returns an error keeps its value. The status row
  then names the variable and the reason.

## The choices that a data source holds

| kind | the choices | the text of `expr` |
|---|---|---|
| `sysinfo` | The name of each query of the data source, and `processes` last. | The data source ignores the text. |
| `prometheus` | Every value of one label. | The name of the label, as `instance`. |
| `testdata` | No choice. The status row reports the reason. | The data source ignores the text. |

The status row writes this sentence for a variable whose data source returns
an error:

```
The variable "NAME" has no choice. REASON
```

## The form of a reference

| text | what reaches the data source |
|---|---|
| `$name` | The value of the variable `name`. |
| `$$` | One `$`. The text refers to nothing. |
| any other `$` | The `$` itself. |

These rules apply to a reference:

- A reference ends at the first character that no name holds. A person
  therefore writes `up{job="$job"}` with no mark around the name.
- Two references can stand together. The text `$job$region` holds two
  references.
- The text `$1` reaches the data source without a change, because no name
  starts with a digit.
- The text `$$job` writes `$job`, and it refers to nothing.
- The program reads the text one time. A value that holds a reference reaches
  the data source as it is. One variable therefore cannot reach another.

A person writes `$$` for a query language that reads `$` in its own way.

## The places that a reference reaches

- A reference reaches the `expr` field of a target, and no other field.
- The `legend` field holds no reference. The page
  [Legend patterns](legend-patterns.md) states the rule of that field.
- A `$` in the title of a panel stays as it is.
- The program rejects a reference to a variable that the dashboard does not
  declare.
- A target that refers to a variable which holds no value runs no query. The
  panel then shows the error `The variable "NAME" holds no value.`

## The keys and the row of the variables

| key | what the key does |
|---|---|
| `v` | Select the next variable of the dashboard. |
| `]` | Take the choice that follows, on the variable that you selected. |
| `[` | Take the choice before it, on the variable that you selected. |

- The program shows the row of the variables above the panels of a dashboard
  that declares a variable. The row names each variable and its value.
- A change of a value returns every panel to `loading…`, and it starts a new
  set of queries.
- The page [The command line](command-line.md#the-row-of-the-variables) states
  the form of the row.

## The pages that give the reason

- [Architecture decision record 0040](../../contributor/adr/0040-a-reference-to-a-variable-reaches-the-query-alone.md)
  states why a reference reaches the query alone.
- [Architecture decision record 0042](../../contributor/adr/0042-the-runtime-asks-for-the-choices-of-a-variable-once.md)
  states why thunderbox asks for the choices one time.
- [Architecture decision record 0044](../../contributor/adr/0044-the-row-of-variables-holds-short-labels.md)
  states why the row of the variables holds short labels.
