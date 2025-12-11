# Test Suite Improvement Plan

## Executive Summary

This document provides a **comprehensive, actionable roadmap** to improve the treesitter-mcp test suite from **B+ (87/100) to A+ (95+/100)**.

**Current State:**
- 6,641 lines of test code across 22 files
- Good coverage, but brittle assertions
- 14 ignored tests blocking progress
- Tests coupled to JSON structure

**Target State:**
- Behavior-focused, maintainable tests
- Zero ignored tests
- Decoupled from implementation details
- Property-based testing for robustness

---

## Phase 1: Quick Wins (Week 1) 🎯

### 1.1 Fix Ignored Tests (HIGH PRIORITY)

**Problem:** 14 tests in `find_usages_max_context_test.rs` are ignored with "Feature not yet implemented".

**Action:**
```bash
# Option A: Remove tests, create GitHub issue
git rm tests/find_usages_max_context_test.rs
gh issue create --title "Implement max_context_lines feature" \
  --body "See removed test file for requirements"

# Option B: Implement the feature
# (Requires backend changes to find_usages)
```

**Success Criteria:**
- [ ] Zero `#[ignore]` attributes in test suite
- [ ] GitHub issue created for unimplemented features
- [ ] CI fails if new ignored tests are added

**Estimated Time:** 2 hours

---

### 1.2 Create Helper Functions (MEDIUM PRIORITY)

**Problem:** Tests manually navigate JSON structure, making them brittle.

**Action:** Create `tests/common/helpers.rs`:

```rust
//! Helper functions for common test assertions

use serde_json::Value;

/// Assert that a shape has a function with the given name
pub fn assert_has_function(shape: &Value, name: &str) {
    let functions = shape["functions"]
        .as_array()
        .unwrap_or_else(|| panic!("Shape should have functions array"));
    
    let found = functions.iter().any(|f| f["name"] == name);
    assert!(found, "Should find function '{}' in {:?}", name, 
        functions.iter().map(|f| &f["name"]).collect::<Vec<_>>());
}

/// Assert that a function has code containing specific text
pub fn assert_function_code_contains(shape: &Value, func_name: &str, code_text: &str) {
    let functions = shape["functions"].as_array().unwrap();
    let func = functions.iter()
        .find(|f| f["name"] == func_name)
        .unwrap_or_else(|| panic!("Should find function '{}'", func_name));
    
    let code = func["code"].as_str()
        .unwrap_or_else(|| panic!("Function '{}' should have code", func_name));
    
    assert!(code.contains(code_text),
        "Function '{}' code should contain '{}', got:\n{}",
        func_name, code_text, code);
}

/// Assert that all paths in a result are relative (no absolute markers)
pub fn assert_all_paths_relative(value: &Value, path_field: &str) {
    let items = value.as_array()
        .or_else(|| value[path_field].as_array())
        .unwrap();
    
    for item in items {
        let path = item["path"].as_str()
            .or_else(|| item["file"].as_str())
            .unwrap();
        
        assert!(!path.contains("/Users/") && 
                !path.contains("/home/") && 
                !path.starts_with("C:\\"),
            "Path should be relative, got: {}", path);
    }
}

/// Assert minimum number of items in an array field
pub fn assert_min_count(value: &Value, field: &str, min: usize) {
    let items = value[field].as_array()
        .unwrap_or_else(|| panic!("Should have '{}' array", field));
    
    assert!(items.len() >= min,
        "Should have at least {} items in '{}', got {}",
        min, field, items.len());
}
```

**Usage Example:**

```rust
// Before (brittle)
let functions = shape["functions"].as_array().unwrap();
let add_fn = functions.iter().find(|f| f["name"] == "add").unwrap();
assert!(add_fn["code"].as_str().unwrap().contains("a + b"));

// After (robust)
assert_has_function(&shape, "add");
assert_function_code_contains(&shape, "add", "a + b");
```

**Success Criteria:**
- [ ] Helper module created
- [ ] At least 5 helper functions implemented
- [ ] 2-3 existing tests migrated to use helpers

**Estimated Time:** 4 hours

---

### 1.3 Fix `file_shape_removal_test.rs` (LOW PRIORITY)

**Problem:** Test is a placeholder that doesn't actually test anything.

**Action:**
```bash
# Option A: Delete if file_shape is already removed
git rm tests/file_shape_removal_test.rs

# Option B: Implement proper test if removal is pending
# (Check if file_shape module still exists)
```

**Success Criteria:**
- [ ] Test either properly validates removal or is deleted
- [ ] No placeholder tests in suite

**Estimated Time:** 30 minutes

---

## Phase 2: Structural Improvements (Week 2) 🏗️

### 2.1 Consolidate Duplicate Tests

**Problem:** Similar tests repeated for each language (Rust, Python, JS, TS).

