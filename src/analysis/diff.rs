//! Diff-Aware Analysis Tools
//!
//! Provides tools for understanding structural changes between file versions
//! and identifying potentially affected code across the codebase.

use crate::analysis::path_utils;
use crate::common::format;
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code, Language};
use regex::Regex;
use serde::Serialize;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::Path;
use std::process::Command;

// ============================================================================
// Data Structures
// ============================================================================

/// Result of parsing a diff for structural changes
#[derive(Debug, Serialize, serde::Deserialize)]
pub struct DiffAnalysis {
    pub file_path: String,
    pub compare_to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compare_to_sha: Option<String>, // Full commit SHA for reference
    pub structural_changes: Vec<StructuralChange>,
    /// True if only non-structural changes (comments, whitespace, formatting)
    pub no_structural_change: bool,
    /// Summary counts
    pub summary: DiffSummary,
}

#[derive(Debug, Serialize, serde::Deserialize)]
pub struct DiffSummary {
    pub added: usize,
    pub removed: usize,
    pub modified: usize,
}

/// A single structural change detected in the diff
#[derive(Debug, Serialize, serde::Deserialize)]
pub struct StructuralChange {
    pub change_type: ChangeType,
    pub symbol_type: SymbolType,
    pub name: String,
    pub line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<ChangeDetail>,
}

#[derive(Debug, Serialize, serde::Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Added,
    Removed,
    SignatureChanged,
    BodyChanged,
}

#[derive(Debug, Serialize, serde::Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SymbolType {
    Function,
    Struct,
    Class,
    Enum,
    Interface,
    Import,
    Constant,
    Static, // Rust static items (renamed from Variable for clarity)
}

/// Detailed information about what changed in a symbol
#[derive(Debug, Serialize, serde::Deserialize)]
pub struct ChangeDetail {
    pub kind: String, // "parameter_type", "return_type", "parameter_added", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

// ============================================================================
// Affected Usages (internal)
// ============================================================================

#[derive(Debug, Serialize)]
pub struct AffectedChange {
    pub symbol: String,
    pub change_type: ChangeType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_details: Option<String>,
    pub potentially_affected: Vec<AffectedUsage>,
}

#[derive(Debug, Serialize)]
pub struct AffectedUsage {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub usage_type: String,
    pub code: String,
    pub risk: RiskLevel,
    pub reason: String,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    High,
    Medium,
    Low,
}

// ============================================================================
// Git Integration
// ============================================================================

/// Validate git revision string to prevent command injection
fn validate_git_revision(revision: &str) -> Result<(), io::Error> {
    // Allow: branch names, commit SHAs, HEAD~N, tags, etc.
    // Pattern: alphanumeric, dash, underscore, slash, tilde, caret, at, colon, dot
    let valid_pattern = Regex::new(r"^[a-zA-Z0-9_\-/.~^@:]+$")
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Regex error: {e}")))?;

    if !valid_pattern.is_match(revision) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid git revision format: {}", revision),
        ));
    }

    // Additional check: reject suspicious patterns
    if revision.contains("..") && !revision.contains("...") {
        // Allow "..." (three dots) for merge base, but be cautious with ".." (two dots)
        log::warn!("Git revision contains '..' which may be a range operator");
    }

    Ok(())
}

/// Get the old version of a file from git
fn get_git_file_content(file_path: &Path, revision: &str) -> Result<String, io::Error> {
    // Validate revision to prevent command injection
    validate_git_revision(revision)?;

    // Construct the git show command: git show <revision>:<path>
    let repo_relative_path = get_repo_relative_path(file_path)?;

    let output = Command::new("git")
        .args(["show", &format!("{}:{}", revision, repo_relative_path)])
        .current_dir(file_path.parent().unwrap_or(Path::new(".")))
        .output()
        .map_err(|e| io::Error::other(format!("Failed to execute git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Git show failed: {stderr}"),
        ));
    }

    String::from_utf8(output.stdout).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid UTF-8 in git output: {e}"),
        )
    })
}

/// Get the repository-relative path for a file
fn get_repo_relative_path(file_path: &Path) -> Result<String, io::Error> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(file_path.parent().unwrap_or(Path::new(".")))
        .output()
        .map_err(|e| io::Error::other(format!("Failed to get git root: {e}")))?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Not a git repository",
        ));
    }

    let repo_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let repo_root = Path::new(&repo_root);

    let canonical_file = file_path.canonicalize()?;
    let relative = canonical_file.strip_prefix(repo_root).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "File is not within the git repository",
        )
    })?;

    Ok(relative.to_string_lossy().to_string())
}

