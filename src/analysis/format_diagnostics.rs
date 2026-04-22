//! Compact formatting for LSP-provided diagnostics.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use tiktoken_rs::cl100k_base;

use crate::analysis::path_utils;
use crate::analysis::shape::{extract_enhanced_shape, EnhancedFileShape};
use crate::common::format;
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code};

const DIAGNOSTIC_HEADER: &str = "severity|file|line|col|owner|source|code|message";
const DEFAULT_MAX_TOKENS: usize = 2000;

#[derive(Debug, Clone)]
struct Diagnostic {
    file: PathBuf,
    line: usize,
    column: usize,
    severity: String,
    message: String,
    source: String,
    code: String,
}

#[derive(Debug, Clone)]
struct DiagnosticRow {
    severity: String,
    file: String,
    line: usize,
    column: usize,
    owner: String,
    source: String,
    code: String,
    message: String,
}

struct ParsedFile {
    shape: Option<EnhancedFileShape>,
}

/// Format LSP diagnostics with compact structural owner context.
pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let diagnostics = arguments["diagnostics"].as_array().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'diagnostics' argument",
        )
    })?;
    let max_tokens = arguments["max_tokens"]
        .as_u64()
        .map(|value| value as usize)
        .unwrap_or(DEFAULT_MAX_TOKENS);

    let mut parsed_diagnostics = Vec::new();
    for diagnostic in diagnostics {
        parsed_diagnostics.push(parse_diagnostic(diagnostic)?);
    }

    let mut file_cache: HashMap<PathBuf, ParsedFile> = HashMap::new();
    let mut rows = Vec::new();

    for diagnostic in parsed_diagnostics {
        let parsed = parsed_file(&diagnostic.file, &mut file_cache)?;
        let owner = parsed
            .shape
            .as_ref()
            .and_then(|shape| owner_for_line(shape, diagnostic.line))
            .unwrap_or_default();

        rows.push(DiagnosticRow {
            severity: diagnostic.severity,
            file: path_utils::to_relative_path(diagnostic.file.to_string_lossy().as_ref()),
            line: diagnostic.line,
            column: diagnostic.column,
            owner,
            source: diagnostic.source,
            code: diagnostic.code,
            message: diagnostic.message,
        });
    }

    rows.sort_by(|a, b| {
        severity_rank(&a.severity)
            .cmp(&severity_rank(&b.severity))
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.column.cmp(&b.column))
            .then_with(|| a.message.cmp(&b.message))
    });

    let (diagnostic_rows, truncated) = rows_with_budget(&rows, max_tokens)?;
    let mut result = json!({
        "h": DIAGNOSTIC_HEADER,
        "d": diagnostic_rows,
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

fn parse_diagnostic(value: &Value) -> Result<Diagnostic, io::Error> {
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
                "Diagnostic is missing 'file', 'file_path', or 'uri'",
            )
        })?;

    let (line, column) = if let Some(start) = value
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
        (line, column)
    } else {
        let line = value.get("line").and_then(Value::as_u64).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Diagnostic is missing 1-based 'line'",
            )
        })? as usize;
        let column = value
            .get("col")
            .or_else(|| value.get("column"))
            .and_then(Value::as_u64)
            .unwrap_or(1) as usize;
        (line, column)
    };

    let message = value
        .get("message")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Diagnostic is missing 'message'",
            )
        })?
        .to_string();

    Ok(Diagnostic {
        file,
        line,
        column,
        severity: parse_severity(value.get("severity")),
        message,
        source: value
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        code: diagnostic_code(value.get("code")),
    })
}

fn parse_severity(value: Option<&Value>) -> String {
    match value {
        Some(Value::Number(number)) => match number.as_u64() {
            Some(1) => "error".to_string(),
            Some(2) => "warning".to_string(),
            Some(3) => "info".to_string(),
            Some(4) => "hint".to_string(),
            Some(other) => other.to_string(),
            None => "unknown".to_string(),
        },
        Some(Value::String(label)) => label.to_ascii_lowercase(),
        _ => "unknown".to_string(),
    }
}

fn diagnostic_code(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(code)) => code.clone(),
        Some(Value::Number(code)) => code.to_string(),
        Some(Value::Object(map)) => map
            .get("value")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    }
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

fn parsed_file<'a>(
    file: &Path,
    cache: &'a mut HashMap<PathBuf, ParsedFile>,
) -> Result<&'a ParsedFile, io::Error> {
    let key = fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    if !cache.contains_key(&key) {
        let source = fs::read_to_string(file).map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Failed to read file {}: {e}", file.display()),
            )
        })?;

        let shape = detect_language(file)
            .ok()
            .and_then(|language| {
                parse_code(&source, language)
                    .ok()
                    .map(|tree| (language, tree))
            })
            .and_then(|(language, tree)| {
                extract_enhanced_shape(&tree, &source, language, file.to_str(), false).ok()
            });

        cache.insert(key.clone(), ParsedFile { shape });
    }

    cache
        .get(&key)
        .ok_or_else(|| io::Error::other(format!("Failed to cache parsed file {}", file.display())))
}

fn owner_for_line(shape: &EnhancedFileShape, line: usize) -> Option<String> {
    let mut owner = None;
    let mut owner_start = 0;

    for function in &shape.functions {
        if function.line <= line && line <= function.end_line && function.line >= owner_start {
            owner = Some(function.name.clone());
            owner_start = function.line;
        }
    }

    for class in &shape.classes {
        if class.line <= line && line <= class.end_line && class.line >= owner_start {
            owner = Some(class.name.clone());
            owner_start = class.line;
        }

        for method in &class.methods {
            if method.line <= line && line <= method.end_line && method.line >= owner_start {
                owner = Some(format!("{}::{}", class.name, method.name));
                owner_start = method.line;
            }
        }
    }

    for block in &shape.impl_blocks {
        for method in &block.methods {
            if method.line <= line && line <= method.end_line && method.line >= owner_start {
                owner = Some(format!("{}::{}", block.type_name, method.name));
                owner_start = method.line;
            }
        }
    }

    owner
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "error" => 0,
        "warning" => 1,
        "info" => 2,
        "hint" => 3,
        _ => 4,
    }
}

fn rows_with_budget(
    rows: &[DiagnosticRow],
    max_tokens: usize,
) -> Result<(String, bool), io::Error> {
    let bpe = cl100k_base()
        .map_err(|e| io::Error::other(format!("Failed to initialize tiktoken tokenizer: {e}")))?;
    let mut kept = rows.to_vec();
    let mut truncated = false;

    loop {
        let formatted = diagnostic_rows(&kept);
        let mut candidate = json!({
            "h": DIAGNOSTIC_HEADER,
            "d": formatted,
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
            return Ok((formatted, truncated));
        }

        if kept.pop().is_none() {
            return Ok((String::new(), true));
        }
        truncated = true;
    }
}

fn diagnostic_rows(rows: &[DiagnosticRow]) -> String {
    rows.iter()
        .map(|row| {
            let line = row.line.to_string();
            let column = row.column.to_string();
            format::format_row(&[
                &row.severity,
                &row.file,
                &line,
                &column,
                &row.owner,
                &row.source,
                &row.code,
                &row.message,
            ])
        })
        .collect::<Vec<_>>()
        .join("\n")
}
