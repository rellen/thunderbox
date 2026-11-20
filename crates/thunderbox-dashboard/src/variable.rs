//! The choices of each variable, and the rule that maps a key to a choice.
//!
//! A dashboard declares a variable, and a person then selects one of its
//! choices. [`Variables`] holds the choices of every variable of one
//! dashboard, and it holds the value that each one carries now.
//! [`crate::DashboardRuntime`] owns that state, and it puts the values into
//! the text of each query.
//!
//! A variable takes its choices from one of two places. A variable that states
//! `options` holds them from the first moment. A variable that states a
//! `query` takes them from a data source, and the runtime asks that data
//! source one time when it starts.

use std::collections::BTreeMap;

use thunderbox_core::Variable;

use crate::view::Key;

/// The text that the row writes for a variable that holds no value.
///
/// The row of the variables holds one short label for each variable. The row
/// of the keys of a program holds one short label for each key, and this label
/// is of that kind. Neither row holds a sentence.
///
/// The complete sentence for this condition names the variable and the fault.
/// [`Variables::error`] returns that sentence, and a program writes it in a
/// place that holds a sentence. The status row of the `thunderbox` program is
/// one such place.
const NO_CHOICE: &str = "no choice";

/// The state of one variable.
///
/// The dashboard declares the name, the label, and the fixed options. The
/// runtime fills the choices of a variable that states a query, and it holds
/// the value that a person selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableState {
    /// The name that a query refers to.
    name: String,
    /// The name that a person reads, if the dashboard states one.
    label: Option<String>,
    /// The choices that a person can select.
    choices: Vec<String>,
    /// The choice that is active now.
    value: Option<String>,
    /// The error of the data source that holds the choices.
    error: Option<String>,
}

impl VariableState {
    /// Returns the name that a query refers to.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the name that a person reads.
    ///
    /// The method returns the `label` field of the dashboard, and it returns
    /// the name of the variable when the dashboard states no label.
    pub fn label(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.name)
    }

    /// Returns the choices that a person can select.
    pub fn choices(&self) -> &[String] {
        &self.choices
    }

    /// Returns the value that this variable holds now.
    ///
    /// The method returns `None` for a variable that holds no choice. A
    /// variable that takes its choices from a data source holds no value until
    /// that data source answers.
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    /// Returns the error of the data source that holds the choices.
    ///
    /// The method returns `None` for a variable that states fixed options, and
    /// for a variable whose data source answered.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

/// The answer of a data source for the choices of one variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Choices {
    /// The data source returned these choices.
    Found(Vec<String>),
    /// The data source did not answer, and this text states the reason.
    Failed(String),
}

/// The variables of one dashboard, and the value that each one holds.
///
/// The type holds one cursor beside the variables. The cursor names the
/// variable that a key changes, and [`variable_key`] moves it. A dashboard
/// that declares several variables therefore needs no key for each one.
///
/// The type is a value. A caller reads it from
/// [`crate::DashboardRuntime::variables`], it asks [`variable_key`] for a new
/// one, and it gives that new one back to
/// [`crate::DashboardRuntime::set_variables`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Variables {
    /// The state of each variable, in the order of the dashboard.
    states: Vec<VariableState>,
    /// The position of the variable that a key changes.
    cursor: usize,
}

impl Variables {
    /// Returns the state of every variable that a dashboard declares.
    ///
    /// A variable that states options takes the value of its `current` field,
    /// and it takes the first option when the dashboard states no such field.
    /// A variable that states a query holds no choice yet, and it takes the
    /// value of its `current` field alone.
    ///
    /// [`crate::DashboardRuntime::new`] calls this method. A program that
    /// holds a dashboard reads the state from
    /// [`crate::DashboardRuntime::variables`], and it needs this method only
    /// to build a state of its own.
    pub fn new(declared: &[Variable]) -> Self {
        let states = declared
            .iter()
            .map(|var| VariableState {
                name: var.name.clone(),
                label: var.label.clone(),
                choices: var.options.clone(),
                value: var.current.clone().or_else(|| var.options.first().cloned()),
                error: None,
            })
            .collect();
        Self { states, cursor: 0 }
    }

    /// Returns the state of each variable, in the order of the dashboard.
    pub fn states(&self) -> &[VariableState] {
        &self.states
    }

