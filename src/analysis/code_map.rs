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

use std::cmp::Reverse;
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
use crate::common::budget;
use crate::common::budget::BudgetTracker;
use crate::common::format;
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

    log::info!(
        "Generating compact code map for: {path_str} (max_tokens: {max_tokens}, detail: {detail_str})"
    );

    let path = Path::new(path_str);
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Path does not exist: {path_str}"),
        ));
    }

    let mut files: Vec<FileSymbols> = Vec::new();

    if path.is_file() {
        if let Ok(entry) = process_file(path, detail_level) {
            files.push(entry);
        }
    } else if path.is_dir() {
        collect_files(path, &mut files, detail_level, pattern)?;
        files.sort_by_key(|entry| Reverse(symbol_count(entry)));
    }

    // Convert all file paths to relative paths
    for entry in &mut files {
        entry.path = path_utils::to_relative_path(&entry.path);
    }

    let (result_map, _truncated) = build_compact_output(&files, detail_level, max_tokens)?;

    let json_text = serde_json::to_string(&Value::Object(result_map)).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize code map to JSON: {e}"),
        )
    })?;

    Ok(CallToolResult::success(json_text))
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
