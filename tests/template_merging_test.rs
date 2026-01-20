//! Tests for Askama template merging

use std::path::PathBuf;
use treesitter_mcp::analysis::askama::find_template_dependencies;
use treesitter_mcp::analysis::askama::find_templates_dir;

#[test]
fn test_find_templates_dir() {
    let path = PathBuf::from("tests/fixtures/rust_project/templates/calculator.html");
    let templates_dir = find_templates_dir(&path).expect("Failed to find templates dir");

    assert!(templates_dir.ends_with("templates"));
    assert!(templates_dir.join("base.html").exists());
}

#[test]
fn test_find_templates_dir_from_partial() {
    let path = PathBuf::from("tests/fixtures/rust_project/templates/partials/header.html");
    let templates_dir = find_templates_dir(&path).expect("Failed to find templates dir");

    assert!(templates_dir.ends_with("templates"));
}

#[test]
fn test_find_templates_dir_not_found() {
    let path = PathBuf::from("tests/fixtures/minimal/simple.html");
    let templates_dir = find_templates_dir(&path);

    assert!(templates_dir.is_none());
}

#[test]
fn test_find_template_dependencies() {
    let source = r#"
{% extends "base.html" %}
{% include "partials/header.html" %}
{% include "partials/footer.html" %}
"#;

    let templates_dir = PathBuf::from("tests/fixtures/rust_project/templates");
    let deps =
        find_template_dependencies(source, &templates_dir).expect("Failed to find dependencies");

    assert_eq!(deps.len(), 3);
    assert!(deps
        .iter()
        .any(|d| d.path == "base.html" && d.dependency_type == "extends"));
    assert!(deps
        .iter()
        .any(|d| d.path == "partials/header.html" && d.dependency_type == "include"));
    assert!(deps
        .iter()
        .any(|d| d.path == "partials/footer.html" && d.dependency_type == "include"));
}

#[test]
fn test_find_template_dependencies_with_quotes() {
    let source = r#"
{% extends 'base.html' %}
{% include 'partials/header.html' %}
"#;

    let templates_dir = PathBuf::from("tests/fixtures/rust_project/templates");
    let deps =
        find_template_dependencies(source, &templates_dir).expect("Failed to find dependencies");

    assert_eq!(deps.len(), 2);
    assert!(deps.iter().any(|d| d.path == "base.html"));
    assert!(deps.iter().any(|d| d.path == "partials/header.html"));
}

#[test]
fn test_find_template_dependencies_nonexistent() {
    let source = r#"
{% extends "nonexistent.html" %}
{% include "also-nonexistent.html" %}
"#;

    let templates_dir = PathBuf::from("tests/fixtures/rust_project/templates");
    let deps =
        find_template_dependencies(source, &templates_dir).expect("Failed to find dependencies");

    // Should not include nonexistent files
    assert_eq!(deps.len(), 0);
}
