mod common;

#[test]
fn test_fixture_dir_accepts_language_alias() {
    let path = common::fixture_dir("rust");
    assert!(path.ends_with("tests/fixtures/rust_project"));
    assert!(path.exists());
}

#[test]
fn test_fixture_dir_accepts_full_project_name() {
    let path = common::fixture_dir("rust_project");
    assert!(path.ends_with("tests/fixtures/rust_project"));
    assert!(path.exists());
}

#[test]
fn test_fixture_path_canonicalizes_existing_files() {
    let path = common::fixture_path("java", "Calculator.java");
    assert!(path.ends_with("tests/fixtures/java_project/Calculator.java"));
    assert!(path.exists());
}