    /// Returns the number of variables that the dashboard declares.
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Returns `true` if the dashboard declares no variable.
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    /// Returns the position of the variable that a key changes.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Returns the value that each variable holds now.
    ///
    /// The map holds no entry for a variable that holds no value.
    /// [`thunderbox_core::variable::apply`] then keeps each reference to that
    /// variable as plain text, and the panel that holds the reference reports
    /// the fault.
    pub fn values(&self) -> BTreeMap<String, String> {
        self.states
            .iter()
            .filter_map(|state| state.value.clone().map(|value| (state.name.clone(), value)))
            .collect()
    }

    /// Returns the sentence for the first variable that holds an error.
    ///
    /// A program writes this sentence in the place where it writes every other
    /// sentence about a fault. The status row of the `thunderbox` program is one
    /// such place.
    pub fn error(&self) -> Option<String> {
        self.states.iter().find_map(|state| {
            state
                .error
                .as_ref()
                .map(|error| format!("The variable {:?} has no choice. {error}", state.name))
        })
    }

    /// Returns the row that names each variable and the value that it holds.
    ///
    /// The row writes the label of each variable, and then the value. It
    /// writes two spaces between two variables, and it puts the variable at
    /// the cursor between two square brackets. A program draws this row in one
    /// line, above or below the panels.
    ///
    /// The row writes `no choice` for a variable that holds no value.
    /// [`Variables::error`] returns the sentence that states the reason.
    pub fn row_text(&self) -> String {
        self.states
            .iter()
            .enumerate()
            .map(|(at, state)| {
                let value = state.value.as_deref().unwrap_or(NO_CHOICE);
                let part = format!("{}: {value}", state.label());
                if at == self.cursor {
                    format!("[{part}]")
                } else {
                    part
                }
            })
            .collect::<Vec<_>>()
            .join("  ")
    }

    /// Takes the choices that the data sources returned.
    ///
    /// The method keeps the value that the dashboard stated when the choices
    /// hold that value. It takes the first choice in every other condition,
    /// and it holds no value when the data source returned no choice.
    ///
    /// The method keeps the value of a variable whose data source returned an
    /// error. A person wrote that value, and the runtime has nothing better to
    /// put in its place.
    pub(crate) fn take_choices(&mut self, loaded: Vec<(String, Choices)>) {
        for (name, choices) in loaded {
            let Some(state) = self.states.iter_mut().find(|state| state.name == name) else {
                continue;
            };
            match choices {
                Choices::Found(found) => {
                    let keep = state
                        .value
                        .as_ref()
                        .is_some_and(|value| found.contains(value));
                    if !keep {
                        state.value = found.first().cloned();
                    }
                    state.choices = found;
                    state.error = None;
                }
                Choices::Failed(error) => state.error = Some(error),
            }
        }
    }

    /// Returns this state with the cursor on the variable that follows.
    ///
    /// The cursor returns to the first variable after the last one.
    fn with_the_next_variable(&self) -> Self {
        let mut next = self.clone();
        if !self.states.is_empty() {
            next.cursor = (self.cursor + 1) % self.states.len();
        }
        next
    }

    /// Returns this state with another choice on the variable at the cursor.
    ///
    /// The `step` parameter is 1 for the choice that follows, and it is -1 for
    /// the choice before. The choice returns to the first one after the last
    /// one, and to the last one before the first one. The method takes the
    /// first choice for a variable whose value is not one of its choices.
    fn with_another_choice(&self, step: i64) -> Self {
        let mut next = self.clone();
        let Some(state) = next.states.get_mut(self.cursor) else {
            return next;
        };
        let count = i64::try_from(state.choices.len()).unwrap_or(i64::MAX);
        if count == 0 {
            return next;
        }
        let at = state
            .choices
            .iter()
            .position(|choice| Some(choice) == state.value.as_ref());
        let chosen = match at {
            Some(at) => (i64::try_from(at).unwrap_or(0) + step).rem_euclid(count),
            None => 0,
        };
        state.value = usize::try_from(chosen)
            .ok()
            .and_then(|chosen| state.choices.get(chosen))
            .cloned();
        next
    }
}

