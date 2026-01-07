use serde_json::json;
use std::path::PathBuf;

#[test]
fn debug_java_ast_structure() {
    let file_path = PathBuf::from("tests/fixtures/java_project/models/Shape.java");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments)
        .expect("parse_file should succeed");

    let text = if let Some(content) = result.content.first() {
        match content {
            rust_mcp_schema::generated_schema::__int_2025_06_18::ContentBlock::Text { text } => {
                text.clone()
            }
            _ => panic!("Expected text content"),
        }
    } else {
        panic!("No content");
    };

    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Pretty print the entire structure
    println!("\n=== FULL PARSED SHAPE ===");
    println!("{}", serde_json::to_string_pretty(&shape).unwrap());

    // Check classes specifically
    if let Some(classes) = shape.get("classes").and_then(|c| c.as_array()) {
        println!("\n=== CLASSES COUNT: {} ===", classes.len());
        for (i, class) in classes.iter().enumerate() {
            println!("\n--- Class {} ---", i);
            println!(
                "Name: {}",
                class.get("name").and_then(|n| n.as_str()).unwrap_or("N/A")
            );
            println!("Implements: {:?}", class.get("implements"));

            if let Some(methods) = class.get("methods").and_then(|m| m.as_array()) {
                println!("Methods count: {}", methods.len());
                for method in methods {
                    println!(
                        "  - Method: {}",
                        method.get("name").and_then(|n| n.as_str()).unwrap_or("N/A")
                    );
                    println!("    Annotations: {:?}", method.get("annotations"));
                }
            }
        }
    }
}