/// Resolve a git revision to its full SHA
fn resolve_git_sha(revision: &str, file_path: &Path) -> Result<String, io::Error> {
    validate_git_revision(revision)?;

    let output = Command::new("git")
        .args(["rev-parse", revision])
        .current_dir(file_path.parent().unwrap_or(Path::new(".")))
        .output()
        .map_err(|e| io::Error::other(format!("Failed to resolve git revision: {e}")))?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Could not resolve revision: {}", revision),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

// ============================================================================
// Symbol Extraction
// ============================================================================

/// Extracted symbol information for comparison
#[derive(Debug, Clone)]
struct ExtractedSymbol {
    symbol_type: SymbolType,
    name: String,
    line: usize,
    signature: Option<String>,
    body_hash: u64, // Hash of body for detecting body-only changes
}

/// Extract all symbols from a parsed tree
fn extract_symbols(
    tree: &tree_sitter::Tree,
    source: &str,
    language: Language,
) -> Result<HashMap<String, ExtractedSymbol>, io::Error> {
    let mut symbols = HashMap::new();

    match language {
        Language::Rust => extract_rust_symbols(tree, source, &mut symbols)?,
        Language::Python => extract_python_symbols(tree, source, &mut symbols)?,
        Language::JavaScript => extract_js_symbols(tree, source, &mut symbols)?,
        Language::TypeScript => extract_ts_symbols(tree, source, &mut symbols)?,
        Language::Go => extract_go_symbols(tree, source, &mut symbols)?,
        Language::Kotlin => extract_kotlin_symbols(tree, source, &mut symbols)?,
        Language::Html | Language::Css | Language::Swift | Language::CSharp | Language::Java => {
            // These languages don't have structural-diff extraction implemented yet.
            // Return empty - structural diff not applicable.
            log::debug!("Structural diff not applicable for {:?}", language);
        }
    }

    Ok(symbols)
}

fn extract_rust_symbols(
    tree: &tree_sitter::Tree,
    source: &str,
    symbols: &mut HashMap<String, ExtractedSymbol>,
) -> Result<(), io::Error> {
    use tree_sitter::{Query, QueryCursor};

    let query = Query::new(
        &tree_sitter_rust::LANGUAGE.into(),
        r#"
        (function_item name: (identifier) @func.name) @func
        (struct_item name: (type_identifier) @struct.name) @struct
        (enum_item name: (type_identifier) @enum.name) @enum
        (const_item name: (identifier) @const.name) @const
        (static_item name: (identifier) @static.name) @static
        "#,
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Query error: {e}")))?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            let node = capture.node;

            // Get the full node (not just the name)
            let (symbol_type, full_node) = match capture_name {
                "func.name" => (SymbolType::Function, node.parent()),
                "struct.name" => (SymbolType::Struct, node.parent()),
                "enum.name" => (SymbolType::Enum, node.parent()),
                "const.name" => (SymbolType::Constant, node.parent()),
                "static.name" => (SymbolType::Static, node.parent()),
                _ => continue,
            };

            if let (Ok(name), Some(full_node)) = (node.utf8_text(source.as_bytes()), full_node) {
                let signature = extract_signature_from_node(&full_node, source);
                let body_hash = hash_node_body(&full_node, source);

                symbols.insert(
                    format!("{:?}::{}", symbol_type, name),
                    ExtractedSymbol {
                        symbol_type,
                        name: name.to_string(),
                        line: node.start_position().row + 1,
                        signature,
                        body_hash,
                    },
                );
            }
        }
    }

    Ok(())
}

fn extract_python_symbols(
    tree: &tree_sitter::Tree,
    source: &str,
    symbols: &mut HashMap<String, ExtractedSymbol>,
) -> Result<(), io::Error> {
    use tree_sitter::{Query, QueryCursor};

    let query = Query::new(
        &tree_sitter_python::LANGUAGE.into(),
        r#"
        (function_definition name: (identifier) @func.name) @func
        (class_definition name: (identifier) @class.name) @class
        "#,
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Query error: {e}")))?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            let node = capture.node;

            let (symbol_type, full_node) = match capture_name {
                "func.name" => (SymbolType::Function, node.parent()),
                "class.name" => (SymbolType::Class, node.parent()),
                _ => continue,
            };

            if let (Ok(name), Some(full_node)) = (node.utf8_text(source.as_bytes()), full_node) {
                let signature = extract_signature_from_node(&full_node, source);
                let body_hash = hash_node_body(&full_node, source);

                symbols.insert(
                    format!("{:?}::{}", symbol_type, name),
                    ExtractedSymbol {
                        symbol_type,
                        name: name.to_string(),
                        line: node.start_position().row + 1,
                        signature,
                        body_hash,
                    },
                );
            }
        }
    }

    Ok(())
}