**Action:** Create parameterized tests in `tests/cross_language_test.rs`:

```rust
//! Cross-language tests to reduce duplication

use serde_json::json;
mod common;

#[test]
fn test_parse_file_extracts_functions_for_all_languages() {
    let test_cases = vec![
        ("rust", "src/calculator.rs", "Rust", vec!["add", "subtract", "multiply", "divide"]),
        ("python", "calculator.py", "Python", vec!["add", "subtract", "multiply", "divide"]),
        ("javascript", "calculator.js", "JavaScript", vec!["add", "subtract", "multiply", "divide"]),
        ("typescript", "calculator.ts", "TypeScript", vec!["add", "subtract", "multiply", "divide"]),
    ];
    
    for (lang, file, expected_lang, expected_funcs) in test_cases {
        let file_path = common::fixture_path(lang, file);
        let arguments = json!({
            "file_path": file_path.to_str().unwrap()
        });
        
        let result = treesitter_mcp::analysis::parse_file::execute(&arguments)
            .unwrap_or_else(|e| panic!("parse_file failed for {}: {}", lang, e));
        
        let text = common::get_result_text(&result);
        let shape: serde_json::Value = serde_json::from_str(&text).unwrap();
        
        // Verify language
        assert_eq!(shape["language"], expected_lang, "Wrong language for {}", lang);
        
        // Verify functions
        for func_name in expected_funcs {
            common::helpers::assert_has_function(&shape, func_name);
        }
    }
}

#[test]
fn test_parse_file_extracts_classes_for_all_languages() {
    let test_cases = vec![
        ("rust", "src/models/mod.rs", vec!["Calculator", "Point"]),
        ("python", "calculator.py", vec!["Calculator", "Point"]),
        ("javascript", "calculator.js", vec!["Calculator", "Point"]),
        ("typescript", "calculator.ts", vec!["Calculator", "Point"]),
    ];
    
    for (lang, file, expected_classes) in test_cases {
        let file_path = common::fixture_path(lang, file);
        let arguments = json!({
            "file_path": file_path.to_str().unwrap()
        });
        
        let result = treesitter_mcp::analysis::parse_file::execute(&arguments).unwrap();
        let text = common::get_result_text(&result);
        let shape: serde_json::Value = serde_json::from_str(&text).unwrap();
        
        let classes = shape["classes"].as_array()
            .or_else(|| shape["structs"].as_array())
            .unwrap_or_else(|| panic!("Should have classes/structs for {}", lang));
        
        for class_name in expected_classes {
            let found = classes.iter().any(|c| c["name"] == class_name);
            assert!(found, "Should find class '{}' in {}", class_name, lang);
        }
    }
}
```

**Files to Consolidate:**
- `parse_file_tool_test.rs` (lines 10-289: language-specific tests)
- `find_usages_tool_test.rs` (lines 10-231: language-specific tests)
- `get_context_test.rs` (lines 10-610: language-specific tests)

**Success Criteria:**
- [ ] New `cross_language_test.rs` file created
- [ ] At least 50% reduction in duplicate test code
- [ ] All languages still tested

**Estimated Time:** 6 hours

---

### 2.2 Rename Tests for Behavior

**Problem:** Test names describe implementation, not behavior.

**Action:** Systematic renaming:

```bash
# Create a script to help with renaming
cat > rename_tests.sh << 'EOF'
#!/bin/bash

# Rename pattern: test_<tool>_<language> → test_<tool>_<behavior>

declare -A renames=(
    ["test_parse_file_rust_functions"]="test_parse_file_extracts_function_signatures"
    ["test_find_usages_rust_function_calls"]="test_find_usages_locates_all_call_sites"
    ["test_get_context_rust_inside_function"]="test_get_context_returns_enclosing_scope"
    ["test_code_map_rust_project_minimal"]="test_code_map_provides_minimal_overview"
    # Add more mappings...
)

for old in "${!renames[@]}"; do
    new="${renames[$old]}"
    echo "Renaming $old → $new"
    find tests -name "*.rs" -exec sed -i '' "s/$old/$new/g" {} \;
done
EOF

chmod +x rename_tests.sh
./rename_tests.sh
```

**Naming Convention:**
```
test_<behavior>_<scenario>_<expected_outcome>

Examples:
✅ test_parse_file_extracts_functions_from_rust_code()
✅ test_find_usages_includes_definition_and_call_sites()
✅ test_get_context_returns_empty_for_top_level_position()
✅ test_code_map_respects_token_limit_for_large_projects()

❌ test_parse_file_rust()
❌ test_find_usages_function()
❌ test_get_context_inside()
```

**Success Criteria:**
- [ ] All test names follow new convention
- [ ] Test names describe "what" not "how"
- [ ] Names are self-documenting

**Estimated Time:** 4 hours

