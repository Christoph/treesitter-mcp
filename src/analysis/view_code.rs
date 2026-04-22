//! View Code Tool
//!
//! Token-efficient, single-file view with optional dependency type context.
//!
//! Breaking schema change (v1):
//! - `p`: relative path of the main file
//! - `h`: header string (pipe-delimited column names)
//! - `f`/`s`/`c`: newline-delimited row strings for functions/structs/classes
//! - `deps`: map of dependency file path -> newline-delimited type rows
//! - Optional meta is under `@` (e.g. `{ "t": true }` for truncated)

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use tiktoken_rs::cl100k_base;

use crate::analysis::dependencies::resolve_dependencies;
use crate::analysis::path_utils;
use crate::analysis::shape::{
    extract_enhanced_shape, EnhancedClassInfo, EnhancedFileShape, EnhancedFunctionInfo,
    EnhancedStructInfo, ImplBlockInfo, ImportInfo, InterfaceInfo, MethodInfo, PropertyInfo,
    TraitInfo,
};
use crate::common::budget;
use crate::common::budget::BudgetTracker;
use crate::common::format;
use crate::extraction::types::{
    extract_go_types, extract_python_types, extract_rust_types, extract_typescript_types,
    TypeDefinition,
};
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code};

#[derive(Debug, Clone, Copy, PartialEq)]
enum DetailLevel {
    Signatures,
    Full,
}

#[derive(Debug, Clone)]
struct DefinitionLocation {
    file: PathBuf,
    line: usize,
    column: usize,
}

impl DetailLevel {
    fn from_args(arguments: &Value) -> Self {
        // Back-compat: some tests call this tool with include_code.
        if let Some(detail) = arguments.get("detail").and_then(Value::as_str) {
            return match detail {
                "full" => DetailLevel::Full,
                _ => DetailLevel::Signatures,
            };
        }

        if let Some(include_code) = arguments.get("include_code").and_then(Value::as_bool) {
            return if include_code {
                DetailLevel::Full
            } else {
                DetailLevel::Signatures
            };
        }

        DetailLevel::Full
    }

    fn header(self) -> &'static str {
        match self {
            DetailLevel::Signatures => "name|line|sig",
            DetailLevel::Full => "name|line|sig|doc|code",
        }
    }

    fn include_code(self) -> bool {
        matches!(self, DetailLevel::Full)
    }

    fn trait_header(self) -> &'static str {
        match self {
            DetailLevel::Signatures => "trait|name|line|sig",
            DetailLevel::Full => "trait|name|line|sig|doc|code",
        }
    }

    fn class_method_header(self) -> &'static str {
        match self {
            DetailLevel::Signatures => "class|name|line|sig|ann",
            DetailLevel::Full => "class|name|line|sig|ann|doc|code",
        }
    }

    fn impl_method_header(self) -> &'static str {
        match self {
            DetailLevel::Signatures => "impl|trait|name|line|sig",
            DetailLevel::Full => "impl|trait|name|line|sig|doc|code",
        }
    }
}

pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path = arguments["file_path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'file_path' argument",
        )
    })?;

    let detail = DetailLevel::from_args(arguments);
    let focus_symbol = arguments.get("focus_symbol").and_then(Value::as_str);

    // Back-compat: tests pass include_deps without tool schema.
    let include_deps = arguments
        .get("include_deps")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let max_tokens = arguments
        .get("max_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(2000) as usize;

    let definition_location = arguments
        .get("definition_location")
        .map(parse_definition_location)
        .transpose()?;

    log::info!(
        "Viewing code: {file_path} (detail: {:?}, focus_symbol: {:?}, include_deps: {include_deps}, max_tokens: {max_tokens})",
        detail,
        focus_symbol
    );

    // Parse main file
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

    let mut main_shape = extract_enhanced_shape(
        &tree,
        &source,
        language,
        Some(file_path),
        detail.include_code(),
    )?;

    if let Some(symbol) = focus_symbol {
        apply_focus(&mut main_shape, symbol);
    }

    // Convert main file path to relative
    let main_path = path_utils::to_relative_path(file_path);

    let bpe = cl100k_base()
        .map_err(|e| io::Error::other(format!("Failed to initialize tiktoken tokenizer: {e}")))?;

    // Build output map (compact)
    let mut out = Map::new();
    out.insert("p".to_string(), json!(main_path));
    out.insert("h".to_string(), json!(detail.header()));

    insert_symbol_tables(&mut out, &main_shape, detail);
    insert_imports_and_traits(&mut out, &main_shape, detail);
    insert_interfaces_and_properties(&mut out, &main_shape, detail);
    insert_class_methods(&mut out, &main_shape, detail);
    insert_impl_methods(&mut out, &main_shape, detail);

    let mut truncated = false;

    if include_deps {
        let project_root = path_utils::find_project_root(Path::new(file_path))
            .or_else(|| Path::new(file_path).parent().map(|p| p.to_path_buf()))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "Could not determine project root or parent directory",
                )
            })?;

        let mut dep_paths =
            resolve_dependencies(language, &source, Path::new(file_path), &project_root);
        if let Some(location) = definition_location.as_ref() {
            push_unique_path(&mut dep_paths, location.file.clone());
        }
        let project_deps = filter_project_dependencies(dep_paths, &project_root);

        let referenced = if let Some(location) = definition_location.as_ref() {
            referenced_type_from_definition_location(location)?
        } else {
            extract_referenced_type_names(
                language,
                &source,
                &main_shape,
                Path::new(file_path),
                focus_symbol,
            )
        };

        let deps_obj = build_dependency_rows(
            &project_deps,
            Path::new(file_path),
            &referenced,
            detail,
            max_tokens,
        )?;

        if !deps_obj.is_empty() {
            out.insert("deps".to_string(), Value::Object(deps_obj));
        }
    }

    // Early estimate-based budget: only used to stop adding deps.
    // Hard enforcement below uses actual token count.
    loop {
        let json_text = serde_json::to_string(&Value::Object(out.clone())).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize view_code output to JSON: {e}"),
            )
        })?;

        if bpe.encode_with_special_tokens(&json_text).len() <= max_tokens {
            break;
        }

        if remove_last_dep_entry(&mut out) {
            truncated = true;
            continue;
        }

        if !shrink_symbol_tables(&mut out) {
            truncated = true;
            break;
        }

        truncated = true;
    }

    if truncated {
        out.insert("@".to_string(), json!({"t": true}));
    }

    let output_json = serde_json::to_string(&Value::Object(out)).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize output to JSON: {e}"),
        )
    })?;

    Ok(CallToolResult::success(output_json))
}

fn parse_definition_location(value: &Value) -> Result<DefinitionLocation, io::Error> {
    let file = value
        .get("file")
        .or_else(|| value.get("file_path"))
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| {
            value
                .get("uri")
                .and_then(Value::as_str)
                .and_then(file_path_from_uri)
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Definition location is missing 'file', 'file_path', or 'uri'",
            )
        })?;

    if let Some(start) = value
        .get("range")
        .and_then(|range| range.get("start"))
        .and_then(Value::as_object)
    {
        let line = start.get("line").and_then(Value::as_u64).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "LSP range is missing start.line",
            )
        })? as usize
            + 1;
        let column = start
            .get("character")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "LSP range is missing start.character",
                )
            })? as usize
            + 1;

        return Ok(DefinitionLocation { file, line, column });
    }

    let line = value.get("line").and_then(Value::as_u64).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Definition location is missing 1-based 'line'",
        )
    })? as usize;

    let column = value
        .get("col")
        .or_else(|| value.get("column"))
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Definition location is missing 1-based 'col' or 'column'",
            )
        })? as usize;

    Ok(DefinitionLocation { file, line, column })
}

fn file_path_from_uri(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let path = rest.strip_prefix("localhost").unwrap_or(rest);
    Some(PathBuf::from(percent_decode(path)))
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut output = String::with_capacity(input.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3]) {
                if let Ok(value) = u8::from_str_radix(hex, 16) {
                    output.push(value as char);
                    index += 3;
                    continue;
                }
            }
        }

        output.push(bytes[index] as char);
        index += 1;
    }

    output
}

fn insert_symbol_tables(
    out: &mut Map<String, Value>,
    shape: &EnhancedFileShape,
    detail: DetailLevel,
) {
    let functions = functions_to_rows(&shape.functions, detail);
    if !functions.is_empty() {
        out.insert("f".to_string(), json!(functions));
    }

    let structs = structs_to_rows(&shape.structs, detail);
    if !structs.is_empty() {
        out.insert("s".to_string(), json!(structs));
    }

    let classes = classes_to_rows(&shape.classes, detail);
    if !classes.is_empty() {
        out.insert("c".to_string(), json!(classes));
    }
}

