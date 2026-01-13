//! Code Map Tool
//!
//! Generates a compact, token-efficient overview of a codebase.
//!
//! Breaking schema change (v1):
//! - Top-level JSON object is keyed by *relative* file path.
//! - Each file maps to an object with abbreviated keys:
//!   - `h`: header string (pipe-delimited column names)
//!   - `f`: functions (newline-delimited rows)
//!   - `s`: structs (newline-delimited rows)
//!   - `c`: classes (newline-delimited rows)
//! - Optional meta is under `@` (e.g. `{ "t": true }` for truncated).
//! - When `with_types=true`, also includes `types` key with type definitions.

use std::cmp::Reverse;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use globset::Glob;
use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use tiktoken_rs::cl100k_base;

use crate::analysis::path_utils;
use crate::analysis::shape::{EnhancedClassInfo, EnhancedFunctionInfo, EnhancedStructInfo};
use crate::analysis::usage_counter::{count_words_in_content, language_for_path};
use crate::common::budget;
use crate::common::budget::BudgetTracker;
use crate::common::format;
use crate::extraction::types::{TypeDefinition, TypeKind};
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::detect_language;

const IGNORE_DIRS: &[&str] = &["target", "node_modules", "dist", "build"];

#[derive(Debug, Clone, Copy, PartialEq)]
enum DetailLevel {
    Minimal,
    Signatures,
    Full,
}

impl DetailLevel {
    fn from_str(s: &str) -> Self {
        match s {
            "minimal" => DetailLevel::Minimal,
            "signatures" => DetailLevel::Signatures,
            "full" => DetailLevel::Full,
            _ => DetailLevel::Signatures,
        }
    }

    fn header(self) -> &'static str {
        match self {
            DetailLevel::Minimal => "name|line",
            DetailLevel::Signatures => "name|line|sig",
            DetailLevel::Full => "name|line|sig|doc|code",
        }
    }
}

#[derive(Debug, Clone)]
struct FileSymbols {
    path: String,
    functions: Vec<Value>,
    structs: Vec<Value>,
    classes: Vec<Value>,
}

/// Options for combined code map and type extraction
#[derive(Debug, Clone, Copy)]
struct ExtractionOptions {
    detail_level: DetailLevel,
    with_types: bool,
    count_usages: bool,
}

/// Result of combined extraction
struct ExtractionResult {
    files: Vec<FileSymbols>,
    types: Vec<TypeDefinition>,
    word_counts: HashMap<String, usize>,
}

pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let path_str = arguments["path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'path' argument",
        )
    })?;

    let max_tokens = arguments["max_tokens"].as_i64().unwrap_or(2000) as usize;
    let detail_str = arguments["detail"].as_str().unwrap_or("signatures");
    let detail_level = DetailLevel::from_str(detail_str);
    let pattern = arguments["pattern"].as_str();
    let with_types = arguments["with_types"].as_bool().unwrap_or(false);
    let count_usages = arguments["count_usages"].as_bool().unwrap_or(false);

    log::info!(
        "Generating compact code map for: {path_str} (max_tokens: {max_tokens}, detail: {detail_str}, with_types: {with_types})"
    );

    let path = Path::new(path_str);
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Path does not exist: {path_str}"),
        ));
    }

    let options = ExtractionOptions {
        detail_level,
        with_types,
        count_usages,
    };

    let mut result = ExtractionResult {
        files: Vec::new(),
        types: Vec::new(),
        word_counts: HashMap::new(),
    };

    if path.is_file() {
        if let Ok(entry) = process_file_combined(path, &options, &mut result) {
            result.files.push(entry);
        }
    } else if path.is_dir() {
        collect_files_combined(path, &mut result, &options, pattern)?;
        result.files.sort_by_key(|entry| Reverse(symbol_count(entry)));
    }

    // Apply usage counts to types if requested
    if with_types && count_usages {
        apply_usage_counts(&mut result.types, &result.word_counts);
        // Sort types by usage count
        result.types.sort_by(|a, b| {
            b.usage_count
                .cmp(&a.usage_count)
                .then_with(|| a.name.cmp(&b.name))
        });
    } else if with_types {
        // Sort by file then line when not counting usages
        result.types.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then_with(|| a.line.cmp(&b.line))
        });
    }

    // Convert all file paths to relative paths
    for entry in &mut result.files {
        entry.path = path_utils::to_relative_path(&entry.path);
    }

    let (result_map, _truncated) = build_compact_output_combined(&result, detail_level, max_tokens, with_types)?;

    let json_text = serde_json::to_string(&Value::Object(result_map)).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize code map to JSON: {e}"),
        )
    })?;

    Ok(CallToolResult::success(json_text))
}

