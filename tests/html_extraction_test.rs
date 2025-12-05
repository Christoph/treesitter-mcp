//! Tests for HTML extraction

use treesitter_mcp::analysis::shape::extract_html_shape;
use treesitter_mcp::parser::{parse_code, Language};

#[test]
fn test_extract_ids() {
    let source = r#"<!DOCTYPE html>
<html>
<body>
  <div id="main">Content</div>
  <section id="sidebar">Sidebar</section>
</body>
</html>"#;

    let tree = parse_code(source, Language::Html).expect("Failed to parse HTML");
    let shape = extract_html_shape(&tree, source, None).expect("Failed to extract HTML shape");

    println!("Found {} IDs:", shape.ids.len());
    for id_info in &shape.ids {
        println!("  {} #{} (line {})", id_info.tag, id_info.id, id_info.line);
    }

    assert_eq!(shape.ids.len(), 2);
    assert_eq!(shape.ids[0].tag, "div");
    assert_eq!(shape.ids[0].id, "main");
    assert_eq!(shape.ids[1].tag, "section");
    assert_eq!(shape.ids[1].id, "sidebar");
}

#[test]
fn test_extract_custom_classes() {
    let source = r#"<!DOCTYPE html>
<html>
<body>
  <div class="card btn-primary flex">Content</div>
  <div class="card hover:bg-blue-500">More</div>
</body>
</html>"#;

    let tree = parse_code(source, Language::Html).expect("Failed to parse HTML");
    let shape = extract_html_shape(&tree, source, None).expect("Failed to extract HTML shape");

    // Should filter out Tailwind utilities (flex, hover:bg-blue-500)
    // Should keep custom classes (card, btn-primary)
    assert!(shape.classes_used.contains(&"card".to_string()));
    assert!(shape.classes_used.contains(&"btn-primary".to_string()));
    assert!(!shape.classes_used.contains(&"flex".to_string()));
    assert!(!shape
        .classes_used
        .contains(&"hover:bg-blue-500".to_string()));
}

#[test]
fn test_extract_scripts() {
    let source = r#"<!DOCTYPE html>
<html>
<head>
  <script src="./app.js"></script>
  <script>console.log('inline');</script>
</head>
</html>"#;

    let tree = parse_code(source, Language::Html).expect("Failed to parse HTML");
    let shape = extract_html_shape(&tree, source, None).expect("Failed to extract HTML shape");

    assert_eq!(shape.scripts.len(), 2);
    assert_eq!(shape.scripts[0].src, Some("./app.js".to_string()));
    assert_eq!(shape.scripts[0].inline, false);
    assert_eq!(shape.scripts[1].src, None);
    assert_eq!(shape.scripts[1].inline, true);
}

#[test]
fn test_extract_styles() {
    let source = r#"<!DOCTYPE html>
<html>
<head>
  <link rel="stylesheet" href="./styles.css">
  <style>.custom { color: red; }</style>
</head>
</html>"#;

    let tree = parse_code(source, Language::Html).expect("Failed to parse HTML");
    let shape = extract_html_shape(&tree, source, None).expect("Failed to extract HTML shape");

    assert_eq!(shape.styles.len(), 2);
    assert_eq!(shape.styles[0].href, Some("./styles.css".to_string()));
    assert_eq!(shape.styles[0].inline, false);
    assert_eq!(shape.styles[1].href, None);
    assert_eq!(shape.styles[1].inline, true);
}

#[test]
fn test_extract_minimal_fixture() {
    let source = std::fs::read_to_string("tests/fixtures/minimal/simple.html")
        .expect("Failed to read fixture");

    let tree = parse_code(&source, Language::Html).expect("Failed to parse HTML");
    let shape = extract_html_shape(&tree, &source, Some("tests/fixtures/minimal/simple.html"))
        .expect("Failed to extract HTML shape");

    assert_eq!(
        shape.path,
        Some("tests/fixtures/minimal/simple.html".to_string())
    );
    assert_eq!(shape.ids.len(), 1);
    assert_eq!(shape.ids[0].id, "main");

    // Should have custom classes (card, btn-primary) but not Tailwind utilities
    assert!(shape.classes_used.contains(&"card".to_string()));
    assert!(shape.classes_used.contains(&"btn-primary".to_string()));
}

#[test]
fn test_empty_html() {
    let source = "<!DOCTYPE html><html></html>";

    let tree = parse_code(source, Language::Html).expect("Failed to parse HTML");
    let shape = extract_html_shape(&tree, source, None).expect("Failed to extract HTML shape");

    assert_eq!(shape.ids.len(), 0);
    assert_eq!(shape.classes_used.len(), 0);
    assert_eq!(shape.scripts.len(), 0);
    assert_eq!(shape.styles.len(), 0);
}

#[test]
fn test_tailwind_utility_filtering() {
    let source = r#"<div class="flex items-center justify-between p-4 m-2 bg-blue-500 text-white rounded-lg shadow-md hover:bg-blue-600 dark:bg-gray-800 sm:p-6 md:p-8 lg:flex-row custom-class another-custom"></div>"#;

    let tree = parse_code(source, Language::Html).expect("Failed to parse HTML");
    let shape = extract_html_shape(&tree, source, None).expect("Failed to extract HTML shape");

    // Should only have custom classes
    assert_eq!(shape.classes_used.len(), 2);
    assert!(shape.classes_used.contains(&"custom-class".to_string()));
    assert!(shape.classes_used.contains(&"another-custom".to_string()));

    // Verify Tailwind utilities are filtered out
    assert!(!shape.classes_used.contains(&"flex".to_string()));
    assert!(!shape.classes_used.contains(&"items-center".to_string()));
    assert!(!shape.classes_used.contains(&"p-4".to_string()));
    assert!(!shape.classes_used.contains(&"bg-blue-500".to_string()));
    assert!(!shape
        .classes_used
        .contains(&"hover:bg-blue-600".to_string()));
}