fn extract_js_symbols(
    tree: &tree_sitter::Tree,
    source: &str,
    symbols: &mut HashMap<String, ExtractedSymbol>,
) -> Result<(), io::Error> {
    use tree_sitter::{Query, QueryCursor};

    let query = Query::new(
        &tree_sitter_javascript::LANGUAGE.into(),
        r#"
        (function_declaration name: (identifier) @func.name) @func
        (class_declaration name: (identifier) @class.name) @class
        (method_definition name: (property_identifier) @method.name) @method
        "#,
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Query error: {e}")))?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            let node = capture.node;

            let (symbol_type, full_node) = match capture_name {
                "func.name" => (SymbolType::Function, node.parent()),
                "class.name" => (SymbolType::Class, node.parent()),
                "method.name" => (SymbolType::Function, node.parent()),
                _ => continue,
            };

            if let (Ok(name), Some(full_node)) = (node.utf8_text(source.as_bytes()), full_node) {
                let signature = extract_signature_from_node(&full_node, source);
                let body_hash = hash_node_body(&full_node, source);

                symbols.insert(
                    format!("{:?}::{}", symbol_type, name),
                    ExtractedSymbol {
                        symbol_type,
                        name: name.to_string(),
                        line: node.start_position().row + 1,
                        signature,
                        body_hash,
                    },
                );
            }
        }
    }

    Ok(())
}

fn extract_ts_symbols(
    tree: &tree_sitter::Tree,
    source: &str,
    symbols: &mut HashMap<String, ExtractedSymbol>,
) -> Result<(), io::Error> {
    use tree_sitter::{Query, QueryCursor};

    let query = Query::new(
        &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        r#"
        (function_declaration name: (identifier) @func.name) @func
        (class_declaration name: (type_identifier) @class.name) @class
        (method_definition name: (property_identifier) @method.name) @method
        (interface_declaration name: (type_identifier) @interface.name) @interface
        "#,
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Query error: {e}")))?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            let node = capture.node;

            let (symbol_type, full_node) = match capture_name {
                "func.name" => (SymbolType::Function, node.parent()),
                "class.name" => (SymbolType::Class, node.parent()),
                "method.name" => (SymbolType::Function, node.parent()),
                "interface.name" => (SymbolType::Interface, node.parent()),
                _ => continue,
            };

            if let (Ok(name), Some(full_node)) = (node.utf8_text(source.as_bytes()), full_node) {
                let signature = extract_signature_from_node(&full_node, source);
                let body_hash = hash_node_body(&full_node, source);

                symbols.insert(
                    format!("{:?}::{}", symbol_type, name),
                    ExtractedSymbol {
                        symbol_type,
                        name: name.to_string(),
                        line: node.start_position().row + 1,
                        signature,
                        body_hash,
                    },
                );
            }
        }
    }

    Ok(())
}

fn extract_go_symbols(
    tree: &tree_sitter::Tree,
    source: &str,
    symbols: &mut HashMap<String, ExtractedSymbol>,
) -> Result<(), io::Error> {
    use tree_sitter::{Node, Query, QueryCursor};

    fn find_parent_by_kind<'a>(mut node: Node<'a>, kind: &str) -> Option<Node<'a>> {
        loop {
            if node.kind() == kind {
                return Some(node);
            }
            node = node.parent()?;
        }
    }

    let query = Query::new(
        &tree_sitter_go::LANGUAGE.into(),
        r#"
        (function_declaration name: (identifier) @func.name) @func
        (method_declaration name: (field_identifier) @method.name) @method
        (type_spec name: (type_identifier) @struct.name type: (struct_type)) @struct
        (type_spec name: (type_identifier) @iface.name type: (interface_type)) @iface
        "#,
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Query error: {e}")))?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            let node = capture.node;

            let (symbol_type, full_node) = match capture_name {
                "func.name" => (
                    SymbolType::Function,
                    find_parent_by_kind(node, "function_declaration"),
                ),
                "method.name" => (
                    SymbolType::Function,
                    find_parent_by_kind(node, "method_declaration"),
                ),
                "struct.name" => (
                    SymbolType::Struct,
                    find_parent_by_kind(node, "type_declaration"),
                ),
                "iface.name" => (
                    SymbolType::Interface,
                    find_parent_by_kind(node, "type_declaration"),
                ),
                _ => continue,
            };

            if let (Ok(name), Some(full_node)) = (node.utf8_text(source.as_bytes()), full_node) {
                let signature = extract_signature_from_node(&full_node, source);
                let body_hash = hash_node_body(&full_node, source);

                symbols.insert(
                    format!("{:?}::{}", symbol_type, name),
                    ExtractedSymbol {
                        symbol_type,
                        name: name.to_string(),
                        line: node.start_position().row + 1,
                        signature,
                        body_hash,
                    },
                );
            }
        }
    }

    Ok(())
}

