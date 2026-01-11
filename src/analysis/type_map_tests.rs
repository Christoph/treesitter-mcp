#[cfg(test)]
mod tests {
    use serde_json::Value;
    use std::fs;
    use tempfile::tempdir;

    fn parse_compact_row(row: &str) -> Vec<String> {
        let mut fields = Vec::new();
        let mut current = String::new();
        let mut escape = false;

        for ch in row.chars() {
            if escape {
                match ch {
                    'n' => current.push('\n'),
                    'r' => current.push('\r'),
                    '|' => current.push('|'),
                    '\\' => current.push('\\'),
                    other => current.push(other),
                }
                escape = false;
                continue;
            }

            match ch {
                '\\' => escape = true,
                '|' => {
                    fields.push(current);
                    current = String::new();
                }
                other => current.push(other),
            }
        }

        fields.push(current);
        fields
    }

    fn parse_compact_rows(rows: &str) -> Vec<Vec<String>> {
        if rows.is_empty() {
            return Vec::new();
        }

        rows.lines().map(parse_compact_row).collect()
    }

    #[test]
    fn test_type_map_full_workflow() -> eyre::Result<()> {
        let dir = tempdir()?;
        let src_dir = dir.path().join("src");
        fs::create_dir(&src_dir)?;

        // Create a Rust file with types
        let rs_content = r#"
            pub struct Config { pub id: u32 }
            pub enum Mode { Fast, Safe }
        "#;
        fs::write(src_dir.join("config.rs"), rs_content)?;

        // Create a TypeScript file using these types
        let ts_content = r#"
            interface User { id: number; }
            // Config should not count in comments: Config
            const asString = "Config";
            const config: Config = { id: 1 };
            const mode = Mode.Fast;
        "#;
        fs::write(src_dir.join("app.ts"), ts_content)?;

        // Execute type_map
        let args = serde_json::json!({
            "path": dir.path().to_str().unwrap(),
            "max_tokens": 5000
        });

        let result = crate::analysis::type_map::execute(&args)?;

        // Extract text content from the first content block
        let first_content = &result.content[0];
        let json_str = serde_json::to_string(first_content).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let text = json_val["text"].as_str().unwrap();

        let json: Value = serde_json::from_str(text)?;

        assert_eq!(json["h"], "name|kind|file|line|usage_count");

        let rows_str = json["types"].as_str().unwrap_or("");
        let rows = parse_compact_rows(rows_str);

        let find_row = |name: &str| {
            rows.iter()
                .find(|r| r.first().map(|v| v.as_str()) == Some(name))
                .unwrap_or_else(|| panic!("Missing type row for '{name}'"))
        };

        // Row columns: name|kind|file|line|usage_count
        let config = find_row("Config");
        assert_eq!(config.get(1).map(|s| s.as_str()), Some("struct"));
        assert_eq!(config.get(4).and_then(|s| s.parse::<u64>().ok()), Some(1));

        let user = find_row("User");
        assert_eq!(user.get(1).map(|s| s.as_str()), Some("interface"));
        assert_eq!(user.get(4).and_then(|s| s.parse::<u64>().ok()), Some(0));

        let mode = find_row("Mode");
        assert_eq!(mode.get(1).map(|s| s.as_str()), Some("enum"));
        assert_eq!(mode.get(4).and_then(|s| s.parse::<u64>().ok()), Some(1));

        // Should not be truncated with max_tokens=5000
        let truncated = json
            .get("@")
            .and_then(|m| m.get("t"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        assert!(!truncated);

        Ok(())
    }
}
