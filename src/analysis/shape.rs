//! Enhanced Shape Extraction Module
//!
//! Extracts detailed file structure with signatures, doc comments, and full code blocks.
//! Supports Rust, Python, JavaScript, and TypeScript.

use crate::parser::Language;
use std::io;
use tree_sitter::{Node, Query, QueryCursor, Tree};

/// Enhanced function information with signature and documentation
#[derive(Debug, serde::Serialize, Clone)]
pub struct EnhancedFunctionInfo {
    pub name: String,
    pub signature: String,
    pub line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Enhanced struct information with documentation
#[derive(Debug, serde::Serialize, Clone)]
pub struct EnhancedStructInfo {
    pub name: String,
    pub line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Enhanced class information with documentation
#[derive(Debug, serde::Serialize, Clone)]
pub struct EnhancedClassInfo {
    pub name: String,
    pub line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Import information with text and line number
#[derive(Debug, serde::Serialize, Clone)]
pub struct ImportInfo {
    pub text: String,
    pub line: usize,
}

/// Enhanced file shape with detailed information
#[derive(Debug, serde::Serialize)]
pub struct EnhancedFileShape {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub functions: Vec<EnhancedFunctionInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub structs: Vec<EnhancedStructInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<EnhancedClassInfo>,
    pub imports: Vec<ImportInfo>,
}

/// Extract enhanced shape from a parsed tree
pub fn extract_enhanced_shape(
    tree: &Tree,
    source: &str,
    language: Language,
    file_path: Option<&str>,
) -> Result<EnhancedFileShape, io::Error> {
    let shape = match language {
        Language::Rust => extract_rust_enhanced(tree, source)?,
        Language::Python => extract_python_enhanced(tree, source)?,
        Language::JavaScript => extract_js_enhanced(tree, source, Language::JavaScript)?,
        Language::TypeScript => extract_js_enhanced(tree, source, Language::TypeScript)?,
        Language::Html | Language::Css => {
            // HTML and CSS don't fit the EnhancedFileShape model
            // Return empty shape - they should use file_shape tool instead
            EnhancedFileShape {
                path: None,
                language: None,
                functions: vec![],
                structs: vec![],
                classes: vec![],
                imports: vec![],
            }
        }
    };

    Ok(EnhancedFileShape {
        path: file_path.map(|p| p.to_string()),
        language: Some(language.name().to_string()),
        ..shape
    })
}

/// Extract enhanced shape from Rust source code
fn extract_rust_enhanced(tree: &Tree, source: &str) -> Result<EnhancedFileShape, io::Error> {
    let mut functions = Vec::new();
    let mut structs = Vec::new();
    let mut imports = Vec::new();

    let query = Query::new(
        &tree_sitter_rust::LANGUAGE.into(),
        r#"
        (function_item name: (identifier) @func.name) @func
        (struct_item name: (type_identifier) @struct.name) @struct
        (use_declaration) @import
        "#,
    )
    .map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to create tree-sitter query: {e}"),
        )
    })?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            let node = capture.node;
            let name_idx = capture.index;
            let capture_name = query.capture_names()[name_idx as usize];

