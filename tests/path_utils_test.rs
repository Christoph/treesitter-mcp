mod common;

use std::path::Path;
use treesitter_mcp::analysis::path_utils::find_project_root;

#[test]
fn test_find_project_root_with_cargo_toml() {
    // Given: A path inside a Cargo project
    let file_path = Path::new("tests/fixtures/rust_project/src/calculator.rs");

    // When: Find project root
    let root = find_project_root(file_path);

    // Then: Should find the directory with Cargo.toml
    assert!(root.is_some());
    assert!(root.unwrap().join("Cargo.toml").exists());
}

#[test]
fn test_find_project_root_with_package_json() {
    // Given: A path inside a Node project (if it exists)
    let file_path = Path::new("tests/fixtures/javascript_project/index.js");

    // When: Find project root
    let root = find_project_root(file_path);

    // Then: Should find a project root (either package.json or .git)
    // Note: The fixture might not have package.json, but should find .git
    assert!(root.is_some());
}

#[test]
fn test_find_project_root_with_git() {
    // Given: A path inside a git repo (this project)
    let file_path = Path::new("src/main.rs");

    // When: Find project root
    let root = find_project_root(file_path);

    // Then: Should find .git directory
    assert!(root.is_some());
    let root_path = root.unwrap();
    assert!(root_path.join(".git").exists() || root_path.join("Cargo.toml").exists());
}

#[test]
fn test_find_project_root_from_directory() {
    // Given: A directory path instead of file path
    let dir_path = Path::new("tests/fixtures/rust_project/src");

    // When: Find project root
    let root = find_project_root(dir_path);

    // Then: Should find the directory with Cargo.toml
    assert!(root.is_some());
    assert!(root.unwrap().join("Cargo.toml").exists());
}

#[test]
fn test_find_project_root_nested_deep() {
    // Given: A deeply nested path
    let file_path = Path::new("tests/fixtures/rust_project/src/models/mod.rs");

    // When: Find project root
    let root = find_project_root(file_path);

    // Then: Should still find the root
    assert!(root.is_some());
    assert!(root.unwrap().join("Cargo.toml").exists());
}
