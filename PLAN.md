# Comprehensive Refactoring Plan: treesitter-mcp Tool Simplification

## Executive Summary

This plan consolidates 10 tools into 7 clearer, more focused tools with automatic type/struct inclusion to prevent LLM hallucinations. Key changes:

- **Merge 3 file viewing tools** (`parse_file`, `read_focused_code`, `file_shape`) → **1 tool** (`view_code`)
- **Merge 2 position tools** (`get_context`, `get_node_at_position`) → **1 tool** (`symbol_at_line`)
- **Auto-include project types** in `view_code` to prevent hallucinations
- **Optimize `code_map`** with `signatures` as default detail level
- **Clean break**: Remove old tools immediately (breaking changes OK)

---

## 1. New Tool Set (7 Tools)

### **Discovery Tools (2)**

#### 1.1 `code_map` - Directory Overview (KEPT, optimized)
**Status**: Minor changes
- Default `detail` changed from `"signatures"` (was implicit) to **explicit `"signatures"`**
- Keep all existing functionality
- No breaking changes to API

**New signature**:
```rust
pub struct CodeMap {
    pub path: String,
    pub max_tokens: Option<u32>,  // default: 2000
    pub detail: Option<String>,   // default: "signatures" (explicit)
    pub pattern: Option<String>,
}
```

#### 1.2 `find_usages` - Find Symbol References (KEPT, unchanged)
**Status**: No changes
- Keep exactly as-is
- Already clear and well-designed

---

### **File Viewing Tool (1)**

#### 1.3 `view_code` - Unified File Viewer (NEW - merges 3 tools)
**Status**: NEW tool, replaces `parse_file`, `read_focused_code`, `file_shape`

**Purpose**: Single tool for viewing files with flexible detail levels and automatic type inclusion

**Signature**:
```rust
pub struct ViewCode {
    /// Path to the source file
    pub file_path: String,
    
    /// Detail level: "signatures" or "full" (default: "full")
    /// - "signatures": Function/class signatures only (no bodies)
    /// - "full": Complete implementation code
    #[serde(default = "default_full")]
    pub detail: String,
    
    /// Optional: Focus on ONE symbol, show full code only for it
    /// When set, returns full code for this symbol + signatures for rest
    /// Overrides detail level for the focused symbol
    #[serde(default)]
    pub focus_symbol: Option<String>,
}
```

**Behavior**:
- **Always includes**:
  - All struct/class/type/interface definitions from the file
  - All struct/class/type/interface definitions from **project dependencies** (not external crates)
  - All imports
  - Function/class signatures or full code (based on `detail`)

- **`detail="signatures"`**: Signatures only (replaces `file_shape`)
- **`detail="full"`**: Full code (replaces `parse_file`)
- **`focus_symbol="foo"`**: Full code for `foo`, signatures for rest (replaces `read_focused_code`)

**Key difference from old tools**: **Always auto-includes type information from project dependencies**

---

### **Change Analysis Tools (2)**

#### 1.4 `parse_diff` - Structural Changes (KEPT, unchanged)
**Status**: No changes

#### 1.5 `affected_by_diff` - Impact Analysis (KEPT, unchanged)
**Status**: No changes

---

### **Navigation Tool (1)**

#### 1.6 `symbol_at_line` - What's at This Line? (NEW - merges 2 tools)
**Status**: NEW tool, replaces `get_context`, `get_node_at_position`

**Purpose**: Find what symbol (function/class) is at a specific line

**Signature**:
```rust
pub struct SymbolAtLine {
    /// Path to the source file
    pub file_path: String,
    
    /// Line number (1-indexed)
    pub line: u32,
    
    /// Column number (1-indexed, default: 1)
    #[serde(default = "default_one")]
    pub column: Option<u32>,
}
```

**Returns**:
- Symbol name (function/class/method)
- Symbol signature
- Scope chain (e.g., "in function X, in class Y, in module Z")
- Line range of the symbol

**Rationale**: 90% of use cases just need "what function is this?" with signature. Removed AST ancestor complexity.

---

### **Advanced Tool (1)**

#### 1.7 `query_pattern` - Custom Tree-sitter Queries (KEPT, unchanged)
**Status**: No changes
- Rarely used, keep for power users

---

## 2. Implementation Details

### 2.1 Auto-Include Project Types Feature

**Core requirement**: When viewing any file, automatically include type definitions from project dependencies to prevent hallucinations.

**Implementation approach**:

```rust
// In src/analysis/view_code.rs (new file)

pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path = get_file_path(arguments)?;
    let detail = arguments["detail"].as_str().unwrap_or("full");
    let focus_symbol = arguments["focus_symbol"].as_str();
    
    // 1. Parse the main file
    let source = fs::read_to_string(&file_path)?;
    let language = detect_language(&file_path)?;
    let tree = parse_code(&source, language)?;
    
    // 2. Extract main file shape
    let mut main_shape = extract_enhanced_shape(&tree, &source, language, detail == "full")?;
    
    // 3. ALWAYS resolve project dependencies and extract their types
    let project_root = path_utils::find_project_root(&file_path);
    let dep_paths = resolve_dependencies(language, &source, &file_path, &project_root);
    
    // Filter: Only project files, NOT external dependencies
    let project_deps = filter_project_dependencies(dep_paths, &project_root);
    
    // 4. Extract ONLY type/struct/class/interface definitions from dependencies
    let mut dependency_types = Vec::new();
    for dep_path in project_deps {
        let dep_types = extract_types_only(&dep_path)?;
        dependency_types.push(dep_types);
    }
    
    // 5. Apply focus if requested
    if let Some(symbol) = focus_symbol {
        apply_focus(&mut main_shape, symbol);
    }
    
    // 6. Build output with types included
    let output = ViewCodeOutput {
        file: main_shape,
        project_types: dependency_types,  // NEW: Always included
    };
    
    Ok(CallToolResult::success(serde_json::to_string(&output)?))
}

/// Extract ONLY type definitions (structs, classes, interfaces, enums)
/// Does NOT include function implementations
fn extract_types_only(file_path: &Path) -> Result<TypeDefinitions, io::Error> {
    let source = fs::read_to_string(file_path)?;
    let language = detect_language(file_path)?;
    let tree = parse_code(&source, language)?;
    
    // Extract shape with include_code=false (signatures only)
    let shape = extract_enhanced_shape(&tree, &source, language, false)?;
    
    // Return only structs/classes/interfaces/enums
    TypeDefinitions {
        path: file_path.to_string_lossy().to_string(),
        structs: shape.structs,
        classes: shape.classes,
        interfaces: shape.interfaces,
        // NOT functions - only type definitions
    }
}

/// Filter dependencies to only include project files, not external libraries
fn filter_project_dependencies(
    dep_paths: Vec<PathBuf>,
    project_root: &Path
) -> Vec<PathBuf> {
    dep_paths
        .into_iter()
        .filter(|path| {
            // Include if path is inside project_root
            path.starts_with(project_root) &&
            // Exclude external dependency directories
            !path.to_string_lossy().contains("/target/") &&
            !path.to_string_lossy().contains("/node_modules/") &&
            !path.to_string_lossy().contains("/venv/") &&
            !path.to_string_lossy().contains("/.venv/") &&
            !path.to_string_lossy().contains("/site-packages/")
        })
        .collect()
}
```

**Data structures**:

```rust
#[derive(Debug, serde::Serialize)]
pub struct ViewCodeOutput {
    /// The main file being viewed
    pub file: EnhancedFileShape,
    
    /// Type definitions from project dependencies
    /// ALWAYS included to prevent hallucinations
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub project_types: Vec<TypeDefinitions>,
}

#[derive(Debug, serde::Serialize)]
pub struct TypeDefinitions {
    /// Source file path
    pub path: String,
    
    /// Struct definitions
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub structs: Vec<EnhancedStructInfo>,
    
    /// Class definitions  
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<EnhancedClassInfo>,
    
    /// Interface definitions (TS/JS)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<InterfaceInfo>,
    
    /// Enum definitions
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub enums: Vec<EnumInfo>,  // NEW: Need to add enum extraction
}
```

### 2.2 Symbol At Line Implementation

```rust
// In src/analysis/symbol_at_line.rs (new file)

pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path = get_file_path(arguments)?;
    let line = get_line(arguments)?;
    let column = arguments["column"].as_u64().unwrap_or(1) as u32;
    
    let source = fs::read_to_string(&file_path)?;
    let language = detect_language(&file_path)?;
    let tree = parse_code(&source, language)?;
    
    // Find node at position
    let node = find_node_at_position(&tree, line, column)?;
    
    // Build scope chain
    let scope_chain = collect_scope_chain(node, &source, language);
    
    // Extract symbol info (innermost scope)
    let symbol_info = if let Some(innermost) = scope_chain.first() {
        SymbolInfo {
            name: innermost.name.clone(),
            signature: innermost.signature.clone(),
            kind: innermost.kind.clone(),
            line_range: innermost.line_range,
        }
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No symbol found at position"
        ));
    };
    
    let output = SymbolAtLineOutput {
        symbol: symbol_info,
        scope_chain,
    };
    
    Ok(CallToolResult::success(serde_json::to_string(&output)?))
}

#[derive(Debug, serde::Serialize)]
pub struct SymbolAtLineOutput {
    /// The symbol at the specified position
    pub symbol: SymbolInfo,
    
    /// Scope chain from innermost to outermost
    /// e.g., ["function foo", "class Bar", "module baz"]
    pub scope_chain: Vec<ScopeInfo>,
}

#[derive(Debug, serde::Serialize)]
pub struct SymbolInfo {
    pub name: String,
    pub signature: String,
    pub kind: String,  // "function", "method", "class", etc.
    pub line_range: (usize, usize),
}

#[derive(Debug, serde::Serialize)]
pub struct ScopeInfo {
    pub name: String,
    pub signature: String,
    pub kind: String,
}
```