fn extract_kotlin_symbols(
    tree: &tree_sitter::Tree,
    source: &str,
    symbols: &mut HashMap<String, ExtractedSymbol>,
) -> Result<(), io::Error> {
    use tree_sitter::{Node, Query, QueryCursor};

    fn find_parent_by_kind<'a>(mut node: Node<'a>, kind: &str) -> Option<Node<'a>> {
        loop {
            if node.kind() == kind {
                return Some(node);
            }
            node = node.parent()?;
        }
    }

    let query = Query::new(
        &tree_sitter_kotlin_ng::LANGUAGE.into(),
        r#"
        (function_declaration name: (simple_identifier) @func.name) @func
        (class_declaration name: (type_identifier) @class.name) @class
        (object_declaration name: (type_identifier) @object.name) @object
        (type_alias name: (type_identifier) @alias.name) @alias
        "#,
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Query error: {e}")))?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            let node = capture.node;

            let (symbol_type, full_node) = match capture_name {
                "func.name" => (
                    SymbolType::Function,
                    find_parent_by_kind(node, "function_declaration"),
                ),
                "class.name" => (
                    SymbolType::Class,
                    find_parent_by_kind(node, "class_declaration"),
                ),
                "object.name" => (
                    SymbolType::Class,
                    find_parent_by_kind(node, "object_declaration"),
                ),
                "alias.name" => (
                    SymbolType::Struct, // Treat type alias as Struct
                    find_parent_by_kind(node, "type_alias"),
                ),
                _ => continue,
            };

            if let (Ok(name), Some(full_node)) = (node.utf8_text(source.as_bytes()), full_node) {
                // For class/object, check if it's an interface
                let actual_symbol_type = if symbol_type == SymbolType::Class {
                    let mut is_interface = false;
                    let mut cursor = full_node.walk();
                    for child in full_node.children(&mut cursor) {
                        if child.kind() == "interface" || child.kind() == "fun" {
                            is_interface = true;
                            break;
                        }
                        if child.kind() == "modifiers" {
                            let mut mod_cursor = child.walk();
                            for mod_child in child.children(&mut mod_cursor) {
                                if mod_child.kind() == "interface" {
                                    is_interface = true;
                                    break;
                                }
                            }
                        }
                    }
                    if is_interface {
                        SymbolType::Interface
                    } else {
                        SymbolType::Class
                    }
                } else {
                    symbol_type
                };

                let signature = extract_signature_from_node(&full_node, source);
                let body_hash = hash_node_body(&full_node, source);

                symbols.insert(
                    format!("{:?}::{}", actual_symbol_type, name),
                    ExtractedSymbol {
                        symbol_type: actual_symbol_type,
                        name: name.to_string(),
                        line: node.start_position().row + 1,
                        signature,
                        body_hash,
                    },
                );
            }
        }
    }

    Ok(())
}

/// Extract signature (first line or up to opening brace)
fn extract_signature_from_node(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let text = node.utf8_text(source.as_bytes()).ok()?;

    // Find the first '{' or ':' (for Python) to get just the signature
    if let Some(brace_pos) = text.find('{') {
        Some(text[..brace_pos].trim().to_string())
    } else if let Some(colon_pos) = text.find(':') {
        // For Python-style definitions
        Some(text[..=colon_pos].trim().to_string())
    } else {
        // For declarations without body
        Some(text.lines().next()?.trim().to_string())
    }
}

