//! The rule that puts the value of a variable into the text of a query.
//!
//! A person declares a variable in a `[[variable]]` table of a dashboard, and
//! that person then selects one of its choices. A target refers to the
//! variable by its name, and [`apply`] puts the selected value in the place of
//! that reference. The data source therefore reads the value, and it never
//! reads the name.
//!
//! The rule has three parts:
//!
//! - the character `$`, and then the name of a variable, is a reference. The
//!   rule puts the value of that variable in the place of the reference;
//! - the text `$$` writes one `$`, and it refers to nothing. A person writes
//!   `$$` for a query language that reads `$` in its own way;
//! - every other `$` stays as it is.
//!
//! A name starts with a letter or an underscore. It then holds letters,
//! digits, and underscores. The letters are the letters of the English
//! alphabet. A reference therefore ends at the first character that no name
//! holds, and a person writes `up{job="$job"}` with no mark around the name.
//!
//! The rule reads the text one time. A value that itself holds a reference
//! reaches the data source as it is. The rule therefore cannot loop, and one
//! variable cannot reach another.
//!
//! The rule reaches the `expr` field of a target, and it reaches no other
//! field. A pattern in the `legend` field therefore holds no reference to a
//! variable. A pattern names a series, and [`crate::legend`] holds that rule
//! alone.
//!
//! The rule keeps a reference that names no value as plain text.
//! [`crate::Dashboard::validate`] rejects a target that names a variable which
//! the dashboard does not declare. A panel therefore meets this condition only
//! for a variable that holds no value.

use std::collections::BTreeMap;

/// The character that starts a reference to a variable.
const MARK: char = '$';

/// One part of the text of a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part<'a> {
    /// Text that holds no reference.
    Plain(&'a str),
    /// The text `$$`, which writes one `$`.
    Mark,
    /// A reference to the variable of this name.
    Reference(&'a str),
}

/// Returns the text with the value of each variable in the place of its name.
///
/// The function puts the value of a variable in the place of each reference to
/// that variable. It keeps a reference that `values` does not hold as plain
/// text. It writes one `$` for each `$$`.
///
/// # Examples
///
/// ```
/// use std::collections::BTreeMap;
/// use thunderbox_core::variable;
///
/// let mut values = BTreeMap::new();
/// values.insert("job".to_string(), "api".to_string());
/// assert_eq!(variable::apply(r#"up{job="$job"}"#, &values), r#"up{job="api"}"#);
/// // The dashboard declares no variable of this name, so the text stands.
/// assert_eq!(variable::apply("$nope", &values), "$nope");
/// ```
pub fn apply(text: &str, values: &BTreeMap<String, String>) -> String {
    if !text.contains(MARK) {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    for part in parts(text) {
        match part {
            Part::Plain(plain) => out.push_str(plain),
            Part::Mark => out.push(MARK),
            Part::Reference(name) => match values.get(name) {
                Some(value) => out.push_str(value),
                None => {
                    out.push(MARK);
                    out.push_str(name);
                }
            },
        }
    }
    out
}

/// Returns the name of each variable that the text refers to.
///
/// The list holds one entry for each reference, and it keeps the order of the
/// text. A text that refers to one variable two times therefore gives a list
/// of two entries. [`crate::Dashboard::validate`] reads this list, and it
/// checks each name against the variables that the dashboard declares.
pub fn references(text: &str) -> Vec<&str> {
    if !text.contains(MARK) {
        return Vec::new();
    }
    parts(text)
        .into_iter()
        .filter_map(|part| match part {
            Part::Reference(name) => Some(name),
            _ => None,
        })
        .collect()
}

/// Returns `true` if a query can refer to a variable of this name.
///
/// A name starts with a letter or an underscore, and it then holds letters,
/// digits, and underscores. An empty text is not a name.
pub fn is_name(name: &str) -> bool {
    !name.is_empty() && name_length(name) == name.len()
}

/// Returns the parts of the text, in the order of the text.
fn parts(text: &str) -> Vec<Part<'_>> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find(MARK) {
        if at > 0 {
            out.push(Part::Plain(&rest[..at]));
        }
        let after = &rest[at + MARK.len_utf8()..];
        if let Some(second) = after.strip_prefix(MARK) {
            out.push(Part::Mark);
            rest = second;
            continue;
        }
        let length = name_length(after);
        if length == 0 {
            // No name follows the mark, so the mark itself is plain text.
            out.push(Part::Plain(&rest[at..at + MARK.len_utf8()]));
            rest = after;
            continue;
        }
        out.push(Part::Reference(&after[..length]));
        rest = &after[length..];
    }
    if !rest.is_empty() {
        out.push(Part::Plain(rest));
    }
    out
}

