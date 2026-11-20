//! The rule that builds the name of a series from a pattern.
//!
//! A person writes a pattern in the `legend` field of a target. The dashboard
//! applies the pattern after the data source answers. A data source never
//! reads a pattern, because [`crate::Query`] holds no pattern.
//!
//! The rule has two parts. A name between two pairs of brace brackets reads a
//! label of the column. The reserved word [`RESERVED_NAME`] reads the name of
//! the column itself.
//!
//! This module gives the name of the column one reserved word, and not two.
//! One name for one thing keeps the rule short. A person then learns one rule,
//! and that rule serves every data source.
//!
//! A person can write a pattern that is not correct. The text between the two
//! pairs of brackets is the name of a label, and the name of a label holds no
//! brace bracket. The rule therefore keeps such a pattern as plain text, from
//! that pair of brackets to the end. The pattern `{{{instance}}}` is an
//! example, and a panel shows that text without a change.
//!
//! The rule is idempotent, which means that a second call gives the same
//! answer as the first call. The private function `render` reads the name of
//! the column and its labels. The function [`apply`] then writes the result
//! into the display name of the column configuration. Neither function reads
//! the display name, and neither function removes a label. A second call of
//! [`apply`] therefore gives the same name as the first call.
//!
//! The rule renames a column of numbers only. A pattern names a series, and a
//! series is a column of numbers. A time column and a text column keep the
//! name that the data source gave them.

use crate::{Field, Frame};

/// The reserved word that reads the name of the column.
///
/// A person writes `{{__name__}}` in a pattern, and the rule then reads
/// [`Field::name`]. The rule reads no label of this name. Prometheus holds the
/// name of a metric in a label of this name. The Prometheus data source moves
/// that value into the name of the column.
///
/// A data source cannot always give one column one name. It then gives every
/// column of one answer the same name, and this reserved word gives every
/// series that same name. A person writes the name of a label in the pattern
/// for such a query.
pub const RESERVED_NAME: &str = "__name__";

/// Applies the pattern of a target to every column of numbers.
///
/// The function writes the name that the pattern builds into the
/// `display_name` field of the column configuration. [`Field::display_name`]
/// then returns that name, and every panel shows it.
///
/// The function returns at once when `pattern` is `None`. The function
/// therefore does no work for a target that states no pattern.
///
/// The function changes no name when the pattern gives an empty text. The
/// pattern `{{nope}}` on a series that holds no label of that name is an
/// example. A panel then shows the name that the data source gave.
///
/// # Examples
///
/// ```
/// use thunderbox_core::{Field, Frame, legend};
///
/// let mut frames = vec![Frame::new(vec![
///     Field::f64("up", vec![Some(1.0)]).with_label("job", "api"),
/// ])];
/// legend::apply(Some("{{__name__}} on {{job}}"), &mut frames);
/// assert_eq!(frames[0].fields[0].display_name(), "up on api");
/// ```
pub fn apply(pattern: Option<&str>, frames: &mut [Frame]) {
    let Some(pattern) = pattern else {
        return;
    };
    for frame in frames {
        for field in frame.fields.iter_mut().filter(|f| f.is_numeric()) {
            if let Some(name) = render(pattern, field) {
                field.config.display_name = Some(name);
            }
        }
    }
}

/// Returns the name that a pattern builds for one column.
///
/// The pattern holds a name between two pairs of brace brackets. The pattern
/// `{{instance}}` is an example. The function puts the value of that label in
/// the place of the name. The name [`RESERVED_NAME`] reads [`Field::name`]
/// instead, and it reads no label.
///
/// The function puts an empty text in the place of a label that the series
/// does not hold. Grafana does the same. The function returns `None` when the
/// whole result is empty, and the caller then keeps the name that the column
/// already has.
///
/// The function keeps a pattern that is not correct as plain text. A pattern
/// that holds no second pair of brackets is one example. A pattern that holds
/// a brace bracket in the name of a label is the other example.
fn render(pattern: &str, field: &Field) -> Option<String> {
    let mut out = String::with_capacity(pattern.len());
    let mut rest = pattern;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let Some(close) = after.find("}}") else {
            // No second pair of brackets follows. The rest is plain text.
            out.push_str(&rest[open..]);
            return finish(out);
        };
        let name = after[..close].trim();
        if name.contains(['{', '}']) {
            // The name of a label holds no brace bracket, so a person wrote
            // this pattern wrongly. The rest is plain text.
            out.push_str(&rest[open..]);
            return finish(out);
        }
        if name == RESERVED_NAME {
            out.push_str(&field.name);
        } else if let Some(value) = field.labels.get(name) {
            out.push_str(value);
        }
        rest = &after[close + 2..];
    }
    out.push_str(rest);
    finish(out)
}

