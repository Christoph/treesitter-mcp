//! Compact best-effort call graph extraction.

use std::collections::{HashSet, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use tiktoken_rs::cl100k_base;
use tree_sitter::{Node, Tree};

use crate::analysis::path_utils;
use crate::analysis::shape::{
    extract_enhanced_shape, EnhancedFileShape, EnhancedFunctionInfo, MethodInfo,
};
use crate::common::format;
use crate::common::project_files::collect_project_files;
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code, Language};

const EDGE_HEADER: &str = "direction|symbol|file|line|scope|depth";
const DEFAULT_MAX_TOKENS: usize = 2000;
const MAX_DEPTH: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Callers,
    Callees,
    Both,
}

#[derive(Debug, Clone)]
struct SymbolDef {
    name: String,
    file: PathBuf,
    line: usize,
    end_line: usize,
    scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Edge {
    direction: &'static str,
    symbol: String,
    file: String,
    line: usize,
    scope: String,
    depth: usize,
}

/// Return a compact caller/callee graph for one symbol.
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
    let direction = parse_direction(arguments["direction"].as_str().unwrap_or("both"))?;
    let depth = arguments["depth"]
        .as_u64()
        .map(|value| value as usize)
        .unwrap_or(1)
        .clamp(1, MAX_DEPTH);
    let max_tokens = arguments["max_tokens"]
        .as_u64()
        .map(|value| value as usize)
        .unwrap_or(DEFAULT_MAX_TOKENS);

    let target_path = PathBuf::from(file_path);
    if !target_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Path does not exist: {file_path}"),
        ));
    }

    let root = path_utils::find_project_root(&target_path)
        .or_else(|| target_path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));
    let files = collect_supported_files(&root)?;
    let definitions = collect_definitions(&files)?;
    let target = find_target_definition(&definitions, &target_path, symbol).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Symbol '{symbol}' not found in {file_path}"),
        )
    })?;

    let mut edges = Vec::new();
    if matches!(direction, Direction::Callees | Direction::Both) {
        collect_callee_edges(&target, &definitions, depth, &mut edges)?;
    }
    if matches!(direction, Direction::Callers | Direction::Both) {
        collect_caller_edges(&target, &files, &definitions, depth, &mut edges)?;
    }

    edges.sort_by(|a, b| {
        a.depth
            .cmp(&b.depth)
            .then_with(|| a.direction.cmp(b.direction))
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.symbol.cmp(&b.symbol))
            .then_with(|| a.scope.cmp(&b.scope))
    });
    edges.dedup();

    let (rows, truncated) = edge_rows_with_budget(&edges, symbol, max_tokens)?;
    let mut result = json!({
        "sym": symbol,
        "h": EDGE_HEADER,
        "edges": rows,
    });
    if truncated {
        result["@"] = json!({"t": true});
    }

    let json_text = serde_json::to_string(&result).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize result to JSON: {e}"),
        )
    })?;

    Ok(CallToolResult::success(json_text))
}

fn parse_direction(value: &str) -> Result<Direction, io::Error> {
    match value {
        "callers" => Ok(Direction::Callers),
        "callees" => Ok(Direction::Callees),
        "both" => Ok(Direction::Both),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid direction '{other}', expected callers, callees, or both"),
        )),
    }
}

fn collect_supported_files(root: &Path) -> Result<Vec<PathBuf>, io::Error> {
    Ok(collect_project_files(root)?
        .into_iter()
        .filter(|path| detect_language(path).is_ok())
        .collect())
}

fn collect_definitions(files: &[PathBuf]) -> Result<Vec<SymbolDef>, io::Error> {
    let mut definitions = Vec::new();
    for file in files {
        let Ok((shape, _tree, _source, _language)) = parse_shape(file) else {
            continue;
        };
        definitions.extend(definitions_from_shape(file, &shape));
    }
    Ok(definitions)
}

fn parse_shape(path: &Path) -> Result<(EnhancedFileShape, Tree, String, Language), io::Error> {
    let source = fs::read_to_string(path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to read file {}: {e}", path.display()),
        )
    })?;
    let language = detect_language(path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            format!("Cannot detect language for file {}: {e}", path.display()),
        )
    })?;
    let tree = parse_code(&source, language).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse {} code: {e}", language.name()),
        )
    })?;
    let shape = extract_enhanced_shape(&tree, &source, language, path.to_str(), false)?;
    Ok((shape, tree, source, language))
}