---

## 3. File Structure Changes

### 3.1 New Files to Create
```
src/analysis/
  view_code.rs          # NEW: Unified file viewer
  symbol_at_line.rs     # NEW: Position-based symbol lookup
```

### 3.2 Files to DELETE
```
src/analysis/
  parse_file.rs         # REMOVE: Merged into view_code
  read_focused_code.rs  # REMOVE: Merged into view_code
  get_context.rs        # REMOVE: Merged into symbol_at_line
  get_node_at_position.rs  # REMOVE: Merged into symbol_at_line
```

**Note**: `file_shape.rs` is renamed to `shape.rs` (already exists!) and the `execute()` function will be removed since it's merged into `view_code`.

### 3.3 Files to MODIFY
```
src/tools.rs          # Update tool definitions
src/analysis/mod.rs   # Update exports
src/handler.rs        # Update tool registration (if needed)
```

---

## 4. Migration Strategy

### 4.1 Code Changes

**Step 1: Create new tools**
- Create `src/analysis/view_code.rs`
- Create `src/analysis/symbol_at_line.rs`
- Add type filtering logic to `dependencies.rs`

**Step 2: Update tool definitions in `src/tools.rs`**
- Remove: `ParseFile`, `ReadFocusedCode`, `FileShape`, `GetContext`, `GetNodeAtPosition`
- Add: `ViewCode`, `SymbolAtLine`
- Keep: `CodeMap`, `FindUsages`, `QueryPattern`, `ParseDiff`, `AffectedByDiff`

**Step 3: Update exports in `src/analysis/mod.rs`**
```rust
pub mod view_code;
pub mod symbol_at_line;
// Remove old exports
```

**Step 4: Delete old files**
- Delete `parse_file.rs`, `read_focused_code.rs`, `get_context.rs`, `get_node_at_position.rs`
- Keep `shape.rs` (has extraction logic)
- Remove `execute()` from `file_shape.rs`

**Step 5: Update `tool_box!` macro**
```rust
tool_box!(
    TreesitterTools,
    [
        CodeMap,
        ViewCode,           // NEW
        FindUsages,
        SymbolAtLine,       // NEW
        ParseDiff,
        AffectedByDiff,
        QueryPattern
    ]
);
```

### 4.2 Test Changes

**Tests to UPDATE**:
- `tests/parse_file_tool_test.rs` → Rename to `tests/view_code_test.rs`, update to use `view_code` with `detail="full"`
- `tests/parse_file_include_code_test.rs` → Merge into `view_code_test.rs`, test `detail="signatures"`
- `tests/parse_file_deps_test.rs` → Update to test new `project_types` output
- `tests/read_focused_code_test.rs` → Update to use `view_code` with `focus_symbol`
- `tests/get_context_test.rs` → Rename to `tests/symbol_at_line_test.rs`

**Tests to DELETE**:
- Remove `get_node_at_position_test.rs` (functionality merged)

**New tests to ADD**:
- Test project type filtering (exclude external deps)
- Test type extraction from dependencies
- Test combined type + code output

### 4.3 Documentation Changes

**Update**:
- `README.md` - Tool list and examples
- `AGENTS.md` - Tool usage guidelines
- Tool descriptions in `src/tools.rs`
- Add migration guide for users

---

## 5. Token Efficiency Improvements

### 5.1 code_map Optimizations
- **Current**: Default to `detail="signatures"`
- **Keep**: Already token-efficient
- **Document**: Recommend `detail="minimal"` for very large projects (>100 files)

### 5.2 view_code Optimizations
- **Type filtering**: Only include project dependencies, not external libraries
- **Signature-only types**: Type definitions always signatures-only (no impl bodies for structs)
- **Smart focus**: When `focus_symbol` is set, dramatically reduce tokens

**Estimated token reduction**:
- Old `parse_file` with `include_deps=true`: **100% baseline**
- New `view_code` with auto project types: **40-60%** (filters external deps, signatures-only for types)

