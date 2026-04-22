//! Minimal edit context for one symbol.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};
use tiktoken_rs::cl100k_base;
use tree_sitter::{Node, Tree};

use crate::analysis::dependencies::resolve_dependencies;
use crate::analysis::path_utils;
use crate::analysis::shape::{
    extract_enhanced_shape, EnhancedFileShape, EnhancedFunctionInfo, EnhancedStructInfo,
    ImportInfo, InterfaceInfo,
};
use crate::common::format;
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code, Language};

const TARGET_HEADER: &str = "name|line|sig|code";
const DEP_HEADER: &str = "kind|name|line|sig";
const IMPORT_HEADER: &str = "line|text";
const TYPE_HEADER: &str = "kind|name|line|sig";

#[derive(Debug, Clone)]
struct TargetSymbol {
    name: String,
    line: usize,
    end_line: usize,
    signature: String,
    code: String,
    scope: String,
}

#[derive(Debug, Clone)]
struct SymbolSignature {
    kind: &'static str,
    name: String,
    line: usize,
    signature: String,
}

/// Return compact context needed to edit one symbol.
pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path = arguments["file_path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'file_path' argument",
        )
    })?;
    let symbol = arguments["symbol_name"]
        .as_str()
        .or_else(|| arguments["symbol"].as_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Missing or invalid 'symbol_name' argument",
            )
        })?;
    let max_tokens = arguments["max_tokens"]
        .as_u64()
        .map(|value| value as usize)
        .unwrap_or(2000);

    let source = fs::read_to_string(file_path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to read file {file_path}: {e}"),
        )
    })?;
    let language = detect_language(file_path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            format!("Cannot detect language for file {file_path}: {e}"),
        )
    })?;
    let tree = parse_code(&source, language).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse {} code: {e}", language.name()),
        )
    })?;
    let shape = extract_enhanced_shape(&tree, &source, language, Some(file_path), true)?;
    let target = find_target_symbol(&shape, symbol).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Symbol '{symbol}' not found in {file_path}"),
        )
    })?;

    let called_names = collect_called_names(&tree, &source, language, target.line, target.end_line);
    let same_file_symbols = collect_same_file_signatures(&shape, None);
    let mut deps: Vec<SymbolSignature> = same_file_symbols
        .iter()
        .filter(|dep| dep.name != target.name && called_names.contains(&dep.name))
        .cloned()
        .collect();
    let mut seen_dep_names: HashSet<String> = deps.iter().map(|dep| dep.name.clone()).collect();
    seen_dep_names.insert(target.name.clone());
    deps.extend(collect_project_dependency_signatures(
        language,
        &source,
        Path::new(file_path),
        &called_names,
        &mut seen_dep_names,
    ));

    let target_text = format!("{}\n{}", target.signature, target.code);
    let imports = relevant_imports(&shape.imports, &target_text);
    let types = relevant_types(&shape, &target_text);

    let mut out = Map::new();
    out.insert(
        "p".to_string(),
        json!(path_utils::to_relative_path(file_path)),
    );
    out.insert("sym".to_string(), json!(symbol));
    out.insert("scope".to_string(), json!(target.scope));
    out.insert("h".to_string(), json!(TARGET_HEADER));
    out.insert("target".to_string(), json!(target_to_row(&target)));

    let dep_rows = signatures_to_rows(&deps);
    if !dep_rows.is_empty() {
        out.insert("dh".to_string(), json!(DEP_HEADER));
        out.insert("deps".to_string(), json!(dep_rows));
    }

    let type_rows = signatures_to_rows(&types);
    if !type_rows.is_empty() {
        out.insert("tyh".to_string(), json!(TYPE_HEADER));
        out.insert("types".to_string(), json!(type_rows));
    }

    let import_rows = imports_to_rows(&imports);
    if !import_rows.is_empty() {
        out.insert("ih".to_string(), json!(IMPORT_HEADER));
        out.insert("imports".to_string(), json!(import_rows));
    }

    enforce_budget(&mut out, max_tokens)?;

    let json_text = serde_json::to_string(&Value::Object(out)).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize result to JSON: {e}"),
        )
    })?;

    Ok(CallToolResult::success(json_text))
}

