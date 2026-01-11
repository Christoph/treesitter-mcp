/// Test that tree-sitter-go dependency is available and can be used
///
/// This test verifies that the tree-sitter-go crate is properly added to
/// Cargo.toml and can be imported and used to parse Go code.
///
/// This is a RED phase TDD test - it should fail initially because
/// the tree-sitter-go dependency is not yet added.

#[test]
fn test_tree_sitter_go_dependency_is_available() {
    // This test will fail to compile if tree-sitter-go is not in Cargo.toml
    // If it compiles, the dependency exists

    // Try to access the tree-sitter-go language grammar
    let _lang = tree_sitter_go::LANGUAGE;

    // If we reach here, the dependency exists
    // We can verify it's a valid tree-sitter Language
    use tree_sitter::Language;
    let _: Language = _lang.into();
}

#[test]
fn test_tree_sitter_go_can_parse_simple_code() {
    use tree_sitter::Parser;

    // This test will fail to compile if tree-sitter-go is not in Cargo.toml

    let source = r#"
package main

func main() {
    println("Hello, World!")
}
"#;

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_go::LANGUAGE.into())
        .expect("Failed to set language");

    let tree = parser.parse(source, None).expect("Failed to parse Go code");
    let root = tree.root_node();

    // Verify we got a valid parse tree without errors
    assert!(!root.has_error(), "Parse tree should not have errors");
    assert_eq!(root.kind(), "source_file");
}
