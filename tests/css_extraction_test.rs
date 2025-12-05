//! Tests for CSS extraction (Tailwind v4)

use treesitter_mcp::analysis::shape::extract_css_tailwind;

#[test]
fn test_debug_layer_extraction() {
    let source = r#"@layer components {
  .btn {
    @apply px-4 py-2;
  }
}"#;

    let shape = extract_css_tailwind(source, None).expect("Failed to extract CSS shape");

    println!("Found {} custom classes", shape.custom_classes.len());
    for class in &shape.custom_classes {
        println!(
            "Class: {}, utilities: {:?}",
            class.name, class.applied_utilities
        );
    }

    assert_eq!(shape.custom_classes.len(), 1);
}

#[test]
fn test_extract_theme_variables() {
    let source = r#"@theme {
  --color-primary: oklch(0.6 0.2 250);
  --spacing-lg: 1.5rem;
}"#;

    let shape = extract_css_tailwind(source, None).expect("Failed to extract CSS shape");

    println!("Theme variables:");
    for var in &shape.theme {
        println!("  {} = {} (line {})", var.name, var.value, var.line);
    }

    assert_eq!(shape.theme.len(), 2);
    assert_eq!(shape.theme[0].name, "--color-primary");
    assert_eq!(shape.theme[0].value, "oklch(0.6 0.2 250)");
    assert_eq!(shape.theme[0].line, 2);

    assert_eq!(shape.theme[1].name, "--spacing-lg");
    assert_eq!(shape.theme[1].value, "1.5rem");
    assert_eq!(shape.theme[1].line, 3);
}

#[test]
fn test_extract_custom_classes() {
    let source = r#"
@layer components {
  .btn {
    @apply px-4 py-2 rounded;
  }
  
  .card {
    @apply bg-white shadow-md;
  }
}
"#;

    let shape = extract_css_tailwind(source, None).expect("Failed to extract CSS shape");

    assert_eq!(shape.custom_classes.len(), 2);

    let btn = &shape.custom_classes[0];
    assert_eq!(btn.name, "btn");
    assert_eq!(btn.applied_utilities, vec!["px-4", "py-2", "rounded"]);
    assert_eq!(btn.layer.as_ref().map(|s| s.as_ref()), Some("components"));

    let card = &shape.custom_classes[1];
    assert_eq!(card.name, "card");
    assert_eq!(card.applied_utilities, vec!["bg-white", "shadow-md"]);
}

#[test]
fn test_extract_keyframes() {
    let source = r#"
@keyframes fade-in {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes slide-up {
  from { transform: translateY(100%); }
  to { transform: translateY(0); }
}
"#;

    let shape = extract_css_tailwind(source, None).expect("Failed to extract CSS shape");

    assert_eq!(shape.keyframes.len(), 2);
    assert_eq!(shape.keyframes[0].name, "fade-in");
    assert_eq!(shape.keyframes[1].name, "slide-up");
}

#[test]
fn test_extract_utilities_layer() {
    let source = r#"
@layer utilities {
  .text-balance {
    text-wrap: balance;
  }
}
"#;

    let shape = extract_css_tailwind(source, None).expect("Failed to extract CSS shape");

    assert_eq!(shape.custom_classes.len(), 1);
    assert_eq!(shape.custom_classes[0].name, "text-balance");
    assert_eq!(
        shape.custom_classes[0].layer.as_ref().map(|s| s.as_ref()),
        Some("utilities")
    );
}

#[test]
fn test_extract_minimal_fixture() {
    let source = std::fs::read_to_string("tests/fixtures/minimal/simple.css")
        .expect("Failed to read fixture");

    let shape = extract_css_tailwind(&source, Some("tests/fixtures/minimal/simple.css"))
        .expect("Failed to extract CSS shape");

    assert_eq!(
        shape.path,
        Some("tests/fixtures/minimal/simple.css".to_string())
    );
    assert_eq!(shape.theme.len(), 1);
    assert_eq!(shape.theme[0].name, "--color-primary");
    assert_eq!(shape.theme[0].value, "blue");

    assert_eq!(shape.custom_classes.len(), 1);
    assert_eq!(shape.custom_classes[0].name, "btn");
    assert_eq!(
        shape.custom_classes[0].applied_utilities,
        vec!["px-4", "py-2"]
    );
}

#[test]
fn test_empty_css() {
    let source = "";
    let shape = extract_css_tailwind(source, None).expect("Failed to extract CSS shape");

    assert_eq!(shape.theme.len(), 0);
    assert_eq!(shape.custom_classes.len(), 0);
    assert_eq!(shape.keyframes.len(), 0);
}

#[test]
fn test_minified_css() {
    let source = "@theme{--color-primary:blue;}@layer components{.btn{@apply px-4 py-2;}}";
    let shape = extract_css_tailwind(source, None).expect("Failed to extract CSS shape");

    assert_eq!(shape.theme.len(), 1);
    assert_eq!(shape.theme[0].name, "--color-primary");
    assert_eq!(shape.theme[0].value, "blue");

    assert_eq!(shape.custom_classes.len(), 1);
    assert_eq!(shape.custom_classes[0].name, "btn");
}