fn definitions_from_shape(file: &Path, shape: &EnhancedFileShape) -> Vec<SymbolDef> {
    let mut definitions = Vec::new();

    for function in &shape.functions {
        definitions.push(def_from_function(file, function, ""));
    }

    for class in &shape.classes {
        for method in &class.methods {
            definitions.push(def_from_function(file, method, &class.name));
        }
    }

    for block in &shape.impl_blocks {
        for method in &block.methods {
            definitions.push(def_from_method(file, method, &block.type_name));
        }
    }

    for interface in &shape.interfaces {
        for method in &interface.methods {
            definitions.push(def_from_function(file, method, &interface.name));
        }
    }

    definitions
}

fn def_from_function(file: &Path, function: &EnhancedFunctionInfo, scope: &str) -> SymbolDef {
    SymbolDef {
        name: function.name.clone(),
        file: file.to_path_buf(),
        line: function.line,
        end_line: function.end_line,
        scope: scope.to_string(),
    }
}

fn def_from_method(file: &Path, method: &MethodInfo, scope: &str) -> SymbolDef {
    SymbolDef {
        name: method.name.clone(),
        file: file.to_path_buf(),
        line: method.line,
        end_line: method.end_line,
        scope: scope.to_string(),
    }
}

fn find_target_definition(
    definitions: &[SymbolDef],
    file_path: &Path,
    symbol: &str,
) -> Option<SymbolDef> {
    let canonical_target = file_path
        .canonicalize()
        .unwrap_or_else(|_| file_path.to_path_buf());
    definitions
        .iter()
        .find(|definition| {
            definition.name == symbol
                && definition
                    .file
                    .canonicalize()
                    .unwrap_or_else(|_| definition.file.clone())
                    == canonical_target
        })
        .cloned()
        .or_else(|| {
            definitions
                .iter()
                .find(|definition| definition.name == symbol)
                .cloned()
        })
}

fn collect_callee_edges(
    target: &SymbolDef,
    definitions: &[SymbolDef],
    max_depth: usize,
    edges: &mut Vec<Edge>,
) -> Result<(), io::Error> {
    let mut queue = VecDeque::from([(target.clone(), 1usize)]);
    let mut visited = HashSet::new();

    while let Some((current, depth)) = queue.pop_front() {
        if depth > max_depth || !visited.insert((current.file.clone(), current.line)) {
            continue;
        }

        let called_names = called_names_for_symbol(&current)?;
        for name in called_names {
            let Some(callee) = resolve_definition(definitions, &name, &current.file) else {
                continue;
            };
            edges.push(edge("callee", &callee, depth));
            if depth < max_depth {
                queue.push_back((callee, depth + 1));
            }
        }
    }

    Ok(())
}

fn collect_caller_edges(
    target: &SymbolDef,
    files: &[PathBuf],
    definitions: &[SymbolDef],
    max_depth: usize,
    edges: &mut Vec<Edge>,
) -> Result<(), io::Error> {
    let mut queue = VecDeque::from([(target.clone(), 1usize)]);
    let mut visited = HashSet::new();

    while let Some((current, depth)) = queue.pop_front() {
        if depth > max_depth || !visited.insert((current.file.clone(), current.line)) {
            continue;
        }

        for caller in callers_for_symbol(&current.name, files, definitions)? {
            edges.push(edge("caller", &caller, depth));
            if depth < max_depth {
                queue.push_back((caller, depth + 1));
            }
        }
    }

    Ok(())
}

fn called_names_for_symbol(symbol: &SymbolDef) -> Result<HashSet<String>, io::Error> {
    let (_shape, tree, source, language) = parse_shape(&symbol.file)?;
    Ok(collect_called_names(
        &tree,
        &source,
        language,
        symbol.line,
        symbol.end_line,
    ))
}