            match capture_name {
                "func.name" => {
                    if let Ok(func_node) = find_parent_by_type(node, "function_item") {
                        if let Ok(name) = node.utf8_text(source.as_bytes()) {
                            let line = func_node.start_position().row + 1;
                            let end_line = func_node.end_position().row + 1;
                            let signature = extract_signature(func_node, source)?;
                            let doc = extract_doc_comment(func_node, source, Language::Rust)?;
                            let code = extract_code(func_node, source)?;

                            functions.push(EnhancedFunctionInfo {
                                name: name.to_string(),
                                signature,
                                line,
                                end_line,
                                doc,
                                code,
                            });
                        }
                    }
                }
                "struct.name" => {
                    if let Ok(struct_node) = find_parent_by_type(node, "struct_item") {
                        if let Ok(name) = node.utf8_text(source.as_bytes()) {
                            let line = struct_node.start_position().row + 1;
                            let end_line = struct_node.end_position().row + 1;
                            let doc = extract_doc_comment(struct_node, source, Language::Rust)?;
                            let code = extract_code(struct_node, source)?;

                            structs.push(EnhancedStructInfo {
                                name: name.to_string(),
                                line,
                                end_line,
                                doc,
                                code,
                            });
                        }
                    }
                }
                "import" => {
                    if let Ok(text) = node.utf8_text(source.as_bytes()) {
                        imports.push(ImportInfo {
                            text: text.to_string(),
                            line: node.start_position().row + 1,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    Ok(EnhancedFileShape {
        path: None,
        language: None,
        functions,
        structs,
        classes: vec![],
        imports,
    })
}

/// Extract enhanced shape from Python source code
fn extract_python_enhanced(tree: &Tree, source: &str) -> Result<EnhancedFileShape, io::Error> {
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut imports = Vec::new();

    let query = Query::new(
        &tree_sitter_python::LANGUAGE.into(),
        r#"
        (function_definition name: (identifier) @func.name) @func
        (class_definition name: (identifier) @class.name) @class
        (import_statement) @import
        (import_from_statement) @import
        "#,
    )
    .map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to create tree-sitter query: {e}"),
        )
    })?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            let node = capture.node;
            let name_idx = capture.index;
            let capture_name = query.capture_names()[name_idx as usize];

            match capture_name {
                "func.name" => {
                    if let Ok(func_node) = find_parent_by_type(node, "function_definition") {
                        if let Ok(name) = node.utf8_text(source.as_bytes()) {
                            let line = func_node.start_position().row + 1;
                            let end_line = func_node.end_position().row + 1;
                            let signature = extract_signature(func_node, source)?;
                            let doc = extract_doc_comment(func_node, source, Language::Python)?;
                            let code = extract_code(func_node, source)?;

                            functions.push(EnhancedFunctionInfo {
                                name: name.to_string(),
                                signature,
                                line,
                                end_line,
                                doc,
                                code,
                            });
                        }
                    }
                }
                "class.name" => {
                    if let Ok(class_node) = find_parent_by_type(node, "class_definition") {
                        if let Ok(name) = node.utf8_text(source.as_bytes()) {
                            let line = class_node.start_position().row + 1;
                            let end_line = class_node.end_position().row + 1;
                            let doc = extract_doc_comment(class_node, source, Language::Python)?;
                            let code = extract_code(class_node, source)?;

                            classes.push(EnhancedClassInfo {
                                name: name.to_string(),
                                line,
                                end_line,
                                doc,
                                code,
                            });
                        }
                    }
                }
                "import" => {
                    if let Ok(text) = node.utf8_text(source.as_bytes()) {
                        imports.push(ImportInfo {
                            text: text.to_string(),
                            line: node.start_position().row + 1,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    Ok(EnhancedFileShape {
        path: None,
        language: None,
        functions,
        structs: vec![],
        classes,
        imports,
    })
}

/// Extract enhanced shape from JavaScript/TypeScript source code
fn extract_js_enhanced(
    tree: &Tree,
    source: &str,
    language: Language,
) -> Result<EnhancedFileShape, io::Error> {
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut imports = Vec::new();

    // Use the correct language for the query
    let ts_language = match language {
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        _ => tree_sitter_javascript::LANGUAGE.into(),
    };

    // Different query patterns for TypeScript vs JavaScript
    let query_str = match language {
        Language::TypeScript => {
            r#"
        (function_declaration) @func
        (class_declaration) @class
        (import_statement) @import
        "#
        }
        _ => {
            r#"
        (function_declaration name: (identifier) @func.name) @func
        (class_declaration name: (identifier) @class.name) @class
        (import_statement) @import
        "#
        }
    };

    let query = Query::new(&ts_language, query_str).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to create tree-sitter query: {e}"),
        )
    })?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    // Track processed nodes to avoid duplicates
    let mut processed_func_nodes = std::collections::HashSet::new();
    let mut processed_class_nodes = std::collections::HashSet::new();

    for match_ in matches {
        for capture in match_.captures {
            let node = capture.node;
            let name_idx = capture.index;
            let capture_name = query.capture_names()[name_idx as usize];

            match capture_name {
                "func.name" => {
                    // JavaScript: named capture for function name
                    if let Ok(func_node) = find_parent_by_type(node, "function_declaration") {
                        let node_id = func_node.id();
                        if !processed_func_nodes.contains(&node_id) {
                            processed_func_nodes.insert(node_id);
                            if let Ok(name) = node.utf8_text(source.as_bytes()) {
                                let line = func_node.start_position().row + 1;
                                let end_line = func_node.end_position().row + 1;
                                let signature = extract_signature(func_node, source)?;
                                let doc =
                                    extract_doc_comment(func_node, source, Language::JavaScript)?;
                                let code = extract_code(func_node, source)?;

                                functions.push(EnhancedFunctionInfo {
                                    name: name.to_string(),
                                    signature,
                                    line,
                                    end_line,
                                    doc,
                                    code,
                                });
                            }
                        }
                    }
                }
                "func" => {
                    // TypeScript: capture the whole function_declaration node
                    if node.kind() == "function_declaration" {
                        let node_id = node.id();
                        if !processed_func_nodes.contains(&node_id) {
                            processed_func_nodes.insert(node_id);
                            // Find the function name
                            if let Some(name_node) = node.child_by_field_name("name") {
                                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                    let line = node.start_position().row + 1;
                                    let end_line = node.end_position().row + 1;
                                    let signature = extract_signature(node, source)?;
                                    let doc = extract_doc_comment(node, source, language)?;
                                    let code = extract_code(node, source)?;

                                    functions.push(EnhancedFunctionInfo {
                                        name: name.to_string(),
                                        signature,
                                        line,
                                        end_line,
                                        doc,
                                        code,
                                    });
                                }
                            }
                        }
                    }
                }
                "class.name" => {
                    // JavaScript: named capture for class name
                    if let Ok(class_node) = find_parent_by_type(node, "class_declaration") {
                        let node_id = class_node.id();
                        if !processed_class_nodes.contains(&node_id) {
                            processed_class_nodes.insert(node_id);
                            if let Ok(name) = node.utf8_text(source.as_bytes()) {
                                let line = class_node.start_position().row + 1;
                                let end_line = class_node.end_position().row + 1;
                                let doc =
                                    extract_doc_comment(class_node, source, Language::JavaScript)?;
                                let code = extract_code(class_node, source)?;

                                classes.push(EnhancedClassInfo {
                                    name: name.to_string(),
                                    line,
                                    end_line,
                                    doc,
                                    code,
                                });
                            }
                        }
                    }
                }
                "class" => {
                    // TypeScript: capture the whole class_declaration node
                    if node.kind() == "class_declaration" {
                        let node_id = node.id();
                        if !processed_class_nodes.contains(&node_id) {
                            processed_class_nodes.insert(node_id);
                            // Find the class name
                            if let Some(name_node) = node.child_by_field_name("name") {
                                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                    let line = node.start_position().row + 1;
                                    let end_line = node.end_position().row + 1;
                                    let doc = extract_doc_comment(node, source, language)?;
                                    let code = extract_code(node, source)?;

                                    classes.push(EnhancedClassInfo {
                                        name: name.to_string(),
                                        line,
                                        end_line,
                                        doc,
                                        code,
                                    });
                                }
                            }
                        }
                    }
                }
                "import" => {
                    if let Ok(text) = node.utf8_text(source.as_bytes()) {
                        imports.push(ImportInfo {
                            text: text.to_string(),
                            line: node.start_position().row + 1,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    Ok(EnhancedFileShape {
        path: None,
        language: None,
        functions,
        structs: vec![],
        classes,
        imports,
    })
}

/// Extract the signature line of a function or struct
/// Uses tree-sitter to find the body node and extract signature efficiently
fn extract_signature(node: Node, source: &str) -> Result<String, io::Error> {
    let source_bytes = source.as_bytes();

    // Try to find the body node using tree-sitter
    // Body node types: block, statement_block, body, compound_statement
    let mut body_start_byte = None;
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if kind == "block"
            || kind == "statement_block"
            || kind == "body"
            || kind == "compound_statement"
            || kind == "field_declaration_list"
        // For structs
        {
            body_start_byte = Some(child.start_byte());
            break;
        }
    }

    // Determine the end of the signature
    let end_byte = if let Some(body_start) = body_start_byte {
        // Signature is everything before the body
        body_start
    } else {
        // No body found (e.g., trait method declaration), use the entire node
        node.end_byte()
    };

    // Extract the signature text
    let start_byte = node.start_byte();
    let signature_bytes = &source_bytes[start_byte..end_byte];
    let signature_text = String::from_utf8_lossy(signature_bytes);

    // Find where the actual declaration starts (after attributes/decorators)
    // Look for keywords that indicate the start of the declaration
    let mut lines: Vec<&str> = signature_text.lines().collect();
    let mut declaration_start_idx = 0;

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("async fn ")
            || trimmed.starts_with("pub async fn ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("pub struct ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("def ")
            || trimmed.starts_with("async def ")
            || trimmed.starts_with("function ")
            || trimmed.starts_with("export function ")
            || trimmed.starts_with("export async function ")
        {
            declaration_start_idx = idx;
            break;
        }
    }

    // Take lines from declaration start onwards
    let signature_lines: Vec<&str> = lines.drain(declaration_start_idx..).collect();
    let signature = signature_lines.join("\n").trim().to_string();

    Ok(signature)
}

/// Extract the full code block of a function or struct
fn extract_code(node: Node, source: &str) -> Result<Option<String>, io::Error> {
    let start_byte = node.start_byte();
    let end_byte = node.end_byte();

    if start_byte >= end_byte {
        return Ok(None);
    }

    let code_bytes = &source.as_bytes()[start_byte..end_byte];
    let code = String::from_utf8_lossy(code_bytes).to_string();

    if code.is_empty() {
        Ok(None)
    } else {
        Ok(Some(code))
    }
}

/// Extract doc comment from a node
fn extract_doc_comment(
    node: Node,
    source: &str,
    language: Language,
) -> Result<Option<String>, io::Error> {
    // Collect all consecutive doc comment lines before the current node
    let mut doc_lines = Vec::new();
    let mut prev_sibling = node.prev_sibling();

    while let Some(sibling) = prev_sibling {
        if is_comment_node(&sibling, language) {
            if let Ok(comment_text) = sibling.utf8_text(source.as_bytes()) {
                let doc = extract_doc_from_comment(comment_text, language);
                // Collect all doc lines, even empty ones (they separate sections)
                doc_lines.insert(0, doc);
            }
        } else if sibling.kind() != "ERROR" && !sibling.kind().is_empty() {
            // Stop if we hit a non-comment node
            break;
        }
        prev_sibling = sibling.prev_sibling();
    }

    if !doc_lines.is_empty() {
        // Find the first non-empty doc line (the actual description)
        if let Some(first_doc) = doc_lines.iter().find(|d| !d.is_empty()) {
            return Ok(Some(first_doc.clone()));
        }
        // If all are empty, return the joined version
        return Ok(Some(doc_lines.join("\n")));
    }

    // Also check parent's previous sibling for doc comments
    if let Some(parent) = node.parent() {
        if let Some(parent_prev) = parent.prev_sibling() {
            if is_comment_node(&parent_prev, language) {
                if let Ok(comment_text) = parent_prev.utf8_text(source.as_bytes()) {
                    let doc = extract_doc_from_comment(comment_text, language);
                    if !doc.is_empty() {
                        return Ok(Some(doc));
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Check if a node is a comment node
fn is_comment_node(node: &Node, language: Language) -> bool {
    let kind = node.kind();
    match language {
        Language::Rust | Language::JavaScript | Language::TypeScript => {
            kind == "line_comment" || kind == "block_comment"
        }
        Language::Python => kind == "comment",
        _ => false,
    }
}

/// Extract documentation text from a comment
fn extract_doc_from_comment(comment_text: &str, language: Language) -> String {
    let trimmed = comment_text.trim();

    match language {
        Language::Rust => {
            // Handle /// doc comments
            if trimmed.starts_with("///") {
                trimmed.strip_prefix("///").unwrap_or("").trim().to_string()
            } else if trimmed.starts_with("//!") {
                trimmed.strip_prefix("//!").unwrap_or("").trim().to_string()
            } else {
                String::new()
            }
        }
        Language::Python => {
            // Handle # comments
            if trimmed.starts_with("#") {
                trimmed.strip_prefix("#").unwrap_or("").trim().to_string()
            } else {
                String::new()
            }
        }
        Language::JavaScript | Language::TypeScript => {
            // Handle /** */ and // comments
            if trimmed.starts_with("/**") && trimmed.ends_with("*/") {
                trimmed
                    .strip_prefix("/**")
                    .and_then(|s| s.strip_suffix("*/"))
                    .unwrap_or("")
                    .trim()
                    .to_string()
            } else if trimmed.starts_with("//") {
                trimmed.strip_prefix("//").unwrap_or("").trim().to_string()
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

/// Find a parent node of a given type
fn find_parent_by_type<'a>(mut node: Node<'a>, target_type: &str) -> Result<Node<'a>, io::Error> {
    while let Some(parent) = node.parent() {
        if parent.kind() == target_type {
            return Ok(parent);
        }
        node = parent;
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Parent node of type '{}' not found", target_type),
    ))
}

// ============================================================================
// CSS/HTML Shape Structures
// ============================================================================

use std::borrow::Cow;

/// Theme variable from @theme block
#[derive(Debug, serde::Serialize, Clone)]
pub struct ThemeVariable {
    pub name: String,  // "--color-primary", "--spacing-lg"
    pub value: String, // "oklch(0.6 0.2 250)", "1.5rem"
    pub line: usize,
}

/// Custom component class (defined with @apply or custom styles)
#[derive(Debug, serde::Serialize, Clone)]
pub struct CustomClass {
    pub name: String,                     // "btn-primary", "card"
    pub applied_utilities: Vec<String>,   // ["bg-primary", "text-white", "px-4"]
    pub layer: Option<Cow<'static, str>>, // "components", "utilities", or None
    pub line: usize,
}

/// Keyframe animation
#[derive(Debug, serde::Serialize, Clone)]
pub struct KeyframeInfo {
    pub name: String,
    pub line: usize,
}

/// CSS file shape (Tailwind v4 focused)
#[derive(Debug, serde::Serialize)]
pub struct CssFileShape {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Theme variables from @theme block
    pub theme: Vec<ThemeVariable>,

    /// Custom component/utility classes (reusable)
    pub custom_classes: Vec<CustomClass>,

    /// @keyframes animations
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keyframes: Vec<KeyframeInfo>,
}

/// HTML element with id
#[derive(Debug, serde::Serialize, Clone)]
pub struct HtmlIdInfo {
    pub tag: String,
    pub id: String,
    pub line: usize,
}

/// Script reference
#[derive(Debug, serde::Serialize, Clone)]
pub struct ScriptInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    pub inline: bool,
    pub line: usize,
}

/// Style reference
#[derive(Debug, serde::Serialize, Clone)]
pub struct StyleInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    pub inline: bool,
    pub line: usize,
}

/// HTML file shape
#[derive(Debug, serde::Serialize)]
pub struct HtmlFileShape {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Elements with IDs (for JS/navigation)
    pub ids: Vec<HtmlIdInfo>,

    /// All unique custom classes used (non-Tailwind utilities)
    pub classes_used: Vec<String>,

    /// Script references
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scripts: Vec<ScriptInfo>,

    /// Style references
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub styles: Vec<StyleInfo>,
}

// ============================================================================
// Tailwind Utility Detection
// ============================================================================

/// Check if a class name is a Tailwind utility (to filter out)
///
/// NOTE: This list covers common Tailwind v4 utilities but is not exhaustive.
/// It may need updates as Tailwind evolves. Consider making this configurable
/// in the future to allow users to add custom utility patterns.
fn is_tailwind_utility(class: &str) -> bool {
    // Handle important modifier at the start
    let class = class.strip_prefix('!').unwrap_or(class);

    // Handle variant prefixes (hover:, dark:, sm:, etc.)
    let base = class.split(':').next_back().unwrap_or(class);

    // Exact match utilities
    let exact = [
        // Layout
        "flex",
        "grid",
        "block",
        "inline",
        "inline-block",
        "inline-flex",
        "inline-grid",
        "hidden",
        "container",
        "table",
        "table-row",
        "table-cell",
        // Position
        "relative",
        "absolute",
        "fixed",
        "sticky",
        "static",
        // Display
        "visible",
        "invisible",
        "collapse",
        // Accessibility
        "sr-only",
        "not-sr-only",
        // Interactivity
        "pointer-events-none",
        "pointer-events-auto",
        // Other common utilities
        "truncate",
        "italic",
        "underline",
        "line-through",
        "no-underline",
        "uppercase",
        "lowercase",
        "capitalize",
        "normal-case",
    ];
    if exact.contains(&base) {
        return true;
    }

    // Prefix-based utilities
    let prefixes = [
        // Spacing
        "p-",
        "px-",
        "py-",
        "pt-",
        "pr-",
        "pb-",
        "pl-",
        "ps-",
        "pe-",
        "m-",
        "mx-",
        "my-",
        "mt-",
        "mr-",
        "mb-",
        "ml-",
        "ms-",
        "me-",
        "-m",
        "gap-",
        "space-",
        // Sizing
        "w-",
        "h-",
        "min-w-",
        "min-h-",
        "max-w-",
        "max-h-",
        "size-",
        // Typography
        "text-",
        "font-",
        "leading-",
        "tracking-",
        "indent-",
        "decoration-",
        "underline-offset-",
        // Colors
        "bg-",
        "from-",
        "via-",
        "to-",
        "fill-",
        "stroke-",
        "border-",
        "outline-",
        "ring-",
        "shadow-",
        // Borders
        "rounded-",
        "divide-",
        // Layout
        "flex-",
        "grid-",
        "col-",
        "row-",
        "order-",
        "items-",
        "justify-",
        "content-",
        "place-",
        "self-",
        "auto-cols-",
        "auto-rows-",
        // Position
        "z-",
        "top-",
        "right-",
        "bottom-",
        "left-",
        "inset-",
        // Transforms
        "scale-",
        "rotate-",
        "translate-",
        "skew-",
        "origin-",
        // Transitions & Animations
        "transition-",
        "duration-",
        "delay-",
        "ease-",
        "animate-",
        // Effects
        "opacity-",
        "mix-blend-",
        "bg-blend-",
        "backdrop-blur-",
        "backdrop-brightness-",
        "backdrop-contrast-",
        "backdrop-grayscale-",
        "backdrop-hue-rotate-",
        "backdrop-invert-",
        "backdrop-opacity-",
        "backdrop-saturate-",
        "backdrop-sepia-",
        // Filters
        "blur-",
        "brightness-",
        "contrast-",
        "drop-shadow-",
        "grayscale-",
        "hue-rotate-",
        "invert-",
        "saturate-",
        "sepia-",
        // Interactivity
        "cursor-",
        "pointer-events-",
        "resize-",
        "select-",
        "user-select-",
        "caret-",
        "accent-",
        // Overflow
        "overflow-",
        "overscroll-",
        "scroll-",
        "snap-",
        // Other
        "aspect-",
        "columns-",
        "break-",
        "break-after-",
        "break-before-",
        "break-inside-",
        "float-",
        "clear-",
        "object-",
        "isolation-",
        "list-",
        "placeholder-",
        "will-change-",
        "touch-",
    ];

    prefixes.iter().any(|p| base.starts_with(p)) || base.contains('[') // Arbitrary values like w-[300px]
}

// ============================================================================
// CSS Extraction (Regex-based for Tailwind)
// ============================================================================

use regex::Regex;

/// Extract CSS shape from Tailwind v4 source code
///
/// This function uses regex to parse Tailwind-specific directives (@theme, @layer, @apply)
/// which are not part of standard CSS and thus not handled by tree-sitter-css.
pub fn extract_css_tailwind(
    source: &str,
    file_path: Option<&str>,
) -> Result<CssFileShape, io::Error> {
    let mut theme = Vec::new();
    let mut custom_classes = Vec::new();
    let mut keyframes = Vec::new();

    // 1. Extract @theme block variables
    let theme_block_re = Regex::new(r"@theme\s*\{([\s\S]*?)\}")
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}")))?;

    if let Some(cap) = theme_block_re.captures(source) {
        let theme_content_start = cap.get(1).unwrap().start(); // Start of captured group 1
        let theme_content = &cap[1];

        let var_re = Regex::new(r"(?m)^\s*(--[\w-]+)\s*:\s*([^;]+);").map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}"))
        })?;

        for var_cap in var_re.captures_iter(theme_content) {
            // Use the start of the variable name (group 1), not the whole match
            let var_name_start_in_theme = var_cap.get(1).unwrap().start();
            let absolute_offset = theme_content_start + var_name_start_in_theme;

            theme.push(ThemeVariable {
                name: var_cap[1].to_string(),
                value: var_cap[2].trim().to_string(),
                line: calculate_line(source, absolute_offset),
            });
        }
    }

    // 2. Extract @layer components/utilities blocks
    // We need to manually parse nested braces since regex can't handle them properly
    let layer_start_re = Regex::new(r"@layer\s+(components|utilities)\s*\{")
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}")))?;

    // Extract class definitions within layer
    let class_re = Regex::new(r"\.([\w-]+)\s*\{([^}]*)\}")
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}")))?;
    let apply_re = Regex::new(r"@apply\s+([^;]+);")
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}")))?;

    for layer_match in layer_start_re.captures_iter(source) {
        let layer_name = match &layer_match[1] {
            "components" => Cow::Borrowed("components"),
            "utilities" => Cow::Borrowed("utilities"),
            _ => Cow::Owned(layer_match[1].to_string()),
        };
        let layer_start = layer_match.get(0).unwrap().end(); // Start after the opening brace

        // Find the matching closing brace
        let mut brace_count = 1;
        let mut layer_end = layer_start;
        let source_bytes = source.as_bytes();

        for (i, &byte) in source_bytes.iter().enumerate().skip(layer_start) {
            match byte {
                b'{' => brace_count += 1,
                b'}' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        layer_end = i;
                        break;
                    }
                }
                _ => {}
            }
        }

        if layer_end == layer_start {
            continue; // No matching closing brace found
        }

        let layer_content = &source[layer_start..layer_end];

        for class_cap in class_re.captures_iter(layer_content) {
            let class_start_in_layer = class_cap.get(0).unwrap().start();
            let absolute_offset = layer_start + class_start_in_layer;
            let class_name = class_cap[1].to_string();
            let class_body = &class_cap[2];

            // Extract @apply utilities
            let mut applied = Vec::new();

            for apply_cap in apply_re.captures_iter(class_body) {
                applied.extend(apply_cap[1].split_whitespace().map(String::from));
            }

            custom_classes.push(CustomClass {
                name: class_name,
                applied_utilities: applied,
                layer: Some(layer_name.clone()),
                line: calculate_line(source, absolute_offset),
            });
        }
    }

    // 3. Extract @keyframes
    let keyframes_re = Regex::new(r"@keyframes\s+([\w-]+)\s*\{")
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid regex: {e}")))?;

    for kf_cap in keyframes_re.captures_iter(source) {
        keyframes.push(KeyframeInfo {
            name: kf_cap[1].to_string(),
            line: calculate_line(source, kf_cap.get(0).unwrap().start()),
        });
    }

    Ok(CssFileShape {
        path: file_path.map(String::from),
        theme,
        custom_classes,
        keyframes,
    })
}

/// Calculate line number from byte offset
fn calculate_line(source: &str, byte_offset: usize) -> usize {
    source[..byte_offset].matches('\n').count() + 1
}

// ============================================================================
// HTML Extraction (Tree-sitter)
// ============================================================================

use std::collections::HashSet;

/// Extract HTML shape from parsed tree
pub fn extract_html_shape(
    tree: &Tree,
    source: &str,
    file_path: Option<&str>,
) -> Result<HtmlFileShape, io::Error> {
    let mut ids = Vec::new();
    let mut all_classes = Vec::new();
    let mut scripts = Vec::new();
    let mut styles = Vec::new();

    // Use a simpler query that captures elements
    let query = Query::new(
        &tree_sitter_html::LANGUAGE.into(),
        r#"
        (element (start_tag) @start_tag)
        (script_element (start_tag) @script_tag)
        (style_element (start_tag) @style_tag)
        "#,
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Query error: {e}")))?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            let node = capture.node;
            let capture_name = query.capture_names()[capture.index as usize];

            match capture_name {
                "start_tag" => {
                    // Extract tag name - look for child with kind "tag_name"
                    let mut tag_name = String::new();
                    let mut tag_cursor = node.walk();
                    for child in node.children(&mut tag_cursor) {
                        if child.kind() == "tag_name" {
                            if let Ok(name) = child.utf8_text(source.as_bytes()) {
                                tag_name = name.to_string();
                                break;
                            }
                        }
                    }

                    let line = node.start_position().row + 1;

                    // Extract attributes
                    let id_attr = extract_attribute(&node, source, "id");
                    let class_attr = extract_attribute(&node, source, "class");
                    let rel_attr = extract_attribute(&node, source, "rel");
                    let href_attr = extract_attribute(&node, source, "href");

                    // Handle id
                    if let Some(id) = id_attr {
                        ids.push(HtmlIdInfo {
                            tag: tag_name.to_string(),
                            id,
                            line,
                        });
                    }

                    // Handle classes
                    if let Some(classes) = class_attr {
                        all_classes.extend(classes.split_whitespace().map(String::from));
                    }

                    // Handle link elements (stylesheets)
                    if tag_name == "link" {
                        if let Some(rel) = rel_attr {
                            if rel == "stylesheet" {
                                styles.push(StyleInfo {
                                    href: href_attr,
                                    inline: false,
                                    line,
                                });
                            }
                        }
                    }
                }
                "script_tag" => {
                    let line = node.start_position().row + 1;
                    let src = extract_attribute(&node, source, "src");
                    scripts.push(ScriptInfo {
                        src: src.clone(),
                        inline: src.is_none(),
                        line,
                    });
                }
                "style_tag" => {
                    let line = node.start_position().row + 1;
                    styles.push(StyleInfo {
                        href: None,
                        inline: true,
                        line,
                    });
                }
                _ => {}
            }
        }
    }

    // Deduplicate and filter classes
    let classes_used: Vec<String> = all_classes
        .into_iter()
        .filter(|c| !is_tailwind_utility(c))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    Ok(HtmlFileShape {
        path: file_path.map(String::from),
        ids,
        classes_used,
        scripts,
        styles,
    })
}

/// Helper to extract attribute value from a node
fn extract_attribute(node: &tree_sitter::Node, source: &str, attr_name: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "attribute" {
            let mut attr_cursor = child.walk();
            let mut found_name = false;
            for attr_child in child.children(&mut attr_cursor) {
                if attr_child.kind() == "attribute_name" {
                    if let Ok(name) = attr_child.utf8_text(source.as_bytes()) {
                        if name == attr_name {
                            found_name = true;
                        }
                    }
                } else if found_name && attr_child.kind() == "quoted_attribute_value" {
                    if let Ok(value) = attr_child.utf8_text(source.as_bytes()) {
                        return Some(value.trim_matches('"').trim_matches('\'').to_string());
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_code;

    #[test]
    fn test_extract_rust_function_signature() {
        let source = r#"
/// Adds two numbers
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let tree = parse_code(source, Language::Rust).expect("Failed to parse");
        let shape = extract_rust_enhanced(&tree, source).expect("Failed to extract shape");

        assert_eq!(shape.functions.len(), 1);
        let func = &shape.functions[0];
        assert_eq!(func.name, "add");
        assert!(func.signature.contains("pub fn add"));
        assert_eq!(func.line, 3);
        assert_eq!(func.end_line, 5);
    }

    #[test]
    fn test_extract_python_function() {
        let source = r#"
def greet(name):
    """Greets a person"""
    return f"Hello, {name}!"
"#;
        let tree = parse_code(source, Language::Python).expect("Failed to parse");
        let shape = extract_python_enhanced(&tree, source).expect("Failed to extract shape");

        assert_eq!(shape.functions.len(), 1);
        let func = &shape.functions[0];
        assert_eq!(func.name, "greet");
        assert_eq!(func.line, 2);
    }

    #[test]
    fn test_extract_js_class() {
        let source = r#"
class Calculator {
    add(a, b) {
        return a + b;
    }
}
"#;
        let tree = parse_code(source, Language::JavaScript).expect("Failed to parse");
        let shape = extract_js_enhanced(&tree, source, Language::JavaScript)
            .expect("Failed to extract shape");

        assert_eq!(shape.classes.len(), 1);
        let cls = &shape.classes[0];
        assert_eq!(cls.name, "Calculator");
        assert_eq!(cls.line, 2);
    }

    #[test]
    fn test_extract_imports() {
        let source = r#"
use std::fmt;
use std::io;

fn main() {}
"#;
        let tree = parse_code(source, Language::Rust).expect("Failed to parse");
        let shape = extract_rust_enhanced(&tree, source).expect("Failed to extract shape");

        assert_eq!(shape.imports.len(), 2);
        assert_eq!(shape.imports[0].text, "use std::fmt;");
        assert_eq!(shape.imports[0].line, 2);
    }
}