### 5.3 Output Format Optimization
- Use `#[serde(skip_serializing_if = "Vec::is_empty")]` extensively
- Omit empty fields from JSON output
- Use compact JSON (no pretty-printing)

---

## 6. Implementation Phases

### Phase 1: Core Infrastructure (Day 1-2)
1. Create `view_code.rs` with basic structure
2. Create `symbol_at_line.rs` with basic structure
3. Add `filter_project_dependencies()` to `dependencies.rs`
4. Add `extract_types_only()` helper
5. Add `TypeDefinitions` struct

### Phase 2: Integration (Day 2-3)
1. Wire up `view_code` to use existing `extract_enhanced_shape()`
2. Implement project type extraction and filtering
3. Implement focus_symbol logic (reuse from `read_focused_code`)
4. Wire up `symbol_at_line` using `get_context` logic

### Phase 3: Tool Registration (Day 3)
1. Update `src/tools.rs` with new tool structs
2. Update `tool_box!` macro
3. Remove old tool structs
4. Update `src/analysis/mod.rs` exports

### Phase 4: Testing (Day 4-5)
1. Update existing tests for new tools
2. Add tests for project type filtering
3. Add tests for focus_symbol with types
4. Run full test suite and fix issues
5. Test token efficiency improvements

### Phase 5: Cleanup & Documentation (Day 5-6)
1. Delete old files
2. Update README.md
3. Update AGENTS.md
4. Add migration guide
5. Update tool descriptions

---

## 7. Testing Strategy

### 7.1 Unit Tests
- Test `filter_project_dependencies()` filters correctly
- Test `extract_types_only()` returns only types
- Test `view_code` with different detail levels
- Test `view_code` with focus_symbol
- Test `symbol_at_line` returns correct symbol + scope

### 7.2 Integration Tests
- Test `view_code` on complex Rust project (use `tests/fixtures/complex_rust_service/`)
- Test project type extraction across multiple files
- Test that external deps are excluded (cargo deps, node_modules, etc.)
- Test JavaScript/TypeScript/Python type extraction

### 7.3 Token Efficiency Tests
- Measure output size of old `parse_file` vs new `view_code`
- Verify type-only extraction is smaller than full code
- Verify external deps are excluded (major token savings)

---

## 8. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking changes upset users | Medium | Provide clear migration guide, version bump to 2.0 |
| Type extraction logic misses types | High | Comprehensive tests on all languages, use existing shape extraction |
| Performance regression from extra type parsing | Low | Benchmark before/after, types are signature-only (fast) |
| External dep filtering too aggressive | Medium | Test on various project structures, allow override parameter in future |
| Focus symbol logic breaks | Medium | Reuse existing `read_focused_code` logic, extensive tests |

---

## 9. Success Criteria

✅ **Tool count reduced**: 10 → 7 tools  
✅ **Clearer purpose**: Each tool has one clear job  
✅ **Type hallucinations prevented**: Project types always included  
✅ **Token efficiency**: 40-60% reduction vs old `parse_file` with deps  
✅ **All tests pass**: 100% test coverage maintained  
✅ **Documentation complete**: README, AGENTS.md, migration guide updated  
✅ **Performance maintained**: No significant slowdown (<10% acceptable)

---

## 10. Open Questions

1. **Should `view_code` have a parameter to DISABLE auto-type-inclusion?**  
   - Recommendation: No, always include types (that's the point)
   - But could add `include_project_types=true` with default true if needed later

2. **Should we add enum extraction to shape.rs?**  
   - Recommendation: Yes, enums are types and prevent hallucinations
   - Add in Phase 1

3. **Should `code_map` have an `exclude_pattern` parameter?**  
   - Recommendation: Not in this refactoring, add later if needed
   - Use `pattern` for inclusion filtering for now

4. **Should `symbol_at_line` include the full code of the symbol?**  
   - Recommendation: No, just signature. User can call `view_code` with `focus_symbol` after
   - Keeps tool focused on "what is at this line?" not "show me code"

---

## 11. Timeline Estimate

| Phase | Duration | Dependencies |
|-------|----------|--------------|
| Phase 1: Core Infrastructure | 1-2 days | None |
| Phase 2: Integration | 1-2 days | Phase 1 |
| Phase 3: Tool Registration | 0.5 days | Phase 2 |
| Phase 4: Testing | 1-2 days | Phase 3 |
| Phase 5: Cleanup & Docs | 1 day | Phase 4 |
| **Total** | **5-7 days** | |

---

## 12. Next Steps

Once approved:

1. Create detailed implementation tickets for each phase
2. Start with Phase 1 (core infrastructure)
3. Provide progress updates after each phase
4. Run tests continuously during development
5. Create draft migration guide early for review
