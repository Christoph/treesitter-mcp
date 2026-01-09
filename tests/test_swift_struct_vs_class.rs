#[test]
fn check_swift_struct_vs_class() {
    use tree_sitter::Parser;
    
    let tests = vec![
        ("struct", "struct Point { var x: Int }"),
        ("class", "class MyClass { var x: Int }"),
    ];
    
    for (name, source) in tests {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_swift::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        if let Some(decl) = tree.root_node().child(0) {
            println!("\n{} declaration:", name);
            println!("  Node kind: {}", decl.kind());
            println!("  Node text: {}", decl.utf8_text(source.as_bytes()).unwrap());
            
            // Check for "struct" or "class" keyword
            for i in 0..decl.child_count() {
                if let Some(child) = decl.child(i) {
                    let text = child.utf8_text(source.as_bytes()).unwrap_or("");
                    println!("  Child {}: {} = '{}'", i, child.kind(), text);
                }
            }
        }
    }
}