fn insert_imports_and_traits(
    out: &mut Map<String, Value>,
    shape: &EnhancedFileShape,
    detail: DetailLevel,
) {
    let imports = imports_to_rows(&shape.imports);
    if !imports.is_empty() {
        out.insert("ih".to_string(), json!("line|text"));
        out.insert("im".to_string(), json!(imports));
    }

    let trait_methods = trait_methods_to_rows(&shape.traits, detail);
    if !trait_methods.is_empty() {
        out.insert("th".to_string(), json!(detail.trait_header()));
        out.insert("tm".to_string(), json!(trait_methods));
    }
}

fn insert_interfaces_and_properties(
    out: &mut Map<String, Value>,
    shape: &EnhancedFileShape,
    detail: DetailLevel,
) {
    let interfaces = interfaces_to_rows(&shape.interfaces, detail);
    if !interfaces.is_empty() {
        out.insert("ah".to_string(), json!(detail.header()));
        out.insert("i".to_string(), json!(interfaces));
    }

    let properties = properties_to_rows(&shape.properties);
    if !properties.is_empty() {
        out.insert("ph".to_string(), json!("name|line|type|doc"));
        out.insert("pr".to_string(), json!(properties));
    }

    let implements = class_implements_to_rows(&shape.classes);
    if !implements.is_empty() {
        out.insert("ch".to_string(), json!("class|iface"));
        out.insert("ci".to_string(), json!(implements));
    }
}

fn insert_class_methods(
    out: &mut Map<String, Value>,
    shape: &EnhancedFileShape,
    detail: DetailLevel,
) {
    let rows = class_methods_to_rows(&shape.classes, detail);
    if rows.is_empty() {
        return;
    }

    out.insert("mh".to_string(), json!(detail.class_method_header()));
    out.insert("cm".to_string(), json!(rows));
}

fn insert_impl_methods(
    out: &mut Map<String, Value>,
    shape: &EnhancedFileShape,
    detail: DetailLevel,
) {
    let rows = impl_methods_to_rows(&shape.impl_blocks, detail);
    if rows.is_empty() {
        return;
    }

    out.insert("bh".to_string(), json!(detail.impl_method_header()));
    out.insert("bm".to_string(), json!(rows));
}