/// Hash the body of a node (excluding signature) for change detection
///
/// Note: Uses DefaultHasher which is fast but not stable across Rust versions.
/// This is acceptable for runtime comparison within a single execution.
/// If persistence is needed in the future, consider using a stable hash like xxhash.
fn hash_node_body(node: &tree_sitter::Node, source: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;

    let text = node.utf8_text(source.as_bytes()).unwrap_or("");

    // Skip to body (after first '{' or ':')
    let body = if let Some(brace_pos) = text.find('{') {
        &text[brace_pos..]
    } else {
        text
    };

    // Normalize whitespace for comparison
    let normalized: String = body.split_whitespace().collect();

    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    hasher.finish()
}

// ============================================================================
// Symbol Comparison
// ============================================================================

/// Compare old and new symbols to find structural changes
fn compare_symbols(
    old_symbols: &HashMap<String, ExtractedSymbol>,
    new_symbols: &HashMap<String, ExtractedSymbol>,
    _old_source: &str,
    _new_source: &str,
) -> Result<Vec<StructuralChange>, io::Error> {
    let mut changes = Vec::new();

    // Find removed symbols
    for (key, old_sym) in old_symbols {
        if !new_symbols.contains_key(key) {
            changes.push(StructuralChange {
                change_type: ChangeType::Removed,
                symbol_type: old_sym.symbol_type.clone(),
                name: old_sym.name.clone(),
                line: old_sym.line,
                before: old_sym.signature.clone(),
                after: None,
                details: vec![],
            });
        }
    }

    // Find added symbols
    for (key, new_sym) in new_symbols {
        if !old_symbols.contains_key(key) {
            changes.push(StructuralChange {
                change_type: ChangeType::Added,
                symbol_type: new_sym.symbol_type.clone(),
                name: new_sym.name.clone(),
                line: new_sym.line,
                before: None,
                after: new_sym.signature.clone(),
                details: vec![],
            });
        }
    }

    // Find modified symbols
    for (key, old_sym) in old_symbols {
        if let Some(new_sym) = new_symbols.get(key) {
            let signature_changed = old_sym.signature != new_sym.signature;
            let body_changed = old_sym.body_hash != new_sym.body_hash;

            if signature_changed {
                let details = analyze_signature_changes(
                    old_sym.signature.as_deref(),
                    new_sym.signature.as_deref(),
                );

                changes.push(StructuralChange {
                    change_type: ChangeType::SignatureChanged,
                    symbol_type: new_sym.symbol_type.clone(),
                    name: new_sym.name.clone(),
                    line: new_sym.line,
                    before: old_sym.signature.clone(),
                    after: new_sym.signature.clone(),
                    details,
                });
            } else if body_changed {
                changes.push(StructuralChange {
                    change_type: ChangeType::BodyChanged,
                    symbol_type: new_sym.symbol_type.clone(),
                    name: new_sym.name.clone(),
                    line: new_sym.line,
                    before: None,
                    after: None,
                    details: vec![ChangeDetail {
                        kind: "implementation_changed".to_string(),
                        name: None,
                        from: None,
                        to: None,
                    }],
                });
            }
        }
    }

    // Sort by line number
    changes.sort_by_key(|c| c.line);

    Ok(changes)
}

/// Analyze what specifically changed in a signature
fn analyze_signature_changes(old_sig: Option<&str>, new_sig: Option<&str>) -> Vec<ChangeDetail> {
    let mut details = Vec::new();

    let (old_sig, new_sig) = match (old_sig, new_sig) {
        (Some(o), Some(n)) => (o, n),
        _ => return details,
    };

    // Simple heuristic: check for common patterns
    // This could be made more sophisticated with proper parsing

    // Check for return type changes (Rust: -> Type, TS: : Type)
    let old_return = extract_return_type(old_sig);
    let new_return = extract_return_type(new_sig);
    if old_return != new_return {
        details.push(ChangeDetail {
            kind: "return_type".to_string(),
            name: None,
            from: old_return,
            to: new_return,
        });
    }

    // Check for parameter changes
    let old_params = extract_parameters(old_sig);
    let new_params = extract_parameters(new_sig);

    if old_params.len() != new_params.len() {
        details.push(ChangeDetail {
            kind: "parameter_count".to_string(),
            name: None,
            from: Some(old_params.len().to_string()),
            to: Some(new_params.len().to_string()),
        });
    }

    // Check individual parameter changes
    for (i, (old_p, new_p)) in old_params.iter().zip(new_params.iter()).enumerate() {
        if old_p != new_p {
            details.push(ChangeDetail {
                kind: "parameter_changed".to_string(),
                name: Some(format!("param_{}", i)),
                from: Some(old_p.clone()),
                to: Some(new_p.clone()),
            });
        }
    }

    details
}

