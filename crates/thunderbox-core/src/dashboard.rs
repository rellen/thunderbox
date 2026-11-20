//! The dashboard document, and the parts that it holds.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{Error, Query, Reduce, Result, Threshold, TimeRange, variable};

/// The width of the layout grid, in grid columns.
///
/// A dashboard places each panel inside this grid. The renderer then scales the
/// grid to the width of the terminal.
pub const GRID_COLUMNS: u16 = 24;

/// A dashboard document.
///
/// The native file format is TOML. A later version can add a reader for the
/// Grafana JSON format. See `docs/ARCHITECTURE.md` section 12.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dashboard {
    /// The name of the dashboard. This name must not be empty.
    pub title: String,
    /// The time between two refreshes, as a duration. The value `5s` is an
    /// example.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh: Option<String>,
    /// The rule that gives one width to the charts of a stack.
    #[serde(default, skip_serializing_if = "Align::is_default")]
    pub align: Align,
    /// The time range that every panel shares.
    #[serde(default)]
    pub time: TimeRange,
    /// The data sources that this dashboard needs. The key is the name of the
    /// data source.
    #[serde(default, rename = "source", skip_serializing_if = "BTreeMap::is_empty")]
    pub sources: BTreeMap<String, SourceDecl>,
    /// The variables that a person can change from the dashboard.
    #[serde(default, rename = "variable", skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<Variable>,
    /// The panels, and their places on the grid.
    #[serde(default, rename = "panel")]
    pub panels: Vec<Panel>,
}

impl Dashboard {
    /// Reads a dashboard from TOML text and checks the content.
    ///
    /// # Errors
    ///
    /// This method returns an error if the text is not valid TOML, or if
    /// [`Dashboard::validate`] rejects the content.
    pub fn from_toml(s: &str) -> Result<Self> {
        let dash: Self = toml::from_str(s)?;
        dash.validate()?;
        Ok(dash)
    }

