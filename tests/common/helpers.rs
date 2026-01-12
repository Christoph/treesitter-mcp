#![allow(dead_code)]

//! Helper functions for common test assertions

use serde_json::Value;

pub fn approx_tokens(text: &str) -> usize {
    text.len() / 4
}

pub fn assert_within_token_budget(text: &str, max_tokens: usize, context: &str) {
    let actual = approx_tokens(text);
    assert!(
        actual <= max_tokens,
        "{}: Output exceeds token budget: {} > {} tokens",
        context,
        actual,
        max_tokens
    );
}

pub fn assert_error_contains(err: &str, expected: &str, context: &str) {
    assert!(
        err.to_lowercase().contains(&expected.to_lowercase()),
        "{}: Error should contain '{}', got: {}",
        context,
        expected,
        err
    );
}

fn is_relative_path(path: &str) -> bool {
    !path.contains("/Users/")
        && !path.contains("/home/")
        && !path.contains("/var/")
        && !path.contains("/mnt/")
        && !path.starts_with("C:\\")
        && !path.starts_with("D:\\")
        && !path.starts_with('/')
}

pub fn assert_path_is_relative(path: &str) {
    assert!(
        is_relative_path(path),
        "Path should be relative, got: {}",
        path
    );
}

pub fn parse_compact_row(row: &str) -> Vec<String> {
    let mut fields: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut escaped = false;

    for ch in row.chars() {
        if escaped {
            match ch {
                'n' => current.push('\n'),
                'r' => current.push('\r'),
                '|' => current.push('|'),
                '\\' => current.push('\\'),
                other => {
                    current.push('\\');
                    current.push(other);
                }
            }
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if ch == '|' {
            fields.push(std::mem::take(&mut current));
            continue;
        }

        current.push(ch);
    }

    if escaped {
        current.push('\\');
    }

    fields.push(current);
    fields
}

pub fn parse_compact_rows(rows: &str) -> Vec<Vec<String>> {
    if rows.is_empty() {
        return Vec::new();
    }

    rows.lines().map(parse_compact_row).collect()
}

/// Assert that a shape has a function with the given name.
///
/// Works with:
/// - legacy `view_code`-style output: `{"functions": [{"name": ...}]}`
/// - compact output: `{"h": "name|...", "f": "name|...\n..."}`
pub fn assert_has_function(shape: &Value, name: &str) {
    if let Some(functions) = shape.get("functions").and_then(Value::as_array) {
        let found = functions.iter().any(|f| f["name"] == name);
        assert!(
            found,
            "Should find function '{}' in {:?}",
            name,
            functions.iter().map(|f| &f["name"]).collect::<Vec<_>>()
        );
        return;
    }

    let rows = shape.get("f").and_then(Value::as_str).unwrap_or("");
    let parsed = parse_compact_rows(rows);
    let found = parsed
        .iter()
        .any(|row| row.first().map(|v| v.as_str()) == Some(name));
    assert!(found, "Should find function '{}' in compact rows", name);
}

/// Assert that a function has code containing specific text.
///
/// For compact output, this expects a `full` header containing a `code` column.
pub fn assert_function_code_contains(shape: &Value, func_name: &str, code_text: &str) {
    if let Some(functions) = shape.get("functions").and_then(Value::as_array) {
        let func = functions
            .iter()
            .find(|f| f["name"] == func_name)
            .unwrap_or_else(|| panic!("Should find function '{}'", func_name));

        let code = func["code"]
            .as_str()
            .unwrap_or_else(|| panic!("Function '{}' should have code", func_name));

        assert!(
            code.contains(code_text),
            "Function '{}' code should contain '{}', got:\n{}",
            func_name,
            code_text,
            code
        );
        return;
    }

    let header = shape.get("h").and_then(Value::as_str).unwrap_or("");
    let code_idx = header
        .split('|')
        .position(|field| field == "code")
        .unwrap_or_else(|| panic!("Compact header missing 'code' column: {header}"));

    let rows = shape.get("f").and_then(Value::as_str).unwrap_or("");
    let parsed = parse_compact_rows(rows);

    let func_row = parsed
        .iter()
        .find(|row| row.first().map(|v| v.as_str()) == Some(func_name))
        .unwrap_or_else(|| panic!("Should find function '{}' in compact rows", func_name));

    let code = func_row
        .get(code_idx)
        .unwrap_or_else(|| panic!("Expected code column at index {code_idx}"));

    assert!(
        code.contains(code_text),
        "Function '{}' code should contain '{}', got:\n{}",
        func_name,
        code_text,
        code
    );
}

pub fn code_map_files(map: &Value) -> Vec<(&str, &Value)> {
    let obj = map
        .as_object()
        .unwrap_or_else(|| panic!("code_map output should be an object"));

    obj.iter()
        .filter_map(|(k, v)| {
            if k == "@" {
                None
            } else {
                Some((k.as_str(), v))
            }
        })
        .collect()
}

pub fn code_map_find_file<'a>(map: &'a Value, suffix: &str) -> (&'a str, &'a Value) {
    code_map_files(map)
        .into_iter()
        .find(|(path, _)| path.contains(suffix))
        .unwrap_or_else(|| panic!("Expected code_map to include file containing '{suffix}'"))
}

pub fn compact_table_get_rows<'a>(file_obj: &'a Value, field: &str) -> Vec<Vec<String>> {
    let rows = file_obj.get(field).and_then(Value::as_str).unwrap_or("");
    parse_compact_rows(rows)
}

pub fn assert_all_code_map_paths_relative(map: &Value) {
    for (path, _) in code_map_files(map) {
        assert_path_is_relative(path);
    }
}

pub fn find_usages_rows(result: &Value) -> Vec<Vec<String>> {
    let rows = result.get("u").and_then(Value::as_str).unwrap_or("");
    parse_compact_rows(rows)
}

pub fn assert_all_find_usages_paths_relative(result: &Value) {
    for row in find_usages_rows(result) {
        let Some(file) = row.first() else {
            continue;
        };
        assert_path_is_relative(file);
    }
}

/// Assert minimum number of items in a field.
///
/// Supports:
/// - legacy array fields (e.g. `usages`)
/// - compact `find_usages` rows (`u`)
/// - compact `code_map` file count (object keys excluding `@`)
pub fn assert_min_count(value: &Value, field: &str, min: usize) {
    if let Some(arr) = value.get(field).and_then(Value::as_array) {
        assert!(
            arr.len() >= min,
            "Should have at least {} items in '{}', got {}",
            min,
            field,
            arr.len()
        );
        return;
    }

    if field == "usages" {
        let rows = find_usages_rows(value);
        assert!(
            rows.len() >= min,
            "Should have at least {} usage rows, got {}",
            min,
            rows.len()
        );
        return;
    }

    if field == "files" {
        let files = code_map_files(value);
        assert!(
            files.len() >= min,
            "Should have at least {} files, got {}",
            min,
            files.len()
        );
        return;
    }

    panic!("Unsupported field for assert_min_count: {field}");
}
