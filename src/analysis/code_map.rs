//! Code Map Tool
//!
//! Generates a high-level overview of a codebase with token budget awareness.
//! Walks directory structure, extracts file shapes, and aggregates results.
//! Supports detail levels: minimal, signatures, and full.

use crate::analysis::path_utils;
use crate::analysis::shape::{EnhancedClassInfo, EnhancedFunctionInfo, EnhancedStructInfo};
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::detect_language;
use globset::Glob;
use serde_json::{json, Value};
use std::cmp::Reverse;
use std::fs;
use std::io;
use std::path::Path;
use tiktoken_rs::cl100k_base;

// NOTE: We use tiktoken for final budgeting (see `apply_token_budget`).
// A loose char-budget is still used as a prefilter for large directories.
const CHARS_PER_TOKEN: usize = 3;

/// Directories to ignore during traversal
const IGNORE_DIRS: &[&str] = &["target", "node_modules", "dist", "build"];

/// Detail level for code map output
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
            _ => DetailLevel::Signatures, // default
        }
    }
}

#[derive(Debug, serde::Serialize, Clone)]
struct CodeMap {
    files: Vec<FileEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncated: Option<bool>,
}

#[derive(Debug, serde::Serialize, Clone)]
struct FileEntry {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    functions: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    structs: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    classes: Option<Vec<Value>>,
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
        "Generating code map for: {path_str} (max_tokens: {max_tokens}, detail: {detail_str})"
    );

    let path = Path::new(path_str);

    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Path does not exist: {}", path_str),
        ));
    }

    let mut files = Vec::new();
    let max_chars = max_tokens * CHARS_PER_TOKEN;
    let mut truncated = false;

    if path.is_file() {
        if let Ok(entry) = process_file(path, detail_level) {
            files.push(entry);
        }
    } else if path.is_dir() {
        // Directory - walk and collect (no truncation yet)
        collect_files(path, &mut files, detail_level, pattern)?;

        // Sort by importance (symbol count DESC)
        files.sort_by_key(|entry| Reverse(symbol_count(entry)));

        // Apply token/char budget after sorting
        let (budgeted, hit_budget) = truncate_files_by_budget(&files, max_chars)?;
        truncated = hit_budget;
        files = budgeted;
    }

    // Convert all file paths to relative paths
    for entry in &mut files {
        entry.path = path_utils::to_relative_path(&entry.path);
    }

    let mut code_map = CodeMap {
        files,
        truncated: if truncated { Some(true) } else { None },
    };

    // Enforce the max_tokens budget using real tiktoken counting.
    // This is critical for token-efficiency tests and LLM safety.
    if apply_token_budget(&mut code_map, max_tokens)? {
        code_map.truncated = Some(true);
    }

    let map_json = serde_json::to_string(&code_map).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize code map to JSON: {e}"),
        )
    })?;
    Ok(CallToolResult::success(map_json))
}

/// Recursively collect files from directory
fn collect_files(
    dir: &Path,
    files: &mut Vec<FileEntry>,
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
            // Check if we can detect language (skip non-source files)
            if detect_language(&path).is_ok() {
                // Apply pattern filter if provided
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

fn symbol_count(entry: &FileEntry) -> usize {
    entry.functions.as_ref().map(|v| v.len()).unwrap_or(0)
        + entry.structs.as_ref().map(|v| v.len()).unwrap_or(0)
        + entry.classes.as_ref().map(|v| v.len()).unwrap_or(0)
}

fn truncate_files_by_budget(
    files: &[FileEntry],
    max_chars: usize,
) -> Result<(Vec<FileEntry>, bool), io::Error> {
    let mut result = Vec::new();
    let mut used = 0;
    let mut hit_budget = false;

    // Once we fall back to path-only, keep all subsequent entries path-only.
    // This preserves the expected "sorted by symbol count" property in tests
    // because symbol_count(path-only) == 0.
    let mut force_minimal = false;

    for entry in files {
        if !force_minimal {
            let full_json = serde_json::to_string(entry).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to serialize entry to JSON: {e}"),
                )
            })?;

            if used + full_json.len() <= max_chars {
                used += full_json.len();
                result.push(FileEntry {
                    path: entry.path.clone(),
                    functions: entry.functions.clone(),
                    structs: entry.structs.clone(),
                    classes: entry.classes.clone(),
                });
                continue;
            }

            // Switching to minimal mode from here onward.
            force_minimal = true;
        }

        // Path-only entry so we can still include more files and keep the map
        // representative across layers.
        let minimal_entry = FileEntry {
            path: entry.path.clone(),
            functions: None,
            structs: None,
            classes: None,
        };

        let minimal_json = serde_json::to_string(&minimal_entry).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize entry to JSON: {e}"),
            )
        })?;

        if used + minimal_json.len() <= max_chars {
            hit_budget = true;
            used += minimal_json.len();
            result.push(minimal_entry);
            continue;
        }

        // Can't fit even the minimal entry.
        hit_budget = true;
        break;
    }

    Ok((result, hit_budget))
}