fn apply_usage_counts(types: &mut [TypeDefinition], word_counts: &HashMap<String, usize>) {
    let mut definition_counts: HashMap<String, usize> = HashMap::new();
    for ty in types.iter() {
        *definition_counts.entry(ty.name.clone()).or_insert(0) += 1;
    }

    for ty in types.iter_mut() {
        if let Some(&count) = word_counts.get(&ty.name) {
            let def_count = definition_counts.get(&ty.name).copied().unwrap_or(1);
            ty.usage_count = count.saturating_sub(def_count);
        }
    }
}

fn build_compact_output(
    files: &[FileSymbols],
    detail_level: DetailLevel,
    max_tokens: usize,
) -> Result<(Map<String, Value>, bool), io::Error> {
    let bpe = cl100k_base()
        .map_err(|e| io::Error::other(format!("Failed to initialize tiktoken tokenizer: {e}")))?;

    let mut output = Map::new();
    let mut ordered_files: Vec<String> = Vec::new();

    // 10% buffer: estimates should err on the safe side.
    let mut budget_tracker = BudgetTracker::new((max_tokens * 9) / 10);

    for file in files {
        let file_value = build_compact_file(file, detail_level);
        let file_json = serde_json::to_string(&file_value).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize code map file entry: {e}"),
            )
        })?;

        let estimated = budget::estimate_symbol_tokens(file_json.len());
        if !budget_tracker.add(estimated) {
            break;
        }

        output.insert(file.path.clone(), file_value);
        ordered_files.push(file.path.clone());
    }

    let mut truncated = ordered_files.len() < files.len();

    // If budget is extremely small, still return at least one file.
    if output.is_empty() && !files.is_empty() {
        let first = &files[0];
        output.insert(first.path.clone(), build_compact_file(first, detail_level));
        ordered_files.push(first.path.clone());
        truncated = true;
    }

    // Hard enforcement with real token counts:
    // - drop least-important files first
    // - if only one file remains, progressively drop rows until it fits
    loop {
        let json_text = serde_json::to_string(&Value::Object(output.clone())).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize code map to JSON: {e}"),
            )
        })?;

        if bpe.encode_with_special_tokens(&json_text).len() <= max_tokens {
            break;
        }

        if ordered_files.len() > 1 {
            let Some(last_path) = ordered_files.pop() else {
                break;
            };
            output.remove(&last_path);
            truncated = true;
            continue;
        }

        let Some(only_path) = ordered_files.first().cloned() else {
            break;
        };

        let Some(file_value) = output.get_mut(&only_path) else {
            break;
        };

        if !shrink_single_file_to_fit(file_value, &bpe, max_tokens) {
            // If we cannot shrink further, leave header-only.
            truncated = true;
            break;
        }

        truncated = true;
    }

    if truncated {
        output.insert("@".to_string(), json!({"t": true}));
    }

    Ok((output, truncated))
}

