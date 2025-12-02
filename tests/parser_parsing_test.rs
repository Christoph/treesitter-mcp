use treesitter_mcp::parser::Language;

#[test]
fn test_parse_rust_code() {
    let code = "fn main() { println!(\"hello\"); }";
    let tree = treesitter_mcp::parser::parse_code(code, Language::Rust).unwrap();

    let root = tree.root_node();
    assert_eq!(root.kind(), "source_file");
    assert!(!root.has_error());
}

#[test]
fn test_parse_python_code() {
    let code = "def hello():\n    print('world')";
    let tree = treesitter_mcp::parser::parse_code(code, Language::Python).unwrap();

    let root = tree.root_node();
    assert_eq!(root.kind(), "module");
    assert!(!root.has_error());
}

#[test]
fn test_parse_javascript_code() {
    let code = "function hello() { console.log('world'); }";
    let tree = treesitter_mcp::parser::parse_code(code, Language::JavaScript).unwrap();

    let root = tree.root_node();
    assert_eq!(root.kind(), "program");
    assert!(!root.has_error());
}

#[test]
fn test_parse_typescript_code() {
    let code = "function hello(): void { console.log('world'); }";
    let tree = treesitter_mcp::parser::parse_code(code, Language::TypeScript).unwrap();

    let root = tree.root_node();
    assert_eq!(root.kind(), "program");
    assert!(!root.has_error());
}

#[test]
fn test_parse_html_code() {
    let code = "<html><body>Hello</body></html>";
    let tree = treesitter_mcp::parser::parse_code(code, Language::Html).unwrap();

    let root = tree.root_node();
    assert_eq!(root.kind(), "document");
    assert!(!root.has_error());
}

#[test]
fn test_parse_css_code() {
    let code = "body { color: red; }";
    let tree = treesitter_mcp::parser::parse_code(code, Language::Css).unwrap();

    let root = tree.root_node();
    assert_eq!(root.kind(), "stylesheet");
    assert!(!root.has_error());
}

#[test]
fn test_parse_invalid_syntax() {
    let code = "fn main( { }"; // Invalid Rust
    let tree = treesitter_mcp::parser::parse_code(code, Language::Rust).unwrap();

    let root = tree.root_node();
    // Tree is still produced but contains error nodes
    assert!(root.has_error());
}

#[test]
fn test_parse_empty_code() {
    let code = "";
    let tree = treesitter_mcp::parser::parse_code(code, Language::Rust).unwrap();

    let root = tree.root_node();
    assert_eq!(root.child_count(), 0);
}

#[test]
fn test_tree_sexp_output() {
    let code = "fn test() {}";
    let tree = treesitter_mcp::parser::parse_code(code, Language::Rust).unwrap();

    let sexp = tree.root_node().to_sexp();
    assert!(sexp.contains("source_file"));
    assert!(sexp.contains("function_item"));
}
