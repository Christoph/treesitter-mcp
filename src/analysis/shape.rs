//! Enhanced Shape Extraction Module
//!
//! Extracts detailed file structure with signatures, doc comments, and full code blocks.
//! Supports Rust, Python, JavaScript, TypeScript, Swift, C#, Java, Go, and Kotlin.

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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<String>,
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

    // NEW: Methods nested in class (Python, JavaScript, TypeScript, C#)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<EnhancedFunctionInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<PropertyInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<PropertyInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub implements: Vec<String>,
}

/// Import information with text and line number
#[derive(Debug, serde::Serialize, Clone)]
pub struct ImportInfo {
    pub text: String,
    pub line: usize,
}

/// Method information from impl blocks
#[derive(Debug, serde::Serialize, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub signature: String,
    pub line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Impl block information (Rust)
#[derive(Debug, serde::Serialize, Clone)]
pub struct ImplBlockInfo {
    pub type_name: String, // "Calculator", "Vec<T>", "Container<T>", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trait_name: Option<String>, // For trait impls: "Display", "Add", etc.
    pub line: usize,
    pub end_line: usize,
    pub methods: Vec<MethodInfo>,
}

/// Trait definition information (Rust)
#[derive(Debug, serde::Serialize, Clone)]
pub struct TraitInfo {
    pub name: String,
    pub line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    pub methods: Vec<MethodInfo>,
}

/// Interface information (TypeScript, C#)
#[derive(Debug, serde::Serialize, Clone)]
pub struct InterfaceInfo {
    pub name: String,
    pub line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<EnhancedFunctionInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<PropertyInfo>,
}

/// Property information (C#, TypeScript, etc.)
#[derive(Debug, serde::Serialize, Clone)]
pub struct PropertyInfo {
    pub name: String,
    pub line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
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

    // NEW: Impl blocks for Rust
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub impl_blocks: Vec<ImplBlockInfo>,

    // NEW: Traits for Rust
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<TraitInfo>,

    // NEW: Interfaces for TypeScript, C#
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<InterfaceInfo>,

    // NEW: Properties for C#, TypeScript
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<PropertyInfo>,

    // NEW: Dependencies (will populate in later phase)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<EnhancedFileShape>,
}

