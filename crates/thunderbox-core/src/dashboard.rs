//! The dashboard document, and the parts that it holds.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{Error, Query, Reduce, Result, Threshold, TimeRange, variable};

/// The width of the layout grid, in grid columns.
///
/// A dashboard places each panel inside this grid. The renderer then scales the
/// grid to the width of the terminal.
pub const GRID_COLUMNS: u16 = 24;

/// The names of the units that a panel can state in its field `unit`.
///
/// `thunderbox-widgets` holds one rule of format for each name, and a test of
/// that crate holds the two lists together. [`Dashboard::validate`] returns an
/// error for a panel that states any other name. The match is exact, so
/// `Percent` is not `percent`. ADR 0080 in `docs/contributor/adr` states the
/// reason.
///
/// A data source can state any unit in [`crate::FieldConfig`]. No dashboard
/// check reads that unit, and a name outside this list then takes the format
/// of a value with no unit.
///
/// # Examples
///
/// ```
/// use thunderbox_core::UNITS;
/// assert!(UNITS.contains(&"bytes/s"));
/// assert!(!UNITS.contains(&"Percent"));
/// ```
pub const UNITS: [&str; 4] = ["percent", "bytes", "bytes/s", "seconds"];

/// A dashboard document.
///
/// The native file format is TOML. A later version can add a reader for the
/// Grafana JSON format. See ADR 0014 in `docs/contributor/adr`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dashboard {
    /// The name of the dashboard. This name must not be empty.
    pub title: String,
    /// The time between two refreshes, as a duration. The value `5s` is an
    /// example. [`Dashboard::validate`] returns an error for a duration of 0. A
    /// dashboard with no such field does not refresh on a clock.
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
    /// - the refresh duration does not fit in a count of milliseconds;
    /// - an end of the `[time]` table is an offset whose sum with the present
    ///   moment does not fit in a count of milliseconds;
    /// - the time range is wider than [`crate::MAX_SPAN_MS`];
    /// - the time range is narrower than [`crate::MIN_SPAN_MS`];
    /// - the refresh duration is 0, in any unit;
    /// - a panel has a width or a height of 0;
    /// - a panel states a unit that [`UNITS`] does not hold;
    /// - a panel reaches past the right edge of the grid;
    /// - a panel sets a `min` that is not smaller than its `max`;
    /// - a panel that is not a top panel states a `sort` column;
    /// - a top panel states a `sort` column that has an empty name;
    /// - a panel that is not a table panel or a top panel states the field
    ///   `left`;
    /// - a panel states a `left` column that has an empty name;
    /// - a panel that is not a table panel or a top panel states the field
    ///   `widths`;
    /// - a panel states a width for a column that has an empty name;
    /// - a panel states the width 0 for a column;
    /// - a row panel does not span the whole width of the grid with one row;
    /// - a row panel holds a target;
    /// - a panel that is not a row panel states the field `collapsed`;
    /// - a panel crosses the line of a row panel;
    /// - two row panels stand on one line;
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
    /// ADR 0016 in `docs/contributor/adr` states that rule, and it states how a wrong name
    /// then reaches the screen.
    pub fn validate(&self) -> Result<()> {
        if self.title.trim().is_empty() {
            return Err(Error::Dashboard("title must not be empty".into()));
        }
        if let Some(r) = &self.refresh
            && crate::time::parse_duration_ms(r)? == 0
        {
            return Err(Error::Dashboard(format!(
                "The field refresh holds {r:?}. The value must be more than 0. A \
                 dashboard with no field refresh does not refresh on a clock."
            )));
        }
        self.time.validate(crate::time::now_ms())?;
        let names = self.validate_variables()?;
        self.validate_rows()?;
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
            // A unit that no rule knows would take the format of a value with
            // no unit. The panel would then look like a data source that
            // answers wrongly.
            if let Some(unit) = &panel.unit
                && !UNITS.contains(&unit.as_str())
            {
                return Err(Error::Dashboard(format!(
                    "The panel {:?} states the unit {unit:?}, and no rule knows that unit. \
                     The known units are {}.",
                    panel.title,
                    UNITS.join(", ")
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
            if let Some(sort) = &panel.sort {
                // A panel of another kind ignores the field. A person who
                // writes it on such a panel expects an order that the panel
                // never draws.
                if panel.kind != PanelKind::Top {
                    return Err(Error::Dashboard(format!(
                        "panel {:?} states the sort column {sort:?}. Only a panel of the \
                         kind top reads that field",
                        panel.title
                    )));
                }
                if sort.trim().is_empty() {
                    return Err(Error::Dashboard(format!(
                        "panel {:?} states a sort column with an empty name",
                        panel.title
                    )));
                }
            }
            if !panel.left.is_empty() {
                // A panel of another kind draws no column of numbers. A person
                // who writes the field on such a panel expects a change that
                // the panel never draws.
                if !matches!(panel.kind, PanelKind::Table | PanelKind::Top) {
                    return Err(Error::Dashboard(format!(
                        "panel {:?} states the field left. Only a panel of the kind table \
                         or top reads that field",
                        panel.title
                    )));
                }
                if panel.left.iter().any(|name| name.trim().is_empty()) {
                    return Err(Error::Dashboard(format!(
                        "panel {:?} states a left column with an empty name",
                        panel.title
                    )));
                }
            }
            if !panel.widths.is_empty() {
                // A panel of another kind draws no column. A person who writes
                // the field on such a panel expects a change that the panel
                // never draws.
                if !matches!(panel.kind, PanelKind::Table | PanelKind::Top) {
                    return Err(Error::Dashboard(format!(
                        "panel {:?} states the field widths. Only a panel of the kind table \
                         or top reads that field",
                        panel.title
                    )));
                }
                if panel.widths.keys().any(|name| name.trim().is_empty()) {
                    return Err(Error::Dashboard(format!(
                        "panel {:?} states a width for a column with an empty name",
                        panel.title
                    )));
                }
                if let Some((name, _)) = panel.widths.iter().find(|(_, width)| **width == 0) {
                    return Err(Error::Dashboard(format!(
                        "panel {:?} states the width 0 for the column {name:?}. A width must \
                         be 1 or more",
                        panel.title
                    )));
                }
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

    /// Checks each row panel, and each panel against the line of a row panel.
    ///
    /// A row panel holds the panels below it by their place on the grid. A
    /// panel that crossed the line of a row panel would belong to two places,
    /// so this method rejects it. A row panel spans the whole width with one
    /// row of the grid, because the renderer draws it as one line across the
    /// screen.
    ///
    /// # Errors
    ///
    /// This method returns an error in each of the conditions that
    /// [`Dashboard::validate`] states for a row panel.
    fn validate_rows(&self) -> Result<()> {
        for panel in &self.panels {
            let is_row = panel.kind == PanelKind::Row;
            if panel.collapsed.is_some() && !is_row {
                return Err(Error::Dashboard(format!(
                    "panel {:?} states the field collapsed. Only a panel of the kind row \
                     reads that field",
                    panel.title
                )));
            }
            if !is_row {
                continue;
            }
            let g = &panel.grid;
            if g.x != 0 || g.w != GRID_COLUMNS || g.h != 1 {
                return Err(Error::Dashboard(format!(
                    "row {:?} must span the whole grid with one row, as in \
                     grid = {{ x = 0, y = {}, w = {GRID_COLUMNS}, h = 1 }}",
                    panel.title, g.y
                )));
            }
            if !panel.targets.is_empty() {
                return Err(Error::Dashboard(format!(
                    "row {:?} holds a target. A row panel shows no data",
                    panel.title
                )));
            }
            for other in &self.panels {
                if std::ptr::eq(other, panel) {
                    continue;
                }
                let o = &other.grid;
                // The other panel covers the rows from `o.y` up to
                // `o.y + o.h`. The addition saturates, as the check of the
                // width does.
                if o.y <= g.y && g.y < o.y.saturating_add(o.h) {
                    return Err(Error::Dashboard(format!(
                        "panel {:?} crosses the line of the row {:?} at y = {}",
                        other.title, panel.title, g.y
                    )));
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
    /// The name of the column that puts the rows of a top panel in order.
    ///
    /// The panel puts the row that holds the largest number of that column
    /// first. The name is the text that the header of the column shows. A top
    /// panel that states no column puts its rows in order by its first column
    /// of numbers.
    ///
    /// Only a panel of the kind [`PanelKind::Top`] reads this field.
    /// [`Dashboard::validate`] rejects the field on every other kind of panel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    /// The headers of the columns that stand at the left of their cells.
    ///
    /// A table panel and a top panel stand a column of numbers at the right,
    /// header and cells. Each name in this list is the text that the header of
    /// a column shows. The panel stands that column at the left, even when the
    /// column holds numbers. A name that no header shows has no effect.
    ///
    /// Only a panel of the kind [`PanelKind::Table`] or [`PanelKind::Top`]
    /// reads this field. [`Dashboard::validate`] rejects the field on every
    /// other kind of panel.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub left: Vec<String>,
    /// The number of cells that each named column takes.
    ///
    /// The key is the text that the header of a column shows. The value is the
    /// number of cells of that column. A table panel and a top panel give a
    /// column of this table exactly that width. The columns that state no width
    /// share the rest of the panel in equal shares. An entry here replaces the
    /// `width` that the data source states for the column. A name that no
    /// header shows has no effect.
    ///
    /// Only a panel of the kind [`PanelKind::Table`] or [`PanelKind::Top`]
    /// reads this field. [`Dashboard::validate`] rejects the field on every
    /// other kind of panel. It also rejects a width of 0 and a name that is
    /// empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub widths: BTreeMap<String, u16>,
    /// States whether a row panel starts collapsed.
    ///
    /// A collapsed row panel hides the panels that it holds, and the panels
    /// below it move up. A person can collapse and expand a row panel while
    /// the program runs. This field states the first state alone, and the
    /// program never writes the file.
    ///
    /// Only a panel of the kind [`PanelKind::Row`] reads this field.
    /// [`Dashboard::validate`] rejects the field on every other kind of panel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
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
    /// A list of rows in order by one column, with the largest value first.
    ///
    /// The program `top` shows the processes of a machine in this form. The
    /// panel reads the field [`Panel::sort`] for the column that puts the rows
    /// in order.
    Top,
    /// A line across the whole grid that holds the panels below it.
    ///
    /// A row panel holds each panel below it on the grid, up to the next row
    /// panel. The file lists those panels beside the row panel, and never
    /// inside it. A person collapses a row panel to hide the panels that it
    /// holds, and the panels below then move up. The field
    /// [`Panel::collapsed`] states the first state.
    ///
    /// A row panel shows no data, so it holds no target. It takes one line of
    /// the terminal, whatever the height of the terminal.
    Row,
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

    /// Returns the text of a dashboard that holds one panel of this kind. The
    /// `sort` argument holds the line of the field, or an empty text.
    fn one_panel(kind: &str, sort: &str) -> String {
        format!(
            r#"
title = "x"
[[panel]]
title = "processes"
kind = "{kind}"
{sort}
grid = {{ x = 0, y = 0, w = 12, h = 8 }}
"#
        )
    }

    #[test]
    fn a_top_panel_reads_the_column_that_puts_its_rows_in_order() {
        let dash = Dashboard::from_toml(&one_panel("top", r#"sort = "memory""#))
            .expect("The reader must accept a top panel that names a column.");
        assert_eq!(dash.panels[0].kind, PanelKind::Top);
        assert_eq!(dash.panels[0].sort.as_deref(), Some("memory"));
        let round = Dashboard::from_toml(&dash.to_toml().unwrap()).unwrap();
        assert_eq!(round, dash, "The column must survive a round trip.");
    }

    #[test]
    fn a_top_panel_can_state_no_column() {
        let dash = Dashboard::from_toml(&one_panel("top", ""))
            .expect("The panel then takes its first column of numbers.");
        assert_eq!(dash.panels[0].sort, None);
        let written = dash.to_toml().unwrap();
        assert!(
            !written.contains("sort"),
            "The writer leaves out a field that the panel does not state. {written}"
        );
    }

    /// Every other kind of panel ignores the field. A person who writes it on
    /// such a panel expects an order that the panel never draws.
    #[test]
    fn validate_returns_an_error_for_a_sort_column_on_a_panel_that_is_not_a_top_panel() {
        for kind in ["timeseries", "stat", "gauge", "bargauge", "table"] {
            let err = Dashboard::from_toml(&one_panel(kind, r#"sort = "cpu""#))
                .unwrap_err()
                .to_string();
            assert!(
                err.contains("kind top"),
                "The message must name the kind that reads the field. {kind} {err}"
            );
        }
    }

    #[test]
    fn validate_returns_an_error_for_a_sort_column_with_an_empty_name() {
        let err = Dashboard::from_toml(&one_panel("top", r#"sort = " ""#))
            .unwrap_err()
            .to_string();
        assert!(err.contains("empty name"), "The message is wrong. {err}");
    }

    #[test]
    fn a_table_panel_and_a_top_panel_read_the_columns_that_stand_at_the_left() {
        for kind in ["table", "top"] {
            let dash = Dashboard::from_toml(&one_panel(kind, r#"left = ["pid", "name"]"#))
                .expect("The reader must accept a panel that names its left columns.");
            assert_eq!(dash.panels[0].left, vec!["pid", "name"]);
            let round = Dashboard::from_toml(&dash.to_toml().unwrap()).unwrap();
            assert_eq!(round, dash, "The columns must survive a round trip.");
        }
    }

    #[test]
    fn a_panel_that_states_no_left_column_writes_no_field() {
        let dash = Dashboard::from_toml(&one_panel("table", "")).unwrap();
        assert!(dash.panels[0].left.is_empty());
        let written = dash.to_toml().unwrap();
        assert!(
            !written.contains("left"),
            "The writer leaves out a field that the panel does not state. {written}"
        );
    }

    #[test]
    fn validate_returns_an_error_for_a_left_column_on_a_panel_that_is_not_a_table_or_a_top() {
        for kind in ["timeseries", "stat", "gauge", "bargauge"] {
            let err = Dashboard::from_toml(&one_panel(kind, r#"left = ["pid"]"#))
                .unwrap_err()
                .to_string();
            assert_eq!(
                err,
                "The dashboard is not correct. panel \"processes\" states the field left. \
                 Only a panel of the kind table or top reads that field",
                "{kind}"
            );
        }
    }

    #[test]
    fn validate_returns_an_error_for_a_left_column_with_an_empty_name() {
        for names in [r#"[""]"#, r#"["pid", "  "]"#] {
            let err = Dashboard::from_toml(&one_panel("top", &format!("left = {names}")))
                .unwrap_err()
                .to_string();
            assert_eq!(
                err,
                "The dashboard is not correct. panel \"processes\" states a left column \
                 with an empty name",
                "{names}"
            );
        }
    }

    #[test]
    fn a_table_panel_and_a_top_panel_read_the_widths_of_their_columns() {
        for kind in ["table", "top"] {
            let dash =
                Dashboard::from_toml(&one_panel(kind, "widths = { pid = 6, cpu = 4, MemB = 4 }"))
                    .expect("The reader must accept a panel that states its widths.");
            assert_eq!(dash.panels[0].widths.get("pid"), Some(&6));
            assert_eq!(dash.panels[0].widths.get("MemB"), Some(&4));
            let round = Dashboard::from_toml(&dash.to_toml().unwrap()).unwrap();
            assert_eq!(round, dash, "The widths must survive a round trip.");
        }
    }

    #[test]
    fn a_panel_that_states_no_widths_writes_no_field() {
        let dash = Dashboard::from_toml(&one_panel("table", "")).unwrap();
        assert!(dash.panels[0].widths.is_empty());
        let written = dash.to_toml().unwrap();
        assert!(
            !written.contains("widths"),
            "The writer leaves out a field that the panel does not state. {written}"
        );
    }

    #[test]
    fn validate_returns_an_error_for_widths_on_a_panel_that_is_not_a_table_or_a_top() {
        for kind in ["timeseries", "stat", "gauge", "bargauge"] {
            let err = Dashboard::from_toml(&one_panel(kind, "widths = { pid = 6 }"))
                .unwrap_err()
                .to_string();
            assert_eq!(
                err,
                "The dashboard is not correct. panel \"processes\" states the field widths. \
                 Only a panel of the kind table or top reads that field",
                "{kind}"
            );
        }
    }

    #[test]
    fn validate_returns_an_error_for_a_width_of_a_column_with_an_empty_name() {
        for names in [r#""""#, r#""  ""#] {
            let err =
                Dashboard::from_toml(&one_panel("top", &format!("widths = {{ {names} = 6 }}")))
                    .unwrap_err()
                    .to_string();
            assert_eq!(
                err,
                "The dashboard is not correct. panel \"processes\" states a width for a column \
                 with an empty name",
                "{names}"
            );
        }
    }

    #[test]
    fn validate_returns_an_error_for_a_width_of_0() {
        let err = Dashboard::from_toml(&one_panel("table", "widths = { pid = 6, cpu = 0 }"))
            .unwrap_err()
            .to_string();
        assert_eq!(
            err,
            "The dashboard is not correct. panel \"processes\" states the width 0 for the \
             column \"cpu\". A width must be 1 or more"
        );
    }

    /// A dashboard of two row panels. The first one holds the chart, and the
    /// second one starts collapsed and holds the table.
    const ROWS: &str = r#"
title = "rows"
[[panel]]
title = "now"
kind = "stat"
grid = { x = 0, y = 0, w = 24, h = 4 }
[[panel]]
title = "processor"
kind = "row"
grid = { x = 0, y = 4, w = 24, h = 1 }
[[panel]]
title = "cpu"
kind = "timeseries"
grid = { x = 0, y = 5, w = 24, h = 6 }
[[panel]]
title = "details"
kind = "row"
collapsed = true
grid = { x = 0, y = 11, w = 24, h = 1 }
[[panel]]
title = "table"
kind = "table"
grid = { x = 0, y = 12, w = 24, h = 6 }
"#;

    #[test]
    fn a_dashboard_reads_a_row_panel_and_the_state_that_it_starts_in() {
        let dash = Dashboard::from_toml(ROWS).expect("The reader must accept row panels.");
        assert_eq!(dash.panels[1].kind, PanelKind::Row);
        assert_eq!(dash.panels[1].collapsed, None);
        assert_eq!(dash.panels[3].collapsed, Some(true));
        let round = Dashboard::from_toml(&dash.to_toml().unwrap()).unwrap();
        assert_eq!(round, dash, "The row panels must survive a round trip.");
    }

    /// Returns the error of the dashboard `ROWS` after one change to its text.
    fn rows_error(old: &str, new: &str) -> String {
        assert!(
            ROWS.contains(old),
            "The test must change a line that exists."
        );
        Dashboard::from_toml(&ROWS.replacen(old, new, 1))
            .unwrap_err()
            .to_string()
    }

    /// The renderer draws a row panel as one line across the screen. A row
    /// panel of another shape would draw a line that the file does not state.
    #[test]
    fn validate_returns_an_error_for_a_row_panel_that_does_not_span_the_grid() {
        for grid in [
            "grid = { x = 2, y = 4, w = 22, h = 1 }",
            "grid = { x = 0, y = 4, w = 12, h = 1 }",
            "grid = { x = 0, y = 4, w = 24, h = 2 }",
        ] {
            let err = rows_error("grid = { x = 0, y = 4, w = 24, h = 1 }", grid);
            assert!(
                err.contains("whole grid"),
                "The message must state the shape. {grid} {err}"
            );
        }
    }

    #[test]
    fn validate_returns_an_error_for_a_row_panel_that_holds_a_target() {
        let err = rows_error(
            "grid = { x = 0, y = 4, w = 24, h = 1 }",
            "grid = { x = 0, y = 4, w = 24, h = 1 }\n[[panel.target]]\nsource = \"s\"\nexpr = \"cpu\"",
        );
        assert!(err.contains("shows no data"), "The message is wrong. {err}");
    }

    /// Every other kind of panel ignores the field. A person who writes it on
    /// such a panel expects a collapse that the panel never draws.
    #[test]
    fn validate_returns_an_error_for_the_field_collapsed_on_a_panel_that_is_not_a_row() {
        let err = rows_error("kind = \"table\"", "kind = \"table\"\ncollapsed = false");
        assert!(err.contains("kind row"), "The message is wrong. {err}");
    }

    /// A row panel holds the panels below it by their place on the grid. A
    /// panel that crosses the line of a row panel would belong to two places.
    #[test]
    fn validate_returns_an_error_for_a_panel_that_crosses_the_line_of_a_row_panel() {
        let err = rows_error(
            "grid = { x = 0, y = 5, w = 24, h = 6 }",
            "grid = { x = 0, y = 5, w = 24, h = 7 }",
        );
        assert!(
            err.contains("crosses the line of the row \"details\""),
            "The message must name the panel and the row panel. {err}"
        );
    }

    #[test]
    fn validate_returns_an_error_for_two_row_panels_on_one_line() {
        let err = rows_error(
            "grid = { x = 0, y = 11, w = 24, h = 1 }",
            "grid = { x = 0, y = 4, w = 24, h = 1 }",
        );
        assert!(
            err.contains("crosses the line"),
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

    /// Returns the text of a dashboard that holds the given `from` value in
    /// the `[time]` table.
    fn dashboard_with_from(from: &str) -> String {
        format!("title = \"x\"\n[time]\nfrom = \"{from}\"\n")
    }

    #[test]
    fn validate_returns_an_error_for_a_refresh_duration_that_does_not_fit() {
        let err = Dashboard::from_toml("title = \"x\"\nrefresh = \"20000000000w\"")
            .expect_err("The reader must not accept a duration that does not fit.")
            .to_string();
        assert!(err.contains("20000000000w"), "The text is {err}");
    }

    #[test]
    fn validate_returns_an_error_for_a_start_that_does_not_fit() {
        let err = Dashboard::from_toml(&dashboard_with_from("now-20000000000w"))
            .expect_err("The reader must not accept a start that does not fit.")
            .to_string();
        assert!(err.contains("20000000000w"), "The text is {err}");
    }

    #[test]
    fn validate_returns_an_error_for_a_range_wider_than_ten_years() {
        let err = Dashboard::from_toml(&dashboard_with_from("now-3651d"))
            .expect_err("The reader must not accept a range above the widest range.")
            .to_string();
        assert!(
            err.contains("`from` field of now-3651d"),
            "The text is {err}"
        );
        assert!(err.contains("wider than 3650 days"), "The text is {err}");
    }

    #[test]
    fn validate_accepts_a_range_of_ten_years() {
        assert!(Dashboard::from_toml(&dashboard_with_from("now-3650d")).is_ok());
    }

    #[test]
    fn validate_returns_an_error_for_a_range_narrower_than_one_second() {
        let err = Dashboard::from_toml(&dashboard_with_from("now-500ms"))
            .expect_err("The reader must not accept a range below the narrowest range.")
            .to_string();
        assert!(err.contains("narrower than 1000 milliseconds"), "{err}");
    }

    #[test]
    fn validate_accepts_a_range_of_one_second() {
        assert!(Dashboard::from_toml(&dashboard_with_from("now-1s")).is_ok());
    }

    /// Returns the text of a dashboard that states this refresh duration.
    fn refresh_text(refresh: &str) -> String {
        format!("title = \"x\"\nrefresh = \"{refresh}\"\n")
    }

    /// A person who writes `0s` asks for the fastest refresh. The runtime would
    /// read the value as no refresh at all, so the check returns an error.
    #[test]
    fn validate_returns_an_error_that_names_refresh_for_a_refresh_of_0() {
        for refresh in ["0s", "0ms", "0m"] {
            let error = Dashboard::from_toml(&refresh_text(refresh))
                .expect_err("A refresh of 0 is a fault.")
                .to_string();
            assert!(
                error.contains("refresh") && error.contains("more than 0"),
                "The message for {refresh} is wrong. {error}"
            );
        }
    }

    #[test]
    fn validate_accepts_a_refresh_above_0_and_a_dashboard_with_no_refresh() {
        for refresh in ["1ms", "1s"] {
            assert!(
                Dashboard::from_toml(&refresh_text(refresh)).is_ok(),
                "The refresh {refresh} is correct."
            );
        }
        assert!(
            Dashboard::from_toml("title = \"x\"\n").is_ok(),
            "A dashboard with no refresh is correct."
        );
    }

    /// Returns the text of a dashboard whose one panel states this unit.
    fn unit_text(unit: &str) -> String {
        format!(
            "title = \"x\"\n[[panel]]\ntitle = \"cpu\"\nkind = \"stat\"\n\
             grid = {{ x = 0, y = 0, w = 6, h = 4 }}\nunit = \"{unit}\"\n"
        )
    }

    #[test]
    fn validate_accepts_each_unit_of_the_list() {
        for unit in UNITS {
            assert!(
                Dashboard::from_toml(&unit_text(unit)).is_ok(),
                "The unit {unit} is in the list, so it is correct."
            );
        }
    }

    /// A unit that no rule knows would draw no letter of a unit. The match is
    /// exact, so a name that differs in case alone also returns an error.
    #[test]
    fn validate_returns_an_error_that_names_a_unit_that_no_rule_knows() {
        for unit in ["byte", "Percent", "bytes/sec", "", "furlongs"] {
            let error = Dashboard::from_toml(&unit_text(unit))
                .expect_err("A unit that no rule knows is a fault.")
                .to_string();
            assert!(
                error.contains(&format!("{unit:?}")),
                "The message must name the unit {unit:?}. {error}"
            );
            assert!(
                error.contains("\"cpu\""),
                "The message must name the panel. {error}"
            );
            for name in UNITS {
                assert!(
                    error.contains(name),
                    "The message must list the unit {name}. {error}"
                );
            }
        }
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
