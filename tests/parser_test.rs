//! Tests for language detection functionality
//!
//! This module tests the `detect_language` function which identifies programming
//! languages based on file extensions. Tests cover:
//! - Detection of all supported languages (Rust, Python, JavaScript, TypeScript, HTML, CSS)
//! - Case-insensitive extension matching
//! - Error handling for unsupported and missing extensions

use treesitter_mcp::parser::{detect_language, Language};

/// Test that Rust files (.rs) are correctly detected
///
/// Verifies that the `detect_language` function properly identifies Rust source files
/// by their .rs extension, which is the standard extension for Rust programs.
#[test]
fn test_detect_language_from_rust_file() {
    let lang = detect_language("src/main.rs").unwrap();
    assert_eq!(lang, Language::Rust);
}

/// Test that Python files (.py) are correctly detected
///
/// Verifies that the `detect_language` function properly identifies Python source files
/// by their .py extension, which is the standard extension for Python programs.
#[test]
fn test_detect_language_from_python_file() {
    let lang = detect_language("script.py").unwrap();
    assert_eq!(lang, Language::Python);
}

/// Test that JavaScript files (.js) are correctly detected
///
/// Verifies that the `detect_language` function properly identifies JavaScript source files
/// by their .js extension. This is the standard extension for JavaScript programs.
#[test]
fn test_detect_language_from_javascript_file() {
    let lang = detect_language("app.js").unwrap();
    assert_eq!(lang, Language::JavaScript);
}

/// Test that TypeScript files (.ts) are correctly detected
///
/// Verifies that the `detect_language` function properly identifies TypeScript source files
/// by their .ts extension. This is the standard extension for TypeScript programs.
#[test]
fn test_detect_language_from_typescript_file() {
    let lang = detect_language("app.ts").unwrap();
    assert_eq!(lang, Language::TypeScript);
}

/// Test that TypeScript React files (.tsx) are correctly detected as TypeScript
///
/// Verifies that the `detect_language` function properly identifies TypeScript React component files
/// by their .tsx extension. TSX files are TypeScript files with JSX syntax and should be parsed
/// using the TypeScript grammar.
#[test]
fn test_detect_language_from_tsx_file() {
    let lang = detect_language("component.tsx").unwrap();
    assert_eq!(lang, Language::TypeScript);
}

/// Test that HTML files (.html) are correctly detected
///
/// Verifies that the `detect_language` function properly identifies HTML markup files
/// by their .html extension. This is the standard extension for HTML documents.
#[test]
fn test_detect_language_from_html_file() {
    let lang = detect_language("index.html").unwrap();
    assert_eq!(lang, Language::Html);
}

/// Test that CSS files (.css) are correctly detected
///
/// Verifies that the `detect_language` function properly identifies CSS stylesheet files
/// by their .css extension. This is the standard extension for CSS stylesheets.
#[test]
fn test_detect_language_from_css_file() {
    let lang = detect_language("style.css").unwrap();
    assert_eq!(lang, Language::Css);
}

/// Test that unsupported file extensions are rejected
///
/// Verifies that the `detect_language` function returns an error when given a file
/// with an unsupported extension (e.g., .txt). This ensures the function properly
/// validates input and doesn't attempt to parse files with unknown languages.
#[test]
fn test_unsupported_language() {
    let result = detect_language("file.txt");
    assert!(result.is_err());
}

/// Test that files without extensions are rejected
///
/// Verifies that the `detect_language` function returns an error when given a file path
/// with no extension (e.g., "Makefile"). This ensures the function requires a file extension
/// to determine the language, preventing ambiguous or incorrect language detection.
#[test]
fn test_no_extension() {
    let result = detect_language("Makefile");
    assert!(result.is_err());
}

/// Test that file extension detection is case-insensitive
///
/// Verifies that the `detect_language` function correctly identifies languages regardless
/// of the case used in the file extension. For example, both ".rs" and ".RS" should be
/// recognized as Rust files. This ensures the function is robust to different naming conventions.
#[test]
fn test_case_insensitive_extension() {
    let lang = detect_language("Test.RS").unwrap();
    assert_eq!(lang, Language::Rust);
}
