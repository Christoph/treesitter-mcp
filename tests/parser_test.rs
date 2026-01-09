//! Tests for language detection functionality
//!
//! This module tests the `detect_language` function which identifies programming
//! languages based on file extensions. Tests cover:
//! - Detection of all supported languages (Rust, Python, JavaScript, TypeScript, HTML, CSS, Swift, C#, Java)
//! - Case-insensitive extension matching
//! - Error handling for unsupported and missing extensions

use treesitter_mcp::parser::{detect_language, parse_code, Language};

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

/// Test that C# files (.cs) are correctly detected
///
/// Verifies that the `detect_language` function properly identifies C# source files
/// by their .cs extension, which is the standard extension for C# programs.
#[test]
fn test_detect_language_from_csharp_file() {
    let lang = detect_language("Program.cs").unwrap();
    assert_eq!(lang, Language::CSharp);
}

/// Test that Java files (.java) are correctly detected
///
/// Verifies that the `detect_language` function properly identifies Java source files
/// by their .java extension, which is the standard extension for Java programs.
#[test]
fn test_detect_language_from_java_file() {
    let lang = detect_language("Main.java").unwrap();
    assert_eq!(lang, Language::Java);
}

/// Test that C# files with uppercase extension are correctly detected
///
/// Verifies that the `detect_language` function correctly identifies C# files regardless
/// of the case used in the file extension, ensuring robustness across different naming conventions.
#[test]
fn test_detect_language_from_csharp_file_case_insensitive() {
    let lang = detect_language("Controller.CS").unwrap();
    assert_eq!(lang, Language::CSharp);
}

/// Test that tree_sitter_language() returns a valid grammar for C#
///
/// Verifies that the `tree_sitter_language` method returns a valid tree-sitter
/// language grammar for C# that can be used to parse C# source code. This test
/// ensures the grammar can parse a simple C# class declaration without errors.
#[test]
fn test_tree_sitter_language_csharp_returns_valid_grammar() {
    let lang = Language::CSharp;
    let _ts_lang = lang.tree_sitter_language();

    // Verify we can parse a simple C# program with the grammar
    let source = r#"
        class Program {
            static void Main(string[] args) {
                System.Console.WriteLine("Hello");
            }
        }
    "#;

    let tree = parse_code(source, lang).unwrap();
    assert!(!tree.root_node().has_error());
}

/// Test that tree_sitter_language() returns a valid grammar for Java
///
/// Verifies that the `tree_sitter_language` method returns a valid tree-sitter
/// language grammar for Java that can be used to parse Java source code. This test
/// ensures the grammar can parse a simple Java class declaration without errors.
#[test]
fn test_tree_sitter_language_java_returns_valid_grammar() {
    let lang = Language::Java;
    let _ts_lang = lang.tree_sitter_language();

    // Verify we can parse a simple Java program with the grammar
    let source = r#"
        public class Main {
            public static void main(String[] args) {
                System.out.println("Hello");
            }
        }
    "#;

    let tree = parse_code(source, lang).unwrap();
    assert!(!tree.root_node().has_error());
}

/// Test that tree_sitter_language() for C# can parse classes
///
/// Verifies that the C# grammar returned by `tree_sitter_language` can parse
/// class declarations. This ensures the grammar recognizes fundamental C# syntax.
#[test]
fn test_tree_sitter_language_csharp_parses_classes() {
    let lang = Language::CSharp;
    let source = "public class MyClass { }";

    let tree = parse_code(source, lang).unwrap();
    let root = tree.root_node();

    // Verify the tree has a class declaration node
    assert!(!root.has_error());
    assert!(root.to_sexp().contains("class"));
}

/// Test that tree_sitter_language() for Java can parse methods
///
/// Verifies that the Java grammar returned by `tree_sitter_language` can parse
/// method declarations. This ensures the grammar recognizes fundamental Java syntax.
#[test]
fn test_tree_sitter_language_java_parses_methods() {
    let lang = Language::Java;
    let source = r#"
        public class Test {
            public void testMethod() {
                int x = 42;
            }
        }
    "#;

    let tree = parse_code(source, lang).unwrap();
    let root = tree.root_node();

    // Verify the tree has a method declaration
    assert!(!root.has_error());
    assert!(root.to_sexp().contains("method"));
}
