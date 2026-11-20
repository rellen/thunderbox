//! The replacement of the text between two markers.

/// The line that starts a generated table of crates.
pub const CRATES_BEGIN: &str = "<!-- crates:begin -->";
/// The line that ends a generated table of crates.
pub const CRATES_END: &str = "<!-- crates:end -->";

/// Replaces the text between the begin line and the end line with the table.
///
/// The new text holds the begin line, one empty line, the table, one empty
/// line, and the end line. The function returns `None` when the text holds no
/// marker.
///
/// # Errors
///
/// Returns an error when one marker has no partner, when a marker stands twice,
/// or when the end line stands before the begin line.
pub fn replace(text: &str, begin: &str, end: &str, table: &str) -> Result<Option<String>, String> {
    let lines: Vec<&str> = text.lines().collect();
    let begins: Vec<usize> = (0..lines.len()).filter(|&i| lines[i] == begin).collect();
    let ends: Vec<usize> = (0..lines.len()).filter(|&i| lines[i] == end).collect();
    if begins.len() > 1 {
        return Err(format!("The text holds more than one line `{begin}`."));
    }
    if ends.len() > 1 {
        return Err(format!("The text holds more than one line `{end}`."));
    }
    let (begin_at, end_at) = match (begins.first().copied(), ends.first().copied()) {
        (None, None) => return Ok(None),
        (Some(_), None) => return Err(format!("The line `{begin}` has no line `{end}` after it.")),
        (None, Some(_)) => {
            return Err(format!("The line `{end}` has no line `{begin}` before it."));
        }
        (Some(b), Some(e)) => (b, e),
    };
    if end_at < begin_at {
        return Err(format!(
            "The line `{end}` stands before the line `{begin}`."
        ));
    }
    let mut out: Vec<&str> = lines[..begin_at].to_vec();
    out.push(begin);
    out.push("");
    out.extend(table.trim_end().lines());
    out.push("");
    out.push(end);
    out.extend(&lines[end_at + 1..]);
    let mut result = out.join("\n");
    result.push('\n');
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    const B: &str = "<!-- a -->";
    const E: &str = "<!-- b -->";

    #[test]
    fn replaces_the_text_between_the_markers() {
        let text = "intro\n<!-- a -->\nold\n<!-- b -->\noutro\n";
        let new = replace(text, B, E, "| t |\n").unwrap().unwrap();
        assert_eq!(
            new, "intro\n<!-- a -->\n\n| t |\n\n<!-- b -->\noutro\n",
            "The text between the markers must change."
        );
    }

    #[test]
    fn returns_none_for_a_text_with_no_marker() {
        assert_eq!(
            replace("plain\n", B, E, "x").unwrap(),
            None,
            "A text with no marker must not change."
        );
    }

    #[test]
    fn returns_an_error_for_a_begin_line_with_no_end_line() {
        let error = replace("<!-- a -->\nold\n", B, E, "x").unwrap_err();
        assert!(
            error.contains("has no line `<!-- b -->` after it"),
            "The error must name the missing end line: {error}"
        );
    }

    #[test]
    fn returns_an_error_for_an_end_line_with_no_begin_line() {
        let error = replace("old\n<!-- b -->\n", B, E, "x").unwrap_err();
        assert!(
            error.contains("has no line `<!-- a -->` before it"),
            "The error must name the missing begin line: {error}"
        );
    }

    #[test]
    fn returns_an_error_for_an_end_line_before_the_begin_line() {
        let error = replace("<!-- b -->\nold\n<!-- a -->\n", B, E, "x").unwrap_err();
        assert!(
            error.contains("stands before"),
            "The error must name the order: {error}"
        );
    }

    #[test]
    fn returns_an_error_for_a_second_begin_line() {
        let text = "<!-- a -->\n<!-- b -->\n<!-- a -->\n<!-- b -->\n";
        let error = replace(text, B, E, "x").unwrap_err();
        assert!(
            error.contains("more than one line `<!-- a -->`"),
            "The error must name the repeated begin line: {error}"
        );
    }

    #[test]
    fn returns_an_error_for_a_second_end_line() {
        let text = "<!-- a -->\n<!-- b -->\n<!-- b -->\n";
        let error = replace(text, B, E, "x").unwrap_err();
        assert!(
            error.contains("more than one line `<!-- b -->`"),
            "The error must name the repeated end line: {error}"
        );
    }

    #[test]
    fn an_indented_marker_is_not_a_marker() {
        assert_eq!(
            replace("  <!-- a -->\nold\n  <!-- b -->\n", B, E, "x").unwrap(),
            None,
            "An indented line must not count as a marker."
        );
    }

    #[test]
    fn a_second_run_gives_the_same_text() {
        let text = "<!-- a -->\nold\n<!-- b -->\n";
        let first = replace(text, B, E, "| t |").unwrap().unwrap();
        let second = replace(&first, B, E, "| t |").unwrap().unwrap();
        assert_eq!(first, second, "A second run must keep the text.");
    }
}
