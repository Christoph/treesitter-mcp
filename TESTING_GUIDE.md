# Testing Guide for treesitter-mcp

This guide provides best practices and patterns for writing maintainable, behavior-focused tests.

## Current Test Suite Status

✅ **Phase 1 & 2 Complete** - Test suite significantly improved:

- **242 passing tests** (1 intentionally ignored)
- **0 ignored tests** (down from 14)
- **30+ tests renamed** to behavior-focused names
- **8 cross-language tests** reducing duplication
- **8 property tests** for robustness
- **4 helper functions** for cleaner assertions
- **Test metrics dashboard** (`test_metrics.sh`)
- **~5 second test duration** (fast feedback)

**See TEST_ORGANIZATION.md for detailed test categorization.**

## Table of Contents

1. [Philosophy](#philosophy)
2. [Quick Start](#quick-start)
3. [Test Patterns](#test-patterns)
4. [Assertion DSL](#assertion-dsl)
5. [Common Pitfalls](#common-pitfalls)
6. [Migration Guide](#migration-guide)
7. [Advanced Techniques](#advanced-techniques)

---

## Philosophy

### The Four Pillars of Good Tests

Every test should maximize these four qualities:

1. **Protection against regressions** - Catches bugs when code breaks
2. **Resistance to refactoring** - Doesn't break when implementation changes
3. **Fast feedback** - Runs quickly
4. **Maintainability** - Easy to understand and modify

### Test Behaviors, Not Implementation

```rust
// ❌ BAD: Tests implementation details
#[test]
fn test_parse_file_returns_json_with_functions_array() {
    let shape = parse_file("test.rs");
    assert!(shape["functions"].is_array());
    assert_eq!(shape["functions"].as_array().unwrap().len(), 5);
}

// ✅ GOOD: Tests behavior
#[test]
fn test_parse_file_extracts_all_function_signatures() {
    let shape = parse_file("test.rs");
    assert_parse_result(&shape)
        .has_functions(&["add", "subtract", "multiply", "divide", "apply_operation"]);
}
```

**Why?** The first test breaks if we change JSON structure. The second test only breaks if we stop extracting functions.

---

## Quick Start

### 1. Use the Helper Functions

```rust
use common::helpers;

#[test]
fn test_example() {
    let shape = parse_file("calculator.rs");
    
    // Use helper functions for cleaner assertions
    helpers::assert_has_function(&shape, "add");
    helpers::assert_function_code_contains(&shape, "add", "a + b");
    helpers::assert_min_count(&shape, "functions", 4);
}
```

### 2. Follow Given-When-Then

```rust
#[test]
fn test_find_usages_includes_all_call_sites() {
    // Given: A project with a function called in multiple places
    let project = fixture_dir("rust");
    
    // When: We search for usages of that function
    let usages = find_usages("add", &project);
    
    // Then: We find all call sites with context
    assert_usages(&usages)
        .has_at_least(3)
        .all_have_code()
        .has_usage_in_file("calculator.rs");
}
```

### 3. Name Tests by Behavior

```rust
// ❌ BAD: Implementation-focused names
test_parse_file_rust()
test_get_context_inside_function()

// ✅ GOOD: Behavior-focused names
test_parse_file_extracts_function_signatures()
test_get_context_returns_enclosing_scope_hierarchy()
test_find_usages_locates_all_references_across_files()
```

---

## Test Patterns

### Pattern 1: Parameterized Tests

**Use when:** Testing the same behavior across multiple inputs.

```rust
#[test]
fn test_parse_file_supports_all_languages() {
    let test_cases = vec![
        ("rust", "calculator.rs", "Rust"),
        ("python", "calculator.py", "Python"),
        ("javascript", "calculator.js", "JavaScript"),
    ];
    
    for (lang, file, expected_lang) in test_cases {
        let shape = parse_file_fixture(lang, file);
        assert_parse_result(&shape)
            .has_language(expected_lang)
            .has_functions(&["add", "subtract"]);
    }
}
```

### Pattern 2: Property-Based Testing

**Use when:** Testing invariants that should hold for any input.

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_parse_file_never_crashes(code in "\\PC{0,1000}") {
        // Property: Parser should never panic, even on random input
        let result = parse_code(&code, Language::Rust);
        assert!(result.is_ok() || result.is_err());
    }
    
    #[test]
    fn test_relative_path_always_shorter_than_absolute(
        depth in 1..10usize,
        filename in "[a-z]{1,20}\\.rs"
    ) {
        // Property: Relative paths should always be shorter
        let abs_path = create_nested_path(depth, &filename);
        let rel_path = make_relative(&abs_path);
        assert!(rel_path.len() < abs_path.len());
    }
}
```

### Pattern 3: Table-Driven Tests

**Use when:** Testing multiple scenarios with different expected outcomes.

```rust
#[test]
fn test_language_detection_from_extension() {
    let test_cases = vec![
        ("file.rs", Some(Language::Rust)),
        ("file.py", Some(Language::Python)),
        ("file.js", Some(Language::JavaScript)),
        ("file.txt", None),
        ("no_extension", None),
    ];
    
    for (filename, expected) in test_cases {
        let result = detect_language(filename);
        assert_eq!(result, expected, "Failed for {}", filename);
    }
}
```

### Pattern 4: Builder Pattern for Complex Setup

**Use when:** Tests need complex fixture setup.

```rust
struct TestProject {
    dir: TempDir,
}

impl TestProject {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        Self { dir }
    }
    
    fn with_file(self, path: &str, content: &str) -> Self {
        let file_path = self.dir.path().join(path);
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, content).unwrap();
        self
    }
    
    fn with_git(self) -> Self {
        Command::new("git").args(["init"]).current_dir(self.dir.path()).output().unwrap();
        self
    }
    
    fn commit(self, message: &str) -> Self {
        Command::new("git").args(["add", "."]).current_dir(self.dir.path()).output().unwrap();
        Command::new("git").args(["commit", "-m", message]).current_dir(self.dir.path()).output().unwrap();
        self
    }
}

#[test]
fn test_diff_detects_added_functions() {
    let project = TestProject::new()
        .with_git()
        .with_file("lib.rs", "fn old() {}")
        .commit("Initial")
        .with_file("lib.rs", "fn old() {}\nfn new() {}")
        ;
    
    let diff = parse_diff(&project.dir.path().join("lib.rs"));
    assert_eq!(diff.added_functions(), vec!["new"]);
}
```

---

## Helper Functions

The `tests/common/helpers.rs` module provides assertion helpers to make tests more readable and maintainable.

### Available Helpers

```rust
use common::helpers;

// Assert that a shape has a function with the given name
helpers::assert_has_function(&shape, "add");

// Assert that a function's code contains specific text
helpers::assert_function_code_contains(&shape, "add", "a + b");

// Assert that all paths in a result are relative (no absolute markers)
helpers::assert_all_paths_relative(&usages, "usages");

// Assert minimum number of items in an array field
helpers::assert_min_count(&shape, "functions", 4);
```

### Example Usage

```rust
#[test]
fn test_parse_file_extracts_function_signatures_and_code() {
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });
    
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();
    
    // Use helpers for cleaner assertions
    common::helpers::assert_has_function(&shape, "add");
    common::helpers::assert_has_function(&shape, "subtract");
    common::helpers::assert_has_function(&shape, "multiply");
    common::helpers::assert_has_function(&shape, "divide");
    common::helpers::assert_function_code_contains(&shape, "add", "a + b");
    common::helpers::assert_function_code_contains(&shape, "add", "pub fn add");
}
```

---

## Common Pitfalls

### Pitfall 1: Testing JSON Structure

```rust
// ❌ BRITTLE: Breaks when JSON structure changes
assert!(shape["functions"].is_array());
let functions = shape["functions"].as_array().unwrap();
assert_eq!(functions[0]["name"], "add");

// ✅ ROBUST: Tests behavior, not structure
assert_parse_result(&shape).has_function("add");
```

### Pitfall 2: Exact Counts

```rust
// ❌ BRITTLE: Breaks when adding helper functions
assert_eq!(functions.len(), 5);

// ✅ ROBUST: Tests minimum requirement
assert_parse_result(&shape).has_functions(&["add", "subtract", "multiply", "divide"]);
```

### Pitfall 3: Absolute Line Numbers

```rust
// ❌ BRITTLE: Breaks when adding comments
assert_eq!(func["line"], 42);

// ✅ ROBUST: Tests relative position or presence
assert_parse_result(&shape)
    .has_function("add").at_or_after_line(10);
```

### Pitfall 4: Over-Mocking

```rust
// ❌ OVER-MOCKED: Tests mock behavior, not real behavior
let mock_parser = MockParser::new();
mock_parser.expect_parse().returning(|_| Ok(mock_tree));

// ✅ REAL: Tests actual parser with real fixtures
let tree = parse_code(fixture_content, Language::Rust);
```

### Pitfall 5: Ignored Tests

```rust
// ❌ BAD: Test doesn't run, feature forgotten
#[test]
#[ignore]
fn test_future_feature() { /* ... */ }

// ✅ GOOD: Remove test, create GitHub issue
// Issue #123: Implement max_context_lines feature
```

---

## Migration Guide

### Step 1: Add Helper Functions (✅ Completed)

1. ✅ Created `tests/common/helpers.rs` with assertion helpers
2. ✅ Updated `tests/common/mod.rs` to export helpers
3. ✅ All tests passing with new helpers

### Step 2: Migrate One Test File (Week 1)

Pick a small file like `parser_test.rs`:

```rust
// Before
#[test]
fn test_detect_language_from_rust_file() {
    let lang = detect_language("test.rs");
    assert_eq!(lang, Some(Language::Rust));
}

// After
#[test]
fn test_language_detection_identifies_rust_files() {
    assert_eq!(detect_language("test.rs"), Some(Language::Rust));
}
```

### Step 3: Consolidate Duplicate Tests (✅ Completed)

✅ Created `cross_language_test.rs` with parameterized tests:

```rust
// Before: 4+ separate tests per language
test_parse_file_rust_functions()
test_parse_file_python_functions()
test_parse_file_javascript_functions()
test_parse_file_typescript_functions()

// After: 1 parameterized test covering all languages
test_parse_file_extracts_functions_from_all_languages()
```

**Result:** 8 cross-language tests covering parse_file, find_usages, get_context, and code_map.

### Step 4: Fix Ignored Tests (✅ Completed)

✅ All ignored tests have been addressed:
1. ✅ Removed `find_usages_max_context_test.rs` (14 ignored tests for unimplemented feature)
2. ✅ Removed `file_shape_removal_test.rs` (placeholder test)
3. ✅ **Result: 0 ignored tests** (down from 14)

### Step 5: Rename Tests (✅ Completed)

✅ Systematically renamed 30+ tests to be behavior-focused:
- `test_parse_file_rust_functions` → `test_parse_file_extracts_function_signatures_and_code`
- `test_find_usages_rust_function_calls` → `test_find_usages_locates_all_call_sites`
- `test_get_context_rust_inside_function` → `test_get_context_returns_function_as_innermost_scope`
- `test_code_map_rust_project_minimal` → `test_code_map_provides_minimal_overview_with_names_only`

**Result:** Test names now describe behavior, not implementation details.

### Step 6: Add Property-Based Tests (✅ Completed)

✅ Added `proptest` to `Cargo.toml` and created `property_tests.rs`:

```toml
[dev-dependencies]
proptest = "1.0"
```

✅ Created 8 property tests for invariants:
- ✅ Parser never panics on invalid paths
- ✅ Find usages never panics on random symbols
- ✅ Get context never panics on random positions
- ✅ Context always has at least one level
- ✅ Find usages paths are consistent
- ✅ Code map respects token limits
- ✅ Parse file is deterministic
- ✅ Find usages is deterministic

---

## Advanced Techniques

### Technique 1: Snapshot Testing

For complex outputs, use snapshot testing:

```rust
use insta::assert_json_snapshot;

#[test]
fn test_parse_file_complete_output() {
    let shape = parse_file("calculator.rs");
    
    // First run creates snapshot, subsequent runs compare
    assert_json_snapshot!(shape, {
        ".path" => "[path]",  // Redact dynamic values
        ".functions[].line" => "[line]",
    });
}
```

### Technique 2: Mutation Testing

Use `cargo-mutants` to verify test effectiveness:

```bash
cargo install cargo-mutants
cargo mutants
```

This mutates your code and checks if tests catch the mutations.

### Technique 3: Coverage-Guided Testing

Use `cargo-tarpaulin` to find untested code:

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

### Technique 4: Benchmark Tests

For performance-critical code:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parse_large_file(c: &mut Criterion) {
    let large_file = generate_large_rust_file(10000); // 10k lines
    
    c.bench_function("parse_large_file", |b| {
        b.iter(|| {
            parse_code(black_box(&large_file), Language::Rust)
        });
    });
}

criterion_group!(benches, bench_parse_large_file);
criterion_main!(benches);
```

### Technique 5: Fuzz Testing

Use `cargo-fuzz` for finding edge cases:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_code(s, Language::Rust);
    }
});
```

---

## Test Organization

### Directory Structure

```
tests/
├── common/
│   ├── mod.rs              # Shared utilities
│   └── helpers.rs          # Helper assertion functions
├── fixtures/               # Test data
│   ├── rust_project/
│   ├── python_project/
│   ├── javascript_project/
│   └── typescript_project/
├── parser_test.rs          # Unit: Parser logic (fast, no I/O)
├── parser_parsing_test.rs  # Unit: AST parsing
├── shape_module_test.rs    # Unit: Shape extraction
├── parse_file_tool_test.rs # Integration: Parse file tool
├── find_usages_tool_test.rs # Integration: Find usages tool
├── get_context_test.rs     # Integration: Get context tool
├── code_map_tool_test.rs   # Integration: Code map tool
├── cross_language_test.rs  # Integration: Cross-language tests
├── property_tests.rs       # Integration: Property-based tests
├── diff_tool_test.rs       # Integration: Diff analysis (uses git)
└── ...                     # Other integration tests
```

**See TEST_ORGANIZATION.md for detailed categorization and run commands.**

### Test Naming Convention

```
test_<behavior>_<scenario>_<expected_outcome>

