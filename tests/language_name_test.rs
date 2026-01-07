//! Tests for Language::name() method
//!
//! This module tests the `name()` method which returns human-readable names
//! for programming languages. Tests cover:
//! - Correct naming for all supported languages
//! - Proper capitalization and formatting

use treesitter_mcp::parser::Language;

/// Test that CSharp language returns correct human-readable name
///
/// Verifies that the `name()` method returns "C#" for C# language,
/// not "CSharp" or other variations. This ensures consistency in
/// user-facing language names.
#[test]
fn test_csharp_name_returns_csharp_symbol() {
    let lang = Language::CSharp;
    assert_eq!(lang.name(), "C#");
}

/// Test that Java language returns correct human-readable name
///
/// Verifies that the `name()` method returns "Java" for Java language,
/// ensuring consistency in user-facing language names.
#[test]
fn test_java_name_returns_java() {
    let lang = Language::Java;
    assert_eq!(lang.name(), "Java");
}

/// Test that all supported languages have non-empty names
///
/// Verifies that every language variant returns a meaningful name,
/// not an empty string or placeholder value.
#[test]
fn test_all_languages_have_names() {
    let languages = vec![
        Language::Rust,
        Language::Python,
        Language::JavaScript,
        Language::TypeScript,
        Language::Html,
        Language::Css,
        Language::Swift,
        Language::CSharp,
        Language::Java,
    ];

    for lang in languages {
        let name = lang.name();
        assert!(!name.is_empty(), "Language {:?} has empty name", lang);
        assert!(
            name.len() > 0,
            "Language {:?} name should not be empty",
            lang
        );
    }
}