fn build_compact_output_combined(
    result: &ExtractionResult,
    detail_level: DetailLevel,
    max_tokens: usize,
    with_types: bool,
) -> Result<(Map<String, Value>, bool), io::Error> {
    let bpe = cl100k_base()
        .map_err(|e| io::Error::other(format!("Failed to initialize tiktoken tokenizer: {e}")))?;

    let mut output = Map::new();
    let mut ordered_files: Vec<String> = Vec::new();

    // Reserve some budget for types if needed
    let types_budget = if with_types && !result.types.is_empty() {
        max_tokens / 4 // Reserve 25% for types
    } else {
        0
    };
    let files_budget = max_tokens - types_budget;

    // 10% buffer for files
    let mut budget_tracker = BudgetTracker::new((files_budget * 9) / 10);

    for file in &result.files {
        let file_value = build_compact_file(file, detail_level);
        let file_json = serde_json::to_string(&file_value).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize code map file entry: {e}"),
            )
        })?;

        let estimated = budget::estimate_symbol_tokens(file_json.len());
        if !budget_tracker.add(estimated) {
            break;
        }

        output.insert(file.path.clone(), file_value);
        ordered_files.push(file.path.clone());
    }

    let mut truncated = ordered_files.len() < result.files.len();

    // If budget is extremely small, still return at least one file.
    if output.is_empty() && !result.files.is_empty() {
        let first = &result.files[0];
        output.insert(first.path.clone(), build_compact_file(first, detail_level));
        ordered_files.push(first.path.clone());
        truncated = true;
    }

    // Add types if requested
    if with_types && !result.types.is_empty() {
        let types_output = build_types_output(&result.types, types_budget);
        output.insert("types".to_string(), types_output);
    }

    // Hard enforcement with real token counts
    loop {
        let json_text = serde_json::to_string(&Value::Object(output.clone())).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize code map to JSON: {e}"),
            )
        })?;

        if bpe.encode_with_special_tokens(&json_text).len() <= max_tokens {
            break;
        }

        // First try to shrink types
        if with_types {
            if let Some(types_val) = output.get_mut("types") {
                if shrink_types_output(types_val) {
                    truncated = true;
                    continue;
                }
            }
        }

        // Then drop files
        if ordered_files.len() > 1 {
            let Some(last_path) = ordered_files.pop() else {
                break;
            };
            output.remove(&last_path);
            truncated = true;
            continue;
        }

        let Some(only_path) = ordered_files.first().cloned() else {
            break;
        };

        let Some(file_value) = output.get_mut(&only_path) else {
            break;
        };

        if !shrink_single_file_to_fit(file_value, &bpe, max_tokens) {
            truncated = true;
            break;
        }

        truncated = true;
    }

    if truncated {
        output.insert("@".to_string(), json!({"t": true}));
    }

    Ok((output, truncated))
}

fn build_types_output(types: &[TypeDefinition], _max_tokens: usize) -> Value {
    let rows: Vec<String> = types
        .iter()
        .map(|ty| {
            let file = path_utils::to_relative_path(ty.file.to_string_lossy().as_ref());
            let kind = type_kind_str(ty.kind);
            let fields = [
                ty.name.as_str(),
                kind,
                file.as_str(),
                &ty.line.to_string(),
                &ty.usage_count.to_string(),
            ];
            format::format_row(&fields)
        })
        .collect();

    json!({
        "h": "name|kind|file|line|usage_count",
        "rows": rows.join("\n")
    })
}

fn shrink_types_output(types_val: &mut Value) -> bool {
    let Some(obj) = types_val.as_object_mut() else {
        return false;
    };
    let Some(rows) = obj.get("rows").and_then(Value::as_str) else {
        return false;
    };
    if rows.is_empty() {
        return false;
    }

    let mut lines: Vec<&str> = rows.lines().collect();
    lines.pop();
    if lines.is_empty() {
        obj.remove("rows");
    } else {
        obj.insert("rows".to_string(), json!(lines.join("\n")));
    }
    true
}

fn type_kind_str(kind: TypeKind) -> &'static str {
    match kind {
        TypeKind::Interface => "interface",
        TypeKind::Class => "class",
        TypeKind::Struct => "struct",
        TypeKind::Enum => "enum",
        TypeKind::Trait => "trait",
        TypeKind::Protocol => "protocol",
        TypeKind::TypeAlias => "type_alias",
        TypeKind::Record => "record",
        TypeKind::TypedDict => "typed_dict",
        TypeKind::NamedTuple => "named_tuple",
    }
}

