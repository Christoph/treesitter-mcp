# Testing Guide for treesitter-mcp

This guide provides best practices and patterns for writing maintainable, behavior-focused tests.

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

### 1. Use the Assertion DSL

```rust
use common::assertions::*;

#[test]
fn test_example() {
    let shape = parse_file("calculator.rs");
    
    // Fluent, readable assertions
    assert_parse_result(&shape)
        .has_language("Rust")
        .has_relative_path()
        .has_function("add")
            .signature_contains("i32")
            .code_contains("a + b")
            .has_doc_containing("Adds two numbers");
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

## Assertion DSL

### ParseResultAssert

```rust
assert_parse_result(&shape)
    .has_language("Rust")                    // Language detected correctly
    .has_relative_path()                     // Path is relative (token optimization)
    .path_contains("calculator.rs")          // Path contains expected component
    .has_function("add")                     // Function exists
        .signature_contains("i32")           // Signature has type
        .code_contains("a + b")              // Implementation is correct
        .has_doc_containing("Adds")          // Documentation present
        .at_or_after_line(10)                // Position check
    .has_class("Calculator")                 // Class/struct exists
        .code_contains("pub value")          // Field present
    .has_import_containing("std::fmt");      // Import present
```

### UsagesAssert

```rust
assert_usages(&usages)
    .has_at_least(3)                         // Minimum usage count
    .has_exactly(5)                          // Exact usage count
    .all_have_code()                         // All usages have code snippets
    .all_have_no_code()                      // No code (max_context_lines=0)
    .has_usage_in_file("calculator.rs")      // Usage in specific file
    .all_paths_relative()                    // All paths are relative
    .total_context_lines_within(100);        // Total context within limit
```

### CodeMapAssert

```rust
assert_code_map(&map)
    .has_at_least_files(5)                   // Minimum file count
    .has_file_matching("calculator.rs")      // Specific file present
    .all_paths_relative();                   // All paths relative
```

### ContextAssert

```rust
assert_context(&context)
    .innermost_is("function_item")           // Innermost scope type
    .innermost_named("add")                  // Innermost scope name
    .innermost_has_code()                    // Code present
    .has_at_least_levels(2);                 // Nesting depth
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

### Step 1: Add Assertion Helpers (Week 1)

1. Copy `tests/common/assertions.rs` (already created)
2. Update `tests/common/mod.rs` to export assertions
3. Run tests to ensure no breakage

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

### Step 3: Consolidate Duplicate Tests (Week 2)

Combine language-specific tests:

```rust
// Before: 4 separate tests
test_parse_file_rust_functions()
test_parse_file_python_functions()
test_parse_file_javascript_functions()
test_parse_file_typescript_functions()

// After: 1 parameterized test
test_parse_file_extracts_functions_for_all_languages()
```

### Step 4: Fix Ignored Tests (Week 2)

For each ignored test:
1. Can it be implemented now? → Implement and enable
2. Is it blocked? → Create GitHub issue, remove test
3. Is it obsolete? → Delete test

### Step 5: Rename Tests (Week 3)

Use search-replace to rename tests:
- `test_<tool>_<language>` → `test_<tool>_<behavior>`
- Focus on "what" not "how"

### Step 6: Add Property-Based Tests (Week 3)

Add `proptest` to `Cargo.toml`:
```toml
[dev-dependencies]
proptest = "1.0"
```

Create property tests for invariants:
- Parser never crashes
- Relative paths always shorter
- Context always has at least one level

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
│   ├── assertions.rs       # Fluent assertion DSL
│   └── builders.rs         # Test fixture builders
├── unit/                   # Fast, no I/O
│   ├── parser_test.rs
│   └── shape_test.rs
├── integration/            # File system, git
│   ├── parse_file_test.rs
│   ├── find_usages_test.rs
│   └── diff_test.rs
├── fixtures/               # Test data
│   ├── rust_project/
│   ├── python_project/
│   └── javascript_project/
└── benches/                # Performance tests
    └── parse_bench.rs
```

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