---

### 2.3 Separate Unit and Integration Tests

**Problem:** Fast unit tests mixed with slow integration tests.

**Action:** Reorganize test directory:

```bash
# Create new structure
mkdir -p tests/unit tests/integration

# Move fast tests (no I/O) to unit/
mv tests/parser_test.rs tests/unit/
mv tests/parser_parsing_test.rs tests/unit/
mv tests/shape_module_test.rs tests/unit/

# Move slow tests (git, filesystem) to integration/
mv tests/diff_tool_test.rs tests/integration/
mv tests/relative_paths_test.rs tests/integration/

# Update Cargo.toml
cat >> Cargo.toml << 'EOF'

[[test]]
name = "unit"
path = "tests/unit/mod.rs"

[[test]]
name = "integration"
path = "tests/integration/mod.rs"
EOF
```

**Run Tests Selectively:**
```bash
# Fast feedback: unit tests only
cargo test --test unit

# Full validation: all tests
cargo test

# Slow tests only
cargo test --test integration
```

**Success Criteria:**
- [ ] Tests organized by speed
- [ ] Unit tests run in < 5 seconds
- [ ] Can run unit tests independently

**Estimated Time:** 3 hours

---

## Phase 3: Advanced Testing (Week 3-4) 🚀

### 3.1 Add Property-Based Testing

**Problem:** Tests only cover known cases, not edge cases.

**Action:** Add `proptest` to `Cargo.toml`:

```toml
[dev-dependencies]
proptest = "1.0"
```

Create `tests/property_tests.rs`:

```rust
use proptest::prelude::*;
use treesitter_mcp::parser::{parse_code, Language};

proptest! {
    /// Property: Parser should never panic on any input
    #[test]
    fn test_parser_never_panics_on_random_input(
        code in "\\PC{0,1000}",  // Any printable chars, 0-1000 length
        lang in prop_oneof![
            Just(Language::Rust),
            Just(Language::Python),
            Just(Language::JavaScript),
        ]
    ) {
        // Should either succeed or return error, never panic
        let result = parse_code(&code, lang);
        prop_assert!(result.is_ok() || result.is_err());
    }
    
    /// Property: Relative paths are always shorter than absolute paths
    #[test]
    fn test_relative_paths_always_shorter(
        depth in 1..10usize,
        filename in "[a-z]{1,20}\\.(rs|py|js)"
    ) {
        let abs_path = create_nested_absolute_path(depth, &filename);
        let rel_path = make_relative(&abs_path);
        
        prop_assert!(
            rel_path.len() < abs_path.len(),
            "Relative path ({}) should be shorter than absolute ({})",
            rel_path.len(), abs_path.len()
        );
    }
    
    /// Property: Context always has at least one level (source_file)
    #[test]
    fn test_context_always_has_source_file(
        line in 1..1000u64,
        column in 1..200u64
    ) {
        let file_path = fixture_path("rust", "src/calculator.rs");
        let context = get_context(&file_path, line, column);
        
        if let Ok(ctx) = context {
            let contexts = ctx["contexts"].as_array().unwrap();
            prop_assert!(!contexts.is_empty(), "Should have at least source_file context");
            
            let outermost = contexts.last().unwrap();
            prop_assert_eq!(outermost["type"], "source_file");
        }
    }
}
```

**Success Criteria:**
- [ ] Property tests added for core invariants
- [ ] Tests find edge cases not covered by examples
- [ ] CI runs property tests with 1000+ cases

**Estimated Time:** 8 hours

---

### 3.2 Add Mutation Testing

**Problem:** Don't know if tests actually catch bugs.

**Action:** Install and run `cargo-mutants`:

```bash
# Install
cargo install cargo-mutants

# Run mutation testing
cargo mutants --test-tool=nextest

# Generate report
cargo mutants --output mutants.out
```

**Interpret Results:**
```
Caught mutations: Tests caught the bug ✅
Missed mutations: Tests didn't catch the bug ❌
Timeout mutations: Tests are too slow ⚠️
```

**Action on Missed Mutations:**
```rust
// Example: Mutant changed `>=` to `>` and tests didn't catch it
// Original code:
if usages.len() >= min_count { ... }

// Add test to catch this:
#[test]
fn test_find_usages_exact_minimum_count() {
    // This test will fail if >= becomes >
    let usages = find_usages("rare_symbol", &project);
    assert_eq!(usages.len(), 1); // Exact count, not >=
}
```

**Success Criteria:**
- [ ] Mutation testing integrated into CI
- [ ] > 80% mutation score
- [ ] Tests added for missed mutations

**Estimated Time:** 6 hours

---

### 3.3 Add Performance Benchmarks

**Problem:** No visibility into performance regressions.