fn extract_return_type(sig: &str) -> Option<String> {
    // Rust: fn foo() -> Type
    if let Some(pos) = sig.find("->") {
        return Some(sig[pos + 2..].trim().to_string());
    }
    // TypeScript: function foo(): Type
    if let Some(pos) = sig.rfind("):") {
        return Some(sig[pos + 2..].trim().to_string());
    }
    None
}

/// Extract parameters from a function signature
///
/// Note: This is a heuristic parser that handles nested generics but may not
/// correctly handle all edge cases (e.g., string literals containing brackets).
/// For production use, consider using tree-sitter to parse parameter lists.
fn extract_parameters(sig: &str) -> Vec<String> {
    // Find content between parentheses
    let start = sig.find('(').map(|i| i + 1).unwrap_or(0);
    let end = sig.rfind(')').unwrap_or(sig.len());

    if start >= end {
        return vec![];
    }

    let params_str = &sig[start..end];

    // Split by comma, handling nested generics/brackets
    let mut params = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for c in params_str.chars() {
        if escape_next {
            current.push(c);
            escape_next = false;
            continue;
        }

        match c {
            '\\' if in_string => {
                escape_next = true;
                current.push(c);
            }
            '"' | '\'' => {
                in_string = !in_string;
                current.push(c);
            }
            '<' | '(' | '[' if !in_string => {
                depth += 1;
                current.push(c);
            }
            '>' | ')' | ']' if !in_string => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 && !in_string => {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    params.push(trimmed);
                }
                current.clear();
            }
            _ => current.push(c),
        }
    }

    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        params.push(trimmed);
    }

    params
}

// ============================================================================
// parse_diff Implementation
// ============================================================================

pub fn execute_parse_diff(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path_str = arguments["file_path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'file_path' argument",
        )
    })?;

    let compare_to = arguments["compare_to"]
        .as_str()
        .unwrap_or("HEAD")
        .to_string();

    let analysis = analyze_diff(file_path_str, compare_to)?;

    let header = "type|name|line|change";
    let changes = analysis
        .structural_changes
        .iter()
        .map(|c| {
            let symbol_type = abbreviate_symbol_type(&c.symbol_type);
            let line = c.line.to_string();
            let change = format_change(c);
            format::format_row(&[symbol_type, &c.name, &line, &change])
        })
        .collect::<Vec<_>>()
        .join("\n");

    let result = json!({
        "p": analysis.file_path,
        "cmp": analysis.compare_to,
        "h": header,
        "changes": changes,
    });

    let result_json = serde_json::to_string(&result).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize result: {e}"),
        )
    })?;

    Ok(CallToolResult::success(result_json))
}

// ============================================================================
// affected_by_diff Implementation
// ============================================================================