    /// Writes the dashboard as TOML text.
    ///
    /// # Errors
    ///
    /// This method returns an error if the dashboard holds a value that the
    /// TOML format cannot express.
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self).map_err(|e| Error::Dashboard(e.to_string()))
    }

    /// Checks that the renderer can show this dashboard.
    ///
    /// # Errors
    ///
    /// This method returns an error in each of these conditions:
    ///
    /// - the title is empty;
    /// - the refresh duration has a form that this crate cannot read;
    /// - a panel has a width or a height of 0;
    /// - a panel reaches past the right edge of the grid;
    /// - a panel sets a `min` that is not smaller than its `max`;
    /// - a panel names a data source that the dashboard does not declare, and
    ///   the dashboard declares at least one data source;
    /// - a variable has a name that no query can refer to;
    /// - two variables carry one name;
    /// - a variable states no choice;
    /// - a variable states its choices in two places;
    /// - a variable states a current value that is not one of its options;
    /// - a variable names a data source that the dashboard does not declare,
    ///   and the dashboard declares at least one data source;
    /// - the query of a variable refers to a variable;
    /// - a panel refers to a variable that the dashboard does not declare.
    ///
    /// Both checks read the data sources that the dashboard declares. This
    /// method therefore accepts any name in a dashboard that declares no data
    /// source. A program that embeds thunderbox can hold such a dashboard,
    /// because that program supplies its own data sources.
    /// `docs/ARCHITECTURE.md` section 12 states that rule, and it states how a
    /// wrong name then reaches the screen.
    pub fn validate(&self) -> Result<()> {
        if self.title.trim().is_empty() {
            return Err(Error::Dashboard("title must not be empty".into()));
        }
        if let Some(r) = &self.refresh {
            crate::time::parse_duration_ms(r)?;
        }
        let names = self.validate_variables()?;
        for panel in &self.panels {
            let g = &panel.grid;
            if g.w == 0 || g.h == 0 {
                return Err(Error::Dashboard(format!(
                    "panel {:?} has zero extent",
                    panel.title
                )));
            }
            // The addition saturates. A panel of `x = 65535` and `w = 1` would
            // otherwise end the program, and this function must return an error
            // for such a panel.
            if g.x.saturating_add(g.w) > GRID_COLUMNS {
                return Err(Error::Dashboard(format!(
                    "panel {:?} spans past the {GRID_COLUMNS}-column grid (x={} w={})",
                    panel.title, g.x, g.w
                )));
            }
            if let (Some(min), Some(max)) = (panel.min, panel.max)
                && min >= max
            {
                return Err(Error::Dashboard(format!(
                    "panel {:?} has min {min} and max {max}; min must be smaller than max",
                    panel.title
                )));
            }
            for target in &panel.targets {
                if !self.sources.is_empty() && !self.sources.contains_key(&target.source) {
                    return Err(Error::Dashboard(format!(
                        "panel {:?} refers to undeclared source {:?}",
                        panel.title, target.source
                    )));
                }
                for name in variable::references(&target.expr) {
                    if !names.contains(name) {
                        return Err(Error::Dashboard(format!(
                            "panel {:?} refers to undeclared variable {:?}",
                            panel.title, name
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// Checks every variable, and returns the name of each one.
    ///
    /// [`Dashboard::validate`] checks each target against the set that this
    /// method returns. A dashboard can therefore declare a variable after the
    /// panel that refers to it.
    ///
    /// # Errors
    ///
    /// This method returns an error in each of the conditions that
    /// [`Dashboard::validate`] states for a variable.
    fn validate_variables(&self) -> Result<BTreeSet<&str>> {
        let mut names: BTreeSet<&str> = BTreeSet::new();
        for var in &self.variables {
            if !variable::is_name(&var.name) {
                return Err(Error::Dashboard(format!(
                    "variable {:?} has a name that no query can refer to. A name holds \
                     letters, digits and underscores, and it starts with a letter or an \
                     underscore",
                    var.name
                )));
            }
            if !names.insert(var.name.as_str()) {
                return Err(Error::Dashboard(format!(
                    "the dashboard declares two variables that carry the name {:?}",
                    var.name
                )));
            }
            match (var.options.is_empty(), &var.query) {
                (true, None) => {
                    return Err(Error::Dashboard(format!(
                        "variable {:?} states no option and no query, so it holds no choice",
                        var.name
                    )));
                }
                (false, Some(_)) => {
                    return Err(Error::Dashboard(format!(
                        "variable {:?} states both options and a query. A variable takes \
                         its choices from one place",
                        var.name
                    )));
                }
                _ => {}
            }
            if let Some(current) = &var.current
                && !var.options.is_empty()
                && !var.options.iter().any(|option| option == current)
            {
                return Err(Error::Dashboard(format!(
                    "variable {:?} states the current value {current:?}, which is not one \
                     of its options",
                    var.name
                )));
            }
            if let Some(query) = &var.query {
                if !self.sources.is_empty() && !self.sources.contains_key(&query.source) {
                    return Err(Error::Dashboard(format!(
                        "variable {:?} refers to undeclared source {:?}",
                        var.name, query.source
                    )));
                }
                if let Some(other) = variable::references(&query.expr).first() {
                    return Err(Error::Dashboard(format!(
                        "variable {:?} refers to the variable {other:?}. The query of a \
                         variable refers to no variable",
                        var.name
                    )));
                }
            }
        }
        Ok(names)
    }
}

/// One data source that a dashboard declares.
///
/// This type takes an unknown key, and it puts that key in the `options` field.
/// Each type of data source reads its own options, and this crate knows none of
/// them. A data source returns an error for an option that it does not know.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceDecl {
    /// The type of the data source. The values `prometheus`, `sysinfo`, and
    /// `testdata` are examples.
    pub kind: String,
    /// The address of the data source, if the data source needs one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The other settings. Each type of data source reads its own settings.
    #[serde(default, flatten)]
    pub options: BTreeMap<String, toml::Value>,
}

/// A variable that a person selects, and that a query then uses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variable {
    /// The name that a query uses to refer to this variable.
    pub name: String,
    /// The name that the dashboard shows to a person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The fixed choices. The `query` field supplies the choices instead when
    /// the dashboard sets that field.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    /// The query that supplies the choices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<Query>,
    /// The choice that is now active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<String>,
}

/// One panel, and its place on the grid.
///
/// The `unit`, `decimals`, `min`, `max`, and `thresholds` fields repeat the
/// fields of [`crate::FieldConfig`]. A data source fills the column configuration. The
/// panel wins where the two disagree, because a person wrote the panel and a
/// machine wrote the column configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Panel {
    /// The name that the panel border shows.
    pub title: String,
    /// The type of the panel.
    pub kind: PanelKind,
    /// The name of the unit. The panel uses this name to format a value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// The number of decimal places that the panel shows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
    /// The smallest value that a meter shows. A gauge needs this value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// The largest value that a meter shows. A gauge needs this value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// The step that turns a column of numbers into the one number that a stat
    /// panel, a gauge panel, or a bar gauge panel shows.
    #[serde(default, skip_serializing_if = "Reduce::is_default")]
    pub reduce: Reduce,
    /// The place of the panel on the grid.
    pub grid: GridPos,
    /// The thresholds that change the colour of a value.
    #[serde(default, rename = "threshold", skip_serializing_if = "Vec::is_empty")]
    pub thresholds: Vec<Threshold>,
    /// The queries that supply the data for this panel.
    #[serde(default, rename = "target", skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<Target>,
}

/// One query that belongs to a panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    /// The name of the data source that answers this query.
    pub source: String,
    /// The query text. Only the data source reads this text.
    pub expr: String,
    /// The pattern that builds the name of each series.
    ///
    /// A name between two pairs of brace brackets reads a label of the column.
    /// The pattern `{{instance}}` is an example. The reserved word
    /// `{{__name__}}` reads the name of the column itself.
    /// [`crate::legend`] states the whole rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend: Option<String>,
}

impl Target {
    /// Returns this target as a [`Query`].
    ///
    /// The method puts the value of each variable into the text of the query.
    /// [`crate::variable::apply`] holds that rule, and the `values` parameter
    /// takes the value that each variable holds now. A data source therefore
    /// reads the value that a person selected, and it reads no name of a
    /// variable.
    ///
    /// The method takes those values as a parameter, and it reads no state of
    /// its own. A caller cannot build the query of a target without them, so
    /// no caller can send a name to a data source by mistake.
    ///
    /// The query holds no pattern. A data source answers the query, and
    /// [`crate::legend::apply`] then applies the pattern to that answer. The
    /// pattern belongs to the side that draws, so it never reaches a data
    /// source.
    pub fn to_query(&self, values: &BTreeMap<String, String>) -> Query {
        Query {
            source: self.source.clone(),
            expr: variable::apply(&self.expr, values),
        }
    }
}

/// The rule that gives one width to the charts of a stack.
///
/// A stack is the set of panels that hold the same columns of the grid, one
/// above another.
///
/// A chart keeps a column of cells at its left edge. That column holds the
/// labels of the axis of values. The chart takes the width of that column from
/// its own widest label. A panel that shows a share of use writes `100%`, and a
/// panel that shows a rate writes `8.3GB/s`. The second panel therefore draws
/// the narrower chart. A person who reads one panel above the other reads two
/// axes of time that start at different cells.
///
/// This rule states what a dashboard does about that.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    /// Each panel measures its own labels. This value is the default value.
    #[default]
    None,
    /// Every chart of a stack starts at the same cell.
    ///
    /// The renderer measures the labels of every panel of the stack. It then
    /// gives the widest of those measurements to each panel of that stack.
    /// Every axis of time of the stack then starts at the same cell, and a
    /// person reads one moment in one place.
    ///
    /// Two panels hold the same columns of the grid when they state the same
    /// `x` and the same `w`. Two panels of different widths keep their own
    /// measurements, because two such charts cannot start and end at the same
    /// two cells.
    Column,
}

impl Align {
    /// Returns `true` for the rule that a dashboard uses while it states no
    /// rule.
    ///
    /// The TOML writer asks this question. It leaves the field out of the
    /// document when the answer is `true`.
    fn is_default(&self) -> bool {
        matches!(self, Self::None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
/// The type of a panel. The type selects the renderer.
pub enum PanelKind {
    /// A chart of one or more values against time.
    TimeSeries,
    /// A single large number.
    Stat,
    /// One value, shown against a range.
    Gauge,
    /// One bar for each value, shown against a range.
    BarGauge,
    /// Rows and columns of values.
    Table,
}

/// The place of a panel on the grid.
///
/// The `x` and `w` fields count grid columns. The grid is [`GRID_COLUMNS`] wide.
/// The `y` and `h` fields count rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridPos {
    /// The distance from the left edge of the grid, in grid columns.
    pub x: u16,
    /// The distance from the top of the grid, in rows.
    pub y: u16,
    /// The width, in grid columns. This value must be 1 or more.
    pub w: u16,
    /// The height, in rows. This value must be 1 or more.
    pub h: u16,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RelativeTime;

    const SAMPLE: &str = r#"
title = "System"
refresh = "1s"

[time]
from = "now-5m"
to = "now"

[source.testdata]
kind = "testdata"

[[panel]]
title = "CPU"
kind = "timeseries"
unit = "percent"
grid = { x = 0, y = 0, w = 24, h = 8 }

[[panel.target]]
source = "testdata"
expr = "random_walk"
legend = "cpu {{series}}"
"#;

    #[test]
    fn from_toml_and_to_toml_keep_the_same_dashboard() {
        let dash = Dashboard::from_toml(SAMPLE).unwrap();
        assert_eq!(dash.title, "System");
        assert_eq!(dash.refresh.as_deref(), Some("1s"));
        assert_eq!(dash.time.from, RelativeTime::Offset(-300_000));
        assert_eq!(dash.panels.len(), 1);
        assert_eq!(dash.panels[0].kind, PanelKind::TimeSeries);
        assert_eq!(dash.panels[0].targets[0].expr, "random_walk");

        let round = Dashboard::from_toml(&dash.to_toml().unwrap()).unwrap();
        assert_eq!(round, dash);
    }

    #[test]
    fn a_dashboard_that_states_no_rule_lets_each_panel_measure_its_own_labels() {
        let dash = Dashboard::from_toml(SAMPLE).unwrap();
        assert_eq!(dash.align, Align::None);
        let written = dash
            .to_toml()
            .expect("The writer must accept the dashboard.");
        assert!(
            !written.contains("align"),
            "The writer leaves out the rule that a dashboard uses by default. {written}"
        );
    }

    #[test]
    fn a_dashboard_can_state_that_a_stack_of_charts_holds_one_width() {
        let text = r#"
title = "stack"
align = "column"

[[panel]]
title = "cpu"
kind = "timeseries"
grid = { x = 0, y = 0, w = 24, h = 8 }
"#;
        let dash = Dashboard::from_toml(text).expect("The reader must accept the rule.");
        assert_eq!(dash.align, Align::Column);
        let round = Dashboard::from_toml(&dash.to_toml().unwrap()).unwrap();
        assert_eq!(round, dash, "The rule must survive a round trip.");
    }

    #[test]
    fn a_dashboard_that_names_a_rule_which_does_not_exist_returns_an_error() {
        let text = r#"
title = "stack"
align = "row"
"#;
        assert!(
            Dashboard::from_toml(text).is_err(),
            "The reader knows two rules only."
        );
    }

    const METERS: &str = r#"
title = "meters"

[[panel]]
title = "load"
kind = "gauge"
unit = "percent"
min = 0.0
max = 100.0
decimals = 1
reduce = "mean"
grid = { x = 0, y = 0, w = 12, h = 6 }

[[panel.threshold]]
value = 80.0
color = "critical"
"#;

    #[test]
    fn from_toml_reads_the_settings_that_a_meter_needs() {
        let dash = Dashboard::from_toml(METERS).unwrap();
        let panel = &dash.panels[0];
        assert_eq!(panel.kind, PanelKind::Gauge);
        assert_eq!(panel.min, Some(0.0));
        assert_eq!(panel.max, Some(100.0));
        assert_eq!(panel.decimals, Some(1));
        assert_eq!(panel.reduce, crate::Reduce::Mean);
        assert_eq!(panel.thresholds.len(), 1);
        assert_eq!(panel.thresholds[0].color, "critical");

        let round = Dashboard::from_toml(&dash.to_toml().unwrap()).unwrap();
        assert_eq!(round, dash);
    }

    /// A panel holds two arrays of tables, which are the thresholds and the
    /// targets. The TOML writer must place both after every plain value.
    #[test]
    fn to_toml_writes_a_panel_that_holds_thresholds_and_targets() {
        let toml = r#"
title = "board"

[source.test]
kind = "testdata"

[[panel]]
title = "load"
kind = "bargauge"
unit = "percent"
min = 0.0
max = 100.0
reduce = "max"
grid = { x = 0, y = 0, w = 12, h = 6 }

[[panel.threshold]]
value = 50.0
color = "warning"

[[panel.threshold]]
value = 90.0
color = "critical"

[[panel.target]]
source = "test"
expr = "random_walk"
legend = "core {{series}}"
"#;
        let dash = Dashboard::from_toml(toml).unwrap();
        let written = dash.to_toml().expect("The writer must accept the panel.");
        let round = Dashboard::from_toml(&written).expect("The reader must accept the text.");
        assert_eq!(
            round, dash,
            "The text that the writer made is this. {written}"
        );
        assert_eq!(round.panels[0].thresholds.len(), 2);
        assert_eq!(round.panels[0].targets.len(), 1);
    }

    #[test]
    fn a_panel_that_names_no_step_uses_the_most_recent_value() {
        let dash = Dashboard::from_toml(SAMPLE).unwrap();
        assert_eq!(dash.panels[0].reduce, crate::Reduce::Last);
        assert!(dash.panels[0].thresholds.is_empty());
    }

    #[test]
    fn validate_returns_an_error_when_min_is_not_smaller_than_max() {
        let toml = r#"
title = "x"
[[panel]]
title = "meter"
kind = "gauge"
min = 100.0
max = 0.0
grid = { x = 0, y = 0, w = 4, h = 4 }
"#;
        let err = Dashboard::from_toml(toml).unwrap_err().to_string();
        assert!(
            err.contains("smaller than max"),
            "The message is wrong. {err}"
        );
    }

    #[test]
    fn validate_returns_an_error_when_a_panel_passes_the_grid_edge() {
        let toml = r#"
title = "x"
[[panel]]
title = "wide"
kind = "stat"
grid = { x = 20, y = 0, w = 8, h = 4 }
"#;
        let err = Dashboard::from_toml(toml).unwrap_err().to_string();
        assert!(err.contains("column grid"), "The message is wrong. {err}");
    }

    /// A panel of a very large `x` must return an error, and it must not end
    /// the program. The check adds `x` and `w`, and that addition passed the
    /// largest number that a `u16` holds before this rule.
    #[test]
    fn validate_returns_an_error_for_a_panel_that_starts_past_the_grid() {
        let toml = r#"
title = "x"
[[panel]]
title = "far"
kind = "stat"
grid = { x = 65535, y = 0, w = 1, h = 4 }
"#;
        let err = Dashboard::from_toml(toml).unwrap_err().to_string();
        assert!(err.contains("column grid"), "The message is wrong. {err}");
    }

    #[test]
    fn validate_returns_an_error_for_a_panel_of_zero_width() {
        let toml = r#"
title = "x"
[[panel]]
title = "flat"
kind = "stat"
grid = { x = 0, y = 0, w = 0, h = 4 }
"#;
        assert!(Dashboard::from_toml(toml).is_err());
    }

    #[test]
    fn validate_returns_an_error_for_an_undeclared_data_source() {
        let toml = r#"
title = "x"
[source.a]
kind = "testdata"
[[panel]]
title = "p"
kind = "stat"
grid = { x = 0, y = 0, w = 4, h = 4 }
[[panel.target]]
source = "b"
expr = "q"
"#;
        let err = Dashboard::from_toml(toml).unwrap_err().to_string();
        assert!(
            err.contains("undeclared source"),
            "The message is wrong. {err}"
        );
    }

    #[test]
    fn validate_returns_an_error_for_an_empty_title() {
        assert!(Dashboard::from_toml("title = \"  \"").is_err());
    }

    #[test]
    fn validate_returns_an_error_for_a_wrong_refresh_duration() {
        assert!(Dashboard::from_toml("title = \"x\"\nrefresh = \"soon\"").is_err());
    }

    #[test]
    fn a_source_keeps_the_options_that_its_own_type_reads() {
        let toml = r#"
title = "x"
[source.prom]
kind = "prometheus"
url = "http://127.0.0.1:9090"
min_step = "15s"
"#;
        let dash = Dashboard::from_toml(toml).unwrap();
        let source = &dash.sources["prom"];
        assert_eq!(source.kind, "prometheus");
        assert_eq!(source.url.as_deref(), Some("http://127.0.0.1:9090"));
        assert_eq!(
            source.options["min_step"].as_str(),
            Some("15s"),
            "Only the data source reads this option, so the dashboard must keep it."
        );
    }

    /// Returns the text of the error that a dashboard of one variable gives.
    ///
    /// The helper writes the title and one data source, so each test below
    /// holds the lines of its own variable alone.
    fn variable_error(body: &str) -> String {
        let text = format!("title = \"x\"\n[source.a]\nkind = \"testdata\"\n{body}");
        Dashboard::from_toml(&text)
            .expect_err("The dashboard must hold a fault.")
            .to_string()
    }

    #[test]
    fn a_dashboard_reads_a_variable_and_the_value_that_it_holds() {
        let toml = r#"
title = "x"
[source.a]
kind = "testdata"

[[variable]]
name = "job"
label = "the job"
options = ["web", "db"]
current = "web"

[[panel]]
title = "p"
kind = "stat"
grid = { x = 0, y = 0, w = 4, h = 4 }
[[panel.target]]
source = "a"
expr = "up{job=\"$job\"}"
"#;
        let dash = Dashboard::from_toml(toml).expect("The dashboard must be correct.");
        assert_eq!(dash.variables.len(), 1);
        assert_eq!(dash.variables[0].name, "job");
        assert_eq!(dash.variables[0].label.as_deref(), Some("the job"));
        assert_eq!(dash.variables[0].options, vec!["web", "db"]);
        assert_eq!(dash.variables[0].current.as_deref(), Some("web"));

        let round = Dashboard::from_toml(&dash.to_toml().unwrap()).unwrap();
        assert_eq!(round, dash);
    }

    #[test]
    fn to_query_puts_the_value_of_a_variable_into_the_text_of_the_query() {
        let target = Target {
            source: "a".into(),
            expr: r#"up{job="$job"}"#.into(),
            legend: None,
        };
        let mut values = BTreeMap::new();
        values.insert("job".to_string(), "web".to_string());
        assert_eq!(target.to_query(&values).expr, r#"up{job="web"}"#);
        assert_eq!(
            target.to_query(&BTreeMap::new()).expr,
            r#"up{job="$job"}"#,
            "The rule keeps a reference that names no value as plain text."
        );
    }

    #[test]
    fn validate_returns_an_error_for_a_variable_that_states_no_choice() {
        let error = variable_error("[[variable]]\nname = \"job\"\n");
        assert!(
            error.contains("no option and no query"),
            "The message is wrong. {error}"
        );
    }

    #[test]
    fn validate_returns_an_error_for_a_variable_that_states_its_choices_two_times() {
        let error = variable_error(
            "[[variable]]\nname = \"job\"\noptions = [\"web\"]\n\
             query = { source = \"a\", expr = \"job\" }\n",
        );
        assert!(error.contains("one place"), "The message is wrong. {error}");
    }

    #[test]
    fn validate_returns_an_error_for_a_variable_name_that_no_query_can_refer_to() {
        for name in ["", "my job", "2job", "$job"] {
            let error = variable_error(&format!(
                "[[variable]]\nname = \"{name}\"\noptions = [\"a\"]\n"
            ));
            assert!(
                error.contains("no query can refer to"),
                "The name {name:?} must not pass. {error}"
            );
        }
    }

    /// `Snapshot` holds the value of each variable under its name. Two
    /// variables of one name would give one of them no place there.
    #[test]
    fn validate_returns_an_error_for_two_variables_that_carry_one_name() {
        let error = variable_error(
            "[[variable]]\nname = \"job\"\noptions = [\"a\"]\n\
             [[variable]]\nname = \"job\"\noptions = [\"b\"]\n",
        );
        assert!(
            error.contains("two variables"),
            "The message is wrong. {error}"
        );
    }

    #[test]
    fn validate_returns_an_error_for_a_current_value_that_is_not_an_option() {
        let error =
            variable_error("[[variable]]\nname = \"job\"\noptions = [\"web\"]\ncurrent = \"db\"\n");
        assert!(
            error.contains("not one of its options"),
            "The message is wrong. {error}"
        );
    }

    #[test]
    fn validate_accepts_a_current_value_of_a_variable_that_states_a_query() {
        let toml = "title = \"x\"\n[source.a]\nkind = \"testdata\"\n\
                    [[variable]]\nname = \"job\"\n\
                    query = { source = \"a\", expr = \"job\" }\ncurrent = \"web\"\n";
        let dash = Dashboard::from_toml(toml)
            .expect("Only the data source knows the choices of such a variable.");
        assert_eq!(dash.variables[0].current.as_deref(), Some("web"));
    }

    #[test]
    fn validate_returns_an_error_for_a_variable_that_names_an_undeclared_source() {
        let error = variable_error(
            "[[variable]]\nname = \"job\"\nquery = { source = \"b\", expr = \"job\" }\n",
        );
        assert!(
            error.contains("undeclared source"),
            "A variable and a panel must report this fault with the same words. {error}"
        );
    }

    /// The rule reads the text of a query one time, so one variable cannot
    /// reach another. A dashboard that asks for that must report the fault.
    #[test]
    fn validate_returns_an_error_for_a_variable_query_that_refers_to_a_variable() {
        let error = variable_error(
            "[[variable]]\nname = \"job\"\noptions = [\"web\"]\n\
             [[variable]]\nname = \"host\"\n\
             query = { source = \"a\", expr = \"up{job=\\\"$job\\\"}\" }\n",
        );
        assert!(
            error.contains("refers to no variable"),
            "The message is wrong. {error}"
        );
    }

    #[test]
    fn validate_returns_an_error_for_a_panel_that_refers_to_an_undeclared_variable() {
        let error = variable_error(
            "[[panel]]\ntitle = \"p\"\nkind = \"stat\"\n\
             grid = { x = 0, y = 0, w = 4, h = 4 }\n\
             [[panel.target]]\nsource = \"a\"\nexpr = \"up{job=\\\"$job\\\"}\"\n",
        );
        assert!(
            error.contains("undeclared variable"),
            "The message is wrong. {error}"
        );
    }

    /// A dashboard declares a variable after the panel that refers to it. The
    /// order of the document must not decide whether the dashboard is
    /// correct.
    #[test]
    fn validate_reads_a_variable_that_the_dashboard_declares_after_the_panel() {
        let toml = r#"
title = "x"
[source.a]
kind = "testdata"

[[panel]]
title = "p"
kind = "stat"
grid = { x = 0, y = 0, w = 4, h = 4 }
[[panel.target]]
source = "a"
expr = "$signal"

[[variable]]
name = "signal"
options = ["sine"]
"#;
        assert!(Dashboard::from_toml(toml).is_ok());
    }

    #[test]
    fn from_toml_returns_an_error_for_an_unknown_key() {
        let err = Dashboard::from_toml("title = \"x\"\nnonsense = 1")
            .unwrap_err()
            .to_string();
        assert!(err.contains("nonsense"), "The message is wrong. {err}");
    }
}