/// Returns the name, or `None` when the name holds no character.
fn finish(name: String) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FieldConfig, Values};

    /// Returns the column that most tests in this module read.
    fn series() -> Field {
        Field::f64("up", vec![Some(1.0)])
            .with_label("instance", "web-1:9100")
            .with_label("job", "node")
    }

    #[test]
    fn render_puts_the_value_of_a_label_in_the_place_of_its_name() {
        assert_eq!(
            render("{{instance}}", &series()).as_deref(),
            Some("web-1:9100")
        );
    }

    #[test]
    fn render_keeps_the_text_around_a_label() {
        assert_eq!(
            render("cpu of {{job}} on {{instance}}", &series()).as_deref(),
            Some("cpu of node on web-1:9100")
        );
    }

    #[test]
    fn render_reads_the_name_of_the_column_for_the_reserved_word() {
        assert_eq!(render("{{__name__}}", &series()).as_deref(), Some("up"));
    }

    #[test]
    fn render_ignores_the_spaces_around_the_name_of_a_label() {
        assert_eq!(render("{{ job }}", &series()).as_deref(), Some("node"));
    }

    #[test]
    fn render_puts_an_empty_text_in_the_place_of_a_label_that_is_not_there() {
        assert_eq!(render("a{{nope}}b", &series()).as_deref(), Some("ab"));
    }

    #[test]
    fn render_returns_none_when_the_result_holds_no_character() {
        assert_eq!(render("{{nope}}", &series()), None);
        assert_eq!(render("   ", &series()), None);
    }

    #[test]
    fn render_keeps_a_pattern_that_has_no_second_pair_of_brackets() {
        assert_eq!(render("cpu {{job", &series()).as_deref(), Some("cpu {{job"));
    }

    #[test]
    fn render_keeps_a_pattern_that_holds_a_brace_bracket_in_the_name_of_a_label() {
        assert_eq!(
            render("{{{instance}}}", &series()).as_deref(),
            Some("{{{instance}}}"),
            "A person who writes one bracket too many must read that text back."
        );
        assert_eq!(
            render("{{a{{b}}}}", &series()).as_deref(),
            Some("{{a{{b}}}}")
        );
        assert_eq!(
            render("cpu of {{job}} and {{{instance}}}", &series()).as_deref(),
            Some("cpu of node and {{{instance}}}")
        );
    }

    #[test]
    fn render_returns_a_pattern_that_holds_no_label_without_a_change() {
        assert_eq!(render("total", &series()).as_deref(), Some("total"));
    }

    #[test]
    fn render_reads_the_name_of_the_column_and_never_a_label_of_that_name() {
        let field = Field::f64("cpu", vec![Some(1.0)]).with_label(RESERVED_NAME, "node_cpu");
        assert_eq!(
            render("{{__name__}}", &field).as_deref(),
            Some("cpu"),
            "The reserved word has one meaning, which is the name of the column."
        );
    }

    #[test]
    fn apply_reads_the_name_of_a_column_that_holds_no_label() {
        let mut frames = vec![Frame::new(vec![Field::f64("0", vec![Some(1.0)])])];
        apply(Some("core {{__name__}}"), &mut frames);
        assert_eq!(frames[0].fields[0].display_name(), "core 0");
    }

    #[test]
    fn apply_keeps_the_name_of_a_time_column() {
        let mut frames = vec![Frame::new(vec![
            Field::time("time", vec![0, 1000]),
            Field::f64("0", vec![Some(1.0), Some(2.0)]),
        ])];
        apply(Some("core {{__name__}}"), &mut frames);
        assert_eq!(
            frames[0].fields[0].display_name(),
            "time",
            "A table panel shows this name in the header of the time column."
        );
        assert_eq!(frames[0].fields[1].display_name(), "core 0");
    }

    #[test]
    fn apply_keeps_the_name_of_a_text_column() {
        let mut frames = vec![Frame::new(vec![Field {
            name: "message".into(),
            values: Values::Str(vec![Some("a".into())]),
            ..Default::default()
        }])];
        apply(Some("line {{__name__}}"), &mut frames);
        assert_eq!(frames[0].fields[0].display_name(), "message");
    }

    #[test]
    fn apply_overwrites_the_name_that_a_data_source_suggested() {
        let mut frames = vec![Frame::new(vec![
            Field::f64("up", vec![Some(1.0)])
                .with_label("job", "api")
                .with_config(FieldConfig {
                    display_name: Some("the answer of the data source".into()),
                    ..Default::default()
                }),
        ])];
        apply(Some("{{job}}"), &mut frames);
        assert_eq!(
            frames[0].fields[0].display_name(),
            "api",
            "A person wrote the pattern, so the pattern wins."
        );
    }

    #[test]
    fn apply_changes_nothing_when_the_target_states_no_pattern() {
        let mut frames = vec![Frame::new(vec![
            Field::f64("up", vec![Some(1.0)]).with_label("job", "api"),
        ])];
        apply(None, &mut frames);
        assert_eq!(frames[0].fields[0].display_name(), r#"up{job="api"}"#);
    }

    #[test]
    fn apply_keeps_the_name_of_a_data_source_for_a_pattern_that_gives_nothing() {
        let mut frames = vec![Frame::new(vec![
            Field::f64("up", vec![Some(1.0)]).with_config(FieldConfig {
                display_name: Some("the answer of the data source".into()),
                ..Default::default()
            }),
        ])];
        apply(Some("{{nope}}"), &mut frames);
        assert_eq!(
            frames[0].fields[0].display_name(),
            "the answer of the data source"
        );
    }

    #[test]
    fn apply_never_builds_a_name_from_a_name_that_it_built() {
        let mut frames = vec![Frame::new(vec![
            Field::f64("up", vec![Some(1.0)]).with_label("job", "{{instance}}"),
        ])];
        apply(Some("{{job}}"), &mut frames);
        assert_eq!(frames[0].fields[0].display_name(), "{{instance}}");
        apply(Some("{{job}}"), &mut frames);
        assert_eq!(
            frames[0].fields[0].display_name(),
            "{{instance}}",
            "The rule reads the name and the labels, and never the display name."
        );
    }

    #[test]
    fn apply_reaches_every_frame_and_every_column_of_numbers() {
        let mut frames = vec![
            Frame::new(vec![Field::f64("a", vec![Some(1.0)])]),
            Frame::new(vec![
                Field::f64("b", vec![Some(1.0)]),
                Field::f64("c", vec![Some(1.0)]),
            ]),
        ];
        apply(Some("s {{__name__}}"), &mut frames);
        assert_eq!(frames[0].fields[0].display_name(), "s a");
        assert_eq!(frames[1].fields[0].display_name(), "s b");
        assert_eq!(frames[1].fields[1].display_name(), "s c");
    }
}