**Action:** Create `benches/parse_bench.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use treesitter_mcp::parser::{parse_code, Language};

fn generate_rust_file(lines: usize) -> String {
    let mut code = String::new();
    for i in 0..lines {
        code.push_str(&format!("fn func_{}() {{ println!(\"test\"); }}\n", i));
    }
    code
}

fn bench_parse_by_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_by_size");
    
    for size in [100, 1000, 5000, 10000].iter() {
        let code = generate_rust_file(*size);
        
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, _| {
                b.iter(|| {
                    parse_code(black_box(&code), Language::Rust)
                });
            },
        );
    }
    
    group.finish();
}

fn bench_find_usages_by_project_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_usages_by_project_size");
    
    // Benchmark on projects of different sizes
    for (name, path) in [
        ("small", "tests/fixtures/minimal"),
        ("medium", "tests/fixtures/rust_project"),
    ].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            path,
            |b, path| {
                b.iter(|| {
                    find_usages(black_box("Calculator"), black_box(path))
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(benches, bench_parse_by_size, bench_find_usages_by_project_size);
criterion_main!(benches);
```

**Run Benchmarks:**
```bash
cargo bench

# Compare with baseline
cargo bench --save-baseline main
# Make changes...
cargo bench --baseline main
```

**Success Criteria:**
- [ ] Benchmarks for core operations
- [ ] CI tracks performance over time
- [ ] Alerts on > 10% regression

**Estimated Time:** 6 hours

---

## Phase 4: Continuous Improvement (Ongoing) 🔄

### 4.1 Test Coverage Monitoring

**Action:** Add coverage tracking:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage

# View report
open coverage/index.html
```

**CI Integration:**
```yaml
# .github/workflows/ci-checks.yml
- name: Generate coverage
  run: cargo tarpaulin --out Xml
  
- name: Upload to Codecov
  uses: codecov/codecov-action@v3
```

**Success Criteria:**
- [ ] Coverage tracked in CI
- [ ] > 80% line coverage
- [ ] Coverage badge in README

---

### 4.2 Test Quality Metrics

**Track these metrics:**

| Metric | Current | Target |
|--------|---------|--------|
| Total tests | ~150 | 150+ |
| Ignored tests | 14 | 0 |
| Test speed (unit) | ~10s | < 5s |
| Test speed (all) | ~30s | < 20s |
| Mutation score | ? | > 80% |
| Code coverage | ? | > 80% |
| Duplicate code | High | Low |

**Dashboard:**
```bash
# Create metrics dashboard
cat > test_metrics.sh << 'EOF'
#!/bin/bash

echo "=== Test Metrics ==="
echo "Total tests: $(rg '^fn test_' tests/ | wc -l)"
echo "Ignored tests: $(rg '#\[ignore\]' tests/ | wc -l)"
echo "Test files: $(find tests -name '*test.rs' | wc -l)"
echo "Test lines: $(find tests -name '*.rs' -exec wc -l {} + | tail -1)"
echo ""
echo "Running tests..."
time cargo test --quiet
EOF

chmod +x test_metrics.sh
./test_metrics.sh
```

---

## Implementation Checklist

### Week 1: Foundation
- [ ] Remove or fix 14 ignored tests
- [ ] Create helper functions module
- [ ] Fix `file_shape_removal_test.rs`
- [ ] Migrate 5-10 tests to use helpers

### Week 2: Structure
- [ ] Create `cross_language_test.rs`
- [ ] Consolidate duplicate tests
- [ ] Rename tests for behavior
- [ ] Separate unit/integration tests

### Week 3: Advanced
- [ ] Add property-based tests
- [ ] Set up mutation testing
- [ ] Create performance benchmarks
- [ ] Add coverage tracking

### Week 4: Polish
- [ ] Update TESTING_GUIDE.md
- [ ] Add test quality metrics
- [ ] CI improvements
- [ ] Documentation

---

## Success Metrics

**Before:**
- 14 ignored tests
- Brittle assertions
- Duplicate code
- No property tests
- No mutation testing
- Unknown coverage

**After:**
- 0 ignored tests ✅
- Behavior-focused assertions ✅
- Minimal duplication ✅
- Property tests for invariants ✅
- > 80% mutation score ✅
- > 80% code coverage ✅

**Grade Improvement:**
- Current: B+ (87/100)
- Target: A+ (95/100)

---

## Resources

- [Unit Testing Principles](https://www.manning.com/books/unit-testing) - Vladimir Khorikov
- [PropTest Book](https://altsysrq.github.io/proptest-book/)
- [Cargo Mutants](https://github.com/sourcefrog/cargo-mutants)
- [Criterion.rs](https://github.com/bheisler/criterion.rs)
- [Tarpaulin](https://github.com/xd009642/tarpaulin)

---

## Questions?

Open a discussion on GitHub or reach out to the team.

Happy testing! 🧪