/// Extract enhanced shape from a parsed tree
pub fn extract_enhanced_shape(
    tree: &Tree,
    source: &str,
    language: Language,
    file_path: Option<&str>,
    include_code: bool,
) -> Result<EnhancedFileShape, io::Error> {
    let shape = match language {
        Language::Rust => extract_rust_enhanced(tree, source, include_code)?,
        Language::Python => extract_python_enhanced(tree, source, include_code)?,
        Language::JavaScript => {
            extract_js_enhanced(tree, source, Language::JavaScript, include_code)?
        }
        Language::TypeScript => {
            extract_js_enhanced(tree, source, Language::TypeScript, include_code)?
        }
        Language::Swift => extract_swift_enhanced(tree, source, include_code)?,
        Language::CSharp => extract_csharp_enhanced(tree, source, include_code)?,
        Language::Java => extract_java_enhanced(tree, source, include_code)?,
        Language::Go => extract_go_enhanced(tree, source, include_code)?,
        Language::Kotlin => extract_kotlin_enhanced(tree, source, include_code)?,
        Language::Html | Language::Css => {
            // HTML and CSS are markup/styling languages and are not suitable for
            // structural shape analysis. They lack the function/class/module structure
            // that other programming languages have. Tools like view_code, code_map,
            // and find_usages are designed for languages with well-defined symbols
            // and scopes (functions, classes, methods, etc.).
            //
            // For HTML/CSS analysis, consider using language-specific tools or parsers
            // designed for markup and styling languages.
            EnhancedFileShape {
                path: None,
                language: None,
                functions: vec![],
                structs: vec![],
                classes: vec![],
                traits: vec![],
                interfaces: vec![],
                properties: vec![],
                imports: vec![],
                impl_blocks: vec![],
                dependencies: vec![],
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
fn extract_rust_enhanced(
    tree: &Tree,
    source: &str,
    include_code: bool,
) -> Result<EnhancedFileShape, io::Error> {
    let mut functions = Vec::new();
    let mut structs = Vec::new();
    let mut imports = Vec::new();
    let mut impl_blocks = Vec::new();
    let mut traits = Vec::new();

    let query = Query::new(
        &tree_sitter_rust::LANGUAGE.into(),
        r#"
        (function_item name: (identifier) @func.name) @func
        (struct_item name: (type_identifier) @struct.name) @struct
        (use_declaration) @import
        (impl_item) @impl
        (trait_item name: (type_identifier) @trait.name) @trait
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
                            let code = if include_code {
                                extract_code(func_node, source)?
                            } else {
                                None
                            };

                            functions.push(EnhancedFunctionInfo {
                                name: name.to_string(),
                                signature,
                                line,
                                end_line,
                                doc,
                                code,
                                annotations: vec![],
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
                            let code = if include_code {
                                extract_code(struct_node, source)?
                            } else {
                                None
                            };

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
                "impl" => {
                    if let Ok(impl_info) = extract_impl_block(node, source, include_code) {
                        impl_blocks.push(impl_info);
                    }
                }
                "trait.name" => {
                    if let Ok(trait_node) = find_parent_by_type(node, "trait_item") {
                        if let Ok(trait_info) = extract_trait(trait_node, source, include_code) {
                            traits.push(trait_info);
                        }
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
        impl_blocks,
        traits,
        interfaces: vec![],
        properties: vec![],
        dependencies: vec![],
    })
}

/// Extract enhanced shape from Python source code
fn extract_python_enhanced(
    tree: &Tree,
    source: &str,
    include_code: bool,
) -> Result<EnhancedFileShape, io::Error> {
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
                        // Skip functions that are inside classes (they'll be extracted as methods)
                        if is_inside_class(func_node) {
                            continue;
                        }

                        if let Ok(name) = node.utf8_text(source.as_bytes()) {
                            let line = func_node.start_position().row + 1;
                            let end_line = func_node.end_position().row + 1;
                            let signature = extract_signature(func_node, source)?;
                            let doc = extract_doc_comment(func_node, source, Language::Python)?;
                            let code = if include_code {
                                extract_code(func_node, source)?
                            } else {
                                None
                            };

                            functions.push(EnhancedFunctionInfo {
                                name: name.to_string(),
                                signature,
                                line,
                                end_line,
                                doc,
                                code,
                                annotations: vec![],
                            });
                        }
                    }
                }
                "class.name" => {
                    if let Ok(class_node) = find_parent_by_type(node, "class_definition") {
                        // Skip nested classes (only extract top-level classes)
                        if is_inside_class(class_node) {
                            continue;
                        }

                        if let Ok(name) = node.utf8_text(source.as_bytes()) {
                            let line = class_node.start_position().row + 1;
                            let end_line = class_node.end_position().row + 1;
                            let doc = extract_doc_comment(class_node, source, Language::Python)?;
                            let code = if include_code {
                                extract_code(class_node, source)?
                            } else {
                                None
                            };

                            // Extract methods from class body (excluding nested classes)
                            let methods = extract_class_methods(
                                class_node,
                                source,
                                Language::Python,
                                include_code,
                            )?;

                            classes.push(EnhancedClassInfo {
                                name: name.to_string(),
                                line,
                                end_line,
                                doc,
                                code,
                                methods,
                                implements: vec![],
                                properties: vec![],
                                fields: vec![],
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
        impl_blocks: vec![],
        traits: vec![],
        interfaces: vec![],
        properties: vec![],
        dependencies: vec![],
    })
}

/// Extract enhanced shape from JavaScript/TypeScript source code
fn extract_js_enhanced(
    tree: &Tree,
    source: &str,
    language: Language,
    include_code: bool,
) -> Result<EnhancedFileShape, io::Error> {
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut imports = Vec::new();
    let mut interfaces = Vec::new();

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
        (interface_declaration name: (type_identifier) @interface.name) @interface
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
                                let code = if include_code {
                                    extract_code(func_node, source)?
                                } else {
                                    None
                                };

                                functions.push(EnhancedFunctionInfo {
                                    name: name.to_string(),
                                    signature,
                                    line,
                                    end_line,
                                    doc,
                                    code,
                                    annotations: vec![],
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
                                    let code = if include_code {
                                        extract_code(node, source)?
                                    } else {
                                        None
                                    };

                                    functions.push(EnhancedFunctionInfo {
                                        name: name.to_string(),
                                        signature,
                                        line,
                                        end_line,
                                        doc,
                                        code,
                                        annotations: vec![],
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
                                let code = if include_code {
                                    extract_code(class_node, source)?
                                } else {
                                    None
                                };

                                // Extract methods from class body
                                let methods = extract_class_methods(
                                    class_node,
                                    source,
                                    Language::JavaScript,
                                    include_code,
                                )?;

                                classes.push(EnhancedClassInfo {
                                    name: name.to_string(),
                                    line,
                                    end_line,
                                    doc,
                                    code,
                                    methods,
                                    implements: vec![],
                                    properties: vec![],
                                    fields: vec![],
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
                            if let Some(name_node) = node.child_by_field_name("name") {
                                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                    let line = node.start_position().row + 1;
                                    let end_line = node.end_position().row + 1;
                                    let doc = extract_doc_comment(node, source, language)?;
                                    let code = if include_code {
                                        extract_code(node, source)?
                                    } else {
                                        None
                                    };

                                    // Extract methods from class body
                                    let methods = extract_class_methods(
                                        node,
                                        source,
                                        language,
                                        include_code,
                                    )?;

                                    classes.push(EnhancedClassInfo {
                                        name: name.to_string(),
                                        line,
                                        end_line,
                                        doc,
                                        code,
                                        methods,
                                        implements: vec![],
                                        properties: vec![],
                                        fields: vec![],
                                    });
                                }
                            }
                        }
                    }
                }
                "interface.name" => {
                    if let Ok(interface_node) = find_parent_by_type(node, "interface_declaration") {
                        if let Ok(interface_info) =
                            extract_interface(interface_node, source, include_code)
                        {
                            interfaces.push(interface_info);
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
        impl_blocks: vec![],
        traits: vec![],
        interfaces,
        properties: vec![],
        dependencies: vec![],
    })
}

/// Extract enhanced shape from Swift source code
fn extract_swift_enhanced(
    tree: &Tree,
    source: &str,
    include_code: bool,
) -> Result<EnhancedFileShape, io::Error> {
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut structs = Vec::new();
    let mut traits = Vec::new();
    let mut imports = Vec::new();

    // Use tree-sitter query API for efficient extraction (Swift grammar)
    let query = Query::new(
        &tree_sitter_swift::LANGUAGE.into(),
        r#"
        (function_declaration name: (simple_identifier) @func.name) @func
        (class_declaration name: (type_identifier) @class.name) @class
        (protocol_declaration name: (type_identifier) @protocol.name) @protocol
        (import_declaration) @import
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
                    if let Ok(func_node) = find_parent_by_type(node, "function_declaration") {
                        // Skip functions inside classes/structs
                        if is_inside_class(func_node) {
                            continue;
                        }

                        if let Ok(name) = node.utf8_text(source.as_bytes()) {
                            let line = func_node.start_position().row + 1;
                            let end_line = func_node.end_position().row + 1;
                            let signature = extract_signature(func_node, source)?;
                            let doc = extract_doc_comment(func_node, source, Language::Swift)?;
                            let code = if include_code {
                                extract_code(func_node, source)?
                            } else {
                                None
                            };

                            functions.push(EnhancedFunctionInfo {
                                name: name.to_string(),
                                signature,
                                line,
                                end_line,
                                doc,
                                code,
                                annotations: vec![],
                            });
                        }
                    }
                }
                "class.name" => {
                    if let Ok(class_node) = find_parent_by_type(node, "class_declaration") {
                        // Skip nested classes
                        if is_inside_class(class_node) {
                            continue;
                        }

                        if let Ok(name) = node.utf8_text(source.as_bytes()) {
                            let line = class_node.start_position().row + 1;
                            let end_line = class_node.end_position().row + 1;
                            let doc = extract_doc_comment(class_node, source, Language::Swift)?;
                            let code = if include_code {
                                extract_code(class_node, source)?
                            } else {
                                None
                            };

                            // Check if this is actually a struct (both use class_declaration in Swift grammar)
                            let is_struct = class_node
                                .child(0)
                                .and_then(|first_child| {
                                    first_child.utf8_text(source.as_bytes()).ok()
                                })
                                .map(|text| text.trim_start().starts_with("struct"))
                                .unwrap_or(false);

                            if is_struct {
                                structs.push(EnhancedStructInfo {
                                    name: name.to_string(),
                                    line,
                                    end_line,
                                    doc,
                                    code,
                                });
                            } else {
                                // Extract methods from class body
                                let methods = extract_class_methods(
                                    class_node,
                                    source,
                                    Language::Swift,
                                    include_code,
                                )?;

                                classes.push(EnhancedClassInfo {
                                    name: name.to_string(),
                                    line,
                                    end_line,
                                    doc,
                                    code,
                                    methods,
                                    implements: vec![],
                                    properties: vec![],
                                    fields: vec![],
                                });
                            }
                        }
                    }
                }
                "protocol.name" => {
                    if let Ok(protocol_node) = find_parent_by_type(node, "protocol_declaration") {
                        // Skip nested protocols
                        if is_inside_class(protocol_node) {
                            continue;
                        }

                        if let Ok(name) = node.utf8_text(source.as_bytes()) {
                            let line = protocol_node.start_position().row + 1;
                            let end_line = protocol_node.end_position().row + 1;
                            let doc = extract_doc_comment(protocol_node, source, Language::Swift)?;

                            // Extract methods from protocol body and convert to MethodInfo
                            let enhanced_methods = extract_class_methods(
                                protocol_node,
                                source,
                                Language::Swift,
                                include_code,
                            )?;
                            let methods = enhanced_methods
                                .into_iter()
                                .map(|m| MethodInfo {
                                    name: m.name,
                                    signature: m.signature,
                                    line: m.line,
                                    end_line: m.end_line,
                                    doc: m.doc,
                                    code: m.code,
                                })
                                .collect();

                            traits.push(TraitInfo {
                                name: name.to_string(),
                                line,
                                end_line,
                                doc,
                                methods,
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
        classes,
        imports,
        impl_blocks: vec![],
        traits,
        interfaces: vec![],
        properties: vec![],
        dependencies: vec![],
    })
}

/// Extract enhanced shape from C# source code
fn extract_csharp_enhanced(
    tree: &Tree,
    source: &str,
    include_code: bool,
) -> Result<EnhancedFileShape, io::Error> {
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut imports = Vec::new();
    let mut interfaces = Vec::new();
    let mut properties = Vec::new();

    let ts_language = tree_sitter_c_sharp::LANGUAGE.into();

    let query_str = r#"
        (method_declaration) @method
        (class_declaration) @class
        (interface_declaration) @interface
        (property_declaration) @property
        (using_directive) @import
    "#;

    let query = Query::new(&ts_language, query_str).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to create tree-sitter query: {e}"),
        )
    })?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    let mut processed_method_nodes = std::collections::HashSet::new();
    let mut processed_class_nodes = std::collections::HashSet::new();
    let mut processed_property_nodes = std::collections::HashSet::new();

    for match_ in matches {
        for capture in match_.captures {
            let node = capture.node;
            let name_idx = capture.index;
            let capture_name = query.capture_names()[name_idx as usize];

            match capture_name {
                "method" => {
                    // In C#, all methods are inside classes, so we extract them all as functions
                    // (unlike JS/TS where we skip class methods)

                    let node_id = node.id();
                    if !processed_method_nodes.contains(&node_id) {
                        processed_method_nodes.insert(node_id);

                        if let Some(name_node) = node.child_by_field_name("name") {
                            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                let line = node.start_position().row + 1;
                                let end_line = node.end_position().row + 1;
                                let signature = extract_signature(node, source)?;
                                let doc = extract_doc_comment(node, source, Language::CSharp)?;
                                let code = if include_code {
                                    extract_code(node, source)?
                                } else {
                                    None
                                };

                                functions.push(EnhancedFunctionInfo {
                                    name: name.to_string(),
                                    signature,
                                    line,
                                    end_line,
                                    doc,
                                    code,
                                    annotations: vec![],
                                });
                            }
                        }
                    }
                }
                "class" => {
                    if node.kind() == "class_declaration" {
                        let node_id = node.id();
                        if !processed_class_nodes.contains(&node_id) {
                            processed_class_nodes.insert(node_id);

                            if let Some(name_node) = node.child_by_field_name("name") {
                                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                    let line = node.start_position().row + 1;
                                    let end_line = node.end_position().row + 1;
                                    let doc = extract_doc_comment(node, source, Language::CSharp)?;
                                    let code = if include_code {
                                        extract_code(node, source)?
                                    } else {
                                        None
                                    };

                                    // Extract implements interfaces from base_list
                                    let implements =
                                        extract_csharp_implemented_interfaces(node, source);

                                    // Extract methods from class
                                    let methods =
                                        extract_csharp_class_methods(node, source, include_code)?;

                                    classes.push(EnhancedClassInfo {
                                        name: name.to_string(),
                                        line,
                                        end_line,
                                        doc,
                                        code,
                                        methods,
                                        implements,
                                        properties: vec![],
                                        fields: vec![],
                                    });
                                }
                            }
                        }
                    }
                }
                "interface" => {
                    if node.kind() == "interface_declaration" {
                        if let Some(name_node) = node.child_by_field_name("name") {
                            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                let line = node.start_position().row + 1;
                                let end_line = node.end_position().row + 1;
                                let doc = extract_doc_comment(node, source, Language::CSharp)?;
                                let code = if include_code {
                                    extract_code(node, source)?
                                } else {
                                    None
                                };

                                // Extract methods from interface
                                let methods =
                                    extract_csharp_interface_methods(node, source, include_code)?;

                                interfaces.push(InterfaceInfo {
                                    name: name.to_string(),
                                    line,
                                    end_line,
                                    doc,
                                    code,
                                    methods,
                                    properties: vec![],
                                });
                            }
                        }
                    }
                }
                "property" => {
                    let node_id = node.id();
                    if !processed_property_nodes.contains(&node_id) {
                        processed_property_nodes.insert(node_id);

                        if let Some(name_node) = node.child_by_field_name("name") {
                            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                let line = node.start_position().row + 1;
                                let end_line = node.end_position().row + 1;
                                let doc = extract_doc_comment(node, source, Language::CSharp)?;

                                // Extract property type
                                let property_type = node
                                    .child_by_field_name("type")
                                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                                    .map(|s| s.to_string());

                                properties.push(PropertyInfo {
                                    name: name.to_string(),
                                    line,
                                    end_line,
                                    property_type,
                                    doc,
                                });
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
        impl_blocks: vec![],
        traits: vec![],
        interfaces,
        properties,
        dependencies: vec![],
    })
}

/// Helper function to extract methods from a C# class
fn extract_csharp_class_methods(
    class_node: Node,
    source: &str,
    include_code: bool,
) -> Result<Vec<EnhancedFunctionInfo>, io::Error> {
    let mut methods = Vec::new();

    if let Some(body) = class_node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "method_declaration" {
                if let Some(method_info) = extract_csharp_method_info(child, source, include_code)?
                {
                    methods.push(method_info);
                }
            }
        }
    }

    Ok(methods)
}

/// Extract method information from a C# method declaration node
fn extract_csharp_method_info(
    method_node: Node,
    source: &str,
    include_code: bool,
) -> Result<Option<EnhancedFunctionInfo>, io::Error> {
    if let Some(name_node) = method_node.child_by_field_name("name") {
        if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
            let line = method_node.start_position().row + 1;
            let end_line = method_node.end_position().row + 1;
            let signature = extract_signature(method_node, source)?;
            let doc = extract_doc_comment(method_node, source, Language::CSharp)?;
            let code = if include_code {
                extract_code(method_node, source)?
            } else {
                None
            };

            return Ok(Some(EnhancedFunctionInfo {
                name: name.to_string(),
                signature,
                line,
                end_line,
                doc,
                code,
                annotations: vec![],
            }));
        }
    }
    Ok(None)
}

/// Extract methods from a C# interface body
fn extract_csharp_interface_methods(
    interface_node: Node,
    source: &str,
    include_code: bool,
) -> Result<Vec<EnhancedFunctionInfo>, io::Error> {
    let mut methods = Vec::new();

    if let Some(body) = interface_node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "method_declaration" {
                if let Some(method_info) = extract_csharp_method_info(child, source, include_code)?
                {
                    methods.push(method_info);
                }
            }
        }
    }

    Ok(methods)
}

/// Extract implemented interface names from a C# class node
fn extract_csharp_implemented_interfaces(class_node: Node, source: &str) -> Vec<String> {
    let mut implements = Vec::new();

    let mut cursor = class_node.walk();
    for child in class_node.children(&mut cursor) {
        if child.kind() == "base_list" {
            let mut bases_cursor = child.walk();
            for base_child in child.children(&mut bases_cursor) {
                // Look for identifier or generic_name nodes
                if base_child.kind() == "identifier" || base_child.kind() == "generic_name" {
                    if let Ok(interface_name) = base_child.utf8_text(source.as_bytes()) {
                        implements.push(interface_name.to_string());
                    }
                }
            }
            break; // base_list is unique, no need to continue
        }
    }

    implements
}

/// Extract enhanced shape from Java source code
fn extract_java_enhanced(
    tree: &Tree,
    source: &str,
    include_code: bool,
) -> Result<EnhancedFileShape, io::Error> {
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut imports = Vec::new();
    let mut interfaces = Vec::new();

    let ts_language = tree_sitter_java::LANGUAGE.into();

    let query_str = r#"
        (method_declaration) @method
        (class_declaration) @class
        (interface_declaration) @interface
        (import_declaration) @import
    "#;

    let query = Query::new(&ts_language, query_str).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to create tree-sitter query: {e}"),
        )
    })?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    let mut processed_method_nodes = std::collections::HashSet::new();
    let mut processed_class_nodes = std::collections::HashSet::new();

    for match_ in matches {
        for capture in match_.captures {
            let node = capture.node;
            let name_idx = capture.index;
            let capture_name = query.capture_names()[name_idx as usize];

            match capture_name {
                "method" => {
                    let node_id = node.id();
                    if !processed_method_nodes.contains(&node_id) {
                        processed_method_nodes.insert(node_id);

                        if let Some(name_node) = node.child_by_field_name("name") {
                            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                let line = node.start_position().row + 1;
                                let end_line = node.end_position().row + 1;
                                let signature = extract_signature(node, source)?;
                                let doc = extract_doc_comment(node, source, Language::Java)?;
                                let code = if include_code {
                                    extract_code(node, source)?
                                } else {
                                    None
                                };

                                // Extract annotations
                                let annotations = extract_java_annotations(node, source);

                                functions.push(EnhancedFunctionInfo {
                                    name: name.to_string(),
                                    signature,
                                    line,
                                    end_line,
                                    doc,
                                    code,
                                    annotations,
                                });
                            }
                        }
                    }
                }
                "class" => {
                    if node.kind() == "class_declaration" {
                        let node_id = node.id();
                        if !processed_class_nodes.contains(&node_id) {
                            processed_class_nodes.insert(node_id);

                            if let Some(name_node) = node.child_by_field_name("name") {
                                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                    let line = node.start_position().row + 1;
                                    let end_line = node.end_position().row + 1;
                                    let doc = extract_doc_comment(node, source, Language::Java)?;
                                    let code = if include_code {
                                        extract_code(node, source)?
                                    } else {
                                        None
                                    };

                                    // Extract implements interfaces from super_interfaces
                                    let implements =
                                        extract_java_implemented_interfaces(node, source);

                                    // Extract methods from class
                                    let methods =
                                        extract_java_class_methods(node, source, include_code)?;

                                    classes.push(EnhancedClassInfo {
                                        name: name.to_string(),
                                        line,
                                        end_line,
                                        doc,
                                        code,
                                        methods,
                                        implements,
                                        properties: vec![],
                                        fields: vec![],
                                    });
                                }
                            }
                        }
                    }
                }
                "interface" => {
                    if node.kind() == "interface_declaration" {
                        if let Some(name_node) = node.child_by_field_name("name") {
                            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                let line = node.start_position().row + 1;
                                let end_line = node.end_position().row + 1;
                                let doc = extract_doc_comment(node, source, Language::Java)?;
                                let code = if include_code {
                                    extract_code(node, source)?
                                } else {
                                    None
                                };

                                // Extract methods from interface
                                let methods =
                                    extract_java_interface_methods(node, source, include_code)?;

                                interfaces.push(InterfaceInfo {
                                    name: name.to_string(),
                                    line,
                                    end_line,
                                    doc,
                                    code,
                                    methods,
                                    properties: vec![],
                                });
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
        impl_blocks: vec![],
        traits: vec![],
        interfaces,
        properties: vec![],
        dependencies: vec![],
    })
}

/// Extract enhanced shape from Go source code
fn extract_go_enhanced(
    tree: &Tree,
    source: &str,
    include_code: bool,
) -> Result<EnhancedFileShape, io::Error> {
    let mut functions = Vec::new();
    let mut structs = Vec::new();
    let mut imports = Vec::new();
    let mut traits = Vec::new();

    let query = Query::new(
        &tree_sitter_go::LANGUAGE.into(),
        r#"
        (function_declaration name: (identifier) @func.name) @func
        (method_declaration name: (field_identifier) @method.name) @method
        (type_declaration (type_spec name: (type_identifier) @type.name)) @type
        (import_spec) @import
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
                "func.name" | "method.name" => {
                    // Go methods are top-level constructs attached to types, so we treat them as functions
                    let func_node = if capture_name == "func.name" {
                        find_parent_by_type(node, "function_declaration").ok()
                    } else {
                        find_parent_by_type(node, "method_declaration").ok()
                    };

                    if let Some(func_node) = func_node {
                        if let Ok(name) = node.utf8_text(source.as_bytes()) {
                            let line = func_node.start_position().row + 1;
                            let end_line = func_node.end_position().row + 1;
                            let signature = extract_signature(func_node, source)?;
                            let doc = extract_doc_comment(func_node, source, Language::Go)?;
                            let code = if include_code {
                                extract_code(func_node, source)?
                            } else {
                                None
                            };

                            functions.push(EnhancedFunctionInfo {
                                name: name.to_string(),
                                signature,
                                line,
                                end_line,
                                doc,
                                code,
                                annotations: vec![],
                            });
                        }
                    }
                }
                "type.name" => {
                    // Check if it's a struct or interface
                    if let Ok(type_spec) = find_parent_by_type(node, "type_spec") {
                        if let Ok(name) = node.utf8_text(source.as_bytes()) {
                            let line = type_spec.start_position().row + 1;
                            let end_line = type_spec.end_position().row + 1;
                            let doc = extract_doc_comment(type_spec, source, Language::Go)?;
                            let code = if include_code {
                                // Extract from parent type_declaration to include 'type' keyword
                                let code_node = type_spec.parent().unwrap_or(type_spec);
                                extract_code(code_node, source)?
                            } else {
                                None
                            };

                            // Check type kind (struct or interface)
                            let is_interface = type_spec
                                .child_by_field_name("type")
                                .map(|t| t.kind() == "interface_type")
                                .unwrap_or(false);

                            if is_interface {
                                // Extract methods for interface
                                let mut methods = Vec::new();
                                if let Some(type_node) = type_spec.child_by_field_name("type") {
                                    let mut cursor = type_node.walk();
                                    for child in type_node.children(&mut cursor) {
                                        if child.kind() == "method_spec" {
                                            if let Some(name_node) =
                                                child.child_by_field_name("name")
                                            {
                                                if let Ok(method_name) =
                                                    name_node.utf8_text(source.as_bytes())
                                                {
                                                    let method_line =
                                                        child.start_position().row + 1;
                                                    let method_end_line =
                                                        child.end_position().row + 1;
                                                    let signature =
                                                        extract_signature(child, source)?;
                                                    let method_doc = extract_doc_comment(
                                                        child,
                                                        source,
                                                        Language::Go,
                                                    )?;

                                                    let code = if include_code {
                                                        extract_code(child, source)?
                                                    } else {
                                                        None
                                                    };

                                                    methods.push(MethodInfo {
                                                        name: method_name.to_string(),
                                                        signature,
                                                        line: method_line,
                                                        end_line: method_end_line,
                                                        doc: method_doc,
                                                        code,
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }

                                traits.push(TraitInfo {
                                    name: name.to_string(),
                                    line,
                                    end_line,
                                    doc,
                                    methods,
                                });
                            } else {
                                // Assume struct for other types
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
        impl_blocks: vec![],
        traits,
        interfaces: vec![],
        properties: vec![],
        dependencies: vec![],
    })
}

fn extract_kotlin_enhanced(
    tree: &Tree,
    source: &str,
    include_code: bool,
) -> Result<EnhancedFileShape, io::Error> {
    let language = tree_sitter_kotlin_ng::LANGUAGE.into();

    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut structs = Vec::new();
    let mut interfaces = Vec::new();
    let mut imports = Vec::new();

    // Query for top-level definitions
    let query_str = r#"
        (function_declaration) @function
        (class_declaration) @class
        (object_declaration) @object
        (type_alias) @alias
        (import_list (import_header)) @import
    "#;

    let query = Query::new(&language, query_str)
        .or_else(|_| {
            Query::new(
                &language,
                r#"
            (function_declaration) @function
            (class_declaration) @class
            (object_declaration) @object
            (type_alias) @alias
            (import_header) @import
        "#,
            )
        })
        .or_else(|_| {
            Query::new(
                &language,
                r#"
            (function_declaration) @function
            (class_declaration) @class
            (object_declaration) @object
            (type_alias) @alias
            (import) @import
        "#,
            )
        })
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to create Kotlin query: {e}"),
            )
        })?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            let node = capture.node;
            let capture_name = query.capture_names()[capture.index as usize];

            match capture_name {
                "function" => {
                    if let Some(name_node) = node.child_by_field_name("name") {
                        let name_node = if !name_node.is_missing() {
                            name_node
                        } else {
                            let mut cursor = node.walk();
                            let mut found = None;
                            for child in node.children(&mut cursor) {
                                if child.kind() == "simple_identifier" {
                                    found = Some(child);
                                    break;
                                }
                            }
                            found.unwrap_or(node)
                        };

                        if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                            if node.kind() == "function_declaration" {
                                let line = node.start_position().row + 1;
                                let end_line = node.end_position().row + 1;
                                let signature = extract_signature(node, source)?;
                                let doc = extract_doc_comment(node, source, Language::Kotlin)?;
                                let code = if include_code {
                                    extract_code(node, source)?
                                } else {
                                    None
                                };

                                functions.push(EnhancedFunctionInfo {
                                    name: name.to_string(),
                                    signature,
                                    line,
                                    end_line,
                                    doc,
                                    code,
                                    annotations: vec![],
                                });
                            }
                        }
                    }
                }
                "class" | "object" => {
                    let mut is_interface = false;
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
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

                    let mut name_found = String::new();
                    if let Some(name_node) = node.child_by_field_name("name") {
                        if let Ok(n) = name_node.utf8_text(source.as_bytes()) {
                            name_found = n.to_string();
                        }
                    } else {
                        let mut cursor = node.walk();
                        for child in node.children(&mut cursor) {
                            if child.kind() == "type_identifier"
                                || child.kind() == "simple_identifier"
                            {
                                if let Ok(n) = child.utf8_text(source.as_bytes()) {
                                    name_found = n.to_string();
                                }
                                break;
                            }
                        }
                    }

                    if !name_found.is_empty() {
                        let line = node.start_position().row + 1;
                        let end_line = node.end_position().row + 1;
                        let doc = extract_doc_comment(node, source, Language::Kotlin)?;
                        let code = if include_code {
                            extract_code(node, source)?
                        } else {
                            None
                        };

                        if is_interface {
                            interfaces.push(InterfaceInfo {
                                name: name_found,
                                line,
                                end_line,
                                doc,
                                code,
                                methods: vec![],
                                properties: vec![],
                            });
                        } else {
                            classes.push(EnhancedClassInfo {
                                name: name_found,
                                line,
                                end_line,
                                doc,
                                code,
                                methods: vec![],
                                fields: vec![],
                                implements: vec![],
                                properties: vec![],
                            });
                        }
                    }
                }
                "alias" => {
                    let mut name_found = String::new();
                    if let Some(name_node) = node.child_by_field_name("name") {
                        if let Ok(n) = name_node.utf8_text(source.as_bytes()) {
                            name_found = n.to_string();
                        }
                    }
                    if name_found.is_empty() {
                        if let Some(name_node) = node.child_by_field_name("type") {
                            if let Ok(n) = name_node.utf8_text(source.as_bytes()) {
                                name_found = n.to_string();
                            }
                        }
                    }

                    if name_found.is_empty() {
                        let mut cursor = node.walk();
                        for child in node.children(&mut cursor) {
                            if child.kind() == "type_identifier"
                                || child.kind() == "simple_identifier"
                                || child.kind() == "identifier"
                            {
                                if let Ok(n) = child.utf8_text(source.as_bytes()) {
                                    name_found = n.to_string();
                                }
                                break;
                            }
                        }
                    }

                    if !name_found.is_empty() {
                        let line = node.start_position().row + 1;
                        let end_line = node.end_position().row + 1;
                        let doc = extract_doc_comment(node, source, Language::Kotlin)?;
                        let code = if include_code {
                            extract_code(node, source)?
                        } else {
                            None
                        };

                        structs.push(EnhancedStructInfo {
                            name: name_found,
                            line,
                            end_line,
                            doc,
                            code,
                        });
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
        language: Some("kotlin".to_string()),
        functions,
        structs,
        classes,
        imports,
        impl_blocks: vec![],
        traits: vec![],
        interfaces,
        properties: vec![],
        dependencies: vec![],
    })
}

/// Helper function to extract methods from a Java class
fn extract_java_class_methods(
    class_node: Node,
    source: &str,
    include_code: bool,
) -> Result<Vec<EnhancedFunctionInfo>, io::Error> {
    let mut methods = Vec::new();

    if let Some(body) = class_node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "method_declaration" {
                if let Some(method_info) = extract_java_method_info(child, source, include_code)? {
                    methods.push(method_info);
                }
            }
        }
    }

    Ok(methods)
}

/// Extract method information from a Java method declaration node
fn extract_java_method_info(
    method_node: Node,
    source: &str,
    include_code: bool,
) -> Result<Option<EnhancedFunctionInfo>, io::Error> {
    if let Some(name_node) = method_node.child_by_field_name("name") {
        if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
            let line = method_node.start_position().row + 1;
            let end_line = method_node.end_position().row + 1;
            let signature = extract_signature(method_node, source)?;
            let doc = extract_doc_comment(method_node, source, Language::Java)?;
            let code = if include_code {
                extract_code(method_node, source)?
            } else {
                None
            };

            // Extract annotations
            let annotations = extract_java_annotations(method_node, source);

            return Ok(Some(EnhancedFunctionInfo {
                name: name.to_string(),
                signature,
                line,
                end_line,
                doc,
                code,
                annotations,
            }));
        }
    }
    Ok(None)
}

/// Extract methods from a Java interface body
fn extract_java_interface_methods(
    interface_node: Node,
    source: &str,
    include_code: bool,
) -> Result<Vec<EnhancedFunctionInfo>, io::Error> {
    let mut methods = Vec::new();

    if let Some(body) = interface_node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "method_declaration" {
                if let Some(method_info) = extract_java_method_info(child, source, include_code)? {
                    methods.push(method_info);
                }
            }
        }
    }

    Ok(methods)
}

/// Extract implemented interface names from a Java class node
fn extract_java_implemented_interfaces(class_node: Node, source: &str) -> Vec<String> {
    let mut implements = Vec::new();

    // Look for the super_interfaces field (kind: "interfaces")
    if let Some(super_interfaces) = class_node.child_by_field_name("interfaces") {
        // The super_interfaces node contains a type_list with type_identifier children
        let mut cursor = super_interfaces.walk();
        for child in super_interfaces.children(&mut cursor) {
            if child.kind() == "type_list" {
                // type_list contains the actual type_identifier nodes
                let mut type_cursor = child.walk();
                for type_child in child.children(&mut type_cursor) {
                    if type_child.kind() == "type_identifier" {
                        if let Ok(interface_name) = type_child.utf8_text(source.as_bytes()) {
                            implements.push(interface_name.to_string());
                        }
                    }
                }
            } else if child.kind() == "type_identifier" {
                // Fallback: sometimes type_identifier is direct child
                if let Ok(interface_name) = child.utf8_text(source.as_bytes()) {
                    implements.push(interface_name.to_string());
                }
            }
        }
    }

    implements
}

/// Extract annotations from a Java node (method or class)
fn extract_java_annotations(node: Node, source: &str) -> Vec<String> {
    let mut annotations = Vec::new();

    // In Java tree-sitter grammar, modifiers node exists but has no field name (empty string)
    // We need to look for 'modifiers' node kind among children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifiers" {
            // Found the modifiers node, now look for annotations inside it
            let mut mod_cursor = child.walk();
            for mod_child in child.children(&mut mod_cursor) {
                if mod_child.kind() == "marker_annotation" || mod_child.kind() == "annotation" {
                    // For marker_annotation: @Override has child with field 'name' that is an identifier
                    // For annotation: @SuppressWarnings(...) has a 'name' field
                    if let Some(name_node) = mod_child.child_by_field_name("name") {
                        if let Ok(annotation_name) = name_node.utf8_text(source.as_bytes()) {
                            annotations.push(annotation_name.to_string());
                        }
                    }
                }
            }
            break;
        }
    }

    annotations
}

/// Extract impl block information from a Rust impl_item node
fn extract_impl_block(
    node: Node,
    source: &str,
    include_code: bool,
) -> Result<ImplBlockInfo, io::Error> {
    let line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;

    // Extract type name (e.g., "Calculator" or "Container<T>")
    let type_name = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    // Extract trait name if it's a trait impl (e.g., "impl Display for Calculator")
    let trait_name = node
        .child_by_field_name("trait")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| {
            // Extract just the trait name, not the full path
            s.split("::").last().unwrap_or(s).to_string()
        });

    // Extract methods from the impl block body
    let mut methods = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "function_item" {
                if let Ok(method) = extract_method(child, source, include_code) {
                    methods.push(method);
                }
            }
        }
    }

    Ok(ImplBlockInfo {
        type_name,
        trait_name,
        line,
        end_line,
        methods,
    })
}

/// Extract method information from a function_item node within an impl block
fn extract_method(node: Node, source: &str, include_code: bool) -> Result<MethodInfo, io::Error> {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let signature = extract_signature(node, source)?;
    let line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let doc = extract_doc_comment(node, source, Language::Rust)?;
    let code = if include_code {
        extract_code(node, source)?
    } else {
        None
    };

    Ok(MethodInfo {
        name,
        signature,
        line,
        end_line,
        doc,
        code,
    })
}

/// Extract trait definition information from a Rust trait_item node
fn extract_trait(node: Node, source: &str, include_code: bool) -> Result<TraitInfo, io::Error> {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let doc = extract_doc_comment(node, source, Language::Rust)?;

    // Extract methods from the trait body
    let mut methods = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "function_item" || child.kind() == "function_signature_item" {
                if let Ok(method) = extract_method(child, source, include_code) {
                    methods.push(method);
                }
            }
        }
    }

    Ok(TraitInfo {
        name,
        line,
        end_line,
        doc,
        methods,
    })
}