/// Returns the selection that a key asks for, or `None` for another key.
///
/// thunderbox consumes no key of a variable.
/// [`crate::DashboardView::handle_key`] reports `Consumed::No` for every key
/// below, so a program that wants those keys for itself keeps them. This
/// function offers a default map from a key to a selection, and it holds the
/// rule alone. The program decides which key reaches it, and a program that
/// wants another map ignores this function. ADR 0043 in `docs/contributor/adr` states
/// that rule.
///
/// The function reads these keys:
///
/// - `v` moves the cursor to the variable that follows;
/// - `]` takes the choice that follows, on the variable at the cursor;
/// - `[` takes the choice before, on the variable at the cursor.
///
/// The function returns `None` for a key that asks for the state that the
/// caller already holds. A dashboard that declares one variable therefore
/// reads no answer from `v`, and a variable that holds one choice reads none
/// from `[` and `]`.
///
/// The caller passes the result to
/// [`crate::DashboardRuntime::set_variables`]. That method starts a new set of
/// queries only when a value changed, so a move of the cursor asks a data
/// source for nothing.
pub fn variable_key(key: Key, variables: &Variables) -> Option<Variables> {
    let asked = match key {
        Key::Char('v') => variables.with_the_next_variable(),
        Key::Char(']') => variables.with_another_choice(1),
        Key::Char('[') => variables.with_another_choice(-1),
        _ => return None,
    };
    (asked != *variables).then_some(asked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use thunderbox_core::Query;

    /// Returns a variable that states fixed options.
    fn declared(name: &str, options: &[&str], current: Option<&str>) -> Variable {
        Variable {
            name: name.to_string(),
            label: None,
            options: options.iter().map(|o| (*o).to_string()).collect(),
            query: None,
            current: current.map(str::to_string),
        }
    }

    /// Returns a variable that takes its choices from a data source.
    fn from_a_query(name: &str, current: Option<&str>) -> Variable {
        Variable {
            name: name.to_string(),
            label: None,
            options: Vec::new(),
            query: Some(Query {
                source: "a".into(),
                expr: "job".into(),
            }),
            current: current.map(str::to_string),
        }
    }

    /// Returns the state of two variables that state fixed options.
    fn two() -> Variables {
        Variables::new(&[
            declared("job", &["web", "db"], Some("web")),
            declared("region", &["eu", "us"], None),
        ])
    }

    /// Returns the value of one variable after a key press.
    fn press(key: Key, variables: &Variables) -> Variables {
        variable_key(key, variables).expect("The key must change the selection.")
    }

    #[test]
    fn a_variable_that_states_no_current_value_takes_its_first_option() {
        let variables = two();
        assert_eq!(variables.states()[0].value(), Some("web"));
        assert_eq!(
            variables.states()[1].value(),
            Some("eu"),
            "A panel must hold a value from the first set of queries."
        );
    }

    #[test]
    fn values_names_each_variable_that_holds_a_value() {
        let values = two().values();
        assert_eq!(values["job"], "web");
        assert_eq!(values["region"], "eu");
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn values_holds_no_entry_for_a_variable_that_holds_no_value() {
        let variables = Variables::new(&[from_a_query("job", None)]);
        assert!(
            variables.values().is_empty(),
            "A data source has not answered yet."
        );
    }

    #[test]
    fn the_key_that_follows_takes_the_choice_that_follows() {
        let ahead = press(Key::Char(']'), &two());
        assert_eq!(ahead.states()[0].value(), Some("db"));
        assert_eq!(
            ahead.states()[1].value(),
            Some("eu"),
            "The key must change the variable at the cursor alone."
        );
    }

    #[test]
    fn the_choice_returns_to_the_first_one_after_the_last_one() {
        let ahead = press(Key::Char(']'), &press(Key::Char(']'), &two()));
        assert_eq!(ahead.states()[0].value(), Some("web"));
    }

    #[test]
    fn the_key_before_takes_the_choice_before() {
        let back = press(Key::Char('['), &two());
        assert_eq!(
            back.states()[0].value(),
            Some("db"),
            "The choice before the first one is the last one."
        );
    }

    #[test]
    fn the_key_of_a_variable_moves_the_cursor_to_the_variable_that_follows() {
        let moved = press(Key::Char('v'), &two());
        assert_eq!(moved.cursor(), 1);
        let round = press(Key::Char('v'), &moved);
        assert_eq!(
            round.cursor(),
            0,
            "The cursor returns to the first variable."
        );
        let ahead = press(Key::Char(']'), &moved);
        assert_eq!(ahead.states()[1].value(), Some("us"));
        assert_eq!(ahead.states()[0].value(), Some("web"));
    }

    /// The runtime starts a new set of queries for every state that differs.
    /// A key that changes nothing must therefore ask for nothing.
    #[test]
    fn a_key_that_changes_nothing_asks_for_no_selection() {
        let one = Variables::new(&[declared("job", &["web"], None)]);
        assert_eq!(
            variable_key(Key::Char('v'), &one),
            None,
            "The dashboard declares one variable, so the cursor cannot move."
        );
        assert_eq!(
            variable_key(Key::Char(']'), &one),
            None,
            "The variable holds one choice."
        );
        assert_eq!(variable_key(Key::Char('['), &one), None);
    }

    #[test]
    fn a_key_of_a_dashboard_that_declares_no_variable_asks_for_nothing() {
        let none = Variables::default();
        for key in [Key::Char('v'), Key::Char('['), Key::Char(']')] {
            assert_eq!(variable_key(key, &none), None);
        }
        assert!(none.is_empty());
        assert_eq!(none.len(), 0);
    }

    /// The program keeps every key that this function does not read. The view
    /// keeps the three keys that it binds, and the time range keeps five.
    #[test]
    fn another_key_changes_no_selection() {
        for key in [
            Key::Tab,
            Key::BackTab,
            Key::Left,
            Key::Right,
            Key::Other,
            Key::Char('m'),
            Key::Char('0'),
        ] {
            assert_eq!(variable_key(key, &two()), None);
        }
    }

    #[test]
    fn take_choices_holds_the_value_that_the_dashboard_states() {
        let mut variables = Variables::new(&[from_a_query("job", Some("db"))]);
        variables.take_choices(vec![(
            "job".into(),
            Choices::Found(vec!["web".into(), "db".into()]),
        )]);
        assert_eq!(
            variables.states()[0].value(),
            Some("db"),
            "A person wrote that value, and the data source holds it."
        );
        assert_eq!(variables.states()[0].choices().len(), 2);
    }

    #[test]
    fn take_choices_takes_the_first_choice_for_a_value_that_the_data_source_does_not_hold() {
        let mut variables = Variables::new(&[from_a_query("job", Some("nope"))]);
        variables.take_choices(vec![(
            "job".into(),
            Choices::Found(vec!["web".into(), "db".into()]),
        )]);
        assert_eq!(variables.states()[0].value(), Some("web"));
    }

    #[test]
    fn take_choices_holds_no_value_for_a_data_source_that_returned_no_choice() {
        let mut variables = Variables::new(&[from_a_query("job", Some("web"))]);
        variables.take_choices(vec![("job".into(), Choices::Found(Vec::new()))]);
        assert_eq!(variables.states()[0].value(), None);
        assert_eq!(variables.row_text(), "[job: no choice]");
    }

    #[test]
    fn take_choices_keeps_the_error_of_a_data_source() {
        let mut variables = Variables::new(&[from_a_query("job", Some("web"))]);
        variables.take_choices(vec![(
            "job".into(),
            Choices::Failed("The data source returned an error. no route to host".into()),
        )]);
        assert_eq!(
            variables.states()[0].value(),
            Some("web"),
            "The runtime has nothing better to put in the place of that value."
        );
        let sentence = variables.error().expect("The state must hold a sentence.");
        assert_eq!(
            sentence,
            "The variable \"job\" has no choice. The data source returned an error. \
             no route to host"
        );
    }

    #[test]
    fn the_row_names_each_variable_and_marks_the_one_at_the_cursor() {
        assert_eq!(two().row_text(), "[job: web]  region: eu");
        assert_eq!(
            press(Key::Char('v'), &two()).row_text(),
            "job: web  [region: eu]"
        );
    }

    #[test]
    fn the_row_writes_the_label_that_the_dashboard_states() {
        let mut declared = declared("job", &["web"], None);
        declared.label = Some("the job".into());
        let variables = Variables::new(&[declared]);
        assert_eq!(variables.row_text(), "[the job: web]");
        assert_eq!(
            variables.states()[0].name(),
            "job",
            "A query refers to the name, and never to the label."
        );
    }

    #[test]
    fn a_dashboard_that_declares_no_variable_writes_an_empty_row() {
        assert_eq!(Variables::default().row_text(), "");
        assert!(Variables::default().error().is_none());
    }
}