fn build_compact_file(file: &FileSymbols, detail_level: DetailLevel) -> Value {
    let mut file_obj = Map::new();
    file_obj.insert("h".to_string(), json!(detail_level.header()));

    if !file.functions.is_empty() {
        file_obj.insert(
            "f".to_string(),
            json!(symbols_to_rows(
                &file.functions,
                detail_level,
                SymbolKind::Function,
            )),
        );
    }

    if !file.structs.is_empty() {
        file_obj.insert(
            "s".to_string(),
            json!(symbols_to_rows(
                &file.structs,
                detail_level,
                SymbolKind::Struct,
            )),
        );
    }

    if !file.classes.is_empty() {
        file_obj.insert(
            "c".to_string(),
            json!(symbols_to_rows(
                &file.classes,
                detail_level,
                SymbolKind::Class,
            )),
        );
    }

    Value::Object(file_obj)
}

fn shrink_single_file_to_fit(
    file_value: &mut Value,
    bpe: &tiktoken_rs::CoreBPE,
    max_tokens: usize,
) -> bool {
    let Some(file_obj) = file_value.as_object_mut() else {
        return false;
    };

    // Prefer removing rows from the largest table first.
    let mut candidates: Vec<(&str, usize)> = Vec::new();
    for key in ["f", "s", "c"] {
        if let Some(rows) = file_obj.get(key).and_then(Value::as_str) {
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

    let Some(rows) = file_obj.get(key).and_then(Value::as_str) else {
        return false;
    };

    if rows.is_empty() {
        file_obj.remove(key);
        return true;
    }

    let mut lines: Vec<&str> = rows.lines().collect();
    if lines.pop().is_none() {
        file_obj.remove(key);
        return true;
    }

    let new_rows = lines.join("\n");
    if new_rows.is_empty() {
        file_obj.remove(key);
    } else {
        file_obj.insert(key.to_string(), json!(new_rows));
    }

    // If we're still wildly over budget, allow dropping entire tables.
    let snapshot = Value::Object(file_obj.clone());
    let tmp_json = serde_json::to_string(&json!({"_": snapshot})).unwrap_or_default();
    if bpe.encode_with_special_tokens(&tmp_json).len() > max_tokens {
        // Prefer dropping `c`, then `s`, then `f`.
        for drop_key in ["c", "s", "f"] {
            if file_obj.contains_key(drop_key) {
                file_obj.remove(drop_key);
                break;
            }
        }
    }

    true
}

#[derive(Debug, Clone, Copy)]
enum SymbolKind {
    Function,
    Struct,
    Class,
}

fn symbols_to_rows(symbols: &[Value], detail_level: DetailLevel, kind: SymbolKind) -> String {
    symbols
        .iter()
        .filter_map(|sym| sym.as_object())
        .map(|obj| {
            let name = obj.get("name").and_then(Value::as_str).unwrap_or_default();
            let line = obj
                .get("line")
                .and_then(Value::as_u64)
                .map(|v| v.to_string())
                .unwrap_or_default();

            let mut fields: Vec<String> = Vec::new();
            fields.push(name.to_string());
            fields.push(line);

            match detail_level {
                DetailLevel::Minimal => {}
                DetailLevel::Signatures => {
                    let signature = match kind {
                        SymbolKind::Function => obj
                            .get("signature")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                        SymbolKind::Struct | SymbolKind::Class => "",
                    };
                    fields.push(signature.to_string());
                }
                DetailLevel::Full => {
                    let signature = match kind {
                        SymbolKind::Function => obj
                            .get("signature")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                        SymbolKind::Struct | SymbolKind::Class => "",
                    };
                    let doc = obj.get("doc").and_then(Value::as_str).unwrap_or_default();
                    let code = obj.get("code").and_then(Value::as_str).unwrap_or_default();

                    fields.push(signature.to_string());
                    fields.push(doc.to_string());
                    fields.push(code.to_string());
                }
            }

            let field_refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
            format::format_row(&field_refs)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect_files(
    dir: &Path,
    files: &mut Vec<FileSymbols>,
    detail_level: DetailLevel,
    pattern: Option<&str>,
) -> Result<(), io::Error> {
    let entries = fs::read_dir(dir).map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to read directory {}: {e}", dir.display()),
        )
    })?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Skip hidden files and common ignore patterns
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || IGNORE_DIRS.contains(&name_str.as_ref()) {
                continue;
            }
        }

        if path.is_file() {
            if detect_language(&path).is_ok() {
                if let Some(pat) = pattern {
                    if !matches_pattern(&path, pat) {
                        continue;
                    }
                }

                if let Ok(entry) = process_file(&path, detail_level) {
                    files.push(entry);
                }
            }
        } else if path.is_dir() {
            collect_files(&path, files, detail_level, pattern)?;
        }
    }

    Ok(())
}