/// Check if a node is inside a class definition
fn is_inside_class(node: Node) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "class_definition"
            || parent.kind() == "class_declaration"
            || parent.kind() == "struct_declaration"
        {
            return true;
        }
        current = parent.parent();
    }
    false
}

/// Extract interface definition information from a TypeScript interface_declaration node
fn extract_interface(
    node: Node,
    source: &str,
    include_code: bool,
) -> Result<InterfaceInfo, io::Error> {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let doc = extract_doc_comment(node, source, Language::TypeScript)?;

    // Extract methods from the interface body
    let mut methods = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            // TypeScript interfaces have method_signature and property_signature nodes
            if child.kind() == "method_signature" || child.kind() == "property_signature" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Ok(method_name) = name_node.utf8_text(source.as_bytes()) {
                        let method_line = child.start_position().row + 1;
                        let method_end_line = child.end_position().row + 1;
                        let signature = extract_signature(child, source)?;
                        let method_doc = extract_doc_comment(child, source, Language::TypeScript)?;

                        // Interfaces don't have code bodies, but we respect the include_code flag
                        let code = if include_code {
                            extract_code(child, source)?
                        } else {
                            None
                        };

                        methods.push(EnhancedFunctionInfo {
                            name: method_name.to_string(),
                            signature,
                            line: method_line,
                            end_line: method_end_line,
                            doc: method_doc,
                            code,
                            annotations: vec![],
                        });
                    }
                }
            }
        }
    }

    let code = if include_code {
        extract_code(node, source)?
    } else {
        None
    };

    Ok(InterfaceInfo {
        name,
        line,
        end_line,
        doc,
        code,
        methods,
        properties: vec![],
    })
}

