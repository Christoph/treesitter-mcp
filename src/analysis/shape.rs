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

    // NEW: Methods nested in class (Python, JavaScript, TypeScript)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<EnhancedFunctionInfo>,
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

/// Interface information (TypeScript)
#[derive(Debug, serde::Serialize, Clone)]
pub struct InterfaceInfo {
    pub name: String,
    pub line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    pub methods: Vec<EnhancedFunctionInfo>,
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

    // NEW: Interfaces for TypeScript
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<InterfaceInfo>,

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
        Language::Go => extract_go_enhanced(tree, source, include_code)?,
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
                impl_blocks: vec![],
                traits: vec![],
                interfaces: vec![],
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
        dependencies: vec![],
    })
}

/// Extract enhanced shape from Go source code
fn extract_go_enhanced(
    tree: &Tree,
    source: &str,
    include_code: bool,
) -> Result<EnhancedFileShape, io::Error> {
    let query = Query::new(
        &tree_sitter_go::LANGUAGE.into(),
        r#"
        (function_declaration
            name: (identifier) @func.name) @func
        
        (method_declaration
            receiver: (parameter_list)
            name: (field_identifier) @method.name) @method
        
        (type_declaration
            (type_spec
                name: (type_identifier) @struct.name
                type: (struct_type))) @struct
        
        (type_declaration
            (type_spec
                name: (type_identifier) @interface.name
                type: (interface_type))) @interface
        
        (import_declaration) @import
        "#,
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    let mut imports = Vec::new();
    let mut functions = Vec::new();
    let mut classes = Vec::new();

    let func_name_idx = query.capture_index_for_name("func.name").unwrap();
    let func_idx = query.capture_index_for_name("func").unwrap();
    let method_name_idx = query.capture_index_for_name("method.name").unwrap();
    let method_idx = query.capture_index_for_name("method").unwrap();
    let struct_name_idx = query.capture_index_for_name("struct.name").unwrap();
    let struct_idx = query.capture_index_for_name("struct").unwrap();
    let interface_name_idx = query.capture_index_for_name("interface.name").unwrap();
    let interface_idx = query.capture_index_for_name("interface").unwrap();
    let import_idx = query.capture_index_for_name("import").unwrap();

    for m in matches {
        for capture in m.captures {
            if capture.index == import_idx {
                let node = capture.node;
                let import_text = node.utf8_text(source.as_bytes()).unwrap_or("");
                imports.push(ImportInfo {
                    text: import_text.to_string(),
                    line: node.start_position().row + 1,
                });
            } else if capture.index == func_name_idx {
                if let Some(func_capture) = m.captures.iter().find(|c| c.index == func_idx) {
                    let name_node = capture.node;
                    let func_node = func_capture.node;
                    let name = name_node.utf8_text(source.as_bytes()).unwrap_or("");
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
                    });
                }
            } else if capture.index == method_name_idx {
                if let Some(method_capture) = m.captures.iter().find(|c| c.index == method_idx) {
                    let name_node = capture.node;
                    let method_node = method_capture.node;
                    let name = name_node.utf8_text(source.as_bytes()).unwrap_or("");
                    let line = method_node.start_position().row + 1;
                    let end_line = method_node.end_position().row + 1;
                    let signature = extract_signature(method_node, source)?;
                    let doc = extract_doc_comment(method_node, source, Language::Go)?;
                    let code = if include_code {
                        extract_code(method_node, source)?
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
                    });
                }
            } else if capture.index == struct_name_idx {
                if let Some(struct_capture) = m.captures.iter().find(|c| c.index == struct_idx) {
                    let name_node = capture.node;
                    let struct_node = struct_capture.node;
                    let name = name_node.utf8_text(source.as_bytes()).unwrap_or("");
                    let line = struct_node.start_position().row + 1;
                    let end_line = struct_node.end_position().row + 1;
                    let doc = extract_doc_comment(struct_node, source, Language::Go)?;
                    let code = if include_code {
                        extract_code(struct_node, source)?
                    } else {
                        None
                    };

                    classes.push(EnhancedClassInfo {
                        name: name.to_string(),
                        line,
                        end_line,
                        doc,
                        code,
                        methods: vec![],
                    });
                }
            } else if capture.index == interface_name_idx {
                if let Some(interface_capture) =
                    m.captures.iter().find(|c| c.index == interface_idx)
                {
                    let name_node = capture.node;
                    let interface_node = interface_capture.node;
                    let name = name_node.utf8_text(source.as_bytes()).unwrap_or("");
                    let line = interface_node.start_position().row + 1;
                    let end_line = interface_node.end_position().row + 1;
                    let doc = extract_doc_comment(interface_node, source, Language::Go)?;
                    let code = if include_code {
                        extract_code(interface_node, source)?
                    } else {
                        None
                    };

                    classes.push(EnhancedClassInfo {
                        name: name.to_string(),
                        line,
                        end_line,
                        doc,
                        code,
                        methods: vec![],
                    });
                }
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
        dependencies: vec![],
    })
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
        if parent.kind() == "class_definition" || parent.kind() == "class_declaration" {
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
                        });
                    }
                }
            }
        }
    }

    Ok(InterfaceInfo {
        name,
        line,
        end_line,
        doc,
        methods,
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
        _ => None,
    };

    if let Some(body_node) = body {
        let mut cursor = body_node.walk();
        for child in body_node.children(&mut cursor) {
            // Skip nested classes
            if child.kind() == "class_definition" || child.kind() == "class_declaration" {
                continue;
            }

            let is_method = match language {
                Language::Python => child.kind() == "function_definition",
                Language::JavaScript | Language::TypeScript => {
                    child.kind() == "method_definition" || child.kind() == "function_declaration"
                }
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
        let shape = extract_rust_enhanced(&tree, source, true).expect("Failed to extract shape");

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
        let shape = extract_python_enhanced(&tree, source, true).expect("Failed to extract shape");

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
        let shape = extract_js_enhanced(&tree, source, Language::JavaScript, true)
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
        let shape = extract_rust_enhanced(&tree, source, true).expect("Failed to extract shape");

        assert_eq!(shape.imports.len(), 2);
        assert_eq!(shape.imports[0].text, "use std::fmt;");
        assert_eq!(shape.imports[0].line, 2);
    }

    // ========================================================================
    // CSS Extraction Tests
    // ========================================================================

    #[test]
    fn test_extract_css_tailwind_theme_variables() {
        let source = r#"
@theme {
  --color-primary: oklch(0.6 0.2 250);
  --color-secondary: #3b82f6;
  --spacing-lg: 1.5rem;
}
"#;
        let shape = extract_css_tailwind(source, Some("test.css")).expect("Failed to extract CSS");

        assert_eq!(shape.theme.len(), 3);
        assert_eq!(shape.theme[0].name, "--color-primary");
        assert_eq!(shape.theme[0].value, "oklch(0.6 0.2 250)");
        assert_eq!(shape.theme[0].line, 3);

        assert_eq!(shape.theme[1].name, "--color-secondary");
        assert_eq!(shape.theme[1].value, "#3b82f6");

        assert_eq!(shape.theme[2].name, "--spacing-lg");
        assert_eq!(shape.theme[2].value, "1.5rem");
    }

    #[test]
    fn test_extract_css_tailwind_custom_classes() {
        let source = r#"
@layer components {
  .btn-primary {
    @apply bg-blue-500 text-white px-4 py-2 rounded;
  }
  .card {
    @apply border rounded-lg p-4;
  }
}
"#;
        let shape = extract_css_tailwind(source, None).expect("Failed to extract CSS");

        assert_eq!(shape.custom_classes.len(), 2);

        let btn = &shape.custom_classes[0];
        assert_eq!(btn.name, "btn-primary");
        assert_eq!(btn.applied_utilities.len(), 5);
        assert!(btn.applied_utilities.contains(&"bg-blue-500".to_string()));
        assert!(btn.applied_utilities.contains(&"text-white".to_string()));
        assert_eq!(btn.layer, Some(std::borrow::Cow::Borrowed("components")));

        let card = &shape.custom_classes[1];
        assert_eq!(card.name, "card");
        assert_eq!(card.applied_utilities.len(), 3);
        assert!(card.applied_utilities.contains(&"border".to_string()));
    }

    #[test]
    fn test_extract_css_tailwind_utilities_layer() {
        let source = r#"
@layer utilities {
  .text-balance {
    text-wrap: balance;
  }
}
"#;
        let shape = extract_css_tailwind(source, None).expect("Failed to extract CSS");

        assert_eq!(shape.custom_classes.len(), 1);
        assert_eq!(shape.custom_classes[0].name, "text-balance");
        assert_eq!(
            shape.custom_classes[0].layer,
            Some(std::borrow::Cow::Borrowed("utilities"))
        );
    }

    #[test]
    fn test_extract_css_tailwind_keyframes() {
        let source = r#"
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@keyframes fade-in {
  0% { opacity: 0; }
  100% { opacity: 1; }
}
"#;
        let shape = extract_css_tailwind(source, None).expect("Failed to extract CSS");

        assert_eq!(shape.keyframes.len(), 2);
        assert_eq!(shape.keyframes[0].name, "spin");
        assert_eq!(shape.keyframes[0].line, 2);
        assert_eq!(shape.keyframes[1].name, "fade-in");
        assert_eq!(shape.keyframes[1].line, 7);
    }

    #[test]
    fn test_extract_css_tailwind_empty_source() {
        let source = "";
        let shape = extract_css_tailwind(source, None).expect("Failed to extract CSS");

        assert_eq!(shape.theme.len(), 0);
        assert_eq!(shape.custom_classes.len(), 0);
        assert_eq!(shape.keyframes.len(), 0);
    }

    #[test]
    fn test_extract_css_tailwind_nested_braces() {
        let source = r#"
@layer components {
  .complex {
    @apply bg-blue-500;
    &:hover {
      @apply bg-blue-600;
    }
  }
}
"#;
        let shape = extract_css_tailwind(source, None).expect("Failed to extract CSS");

        // Should extract the outer class
        assert!(shape.custom_classes.iter().any(|c| c.name == "complex"));
    }

    #[test]
    fn test_extract_css_tailwind_multiple_layers() {
        let source = r#"
@layer components {
  .btn { @apply px-4 py-2; }
}

@layer utilities {
  .custom-util { color: red; }
}
"#;
        let shape = extract_css_tailwind(source, None).expect("Failed to extract CSS");

        assert_eq!(shape.custom_classes.len(), 2);
        assert_eq!(
            shape.custom_classes[0].layer,
            Some(std::borrow::Cow::Borrowed("components"))
        );
        assert_eq!(
            shape.custom_classes[1].layer,
            Some(std::borrow::Cow::Borrowed("utilities"))
        );
    }

    #[test]
    fn test_extract_css_tailwind_malformed_no_panic() {
        // Test that malformed CSS doesn't panic
        let source = r#"
@theme {
  --incomplete
}
@layer components {
  .unclosed {
"#;
        // Should not panic, just extract what it can
        let result = extract_css_tailwind(source, None);
        assert!(result.is_ok());
    }

    // ========================================================================
    // HTML Extraction Tests
    // ========================================================================

    #[test]
    fn test_extract_html_shape_ids() {
        let source = r#"
<!DOCTYPE html>
<html>
<body>
  <div id="header">Header</div>
  <nav id="main-nav">Navigation</nav>
  <section id="content">Content</section>
</body>
</html>
"#;
        let tree = parse_code(source, Language::Html).expect("Failed to parse HTML");
        let shape =
            extract_html_shape(&tree, source, Some("test.html")).expect("Failed to extract HTML");

        assert_eq!(shape.ids.len(), 3);
        assert_eq!(shape.ids[0].tag, "div");
        assert_eq!(shape.ids[0].id, "header");
        assert_eq!(shape.ids[1].tag, "nav");
        assert_eq!(shape.ids[1].id, "main-nav");
        assert_eq!(shape.ids[2].tag, "section");
        assert_eq!(shape.ids[2].id, "content");
    }

    #[test]
    fn test_extract_html_shape_custom_classes() {
        let source = r#"
<div class="custom-card bg-white p-4">
  <h1 class="title text-xl font-bold">Hello</h1>
  <p class="description">Text</p>
</div>
"#;
        let tree = parse_code(source, Language::Html).expect("Failed to parse HTML");
        let shape = extract_html_shape(&tree, source, None).expect("Failed to extract HTML");

        // Should filter out Tailwind utilities and keep only custom classes
        assert!(shape.classes_used.contains(&"custom-card".to_string()));
        assert!(shape.classes_used.contains(&"title".to_string()));
        assert!(shape.classes_used.contains(&"description".to_string()));

        // Should NOT contain Tailwind utilities
        assert!(!shape.classes_used.contains(&"bg-white".to_string()));
        assert!(!shape.classes_used.contains(&"p-4".to_string()));
        assert!(!shape.classes_used.contains(&"text-xl".to_string()));
        assert!(!shape.classes_used.contains(&"font-bold".to_string()));
    }

    #[test]
    fn test_extract_html_shape_scripts() {
        let source = r#"
<html>
<head>
  <script src="app.js"></script>
  <script>console.log('inline');</script>
</head>
</html>
"#;
        let tree = parse_code(source, Language::Html).expect("Failed to parse HTML");
        let shape = extract_html_shape(&tree, source, None).expect("Failed to extract HTML");

        assert_eq!(shape.scripts.len(), 2);

        // External script
        assert_eq!(shape.scripts[0].src, Some("app.js".to_string()));
        assert!(!shape.scripts[0].inline);

        // Inline script
        assert_eq!(shape.scripts[1].src, None);
        assert!(shape.scripts[1].inline);
    }

    #[test]
    fn test_extract_html_shape_styles() {
        let source = r#"
<html>
<head>
  <link rel="stylesheet" href="styles.css">
  <style>body { margin: 0; }</style>
</head>
</html>
"#;
        let tree = parse_code(source, Language::Html).expect("Failed to parse HTML");
        let shape = extract_html_shape(&tree, source, None).expect("Failed to extract HTML");

        assert_eq!(shape.styles.len(), 2);

        // External stylesheet
        assert_eq!(shape.styles[0].href, Some("styles.css".to_string()));
        assert!(!shape.styles[0].inline);

        // Inline style
        assert_eq!(shape.styles[1].href, None);
        assert!(shape.styles[1].inline);
    }

    #[test]
    fn test_extract_html_shape_tailwind_variants() {
        let source = r#"
<div class="hover:bg-blue-500 dark:text-white sm:p-4 custom-class">
  Content
</div>
"#;
        let tree = parse_code(source, Language::Html).expect("Failed to parse HTML");
        let shape = extract_html_shape(&tree, source, None).expect("Failed to extract HTML");

        // Should keep custom class
        assert!(shape.classes_used.contains(&"custom-class".to_string()));

        // Should filter out Tailwind utilities with variants
        assert!(!shape
            .classes_used
            .contains(&"hover:bg-blue-500".to_string()));
        assert!(!shape.classes_used.contains(&"dark:text-white".to_string()));
        assert!(!shape.classes_used.contains(&"sm:p-4".to_string()));
    }

    #[test]
    fn test_extract_html_shape_empty_document() {
        let source = "<html></html>";
        let tree = parse_code(source, Language::Html).expect("Failed to parse HTML");
        let shape = extract_html_shape(&tree, source, None).expect("Failed to extract HTML");

        assert_eq!(shape.ids.len(), 0);
        assert_eq!(shape.classes_used.len(), 0);
        assert_eq!(shape.scripts.len(), 0);
        assert_eq!(shape.styles.len(), 0);
    }

    #[test]
    fn test_extract_html_shape_deduplicates_classes() {
        let source = r#"
<div class="custom-class">
  <span class="custom-class">Text</span>
  <p class="custom-class">More</p>
</div>
"#;
        let tree = parse_code(source, Language::Html).expect("Failed to parse HTML");
        let shape = extract_html_shape(&tree, source, None).expect("Failed to extract HTML");

        // Should only have one instance of custom-class
        assert_eq!(
            shape
                .classes_used
                .iter()
                .filter(|c| *c == "custom-class")
                .count(),
            1
        );
    }

    #[test]
    fn test_extract_html_shape_line_numbers() {
        let source = r#"
<html>
<body>
  <div id="first">Line 4</div>
  <div id="second">Line 5</div>
</body>
</html>
"#;
        let tree = parse_code(source, Language::Html).expect("Failed to parse HTML");
        let shape = extract_html_shape(&tree, source, None).expect("Failed to extract HTML");

        assert_eq!(shape.ids.len(), 2);
        assert_eq!(shape.ids[0].line, 4);
        assert_eq!(shape.ids[1].line, 5);
    }

    #[test]
    fn test_extract_html_shape_malformed_no_panic() {
        // Test that malformed HTML doesn't panic
        let source = r#"
<div id="test" class="custom
<p>Unclosed
"#;
        let tree = parse_code(source, Language::Html).expect("Failed to parse HTML");
        // Should not panic, tree-sitter handles malformed HTML gracefully
        let result = extract_html_shape(&tree, source, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_html_shape_arbitrary_tailwind_values() {
        let source = r#"
<div class="w-[300px] h-[calc(100vh-64px)] custom-width">
  Content
</div>
"#;
        let tree = parse_code(source, Language::Html).expect("Failed to parse HTML");
        let shape = extract_html_shape(&tree, source, None).expect("Failed to extract HTML");

        // Should keep custom class
        assert!(shape.classes_used.contains(&"custom-width".to_string()));

        // Should filter out arbitrary Tailwind values
        assert!(!shape.classes_used.contains(&"w-[300px]".to_string()));
        assert!(!shape
            .classes_used
            .contains(&"h-[calc(100vh-64px)]".to_string()));
    }

    #[test]
    fn test_extract_html_shape_important_modifier() {
        let source = r#"
<div class="!bg-red-500 !important-custom">
  Content
</div>
"#;
        let tree = parse_code(source, Language::Html).expect("Failed to parse HTML");
        let shape = extract_html_shape(&tree, source, None).expect("Failed to extract HTML");

        // Should keep custom class with ! prefix
        assert!(shape
            .classes_used
            .contains(&"!important-custom".to_string()));

        // Should filter out Tailwind utility with ! prefix
        assert!(!shape.classes_used.contains(&"!bg-red-500".to_string()));
    }
}