fn symbol_count(entry: &FileSymbols) -> usize {
    entry.functions.len() + entry.structs.len() + entry.classes.len()
}

fn process_file(path: &Path, detail_level: DetailLevel) -> Result<FileSymbols, io::Error> {
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
    let tree = crate::parser::parse_code(&source, language).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse {} code: {e}", language.name()),
        )
    })?;

    let include_code = detail_level == DetailLevel::Full;
    let enhanced_shape = crate::analysis::shape::extract_enhanced_shape(
        &tree,
        &source,
        language,
        Some(&path.to_string_lossy()),
        include_code,
    )?;

    let functions = if enhanced_shape.functions.is_empty() {
        Vec::new()
    } else {
        enhanced_shape
            .functions
            .iter()
            .map(|f| filter_function_by_detail(f, detail_level))
            .collect()
    };

    let structs = if enhanced_shape.structs.is_empty() {
        Vec::new()
    } else {
        enhanced_shape
            .structs
            .iter()
            .map(|s| filter_struct_by_detail(s, detail_level))
            .collect()
    };

    let classes = if enhanced_shape.classes.is_empty() {
        Vec::new()
    } else {
        enhanced_shape
            .classes
            .iter()
            .map(|c| filter_class_by_detail(c, detail_level))
            .collect()
    };

    Ok(FileSymbols {
        path: path.to_string_lossy().to_string(),
        functions,
        structs,
        classes,
    })
}

fn collect_files_combined(
    dir: &Path,
    result: &mut ExtractionResult,
    options: &ExtractionOptions,
    pattern: Option<&str>,
) -> Result<(), io::Error> {
    let entries = fs::read_dir(dir).map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to read directory {}: {e}", dir.display()),
        )
    })?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Skip hidden files and common ignore patterns
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || IGNORE_DIRS.contains(&name_str.as_ref()) {
                continue;
            }
        }

        if path.is_file() {
            // For usage counting, we process ALL files to count words
            if options.count_usages {
                if let Ok(content) = fs::read_to_string(&path) {
                    let lang = language_for_path(&path);
                    count_words_in_content(&content, lang, &mut result.word_counts);
                }
            }

            if detect_language(&path).is_ok() {
                if let Some(pat) = pattern {
                    if !matches_pattern(&path, pat) {
                        continue;
                    }
                }

                if let Ok(entry) = process_file_combined(&path, options, result) {
                    result.files.push(entry);
                }
            }
        } else if path.is_dir() {
            collect_files_combined(&path, result, options, pattern)?;
        }
    }

    Ok(())
}

fn process_file_combined(
    path: &Path,
    options: &ExtractionOptions,
    result: &mut ExtractionResult,
) -> Result<FileSymbols, io::Error> {
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

    let tree = crate::parser::parse_code(&source, language).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse {} code: {e}", language.name()),
        )
    })?;

    let include_code = options.detail_level == DetailLevel::Full;
    let enhanced_shape = crate::analysis::shape::extract_enhanced_shape(
        &tree,
        &source,
        language,
        Some(&path.to_string_lossy()),
        include_code,
    )?;

    // Extract types if requested
    if options.with_types {
        let rel_path = std::path::PathBuf::from(path);
        if let Ok(file_types) = extract_types_from_source(&source, &rel_path, language) {
            result.types.extend(file_types);
        }
    }

    let functions = if enhanced_shape.functions.is_empty() {
        Vec::new()
    } else {
        enhanced_shape
            .functions
            .iter()
            .map(|f| filter_function_by_detail(f, options.detail_level))
            .collect()
    };

    let structs = if enhanced_shape.structs.is_empty() {
        Vec::new()
    } else {
        enhanced_shape
            .structs
            .iter()
            .map(|s| filter_struct_by_detail(s, options.detail_level))
            .collect()
    };

    let classes = if enhanced_shape.classes.is_empty() {
        Vec::new()
    } else {
        enhanced_shape
            .classes
            .iter()
            .map(|c| filter_class_by_detail(c, options.detail_level))
            .collect()
    };

    Ok(FileSymbols {
        path: path.to_string_lossy().to_string(),
        functions,
        structs,
        classes,
    })
}