/// Extract methods from a class body (Python, JavaScript, TypeScript)
fn extract_class_methods(
    class_node: Node,
    source: &str,
    language: Language,
    include_code: bool,
) -> Result<Vec<EnhancedFunctionInfo>, io::Error> {
    let mut methods = Vec::new();

    // Find the class body
    let body = match language {
        Language::Python => class_node.child_by_field_name("body"),
        Language::JavaScript | Language::TypeScript => class_node.child_by_field_name("body"),
        Language::Swift => class_node.child_by_field_name("body"),
        _ => None,
    };

    if let Some(body_node) = body {
        let mut cursor = body_node.walk();
        for child in body_node.children(&mut cursor) {
            // Skip nested classes
            if child.kind() == "class_definition"
                || child.kind() == "class_declaration"
                || child.kind() == "struct_declaration"
            {
                continue;
            }

            let is_method = match language {
                Language::Python => child.kind() == "function_definition",
                Language::JavaScript | Language::TypeScript => {
                    child.kind() == "method_definition" || child.kind() == "function_declaration"
                }
                Language::Swift => child.kind() == "function_declaration",
                _ => false,
            };

            if is_method {
                // Extract method name
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                        let line = child.start_position().row + 1;
                        let end_line = child.end_position().row + 1;
                        let signature = extract_signature(child, source)?;
                        let doc = extract_doc_comment(child, source, language)?;
                        let code = if include_code {
                            extract_code(child, source)?
                        } else {
                            None
                        };

                        methods.push(EnhancedFunctionInfo {
                            name: name.to_string(),
                            signature,
                            line,
                            end_line,
                            doc,
                            code,
                            annotations: vec![],
                        });
                    }
                }
            }
        }
    }

    Ok(methods)
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
            || kind == "function_body"
            || kind == "class_body"
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
            || trimmed.starts_with("func ")
            || trimmed.starts_with("type ")
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
        Language::Rust
        | Language::JavaScript
        | Language::TypeScript
        | Language::Swift
        | Language::CSharp
        | Language::Java
        | Language::Go => kind == "line_comment" || kind == "block_comment" || kind == "comment",
        Language::Python => kind == "comment",
        Language::Kotlin => kind == "line_comment" || kind == "block_comment",
        _ => false,
    }
}

