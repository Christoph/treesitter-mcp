fn escape_field(s: &str) -> String {
    // Keep row format parseable:
    // - rows split on "\n"
    // - columns split on "|"
    // So we must escape backslashes first, then delimiters/newlines.
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('|', "\\|")
}

pub fn format_row(fields: &[&str]) -> String {
    fields
        .iter()
        .map(|field| escape_field(field))
        .collect::<Vec<_>>()
        .join("|")
}