/// Process a single file and extract its shape
fn apply_token_budget(code_map: &mut CodeMap, max_tokens: usize) -> Result<bool, io::Error> {
    let bpe = cl100k_base()
        .map_err(|e| io::Error::other(format!("Failed to initialize tiktoken tokenizer: {e}")))?;

    let mut json = serde_json::to_string(code_map).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize code map to JSON: {e}"),
        )
    })?;

    if bpe.encode_with_special_tokens(&json).len() <= max_tokens {
        return Ok(false);
    }

    // 1) Drop `code` fields (largest contributor)
    for entry in &mut code_map.files {
        drop_object_field(entry.functions.as_mut(), "code");
        drop_object_field(entry.structs.as_mut(), "code");
        drop_object_field(entry.classes.as_mut(), "code");
    }

    json = serde_json::to_string(code_map).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize code map to JSON: {e}"),
        )
    })?;

    if bpe.encode_with_special_tokens(&json).len() <= max_tokens {
        return Ok(true);
    }

    // 2) Drop `doc`
    for entry in &mut code_map.files {
        drop_object_field(entry.functions.as_mut(), "doc");
        drop_object_field(entry.structs.as_mut(), "doc");
        drop_object_field(entry.classes.as_mut(), "doc");
    }

    json = serde_json::to_string(code_map).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize code map to JSON: {e}"),
        )
    })?;

    if bpe.encode_with_special_tokens(&json).len() <= max_tokens {
        return Ok(true);
    }

    // 3) Drop signatures / end_line fields to keep only names + line numbers.
    for entry in &mut code_map.files {
        drop_object_field(entry.functions.as_mut(), "signature");
        drop_object_field(entry.functions.as_mut(), "end_line");
        drop_object_field(entry.structs.as_mut(), "end_line");
        drop_object_field(entry.classes.as_mut(), "end_line");
    }

    json = serde_json::to_string(code_map).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize code map to JSON: {e}"),
        )
    })?;

    if bpe.encode_with_special_tokens(&json).len() <= max_tokens {
        return Ok(true);
    }

    // 4) Fall back to path-only entries and truncate file list to fit.
    for entry in &mut code_map.files {
        entry.functions = None;
        entry.structs = None;
        entry.classes = None;
    }

    // Binary search for the max number of files that fit.
    let original_len = code_map.files.len();
    let mut low = 0usize;
    let mut high = original_len;

    while low < high {
        let mid = (low + high).div_ceil(2);
        let candidate = CodeMap {
            files: code_map.files[..mid].to_vec(),
            truncated: Some(true),
        };

        let candidate_json = serde_json::to_string(&candidate).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize code map to JSON: {e}"),
            )
        })?;

        if bpe.encode_with_special_tokens(&candidate_json).len() <= max_tokens {
            low = mid;
        } else {
            high = mid - 1;
        }
    }

    code_map.files.truncate(low);

    Ok(true)
}

fn drop_object_field(values: Option<&mut Vec<Value>>, field: &str) {
    let Some(values) = values else {
        return;
    };

    for value in values {
        let Some(obj) = value.as_object_mut() else {
            continue;
        };
        obj.remove(field);
    }
}

fn process_file(path: &Path, detail_level: DetailLevel) -> Result<FileEntry, io::Error> {
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

    // Use the enhanced shape extraction
    let enhanced_shape = crate::analysis::shape::extract_enhanced_shape(
        &tree,
        &source,
        language,
        Some(&path.to_string_lossy()),
        include_code,
    )?;

    let path_str = path.to_string_lossy().to_string();

    let functions = if !enhanced_shape.functions.is_empty() {
        Some(
            enhanced_shape
                .functions
                .iter()
                .map(|f| filter_function_by_detail(f, detail_level))
                .collect(),
        )
    } else {
        None
    };

    let structs = if !enhanced_shape.structs.is_empty() {
        Some(
            enhanced_shape
                .structs
                .iter()
                .map(|s| filter_struct_by_detail(s, detail_level))
                .collect(),
        )
    } else {
        None
    };

    let classes = if !enhanced_shape.classes.is_empty() {
        Some(
            enhanced_shape
                .classes
                .iter()
                .map(|c| filter_class_by_detail(c, detail_level))
                .collect(),
        )
    } else {
        None
    };

    Ok(FileEntry {
        path: path_str,
        functions,
        structs,
        classes,
    })
}

/// Filter function info based on detail level
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
                "signature": func.signature,
                "line": func.line,
                "end_line": func.end_line,
            })
        }
        DetailLevel::Full => {
            let mut obj = json!({
                "name": func.name,
                "signature": func.signature,
                "line": func.line,
                "end_line": func.end_line,
            });
            if let Some(doc) = &func.doc {
                obj["doc"] = json!(doc);
            }
            if let Some(code) = &func.code {
                obj["code"] = json!(code);
            }
            obj
        }
    }
}

/// Filter struct info based on detail level
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
                "end_line": s.end_line,
            })
        }
        DetailLevel::Full => {
            let mut obj = json!({
                "name": s.name,
                "line": s.line,
                "end_line": s.end_line,
            });
            if let Some(doc) = &s.doc {
                obj["doc"] = json!(doc);
            }
            if let Some(code) = &s.code {
                obj["code"] = json!(code);
            }
            obj
        }
    }
}

/// Filter class info based on detail level
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
                "end_line": cls.end_line,
            })
        }
        DetailLevel::Full => {
            let mut obj = json!({
                "name": cls.name,
                "line": cls.line,
                "end_line": cls.end_line,
            });
            if let Some(doc) = &cls.doc {
                obj["doc"] = json!(doc);
            }
            if let Some(code) = &cls.code {
                obj["code"] = json!(code);
            }
            obj
        }
    }
}

/// Check if a file path matches a glob pattern
/// Supports full glob syntax including:
/// - `*.ext` for file extensions
/// - `**/*.ext` for recursive patterns
/// - `test_*.rs` for prefix/suffix patterns
/// - Character classes like `[abc].rs`
fn matches_pattern(path: &Path, pattern: &str) -> bool {
    // Try to compile the glob pattern
    match Glob::new(pattern) {
        Ok(glob) => {
            let matcher = glob.compile_matcher();
            // Match against the full path for patterns with path separators
            // Otherwise match against just the filename
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