pub fn execute_affected_by_diff(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path_str = arguments["file_path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'file_path' argument",
        )
    })?;

    let compare_to = arguments["compare_to"]
        .as_str()
        .unwrap_or("HEAD")
        .to_string();

    let scope = arguments["scope"].as_str();

    log::info!("Finding affected usages for: {file_path_str}");

    let file_path = Path::new(file_path_str);

    // Get the structural changes (internal representation)
    let diff_analysis = analyze_diff(file_path_str, compare_to)?;

    if diff_analysis.no_structural_change {
        let result = json!({
            "p": diff_analysis.file_path,
            "h": "symbol|change|file|line|risk",
            "affected": "",
        });

        let result_json = serde_json::to_string(&result).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize: {e}"),
            )
        })?;

        return Ok(CallToolResult::success(result_json));
    }

    // Determine search scope
    let search_path = if let Some(s) = scope {
        Path::new(s).to_path_buf()
    } else {
        find_project_root(file_path)
            .unwrap_or_else(|| file_path.parent().unwrap_or(Path::new(".")).to_path_buf())
    };

    let mut affected_changes = Vec::new();

    // For each changed symbol, find usages
    for change in &diff_analysis.structural_changes {
        // Skip removed symbols (no usages to find) and body-only changes (low impact)
        if change.change_type == ChangeType::Removed {
            continue;
        }

        // Find usages of this symbol
        let usages_args = serde_json::json!({
            "symbol": change.name,
            "path": search_path.to_str().unwrap(),
            "context_lines": 1
        });

        let usages_result = crate::analysis::find_usages::execute(&usages_args)?;
        let usages_text = get_result_text(&usages_result);
        let usages: serde_json::Value = serde_json::from_str(&usages_text).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to parse usages: {e}"),
            )
        })?;

        let rel_changed_file = path_utils::to_relative_path(file_path_str);
        let usage_rows = usages
            .get("u")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");

        // Filter out the definition itself and classify risk
        let mut potentially_affected = Vec::new();

        for row in usage_rows.lines() {
            let fields = parse_compact_row(row);

            let usage_file = fields.first().map(String::as_str).unwrap_or("");
            let usage_line = fields
                .get(1)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            let usage_column = fields
                .get(2)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            let usage_type = fields.get(3).map(String::as_str).unwrap_or("reference");
            let context = fields.get(4).map(String::as_str).unwrap_or("");

            // Skip the definition itself
            if usage_type == "definition" {
                continue;
            }

            // Skip usages in the same file at the same location
            if usage_file == rel_changed_file && usage_line == change.line {
                continue;
            }

            let (risk, reason) = assess_risk(change, usage_type);

            potentially_affected.push(AffectedUsage {
                file: usage_file.to_string(),
                line: usage_line,
                column: usage_column,
                usage_type: usage_type.to_string(),
                code: context.to_string(),
                risk,
                reason,
            });
        }

        if !potentially_affected.is_empty() {
            // Sort by risk (high first)
            potentially_affected.sort_by(|a, b| {
                let risk_order = |r: &RiskLevel| match r {
                    RiskLevel::High => 0,
                    RiskLevel::Medium => 1,
                    RiskLevel::Low => 2,
                };
                risk_order(&a.risk).cmp(&risk_order(&b.risk))
            });

            affected_changes.push(AffectedChange {
                symbol: change.name.clone(),
                change_type: change.change_type.clone(),
                change_details: change.after.clone(),
                potentially_affected,
            });
        }
    }

    let affected_rows = affected_changes
        .iter()
        .flat_map(|chg| {
            let change_key = change_key(&chg.change_type);
            chg.potentially_affected.iter().map(move |u| {
                let line = u.line.to_string();
                let risk = risk_key(&u.risk);
                format::format_row(&[&chg.symbol, change_key, &u.file, &line, risk])
            })
        })
        .collect::<Vec<_>>()
        .join("\n");

    let result = json!({
        "p": diff_analysis.file_path,
        "h": "symbol|change|file|line|risk",
        "affected": affected_rows,
    });

    let result_json = serde_json::to_string(&result).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize: {e}"),
        )
    })?;

    Ok(CallToolResult::success(result_json))
}

fn analyze_diff(file_path_str: &str, compare_to: String) -> Result<DiffAnalysis, io::Error> {
    log::info!("Analyzing diff for: {file_path_str} against {compare_to}");

    let file_path = Path::new(file_path_str);

    if !file_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File does not exist: {file_path_str}"),
        ));
    }

    let language = detect_language(file_path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            format!("Cannot detect language: {e}"),
        )
    })?;

    let current_content = std::fs::read_to_string(file_path)?;
    let old_content = get_git_file_content(file_path, &compare_to)?;

    let old_tree = parse_code(&old_content, language).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse old version: {e}"),
        )
    })?;

    let new_tree = parse_code(&current_content, language).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse current version: {e}"),
        )
    })?;

    let old_symbols = extract_symbols(&old_tree, &old_content, language)?;
    let new_symbols = extract_symbols(&new_tree, &current_content, language)?;

    let structural_changes =
        compare_symbols(&old_symbols, &new_symbols, &old_content, &current_content)?;

    let summary = DiffSummary {
        added: structural_changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Added)
            .count(),
        removed: structural_changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Removed)
            .count(),
        modified: structural_changes
            .iter()
            .filter(|c| {
                c.change_type == ChangeType::SignatureChanged
                    || c.change_type == ChangeType::BodyChanged
            })
            .count(),
    };

    let compare_to_sha = resolve_git_sha(&compare_to, file_path).ok();

    Ok(DiffAnalysis {
        file_path: path_utils::to_relative_path(file_path_str),
        compare_to,
        compare_to_sha,
        no_structural_change: structural_changes.is_empty(),
        structural_changes,
        summary,
    })
}