/// Extract documentation text from a comment
fn extract_doc_from_comment(comment_text: &str, language: Language) -> String {
    let trimmed = comment_text.trim();

    match language {
        Language::Rust | Language::Swift | Language::CSharp => {
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
        Language::JavaScript
        | Language::TypeScript
        | Language::Java
        | Language::Go
        | Language::Kotlin => {
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
#[allow(dead_code)]
#[derive(Debug, serde::Serialize, Clone)]
pub struct ThemeVariable {
    pub name: String,  // "--color-primary", "--spacing-lg"
    pub value: String, // "oklch(0.6 0.2 250)", "1.5rem"
    pub line: usize,
}

/// Custom component class (defined with @apply or custom styles)
#[allow(dead_code)]
#[derive(Debug, serde::Serialize, Clone)]
pub struct CustomClass {
    pub name: String,                     // "btn-primary", "card"
    pub applied_utilities: Vec<String>,   // ["bg-primary", "text-white", "px-4"]
    pub layer: Option<Cow<'static, str>>, // "components", "utilities", or None
    pub line: usize,
}

/// Keyframe animation
#[allow(dead_code)]
#[derive(Debug, serde::Serialize, Clone)]
pub struct KeyframeInfo {
    pub name: String,
    pub line: usize,
}

/// CSS file shape (Tailwind v4 focused)
#[allow(dead_code)]
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
#[allow(dead_code)]
#[derive(Debug, serde::Serialize, Clone)]
pub struct HtmlIdInfo {
    pub tag: String,
    pub id: String,
    pub line: usize,
}

/// Script reference
#[allow(dead_code)]
#[derive(Debug, serde::Serialize, Clone)]
pub struct ScriptInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    pub inline: bool,
    pub line: usize,
}

/// Style reference
#[allow(dead_code)]
#[derive(Debug, serde::Serialize, Clone)]
pub struct StyleInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    pub inline: bool,
    pub line: usize,
}

/// HTML file shape
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
fn calculate_line(source: &str, byte_offset: usize) -> usize {
    source[..byte_offset].matches('\n').count() + 1
}

// ============================================================================
// HTML Extraction (Tree-sitter)
// ============================================================================

use std::collections::HashSet;

/// Extract HTML shape from parsed tree
#[allow(dead_code)]
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
#[allow(dead_code)]
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
