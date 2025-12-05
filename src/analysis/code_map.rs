//! Code Map Tool
//!
//! Generates a high-level overview of a codebase with token budget awareness.
//! Walks directory structure, extracts file shapes, and aggregates results.
//! Supports detail levels: minimal, signatures, and full.

use crate::analysis::shape::{EnhancedClassInfo, EnhancedFunctionInfo, EnhancedStructInfo};
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::detect_language;
use globset::Glob;
use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::Path;

// Token estimation for code: Using 3 chars/token as a conservative estimate.
// This is an approximation - actual tokenization varies by language and content.
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

#[derive(Debug, serde::Serialize)]
struct CodeMap {
    files: Vec<FileEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncated: Option<bool>,
}

#[derive(Debug, serde::Serialize)]
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
    let mut current_tokens = 0;
    let max_chars = max_tokens * CHARS_PER_TOKEN;
    let mut truncated = false;

    if path.is_file() {
        // Single file
        if let Ok(entry) = process_file(path, detail_level) {
            files.push(entry);
        }
    } else if path.is_dir() {
        // Directory - walk and collect
        collect_files(
            path,
            &mut files,
            &mut current_tokens,
            max_chars,
            &mut truncated,
            detail_level,
            pattern,
        )?;
    }

    let code_map = CodeMap {
        files,
        truncated: if truncated { Some(true) } else { None },
    };

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
    current_tokens: &mut usize,
    max_chars: usize,
    truncated: &mut bool,
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
                    let entry_json = serde_json::to_string(&entry).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Failed to serialize entry to JSON: {e}"),
                        )
                    })?;
                    let entry_size = entry_json.len();

                    if *current_tokens + entry_size > max_chars {
                        *truncated = true;
                        break;
                    }

                    *current_tokens += entry_size;
                    files.push(entry);
                }
            }
        } else if path.is_dir() {
            collect_files(
                &path,
                files,
                current_tokens,
                max_chars,
                truncated,
                detail_level,
                pattern,
            )?;
            if *truncated {
                break;
            }
        }
    }

    Ok(())
}

/// Process a single file and extract its shape
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

    // Use the enhanced shape extraction
    let enhanced_shape = crate::analysis::shape::extract_enhanced_shape(
        &tree,
        &source,
        language,
        Some(&path.to_string_lossy()),
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
