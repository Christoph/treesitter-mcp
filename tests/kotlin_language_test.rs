use treesitter_mcp::analysis::shape::extract_enhanced_shape;
use treesitter_mcp::parser::{detect_language, parse_code, Language};

#[test]
fn test_detect_kotlin_language() {
    assert_eq!(detect_language("test.kt").unwrap(), Language::Kotlin);
    assert_eq!(detect_language("script.kts").unwrap(), Language::Kotlin);
}

#[test]
fn test_kotlin_language_name() {
    assert_eq!(Language::Kotlin.name(), "Kotlin");
}

#[test]
fn test_parse_kotlin_code() {
    let code = r#"
        fun main() {
            println("Hello")
        }
    "#;
    let tree = parse_code(code, Language::Kotlin).expect("Failed to parse Kotlin code");
    let root = tree.root_node();
    assert_eq!(root.kind(), "source_file");
}

#[test]
fn test_extract_kotlin_shape() {
    let code = r#"
        package com.example

        import java.util.Date

        /**
         * A user class
         */
        class User(val name: String) {
            fun getName(): String = name
        }

        interface Repository {
            fun save(user: User)
        }

        object Config {
            const val VERSION = "1.0"
        }

        typealias UserId = Int
    "#;

    let tree = parse_code(code, Language::Kotlin).unwrap();
    println!("Tree S-exp: {}", tree.root_node().to_sexp());
    let shape = extract_enhanced_shape(&tree, code, Language::Kotlin, None, true)
        .expect("Failed to extract shape");

    // Check imports

    assert!(shape
        .imports
        .iter()
        .any(|i| i.text.contains("java.util.Date")));

    // Check classes
    let user_class = shape
        .classes
        .iter()
        .find(|c| c.name == "User")
        .expect("User class not found");
    assert_eq!(user_class.doc.as_deref(), Some("* A user class"));

    let _object_config = shape
        .classes
        .iter()
        .find(|c| c.name == "Config")
        .expect("Config object not found");

    // Check interfaces
    let _repo_interface = shape
        .interfaces
        .iter()
        .find(|i| i.name == "Repository")
        .expect("Repository interface not found");

    // Check type alias (mapped to structs)
    let user_id_alias = shape
        .structs
        .iter()
        .find(|s| s.name == "UserId")
        .expect("UserId type alias not found");
    assert_eq!(user_id_alias.line, 21); // 50 in original string?
}