fn find_target_symbol(shape: &EnhancedFileShape, symbol: &str) -> Option<TargetSymbol> {
    for function in &shape.functions {
        if function.name == symbol {
            return Some(target_from_function(function, ""));
        }
    }

    for class in &shape.classes {
        if class.name == symbol {
            return class.code.as_ref().map(|code| TargetSymbol {
                name: class.name.clone(),
                line: class.line,
                end_line: class.end_line,
                signature: class.name.clone(),
                code: code.clone(),
                scope: class.name.clone(),
            });
        }

        for method in &class.methods {
            if method.name == symbol {
                return Some(target_from_function(method, &class.name));
            }
        }
    }

    for block in &shape.impl_blocks {
        for method in &block.methods {
            if method.name == symbol {
                return Some(TargetSymbol {
                    name: method.name.clone(),
                    line: method.line,
                    end_line: method.end_line,
                    signature: method.signature.clone(),
                    code: method.code.clone().unwrap_or_default(),
                    scope: block.type_name.clone(),
                });
            }
        }
    }

    None
}

fn target_from_function(function: &EnhancedFunctionInfo, parent_scope: &str) -> TargetSymbol {
    TargetSymbol {
        name: function.name.clone(),
        line: function.line,
        end_line: function.end_line,
        signature: function.signature.clone(),
        code: function.code.clone().unwrap_or_default(),
        scope: parent_scope.to_string(),
    }
}

fn collect_same_file_signatures(
    shape: &EnhancedFileShape,
    source_file: Option<&Path>,
) -> Vec<SymbolSignature> {
    let mut signatures = Vec::new();

    for function in &shape.functions {
        signatures.push(SymbolSignature {
            kind: dependency_kind("fn", source_file),
            name: function.name.clone(),
            line: function.line,
            signature: function.signature.clone(),
        });
    }

    for class in &shape.classes {
        for method in &class.methods {
            signatures.push(SymbolSignature {
                kind: dependency_kind("method", source_file),
                name: method.name.clone(),
                line: method.line,
                signature: method.signature.clone(),
            });
        }
    }

    for block in &shape.impl_blocks {
        for method in &block.methods {
            signatures.push(SymbolSignature {
                kind: dependency_kind("method", source_file),
                name: method.name.clone(),
                line: method.line,
                signature: method.signature.clone(),
            });
        }
    }

    signatures
}

fn dependency_kind(base: &'static str, source_file: Option<&Path>) -> &'static str {
    match (base, source_file.is_some()) {
        ("fn", true) => "dep_fn",
        ("method", true) => "dep_method",
        ("fn", false) => "fn",
        ("method", false) => "method",
        _ => base,
    }
}

fn collect_project_dependency_signatures(
    language: Language,
    source: &str,
    file_path: &Path,
    called_names: &HashSet<String>,
    seen_names: &mut HashSet<String>,
) -> Vec<SymbolSignature> {
    if called_names.is_empty() {
        return Vec::new();
    }

    let project_root = path_utils::find_project_root(file_path)
        .or_else(|| file_path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));
    let dependency_files = resolve_dependencies(language, source, file_path, &project_root);
    let mut signatures = Vec::new();

    for dependency_file in dependency_files {
        let Ok(dependency_source) = fs::read_to_string(&dependency_file) else {
            continue;
        };
        let Ok(dependency_language) = detect_language(&dependency_file) else {
            continue;
        };
        let Ok(dependency_tree) = parse_code(&dependency_source, dependency_language) else {
            continue;
        };
        let Ok(dependency_shape) = extract_enhanced_shape(
            &dependency_tree,
            &dependency_source,
            dependency_language,
            dependency_file.to_str(),
            false,
        ) else {
            continue;
        };

        for signature in collect_same_file_signatures(&dependency_shape, Some(&dependency_file)) {
            if called_names.contains(&signature.name) && seen_names.insert(signature.name.clone()) {
                signatures.push(signature);
            }
        }
    }

    signatures
}

fn collect_called_names(
    tree: &Tree,
    source: &str,
    language: Language,
    start_line: usize,
    end_line: usize,
) -> HashSet<String> {
    let mut names = HashSet::new();
    let cursor = tree.root_node().walk();
    collect_called_names_from_node(
        cursor.node(),
        source,
        language,
        start_line.saturating_sub(1),
        end_line.saturating_sub(1),
        &mut names,
    );
    drop(cursor);
    names
}