fn callers_for_symbol(
    symbol_name: &str,
    files: &[PathBuf],
    definitions: &[SymbolDef],
) -> Result<Vec<SymbolDef>, io::Error> {
    let mut callers = Vec::new();

    for file in files {
        let Ok((_shape, tree, source, language)) = parse_shape(file) else {
            continue;
        };
        let call_sites = collect_call_sites(&tree, &source, language, symbol_name);
        for line in call_sites {
            if let Some(caller) = definition_containing_line(definitions, file, line) {
                callers.push(caller.clone());
            }
        }
    }

    callers.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.name.cmp(&b.name))
    });
    callers.dedup_by(|a, b| a.file == b.file && a.line == b.line && a.name == b.name);
    Ok(callers)
}

fn resolve_definition(
    definitions: &[SymbolDef],
    name: &str,
    current_file: &Path,
) -> Option<SymbolDef> {
    definitions
        .iter()
        .find(|definition| definition.name == name && definition.file == current_file)
        .cloned()
        .or_else(|| {
            definitions
                .iter()
                .find(|definition| definition.name == name)
                .cloned()
        })
}

fn definition_containing_line<'a>(
    definitions: &'a [SymbolDef],
    file: &Path,
    line: usize,
) -> Option<&'a SymbolDef> {
    definitions
        .iter()
        .filter(|definition| definition.file == file)
        .filter(|definition| definition.line <= line && line <= definition.end_line)
        .max_by_key(|definition| definition.line)
}

fn edge(direction: &'static str, definition: &SymbolDef, depth: usize) -> Edge {
    Edge {
        direction,
        symbol: definition.name.clone(),
        file: path_utils::to_relative_path(&definition.file.to_string_lossy()),
        line: definition.line,
        scope: definition.scope.clone(),
        depth,
    }
}

fn collect_called_names(
    tree: &Tree,
    source: &str,
    language: Language,
    start_line: usize,
    end_line: usize,
) -> HashSet<String> {
    let mut names = HashSet::new();
    collect_called_names_from_node(
        tree.root_node(),
        source,
        language,
        start_line.saturating_sub(1),
        end_line.saturating_sub(1),
        &mut names,
    );
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

fn collect_call_sites(
    tree: &Tree,
    source: &str,
    language: Language,
    symbol_name: &str,
) -> Vec<usize> {
    let mut lines = Vec::new();
    collect_call_sites_from_node(tree.root_node(), source, language, symbol_name, &mut lines);
    lines
}

fn collect_call_sites_from_node(
    node: Node<'_>,
    source: &str,
    language: Language,
    symbol_name: &str,
    lines: &mut Vec<usize>,
) {
    if is_call_node(node.kind(), language)
        && call_name(node, source).as_deref() == Some(symbol_name)
    {
        lines.push(node.start_position().row + 1);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_call_sites_from_node(child, source, language, symbol_name, lines);
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

fn edge_rows_with_budget(
    edges: &[Edge],
    symbol: &str,
    max_tokens: usize,
) -> Result<(String, bool), io::Error> {
    let bpe = cl100k_base()
        .map_err(|e| io::Error::other(format!("Failed to initialize tiktoken tokenizer: {e}")))?;
    let mut kept = edges.to_vec();
    let mut truncated = false;

    loop {
        let rows = edge_rows(&kept);
        let mut candidate = json!({
            "sym": symbol,
            "h": EDGE_HEADER,
            "edges": rows,
        });
        if truncated {
            candidate["@"] = json!({"t": true});
        }

        let candidate = serde_json::to_string(&candidate).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize result to JSON: {e}"),
            )
        })?;

        if bpe.encode_with_special_tokens(&candidate).len() <= max_tokens {
            return Ok((rows, truncated));
        }

        if kept.pop().is_none() {
            return Ok((String::new(), true));
        }
        truncated = true;
    }
}

fn edge_rows(edges: &[Edge]) -> String {
    edges
        .iter()
        .map(|edge| {
            let line = edge.line.to_string();
            let depth = edge.depth.to_string();
            format::format_row(&[
                edge.direction,
                &edge.symbol,
                &edge.file,
                &line,
                &edge.scope,
                &depth,
            ])
        })
        .collect::<Vec<_>>()
        .join("\n")
}