Examples:
- test_parse_file_extracts_functions_from_rust_code()
- test_find_usages_locates_all_references_across_files()
- test_get_context_returns_empty_for_invalid_position()
- test_code_map_respects_token_limit_when_large_project()
```

---

## Checklist for New Tests

Before submitting a test, verify:

- [ ] Test name describes behavior, not implementation
- [ ] Uses Given-When-Then structure
- [ ] Uses assertion DSL instead of manual JSON navigation
- [ ] Tests behavior, not JSON structure
- [ ] No hardcoded line numbers (use relative checks)
- [ ] No exact counts (use `has_at_least` instead)
- [ ] Fast (< 100ms for unit tests)
- [ ] Independent (doesn't depend on other tests)
- [ ] No `#[ignore]` without GitHub issue reference
- [ ] Assertion messages explain what failed

---

## Resources

- [Unit Testing Principles, Practices, and Patterns](https://www.manning.com/books/unit-testing) by Vladimir Khorikov
- [Property-Based Testing with PropTest](https://altsysrq.github.io/proptest-book/intro.html)
- [The Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Mutation Testing with cargo-mutants](https://github.com/sourcefrog/cargo-mutants)

---

## Getting Help

- **Questions?** Open a discussion on GitHub
- **Found a bug?** Open an issue with a failing test
- **Want to contribute?** Start by improving test coverage

Happy testing! 🧪
