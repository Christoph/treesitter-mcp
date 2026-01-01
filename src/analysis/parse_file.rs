//! Parse File Tool
//!
//! This tool parses a source file using tree-sitter and returns the
//! file shape - structured JSON with functions, classes, imports, and their signatures.

use crate::analysis::dependencies::resolve_dependencies;
use crate::analysis::path_utils;
use crate::analysis::shape::extract_enhanced_shape;
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;

/// Execute the parse_file tool
///
/// # Arguments
/// * `arguments` - JSON object with `file_path` field
///
/// # Returns
/// Returns a `CallToolResult` with structured JSON containing:
/// - `path`: File path
/// - `language`: Detected language
/// - `functions`: Array of function definitions with signatures and code
/// - `structs`: Array of struct definitions (Rust)
/// - `classes`: Array of class definitions (Python, JavaScript, TypeScript)
/// - `imports`: Array of import statements
///
/// # Errors
/// Returns an error if:
/// - The `file_path` argument is missing or invalid
/// - The file cannot be read
/// - The file extension is not supported
/// - Parsing fails completely (very rare)
pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path = arguments["file_path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'file_path' argument",
        )
    })?;

    let include_code = arguments["include_code"].as_bool().unwrap_or(true);
    let include_deps = arguments["include_deps"].as_bool().unwrap_or(false);

    log::info!(
        "Parsing file: {file_path} (include_code: {include_code}, include_deps: {include_deps})"
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

    log::debug!("Detected language: {}", language.name());

    let tree = parse_code(&source, language).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse {} code: {e}", language.name()),
        )
    })?;

    let mut shape =
        extract_enhanced_shape(&tree, &source, language, Some(file_path), include_code)?;

    // NEW: Optionally include dependencies
    if include_deps {
        let project_root =
            path_utils::find_project_root(Path::new(file_path)).ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "Could not determine project root")
            })?;

        let mut visited = HashSet::new();

        // Mark main file as visited
        if let Ok(canonical) = fs::canonicalize(file_path) {
            visited.insert(canonical);
        }

        let dep_paths =
            resolve_dependencies(language, &source, Path::new(file_path), &project_root);

        log::debug!("Found {} dependencies for {}", dep_paths.len(), file_path);

        for dep_path in dep_paths {
            // Canonicalize and check if already visited
            let canonical = match fs::canonicalize(&dep_path) {
                Ok(p) => p,
                Err(e) => {
                    log::warn!("Failed to canonicalize {}: {}", dep_path.display(), e);
                    continue;
                }
            };

            if visited.contains(&canonical) {
                log::debug!("Skipping already visited: {}", dep_path.display());
                continue; // Avoid cycles
            }
            visited.insert(canonical);

            // Read and parse dependency
            let dep_source = match fs::read_to_string(&dep_path) {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("Failed to read dependency {}: {}", dep_path.display(), e);
                    continue; // Skip missing files gracefully
                }
            };

            let dep_language = match detect_language(&dep_path) {
                Ok(l) => l,
                Err(e) => {
                    log::warn!(
                        "Failed to detect language for {}: {}",
                        dep_path.display(),
                        e
                    );
                    continue;
                }
            };

            let dep_tree = match parse_code(&dep_source, dep_language) {
                Ok(t) => t,
                Err(e) => {
                    log::warn!("Failed to parse {}: {}", dep_path.display(), e);
                    continue;
                }
            };

            // Dependencies are ALWAYS signatures-only (include_code=false)
            let mut dep_shape = extract_enhanced_shape(
                &dep_tree,
                &dep_source,
                dep_language,
                Some(dep_path.to_str().unwrap_or("unknown")),
                false, // IMPORTANT: No code bodies for deps
            )?;

            // Convert to relative path
            if let Some(ref path) = dep_shape.path {
                dep_shape.path = Some(path_utils::to_relative_path(path));
            }

            shape.dependencies.push(dep_shape);
        }
    }

    // Convert main file path to relative
    if let Some(ref path) = shape.path {
        shape.path = Some(path_utils::to_relative_path(path));
    }

    log::debug!(
        "Extracted file shape with {} functions and {} dependencies",
        shape.functions.len(),
        shape.dependencies.len()
    );

    let shape_json = serde_json::to_string(&shape).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize shape to JSON: {e}"),
        )
    })?;

    Ok(CallToolResult::success(shape_json))
}
