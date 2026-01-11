#[cfg(test)]
mod tests {
    use serde_json::Value;
    use std::fs;
    use tempfile::tempdir;

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

        // Execute type_map via the handler logic (orchestrated)
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

        let types = json["types"].as_array().unwrap();

        // Config should have 1 usage (in app.ts)
        let config = types.iter().find(|t| t["name"] == "Config").unwrap();
        assert_eq!(config["usage_count"], 1);
        assert_eq!(config["kind"], "struct");

        // User should have 0 usage
        let user = types.iter().find(|t| t["name"] == "User").unwrap();
        assert_eq!(user["usage_count"], 0);
        assert_eq!(user["kind"], "interface");

        // Mode should have 1 usage
        let mode = types.iter().find(|t| t["name"] == "Mode").unwrap();
        assert_eq!(mode["usage_count"], 1);

        assert_eq!(json["total_types"], 3);
        assert_eq!(json["truncated"], false);

        Ok(())
    }
}