/// Extract types from source code - wrapper around extraction/types functions
fn extract_types_from_source(
    source: &str,
    path: &std::path::Path,
    language: crate::parser::Language,
) -> Result<Vec<TypeDefinition>, io::Error> {
    use crate::parser::Language;

    let types = match language {
        Language::Rust => {
            crate::extraction::types::extract_rust_types(source, path)
                .map_err(|e| io::Error::other(e.to_string()))?
        }
        Language::TypeScript => {
            crate::extraction::types::extract_typescript_types(source, path, true)
                .map_err(|e| io::Error::other(e.to_string()))?
        }
        Language::JavaScript => {
            crate::extraction::types::extract_typescript_types(source, path, false)
                .map_err(|e| io::Error::other(e.to_string()))?
        }
        Language::Python => {
            crate::extraction::types::extract_python_types(source, path)
                .map_err(|e| io::Error::other(e.to_string()))?
        }
        Language::Java | Language::Go | Language::CSharp => {
            // Type extraction for these languages uses different extractors
            Vec::new()
        }
        Language::Html | Language::Css | Language::Swift => {
            // These languages don't have type definitions
            Vec::new()
        }
    };

    Ok(types)
}

fn filter_function_by_detail(func: &EnhancedFunctionInfo, detail_level: DetailLevel) -> Value {
    match detail_level {
        DetailLevel::Minimal => {
            json!({
                "name": func.name,
                "line": func.line,
            })
        }
        DetailLevel::Signatures => {
            json!({
                "name": func.name,
                "line": func.line,
                "signature": func.signature,
            })
        }
        DetailLevel::Full => {
            json!({
                "name": func.name,
                "line": func.line,
                "signature": func.signature,
                "doc": func.doc,
                "code": func.code,
            })
        }
    }
}

fn filter_struct_by_detail(s: &EnhancedStructInfo, detail_level: DetailLevel) -> Value {
    match detail_level {
        DetailLevel::Minimal => {
            json!({
                "name": s.name,
                "line": s.line,
            })
        }
        DetailLevel::Signatures => {
            json!({
                "name": s.name,
                "line": s.line,
            })
        }
        DetailLevel::Full => {
            json!({
                "name": s.name,
                "line": s.line,
                "doc": s.doc,
                "code": s.code,
            })
        }
    }
}

fn filter_class_by_detail(cls: &EnhancedClassInfo, detail_level: DetailLevel) -> Value {
    match detail_level {
        DetailLevel::Minimal => {
            json!({
                "name": cls.name,
                "line": cls.line,
            })
        }
        DetailLevel::Signatures => {
            json!({
                "name": cls.name,
                "line": cls.line,
            })
        }
        DetailLevel::Full => {
            json!({
                "name": cls.name,
                "line": cls.line,
                "doc": cls.doc,
                "code": cls.code,
            })
        }
    }
}

fn matches_pattern(path: &Path, pattern: &str) -> bool {
    match Glob::new(pattern) {
        Ok(glob) => {
            let matcher = glob.compile_matcher();
            if pattern.contains('/') || pattern.contains("**") {
                matcher.is_match(path)
            } else if let Some(file_name) = path.file_name() {
                matcher.is_match(file_name)
            } else {
                false
            }
        }
        Err(e) => {
            log::warn!("Invalid glob pattern '{}': {}", pattern, e);
            false
        }
    }
}