fn collect_called_names_from_node(
    node: Node<'_>,
    source: &str,
    language: Language,
    start_row: usize,
    end_row: usize,
    names: &mut HashSet<String>,
) {
    let node_start = node.start_position().row;
    let node_end = node.end_position().row;
    if node_end < start_row || node_start > end_row {
        return;
    }

    if is_call_node(node.kind(), language) {
        if let Some(name) = call_name(node, source) {
            names.insert(name);
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_called_names_from_node(child, source, language, start_row, end_row, names);
    }
}

fn is_call_node(kind: &str, language: Language) -> bool {
    match language {
        Language::Rust => matches!(kind, "call_expression" | "method_call_expression"),
        Language::Python => kind == "call",
        Language::JavaScript | Language::TypeScript | Language::Go => kind == "call_expression",
        Language::Java | Language::CSharp | Language::Swift => kind.ends_with("invocation"),
        Language::Html | Language::Css => false,
    }
}

fn call_name(node: Node<'_>, source: &str) -> Option<String> {
    for field in ["function", "name", "method", "field"] {
        if let Some(child) = node.child_by_field_name(field) {
            if let Some(name) = last_identifier_text(child, source) {
                return Some(name);
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(name) = last_identifier_text(child, source) {
            return Some(name);
        }
    }

    None
}

fn last_identifier_text(node: Node<'_>, source: &str) -> Option<String> {
    if node.kind() == "identifier" || node.kind().ends_with("_identifier") {
        return node
            .utf8_text(source.as_bytes())
            .ok()
            .map(ToOwned::to_owned);
    }

    let mut found = None;
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(name) = last_identifier_text(child, source) {
            found = Some(name);
        }
    }

    found
}

fn relevant_imports(imports: &[ImportInfo], target_text: &str) -> Vec<ImportInfo> {
    imports
        .iter()
        .filter(|import| {
            import_tokens(&import.text)
                .iter()
                .any(|tok| token_in_text(tok, target_text))
        })
        .cloned()
        .collect()
}

fn import_tokens(import_text: &str) -> Vec<String> {
    import_text
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .filter(|token| token.len() > 1)
        .filter(|token| {
            !matches!(
                *token,
                "use" | "import" | "from" | "as" | "type" | "crate" | "self" | "super"
            )
        })
        .map(ToOwned::to_owned)
        .collect()
}

fn relevant_types(shape: &EnhancedFileShape, target_text: &str) -> Vec<SymbolSignature> {
    let mut types = Vec::new();

    for s in &shape.structs {
        push_type_if_relevant(
            &mut types,
            "struct",
            &s.name,
            s.line,
            signature_for_struct(s),
            target_text,
        );
    }
    for c in &shape.classes {
        push_type_if_relevant(
            &mut types,
            "class",
            &c.name,
            c.line,
            c.name.clone(),
            target_text,
        );
    }
    for i in &shape.interfaces {
        push_type_if_relevant(
            &mut types,
            "interface",
            &i.name,
            i.line,
            signature_for_interface(i),
            target_text,
        );
    }

    types
}

fn push_type_if_relevant(
    types: &mut Vec<SymbolSignature>,
    kind: &'static str,
    name: &str,
    line: usize,
    signature: String,
    target_text: &str,
) {
    if token_in_text(name, target_text) {
        types.push(SymbolSignature {
            kind,
            name: name.to_string(),
            line,
            signature,
        });
    }
}

fn signature_for_struct(info: &EnhancedStructInfo) -> String {
    info.code
        .as_deref()
        .and_then(|code| code.lines().next())
        .unwrap_or(&info.name)
        .to_string()
}

fn signature_for_interface(info: &InterfaceInfo) -> String {
    info.code
        .as_deref()
        .and_then(|code| code.lines().next())
        .unwrap_or(&info.name)
        .to_string()
}

fn token_in_text(token: &str, text: &str) -> bool {
    text.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .any(|part| part == token)
}

fn target_to_row(target: &TargetSymbol) -> String {
    let line = target.line.to_string();
    format::format_row(&[&target.name, &line, &target.signature, &target.code])
}

fn signatures_to_rows(signatures: &[SymbolSignature]) -> String {
    signatures
        .iter()
        .map(|sig| {
            let line = sig.line.to_string();
            format::format_row(&[sig.kind, &sig.name, &line, &sig.signature])
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn imports_to_rows(imports: &[ImportInfo]) -> String {
    imports
        .iter()
        .map(|import| {
            let line = import.line.to_string();
            format::format_row(&[&line, &import.text])
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn enforce_budget(out: &mut Map<String, Value>, max_tokens: usize) -> Result<(), io::Error> {
    let bpe = cl100k_base()
        .map_err(|e| io::Error::other(format!("Failed to initialize tiktoken tokenizer: {e}")))?;

    for keys in [
        &["imports", "ih"][..],
        &["types", "tyh"][..],
        &["deps", "dh"][..],
    ] {
        let json_text = serde_json::to_string(&Value::Object(out.clone())).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize result to JSON: {e}"),
            )
        })?;
        if bpe.encode_with_special_tokens(&json_text).len() <= max_tokens {
            return Ok(());
        }

        for key in keys {
            out.remove(*key);
        }
        out.insert("@".to_string(), json!({"t": true}));
    }

    Ok(())
}