/// Returns the number of bytes of the name that starts this text.
///
/// The function returns 0 when the text starts with no name.
fn name_length(text: &str) -> usize {
    let mut length = 0;
    for (at, letter) in text.char_indices() {
        let holds = if at == 0 {
            letter.is_ascii_alphabetic() || letter == '_'
        } else {
            letter.is_ascii_alphanumeric() || letter == '_'
        };
        if !holds {
            break;
        }
        length = at + letter.len_utf8();
    }
    length
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the values that most tests in this module read.
    fn values() -> BTreeMap<String, String> {
        let mut values = BTreeMap::new();
        values.insert("job".to_string(), "api".to_string());
        values.insert("region".to_string(), "eu".to_string());
        values
    }

    #[test]
    fn apply_puts_the_value_of_a_variable_in_the_place_of_its_name() {
        assert_eq!(apply("$job", &values()), "api");
    }

    #[test]
    fn apply_ends_a_reference_at_the_first_character_that_no_name_holds() {
        assert_eq!(
            apply(r#"up{job="$job",region="$region"}"#, &values()),
            r#"up{job="api",region="eu"}"#
        );
    }

    #[test]
    fn apply_reads_a_reference_that_stands_beside_another_reference() {
        assert_eq!(apply("$job$region", &values()), "apieu");
    }

    #[test]
    fn apply_returns_a_text_that_holds_no_reference_without_a_change() {
        assert_eq!(apply("up{job=\"api\"}", &values()), "up{job=\"api\"}");
    }

    #[test]
    fn apply_keeps_a_reference_that_names_no_value_as_plain_text() {
        assert_eq!(apply("$nope and $job", &values()), "$nope and api");
    }

    #[test]
    fn apply_writes_one_mark_for_two_marks() {
        assert_eq!(
            apply("select $$1, $job", &values()),
            "select $1, api",
            "A person writes two marks for a query language that gives one mark a meaning."
        );
        assert_eq!(apply("$$job", &values()), "$job");
    }

    #[test]
    fn apply_keeps_a_mark_that_starts_no_name() {
        assert_eq!(apply("100$ and $1", &values()), "100$ and $1");
        assert_eq!(apply("$", &values()), "$");
    }

    /// The rule reads the text one time. A value that holds a reference
    /// therefore reaches the data source as it is, and one variable cannot
    /// reach another.
    #[test]
    fn apply_reads_the_text_one_time() {
        let mut values = BTreeMap::new();
        values.insert("a".to_string(), "$b".to_string());
        values.insert("b".to_string(), "the value of b".to_string());
        assert_eq!(apply("$a", &values), "$b");
    }

    #[test]
    fn references_names_each_variable_of_the_text_in_order() {
        assert_eq!(
            references(r#"sum by ($region) (up{job="$job",region="$region"})"#),
            vec!["region", "job", "region"]
        );
    }

    #[test]
    fn references_reads_no_name_after_two_marks() {
        assert_eq!(references("$$job"), Vec::<&str>::new());
        assert_eq!(references("$$job $region"), vec!["region"]);
    }

    #[test]
    fn references_returns_an_empty_list_for_a_text_that_holds_no_mark() {
        assert_eq!(references("up"), Vec::<&str>::new());
        assert_eq!(references("$1 and 100$"), Vec::<&str>::new());
    }

    #[test]
    fn is_name_accepts_a_name_that_a_query_can_refer_to() {
        assert!(is_name("job"));
        assert!(is_name("_job"));
        assert!(is_name("job_2"));
        assert!(is_name("J"));
    }

    #[test]
    fn is_name_rejects_a_name_that_a_query_cannot_refer_to() {
        assert!(!is_name(""), "An empty name refers to nothing.");
        assert!(!is_name("2job"), "A name starts with a letter.");
        assert!(!is_name("my job"));
        assert!(!is_name("$job"));
        assert!(!is_name("{job}"));
        assert!(!is_name("jôb"), "A name holds the letters of English.");
    }

    /// A value can hold any character. The rule copies it, and it reads it no
    /// second time.
    #[test]
    fn apply_copies_a_value_that_holds_a_bracket_or_a_quotation_mark() {
        let mut values = BTreeMap::new();
        values.insert("job".to_string(), r#"a"b{c}"#.to_string());
        assert_eq!(apply("$job", &values), r#"a"b{c}"#);
    }
}
