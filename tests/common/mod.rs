use std::path::PathBuf;

pub mod helpers;

/// Get path to a fixture file
pub fn fixture_path(lang: &str, file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(format!("{}_project", lang))
        .join(file)
}

/// Get path to a fixture directory
pub fn fixture_dir(lang: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(format!("{}_project", lang))
}

/// Helper to extract text from CallToolResult
pub fn get_result_text(result: &treesitter_mcp::mcp_types::CallToolResult) -> String {
    // The CallToolResult has a content field which is a Vec<ContentBlock>
    // We serialize and deserialize to extract the text field
    if let Some(first_content) = result.content.first() {
        let json_str = serde_json::to_string(first_content).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        json_val["text"].as_str().unwrap().to_string()
    } else {
        panic!("No content in result");
    }
}