fn abbreviate_symbol_type(symbol_type: &SymbolType) -> &'static str {
    match symbol_type {
        SymbolType::Function => "fn",
        SymbolType::Struct => "s",
        SymbolType::Class => "c",
        SymbolType::Enum => "e",
        SymbolType::Interface => "iface",
        SymbolType::Import => "imp",
        SymbolType::Constant => "const",
        SymbolType::Static => "static",
    }
}

fn change_key(change_type: &ChangeType) -> &'static str {
    match change_type {
        ChangeType::Added => "added",
        ChangeType::Removed => "removed",
        ChangeType::SignatureChanged => "sig_changed",
        ChangeType::BodyChanged => "body_changed",
    }
}

fn risk_key(risk: &RiskLevel) -> &'static str {
    match risk {
        RiskLevel::High => "high",
        RiskLevel::Medium => "medium",
        RiskLevel::Low => "low",
    }
}

fn format_change(change: &StructuralChange) -> String {
    match change.change_type {
        ChangeType::Added => "added".to_string(),
        ChangeType::Removed => "removed".to_string(),
        ChangeType::BodyChanged => "body_changed".to_string(),
        ChangeType::SignatureChanged => match &change.after {
            Some(after) if !after.is_empty() => format!("sig_changed: {after}"),
            _ => "sig_changed".to_string(),
        },
    }
}

fn parse_compact_row(row: &str) -> Vec<String> {
    let mut fields: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut escaped = false;

    for ch in row.chars() {
        if escaped {
            match ch {
                'n' => current.push('\n'),
                'r' => current.push('\r'),
                '|' => current.push('|'),
                '\\' => current.push('\\'),
                other => {
                    current.push('\\');
                    current.push(other);
                }
            }
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if ch == '|' {
            fields.push(std::mem::take(&mut current));
            continue;
        }

        current.push(ch);
    }

    if escaped {
        current.push('\\');
    }

    fields.push(current);
    fields
}

/// Assess the risk level of a usage based on the change type
///
/// Future enhancement: Consider file proximity (same module = higher risk)
/// and whether usage is in test code (typically lower risk for production).
fn assess_risk(change: &StructuralChange, usage_type: &str) -> (RiskLevel, String) {
    match (&change.change_type, usage_type) {
        // Signature changes are high risk for calls
        (ChangeType::SignatureChanged, "call") => {
            let reason = if change.details.iter().any(|d| d.kind == "parameter_count") {
                "Call site may have wrong number of arguments"
            } else if change.details.iter().any(|d| d.kind.contains("parameter")) {
                "Call site may pass wrong argument types"
            } else if change.details.iter().any(|d| d.kind == "return_type") {
                "Return type changed - check how result is used"
            } else {
                "Signature changed - verify call is still valid"
            };
            (RiskLevel::High, reason.to_string())
        }

        // Signature changes are medium risk for type references
        (ChangeType::SignatureChanged, "type_reference") => (
            RiskLevel::Medium,
            "Type signature changed - verify compatibility".to_string(),
        ),

        // Body changes are low risk (behavior might change but API is same)
        (ChangeType::BodyChanged, _) => (
            RiskLevel::Low,
            "Implementation changed - behavior may differ".to_string(),
        ),

        // Added symbols are low risk (new code, no existing usages should break)
        (ChangeType::Added, _) => (
            RiskLevel::Low,
            "New symbol - this is a new usage".to_string(),
        ),

        // Default case
        _ => (
            RiskLevel::Medium,
            "Symbol changed - verify usage is still valid".to_string(),
        ),
    }
}

/// Helper to extract text from CallToolResult
/// Uses JSON serialization approach consistent with tests/common/mod.rs
fn get_result_text(result: &CallToolResult) -> String {
    if let Some(first_content) = result.content.first() {
        // Serialize and deserialize to extract the text field
        if let Ok(json_str) = serde_json::to_string(first_content) {
            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                if let Some(text) = json_val["text"].as_str() {
                    return text.to_string();
                }
            }
        }
    }
    String::new()
}

/// Find project root by walking up to the nearest directory containing Cargo.toml
/// Duplicated from file_shape.rs since that function is private
fn find_project_root(start: &Path) -> Option<std::path::PathBuf> {
    let mut current = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent()?.to_path_buf()
    };

    loop {
        let candidate = current.join("Cargo.toml");
        if candidate.is_file() {
            return Some(current);
        }

        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    None
}