fn functions_to_rows(functions: &[EnhancedFunctionInfo], detail: DetailLevel) -> String {
    functions
        .iter()
        .map(|func| {
            let line = func.line.to_string();

            let mut fields: Vec<String> = Vec::new();
            fields.push(func.name.clone());
            fields.push(line);
            fields.push(func.signature.clone());

            if detail == DetailLevel::Full {
                fields.push(func.doc.clone().unwrap_or_default());
                fields.push(func.code.clone().unwrap_or_default());
            }

            let refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
            format::format_row(&refs)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn structs_to_rows(structs: &[EnhancedStructInfo], detail: DetailLevel) -> String {
    structs
        .iter()
        .map(|s| {
            let mut fields: Vec<String> = Vec::new();
            fields.push(s.name.clone());
            fields.push(s.line.to_string());

            let sig = signature_snippet_from_code(s.code.as_deref());
            fields.push(sig);

            if detail == DetailLevel::Full {
                fields.push(s.doc.clone().unwrap_or_default());
                fields.push(s.code.clone().unwrap_or_default());
            }

            let refs: Vec<&str> = fields.iter().map(|v| v.as_str()).collect();
            format::format_row(&refs)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn classes_to_rows(classes: &[EnhancedClassInfo], detail: DetailLevel) -> String {
    classes
        .iter()
        .map(|c| {
            let mut fields: Vec<String> = Vec::new();
            fields.push(c.name.clone());
            fields.push(c.line.to_string());

            let sig = signature_snippet_from_code(c.code.as_deref());
            fields.push(sig);

            if detail == DetailLevel::Full {
                fields.push(c.doc.clone().unwrap_or_default());
                fields.push(c.code.clone().unwrap_or_default());
            }

            let refs: Vec<&str> = fields.iter().map(|v| v.as_str()).collect();
            format::format_row(&refs)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn imports_to_rows(imports: &[ImportInfo]) -> String {
    imports
        .iter()
        .map(|import| {
            let line = import.line.to_string();
            let fields = [line.as_str(), import.text.as_str()];
            format::format_row(&fields)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn interfaces_to_rows(interfaces: &[InterfaceInfo], detail: DetailLevel) -> String {
    interfaces
        .iter()
        .map(|interface| {
            let mut fields: Vec<String> = Vec::new();
            fields.push(interface.name.clone());
            fields.push(interface.line.to_string());

            let sig = signature_snippet_from_code(interface.code.as_deref());
            fields.push(sig);

            if detail == DetailLevel::Full {
                fields.push(interface.doc.clone().unwrap_or_default());
                fields.push(interface.code.clone().unwrap_or_default());
            }

            let refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
            format::format_row(&refs)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn properties_to_rows(properties: &[PropertyInfo]) -> String {
    properties
        .iter()
        .map(|prop| {
            let line = prop.line.to_string();
            let prop_type = prop.property_type.clone().unwrap_or_default();
            let doc = prop.doc.clone().unwrap_or_default();

            let owned = [prop.name.clone(), line, prop_type, doc];
            let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
            format::format_row(&refs)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn class_implements_to_rows(classes: &[EnhancedClassInfo]) -> String {
    let mut rows = Vec::new();

    for class in classes {
        for iface in &class.implements {
            let owned = [class.name.clone(), iface.clone()];
            let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
            rows.push(format::format_row(&refs));
        }
    }

    rows.join("\n")
}

fn trait_methods_to_rows(traits: &[TraitInfo], detail: DetailLevel) -> String {
    let mut rows = Vec::new();

    for tr in traits {
        for method in &tr.methods {
            rows.push(trait_method_to_row(tr.name.as_str(), method, detail));
        }
    }

    rows.join("\n")
}

fn class_methods_to_rows(classes: &[EnhancedClassInfo], detail: DetailLevel) -> String {
    let mut rows = Vec::new();

    for class in classes {
        for method in &class.methods {
            rows.push(class_method_to_row(class.name.as_str(), method, detail));
        }
    }

    rows.join("\n")
}

fn class_method_to_row(
    class_name: &str,
    method: &EnhancedFunctionInfo,
    detail: DetailLevel,
) -> String {
    let line = method.line.to_string();
    let ann = if method.annotations.is_empty() {
        String::new()
    } else {
        method.annotations.join(",")
    };

    if detail == DetailLevel::Full {
        let owned = [
            class_name.to_string(),
            method.name.clone(),
            line,
            method.signature.clone(),
            ann,
            method.doc.clone().unwrap_or_default(),
            method.code.clone().unwrap_or_default(),
        ];
        let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
        return format::format_row(&refs);
    }

    let owned = [
        class_name.to_string(),
        method.name.clone(),
        line,
        method.signature.clone(),
        ann,
    ];
    let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    format::format_row(&refs)
}

fn impl_methods_to_rows(impl_blocks: &[ImplBlockInfo], detail: DetailLevel) -> String {
    let mut rows = Vec::new();

    for block in impl_blocks {
        for method in &block.methods {
            rows.push(impl_method_to_row(block, method, detail));
        }
    }

    rows.join("\n")
}

fn impl_method_to_row(block: &ImplBlockInfo, method: &MethodInfo, detail: DetailLevel) -> String {
    let line = method.line.to_string();
    let trait_name = block.trait_name.clone().unwrap_or_default();

    if detail == DetailLevel::Full {
        let owned = [
            block.type_name.clone(),
            trait_name,
            method.name.clone(),
            line,
            method.signature.clone(),
            method.doc.clone().unwrap_or_default(),
            method.code.clone().unwrap_or_default(),
        ];
        let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
        return format::format_row(&refs);
    }

    let owned = [
        block.type_name.clone(),
        trait_name,
        method.name.clone(),
        line,
        method.signature.clone(),
    ];
    let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    format::format_row(&refs)
}

fn trait_method_to_row(trait_name: &str, method: &MethodInfo, detail: DetailLevel) -> String {
    let line = method.line.to_string();

    if detail == DetailLevel::Full {
        let owned = [
            trait_name.to_string(),
            method.name.clone(),
            line,
            method.signature.clone(),
            method.doc.clone().unwrap_or_default(),
            method.code.clone().unwrap_or_default(),
        ];
        let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
        return format::format_row(&refs);
    }

    let owned = [
        trait_name.to_string(),
        method.name.clone(),
        line,
        method.signature.clone(),
    ];
    let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    format::format_row(&refs)
}

fn signature_snippet_from_code(code: Option<&str>) -> String {
    let Some(code) = code else {
        return String::new();
    };

    code.lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .unwrap_or_default()
}

fn extract_referenced_type_names(
    language: crate::parser::Language,
    source: &str,
    main_shape: &EnhancedFileShape,
    file_path: &Path,
    focus_symbol: Option<&str>,
) -> HashSet<String> {
    match language {
        crate::parser::Language::Rust
        | crate::parser::Language::TypeScript
        | crate::parser::Language::Python
        | crate::parser::Language::Go => {
            extract_ast_position_type_names(language, source, main_shape, file_path, focus_symbol)
        }
        _ => extract_referenced_type_names_fallback(source, main_shape),
    }
}

fn referenced_type_from_definition_location(
    location: &DefinitionLocation,
) -> Result<HashSet<String>, io::Error> {
    let rows = extract_dependency_types(&location.file, &HashSet::new(), DetailLevel::Signatures)?;
    let mut referenced = HashSet::new();

    if let Some(row) = rows.iter().find(|row| row.line == location.line) {
        log::debug!(
            "Resolved definition location {}:{}:{} to type {}",
            location.file.display(),
            location.line,
            location.column,
            row.name
        );
        referenced.insert(row.name.clone());
    }

    Ok(referenced)
}

fn extract_ast_position_type_names(
    language: crate::parser::Language,
    source: &str,
    main_shape: &EnhancedFileShape,
    file_path: &Path,
    focus_symbol: Option<&str>,
) -> HashSet<String> {
    let mut names = HashSet::new();
    collect_shape_type_references(main_shape, focus_symbol, &mut names);

    let local_types = extract_local_type_definitions(language, source, file_path);
    if let Some(type_defs) = local_types.as_deref() {
        collect_type_definition_references(type_defs, focus_symbol, &mut names);
        remove_local_type_names(main_shape, Some(type_defs), &mut names);
    } else {
        remove_local_type_names(main_shape, None, &mut names);
    }

    names
}

fn extract_local_type_definitions(
    language: crate::parser::Language,
    source: &str,
    file_path: &Path,
) -> Option<Vec<TypeDefinition>> {
    match language {
        crate::parser::Language::Rust => extract_rust_types(source, file_path).ok(),
        crate::parser::Language::TypeScript => {
            extract_typescript_types(source, file_path, true).ok()
        }
        crate::parser::Language::Python => extract_python_types(source, file_path).ok(),
        crate::parser::Language::Go => extract_go_types(source, file_path).ok(),
        _ => None,
    }
}

fn collect_shape_type_references(
    shape: &EnhancedFileShape,
    focus_symbol: Option<&str>,
    out: &mut HashSet<String>,
) {
    if focus_symbol.is_none() {
        for import in &shape.imports {
            collect_type_like_tokens(&import.text, out);
        }
    }

    for func in &shape.functions {
        if matches_focus(&func.name, focus_symbol) {
            collect_type_like_tokens(&func.signature, out);
        }
    }

    for class in &shape.classes {
        let class_selected = matches_focus(&class.name, focus_symbol);

        if class_selected {
            for implemented in &class.implements {
                collect_type_like_tokens(implemented, out);
            }

            for property in class.properties.iter().chain(class.fields.iter()) {
                if let Some(property_type) = property.property_type.as_deref() {
                    collect_type_like_tokens(property_type, out);
                }
            }
        }

        for method in &class.methods {
            if focus_symbol.is_none() || class_selected || matches_focus(&method.name, focus_symbol)
            {
                collect_type_like_tokens(&method.signature, out);
            }
        }
    }

    for interface in &shape.interfaces {
        let interface_selected = matches_focus(&interface.name, focus_symbol);

        if interface_selected {
            for property in &interface.properties {
                if let Some(property_type) = property.property_type.as_deref() {
                    collect_type_like_tokens(property_type, out);
                }
            }
        }

        for method in &interface.methods {
            if focus_symbol.is_none()
                || interface_selected
                || matches_focus(&method.name, focus_symbol)
            {
                collect_type_like_tokens(&method.signature, out);
            }
        }
    }

    for tr in &shape.traits {
        let trait_selected = matches_focus(&tr.name, focus_symbol);
        for method in &tr.methods {
            if focus_symbol.is_none() || trait_selected || matches_focus(&method.name, focus_symbol)
            {
                collect_type_like_tokens(&method.signature, out);
            }
        }
    }

    for block in &shape.impl_blocks {
        let impl_selected = matches_focus(&block.type_name, focus_symbol)
            || block
                .trait_name
                .as_deref()
                .is_some_and(|trait_name| matches_focus(trait_name, focus_symbol));

        if impl_selected {
            collect_type_like_tokens(&block.type_name, out);
            if let Some(trait_name) = block.trait_name.as_deref() {
                collect_type_like_tokens(trait_name, out);
            }
        }

        for method in &block.methods {
            if focus_symbol.is_none() || impl_selected || matches_focus(&method.name, focus_symbol)
            {
                collect_type_like_tokens(&method.signature, out);
            }
        }
    }
}

fn collect_type_definition_references(
    type_defs: &[TypeDefinition],
    focus_symbol: Option<&str>,
    out: &mut HashSet<String>,
) {
    for ty in type_defs {
        if !matches_focus(&ty.name, focus_symbol) {
            continue;
        }

        collect_type_like_tokens(&ty.signature, out);

        if let Some(fields) = ty.fields.as_deref() {
            for field in fields {
                collect_type_like_tokens(&field.type_annotation, out);
            }
        }

        if let Some(members) = ty.members.as_deref() {
            for member in members {
                collect_type_like_tokens(&member.type_annotation, out);
            }
        }

        if let Some(variants) = ty.variants.as_deref() {
            for variant in variants {
                if let Some(type_annotation) = variant.type_annotation.as_deref() {
                    collect_type_like_tokens(type_annotation, out);
                }
            }
        }
    }
}

fn remove_local_type_names(
    shape: &EnhancedFileShape,
    local_types: Option<&[TypeDefinition]>,
    out: &mut HashSet<String>,
) {
    for s in &shape.structs {
        out.remove(&s.name);
    }
    for c in &shape.classes {
        out.remove(&c.name);
    }
    for i in &shape.interfaces {
        out.remove(&i.name);
    }
    for tr in &shape.traits {
        out.remove(&tr.name);
    }

    if let Some(type_defs) = local_types {
        for ty in type_defs {
            out.remove(&ty.name);
        }
    }
}

fn matches_focus(name: &str, focus_symbol: Option<&str>) -> bool {
    focus_symbol.is_none_or(|focus| focus == name)
}

fn extract_referenced_type_names_fallback(
    source: &str,
    main_shape: &EnhancedFileShape,
) -> HashSet<String> {
    let mut names = HashSet::new();

    for func in &main_shape.functions {
        collect_type_like_tokens(&func.signature, &mut names);
    }

    collect_type_like_tokens(source, &mut names);
    remove_local_type_names(main_shape, None, &mut names);

    names
}

fn collect_type_like_tokens(text: &str, out: &mut HashSet<String>) {
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch);
            continue;
        }

        flush_type_token(&mut current, out);
    }

    flush_type_token(&mut current, out);
}

fn flush_type_token(token: &mut String, out: &mut HashSet<String>) {
    if token.is_empty() {
        return;
    }

    if token
        .chars()
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false)
    {
        // Heuristic: ignore single-letter generics like `T`.
        if token.len() > 1 {
            out.insert(token.clone());
        }
    }

    token.clear();
}

fn build_dependency_rows(
    dep_paths: &[PathBuf],
    main_file: &Path,
    referenced: &HashSet<String>,
    detail: DetailLevel,
    max_tokens: usize,
) -> Result<Map<String, Value>, io::Error> {
    if dep_paths.is_empty() {
        return Ok(Map::new());
    }

    let bpe = cl100k_base()
        .map_err(|e| io::Error::other(format!("Failed to initialize tiktoken tokenizer: {e}")))?;

    // 10% buffer: estimates should err on the safe side.
    let mut budget_tracker = BudgetTracker::new((max_tokens * 9) / 10);

    // Mark main file as visited.
    let mut visited = HashSet::new();
    if let Ok(canonical) = fs::canonicalize(main_file) {
        visited.insert(canonical);
    }

    let mut deps: Map<String, Value> = Map::new();

    // Preserve dependency ordering: HashMap iteration order is randomized and would
    // make output flaky (tests and clients expect stable selection).
    let mut dep_type_candidates: Vec<(String, Vec<TypeRow>)> = Vec::new();

    for dep_path in dep_paths {
        let canonical = match fs::canonicalize(dep_path) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("Failed to canonicalize {}: {}", dep_path.display(), e);
                continue;
            }
        };

        if visited.contains(&canonical) {
            continue;
        }
        visited.insert(canonical);

        let dep_rows = extract_dependency_types(dep_path, referenced, detail)?;
        if dep_rows.is_empty() {
            continue;
        }

        let rel = path_utils::to_relative_path(dep_path.to_string_lossy().as_ref());
        dep_type_candidates.push((rel, dep_rows));
    }

    // Only return explicitly referenced dependency types.
    for (dep_path, rows) in &dep_type_candidates {
        let selected: Vec<&TypeRow> = rows.iter().filter(|row| row.referenced).collect();
        if selected.is_empty() {
            continue;
        }

        let rows_str = type_rows_to_string(&selected, detail);
        if rows_str.is_empty() {
            continue;
        }

        let estimated = budget::estimate_symbol_tokens(rows_str.len() + dep_path.len() + 16);
        if !budget_tracker.add(estimated) {
            break;
        }

        deps.insert(dep_path.clone(), json!(rows_str));
    }

    // Hard enforcement: drop deps until within token budget.
    loop {
        if deps.is_empty() {
            break;
        }

        let snapshot = serde_json::to_string(&json!({"deps": deps.clone()})).unwrap_or_default();
        if bpe.encode_with_special_tokens(&snapshot).len() <= max_tokens {
            break;
        }

        // Remove last inserted key (Map iteration order is insertion order).
        let Some(last_key) = deps.keys().next_back().cloned() else {
            break;
        };
        deps.remove(&last_key);
    }

    Ok(deps)
}
#[derive(Debug, Clone)]
struct TypeRow {
    name: String,
    line: usize,
    signature: Option<String>,
    doc: Option<String>,
    code: Option<String>,
    referenced: bool,
}

fn extract_dependency_types(
    file_path: &Path,
    referenced: &HashSet<String>,
    detail: DetailLevel,
) -> Result<Vec<TypeRow>, io::Error> {
    let source = fs::read_to_string(file_path)?;
    let language = detect_language(file_path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            format!("Cannot detect language: {e}"),
        )
    })?;

    let tree = parse_code(&source, language).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse code: {e}"),
        )
    })?;

    // Always parse deps with code: signatures mode still needs a signature snippet.
    let shape = extract_enhanced_shape(
        &tree,
        &source,
        language,
        Some(file_path.to_str().unwrap_or("unknown")),
        true,
    )?;
    let type_signatures: HashMap<String, String> =
        extract_local_type_definitions(language, &source, file_path)
            .unwrap_or_default()
            .into_iter()
            .map(|ty| (ty.name, ty.signature))
            .collect();

    let mut rows = Vec::new();

    for s in shape.structs {
        rows.push(TypeRow {
            referenced: referenced.contains(&s.name),
            signature: type_signatures.get(&s.name).cloned(),
            name: s.name,
            line: s.line,
            doc: if detail == DetailLevel::Full {
                s.doc
            } else {
                None
            },
            code: s.code,
        });
    }

    for c in shape.classes {
        rows.push(TypeRow {
            referenced: referenced.contains(&c.name),
            signature: type_signatures.get(&c.name).cloned(),
            name: c.name,
            line: c.line,
            doc: if detail == DetailLevel::Full {
                c.doc
            } else {
                None
            },
            code: c.code,
        });
    }

    for i in shape.interfaces {
        rows.push(TypeRow {
            referenced: referenced.contains(&i.name),
            signature: type_signatures.get(&i.name).cloned(),
            name: i.name,
            line: i.line,
            doc: if detail == DetailLevel::Full {
                i.doc
            } else {
                None
            },
            code: i.code,
        });
    }

    for tr in shape.traits {
        rows.push(TypeRow {
            referenced: referenced.contains(&tr.name),
            signature: type_signatures.get(&tr.name).cloned(),
            name: tr.name,
            line: tr.line,
            doc: if detail == DetailLevel::Full {
                tr.doc
            } else {
                None
            },
            code: None,
        });
    }

    // Stable ordering: file order.
    rows.sort_by_key(|row| row.line);

    Ok(rows)
}

fn type_rows_to_string(rows: &[&TypeRow], detail: DetailLevel) -> String {
    rows.iter()
        .map(|row| {
            let line = row.line.to_string();
            let sig = row
                .signature
                .clone()
                .unwrap_or_else(|| signature_snippet_from_code(row.code.as_deref()));

            if detail == DetailLevel::Full {
                let owned = [
                    row.name.clone(),
                    line,
                    sig,
                    row.doc.clone().unwrap_or_default(),
                    row.code.clone().unwrap_or_default(),
                ];
                let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
                return format::format_row(&refs);
            }

            let owned = [row.name.clone(), line, sig];
            let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
            format::format_row(&refs)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Filter dependencies to only include project files, not external libraries
fn filter_project_dependencies(dep_paths: Vec<PathBuf>, project_root: &Path) -> Vec<PathBuf> {
    dep_paths
        .into_iter()
        .filter(|path| {
            // Include if path is inside project_root
            path.starts_with(project_root)
                // Exclude external dependency directories
                && !path.to_string_lossy().contains("/target/")
                && !path.to_string_lossy().contains("/node_modules/")
                && !path.to_string_lossy().contains("/venv/")
                && !path.to_string_lossy().contains("/.venv/")
                && !path.to_string_lossy().contains("/site-packages/")
                && !path.to_string_lossy().contains("\\target\\")
                && !path.to_string_lossy().contains("\\node_modules\\")
                && !path.to_string_lossy().contains("\\venv\\")
                && !path.to_string_lossy().contains("\\.venv\\")
                && !path.to_string_lossy().contains("\\site-packages\\")
        })
        .collect()
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if paths.iter().any(|existing| existing == &path) {
        return;
    }

    paths.push(path);
}

/// Apply focus to show full code only for the specified symbol
fn apply_focus(shape: &mut EnhancedFileShape, focus_symbol: &str) {
    let mut found = false;

    for func in &mut shape.functions {
        if func.name == focus_symbol {
            found = true;
        } else {
            func.code = None;
        }
    }

    for struct_info in &mut shape.structs {
        if struct_info.name == focus_symbol {
            found = true;
        } else {
            struct_info.code = None;
        }
    }

    for class in &mut shape.classes {
        if class.name == focus_symbol {
            found = true;
        } else {
            class.code = None;
            for method in &mut class.methods {
                method.code = None;
            }
        }
    }

    for tr in &mut shape.traits {
        for method in &mut tr.methods {
            if method.name == focus_symbol {
                found = true;
            } else {
                method.code = None;
            }
        }
    }

    for block in &mut shape.impl_blocks {
        for method in &mut block.methods {
            if method.name == focus_symbol {
                found = true;
            } else {
                method.code = None;
            }
        }
    }

    if !found {
        log::warn!("Focus symbol '{}' not found in file", focus_symbol);
    }
}

fn remove_last_dep_entry(out: &mut Map<String, Value>) -> bool {
    let Some(deps_value) = out.get_mut("deps") else {
        return false;
    };

    let Some(deps_obj) = deps_value.as_object_mut() else {
        out.remove("deps");
        return true;
    };

    let Some(last_key) = deps_obj.keys().next_back().cloned() else {
        out.remove("deps");
        return true;
    };

    deps_obj.remove(&last_key);
    if deps_obj.is_empty() {
        out.remove("deps");
    }

    true
}

fn shrink_symbol_tables(out: &mut Map<String, Value>) -> bool {
    // Prefer removing rows from the largest table first.
    let mut candidates: Vec<(&str, usize)> = Vec::new();
    for key in ["f", "s", "c", "im", "tm", "i", "pr", "ci", "cm", "bm"] {
        if let Some(rows) = out.get(key).and_then(Value::as_str) {
            let count = if rows.is_empty() {
                0
            } else {
                rows.lines().count()
            };
            candidates.push((key, count));
        }
    }

    candidates.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    let Some((key, _)) = candidates.first().copied() else {
        return false;
    };

    let Some(rows) = out.get(key).and_then(Value::as_str) else {
        return false;
    };

    if rows.is_empty() {
        remove_table_and_header(out, key);
        return true;
    }

    let mut lines: Vec<&str> = rows.lines().collect();
    if lines.pop().is_none() {
        remove_table_and_header(out, key);
        return true;
    }

    let new_rows = lines.join("\n");
    if new_rows.is_empty() {
        remove_table_and_header(out, key);
    } else {
        out.insert(key.to_string(), json!(new_rows));
    }

    true
}

fn remove_table_and_header(out: &mut Map<String, Value>, key: &str) {
    out.remove(key);

    let header_key = match key {
        "im" => Some("ih"),
        "tm" => Some("th"),
        "i" => Some("ah"),
        "pr" => Some("ph"),
        "ci" => Some("ch"),
        "cm" => Some("mh"),
        "bm" => Some("bh"),
        _ => None,
    };

    if let Some(header_key) = header_key {
        out.remove(header_key);
    }
}
