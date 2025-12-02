use treesitter_mcp::parser::{detect_language, Language};

#[test]
fn test_detect_language_from_rust_file() {
    let lang = detect_language("src/main.rs").unwrap();
    assert_eq!(lang, Language::Rust);
}

#[test]
fn test_detect_language_from_python_file() {
    let lang = detect_language("script.py").unwrap();
    assert_eq!(lang, Language::Python);
}

#[test]
fn test_detect_language_from_javascript_file() {
    let lang = detect_language("app.js").unwrap();
    assert_eq!(lang, Language::JavaScript);
}

#[test]
fn test_detect_language_from_typescript_file() {
    let lang = detect_language("app.ts").unwrap();
    assert_eq!(lang, Language::TypeScript);
}

#[test]
fn test_detect_language_from_tsx_file() {
    let lang = detect_language("component.tsx").unwrap();
    assert_eq!(lang, Language::TypeScript);
}

#[test]
fn test_detect_language_from_html_file() {
    let lang = detect_language("index.html").unwrap();
    assert_eq!(lang, Language::Html);
}

#[test]
fn test_detect_language_from_css_file() {
    let lang = detect_language("style.css").unwrap();
    assert_eq!(lang, Language::Css);
}

#[test]
fn test_unsupported_language() {
    let result = detect_language("file.txt");
    assert!(result.is_err());
}

#[test]
fn test_no_extension() {
    let result = detect_language("Makefile");
    assert!(result.is_err());
}

#[test]
fn test_case_insensitive_extension() {
    let lang = detect_language("Test.RS").unwrap();
    assert_eq!(lang, Language::Rust);
}
