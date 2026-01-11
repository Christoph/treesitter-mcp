mod common;

use treesitter_mcp::extraction::types::{extract_types, LimitHit, TypeKind};

#[test]
fn extracts_typescript_interfaces_and_aliases() {
    let file_path = common::fixture_path("typescript", "types/models.ts");
    let result = extract_types(&file_path, None, 1000).expect("type extraction should succeed");

    let point = result
        .types
        .iter()
        .find(|t| t.name == "Point")
        .expect("Point interface should be extracted");
    assert_eq!(point.kind, TypeKind::Interface);
    assert!(point
        .members
        .as_ref()
        .expect("interfaces should include members")
        .iter()
        .any(|member| member.name == "x" && member.type_annotation == "number"));

    let alias = result
        .types
        .iter()
        .find(|t| t.name == "OperationResult")
        .expect("type alias should be extracted");
    assert_eq!(alias.kind, TypeKind::TypeAlias);
}

#[test]
fn extracts_rust_struct_with_fields_and_methods() {
    let file_path = common::fixture_path("rust", "src/models/mod.rs");
    let result = extract_types(&file_path, None, 1000).expect("type extraction should succeed");

    let point = result
        .types
        .iter()
        .find(|t| t.name == "Point")
        .expect("Point struct should be extracted");
    assert_eq!(point.kind, TypeKind::Struct);

    let fields = point.fields.as_ref().expect("struct should expose fields");
    assert!(fields
        .iter()
        .any(|field| field.name == "x" && field.type_annotation == "i32"));
    assert!(fields.iter().any(|field| field.name == "y"));

    let members = point
        .members
        .as_ref()
        .expect("impl block should expose members");
    assert!(members.iter().any(|member| member.name == "new"));
}

#[test]
fn directory_scan_respects_pattern_and_limit() {
    let dir_path = common::fixture_dir("typescript");
    let result =
        extract_types(&dir_path, Some("**/*.ts"), 1).expect("type extraction should succeed");

    assert!(result.types.len() <= 1);
    assert_eq!(result.limit_hit, Some(LimitHit::TypeLimit));
}
