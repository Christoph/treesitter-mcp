//! View Code Tool
//!
//! Unified file viewer with flexible detail levels and automatic type inclusion.
//! Merges functionality from parse_file, read_focused_code, and file_shape.

use crate::analysis::askama::find_templates_dir;
use crate::analysis::askama::TemplateStructInfo;
use crate::analysis::dependencies::resolve_dependencies;
use crate::analysis::path_utils;
use crate::analysis::shape::{
    extract_enhanced_shape, EnhancedClassInfo, EnhancedFileShape, EnhancedStructInfo,
    ImplBlockInfo, InterfaceInfo, TraitInfo,
};
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code, Language};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Type definitions from a dependency file
#[derive(Debug, serde::Serialize)]
pub struct TypeDefinitions {
    /// Source file path
    pub path: String,

    /// Struct definitions
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub structs: Vec<EnhancedStructInfo>,

    /// Class definitions
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<EnhancedClassInfo>,

    /// Interface definitions (TS/JS)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<InterfaceInfo>,

    /// Impl blocks (Rust)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub impl_blocks: Vec<ImplBlockInfo>,

    /// Traits (Rust)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<TraitInfo>,
}

/// Output structure for view_code
#[derive(Debug, serde::Serialize)]
pub struct ViewCodeOutput {
    /// The main file being viewed
    #[serde(flatten)]
    pub file: EnhancedFileShape,

    /// Type definitions from project dependencies
    /// ALWAYS included to prevent hallucinations
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub project_types: Vec<TypeDefinitions>,

    /// Askama template struct context (only for HTML files in templates/ directory)
    /// Provides struct definitions that the template has access to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_structs: Option<Vec<TemplateStructInfo>>,
}

/// Execute the view_code tool
///
/// # Arguments
/// * `arguments` - JSON object with:
///   - `file_path`: String - Path to the source file
///   - `detail`: String - "signatures" or "full" (default: "full")
///   - `focus_symbol`: Option<String> - Symbol to focus on
///
/// # Returns
/// Returns a `CallToolResult` with structured JSON containing the file shape
/// and project type definitions
pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path = arguments["file_path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'file_path' argument",
        )
    })?;

    let detail = arguments["detail"].as_str().unwrap_or("full");
    let focus_symbol = arguments["focus_symbol"].as_str();

    let include_code = detail == "full";

    log::info!(
        "Viewing code: {file_path} (detail: {detail}, focus_symbol: {:?})",
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

    // Extract main file shape
    let mut main_shape =
        extract_enhanced_shape(&tree, &source, language, Some(file_path), include_code)?;

    // ALWAYS resolve project dependencies and extract their types
    // If no project root found, use the file's parent directory
    let project_root = path_utils::find_project_root(Path::new(file_path))
        .or_else(|| Path::new(file_path).parent().map(|p| p.to_path_buf()))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine project root or parent directory",
            )
        })?;

    let dep_paths = resolve_dependencies(language, &source, Path::new(file_path), &project_root);

    log::debug!("Found {} dependencies for {}", dep_paths.len(), file_path);

    // Filter: Only project files, NOT external dependencies
    let project_deps = filter_project_dependencies(dep_paths, &project_root);

    log::debug!("Filtered to {} project dependencies", project_deps.len());

    // Extract ONLY type/struct/class/interface definitions from dependencies
    let mut project_types = Vec::new();
    let mut visited = HashSet::new();

    // Mark main file as visited
    if let Ok(canonical) = fs::canonicalize(file_path) {
        visited.insert(canonical);
    }

    for dep_path in project_deps {
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
            continue;
        }
        visited.insert(canonical);

        match extract_types_only(&dep_path) {
            Ok(types) => {
                project_types.push(types);
            }
            Err(e) => {
                log::warn!("Failed to extract types from {}: {}", dep_path.display(), e);
                continue;
            }
        }
    }

    // Apply focus if requested
    if let Some(symbol) = focus_symbol {
        apply_focus(&mut main_shape, symbol);
    }

    // Convert main file path to relative
    if let Some(ref path) = main_shape.path {
        main_shape.path = Some(path_utils::to_relative_path(path));
    }

    // Check if this is an HTML template file and find associated Askama structs
    let template_structs = if language == Language::Html {
        find_askama_template_structs(Path::new(file_path), &project_root)
    } else {
        None
    };

    let output = ViewCodeOutput {
        file: main_shape,
        project_types,
        template_structs,
    };

    let output_json = serde_json::to_string(&output).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize output to JSON: {e}"),
        )
    })?;

    Ok(CallToolResult::success(output_json))
}

/// Extract ONLY type definitions (structs, classes, interfaces, enums)
/// Does NOT include function implementations
fn extract_types_only(file_path: &Path) -> Result<TypeDefinitions, io::Error> {
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

    // Extract shape with include_code=false (signatures only)
    let shape = extract_enhanced_shape(
        &tree,
        &source,
        language,
        Some(file_path.to_str().unwrap_or("unknown")),
        false,
    )?;

    // Convert to relative path
    let relative_path = path_utils::to_relative_path(
        shape
            .path
            .as_deref()
            .unwrap_or(file_path.to_str().unwrap_or("unknown")),
    );

    Ok(TypeDefinitions {
        path: relative_path,
        structs: shape.structs,
        classes: shape.classes,
        interfaces: shape.interfaces,
        impl_blocks: shape.impl_blocks,
        traits: shape.traits,
    })
}

/// Filter dependencies to only include project files, not external libraries
fn filter_project_dependencies(dep_paths: Vec<PathBuf>, project_root: &Path) -> Vec<PathBuf> {
    dep_paths
        .into_iter()
        .filter(|path| {
            // Include if path is inside project_root
            path.starts_with(project_root) &&
            // Exclude external dependency directories
            !path.to_string_lossy().contains("/target/") &&
            !path.to_string_lossy().contains("/node_modules/") &&
            !path.to_string_lossy().contains("/venv/") &&
            !path.to_string_lossy().contains("/.venv/") &&
            !path.to_string_lossy().contains("/site-packages/") &&
            !path.to_string_lossy().contains("\\target\\") &&
            !path.to_string_lossy().contains("\\node_modules\\") &&
            !path.to_string_lossy().contains("\\venv\\") &&
            !path.to_string_lossy().contains("\\.venv\\") &&
            !path.to_string_lossy().contains("\\site-packages\\")
        })
        .collect()
}

/// Apply focus to show full code only for the specified symbol
fn apply_focus(shape: &mut EnhancedFileShape, focus_symbol: &str) {
    // Find focus in functions
    let mut found = false;
    for func in &mut shape.functions {
        if func.name == focus_symbol {
            found = true;
            // Keep code for focused symbol
        } else {
            // Remove code for non-focused symbols
            func.code = None;
        }
    }

    // Find focus in structs
    for struct_info in &mut shape.structs {
        if struct_info.name == focus_symbol {
            found = true;
        } else {
            struct_info.code = None;
        }
    }

    // Find focus in classes
    for class in &mut shape.classes {
        if class.name == focus_symbol {
            found = true;
        } else {
            class.code = None;
            // Also remove code from methods
            for method in &mut class.methods {
                method.code = None;
            }
        }
    }

    // Find focus in impl blocks
    for impl_block in &mut shape.impl_blocks {
        if impl_block.type_name == focus_symbol {
            found = true;
        } else {
            // Remove code from methods
            for method in &mut impl_block.methods {
                method.code = None;
            }
        }
    }

    if !found {
        log::warn!("Focus symbol '{}' not found in file", focus_symbol);
    }
}

/// Find Askama template structs for an HTML template file
fn find_askama_template_structs(
    template_path: &Path,
    project_root: &Path,
) -> Option<Vec<TemplateStructInfo>> {
    // Check if file is in a templates directory
    if let Some(parent) = template_path.parent() {
        if find_templates_dir(parent).is_some() {
            // Try to find associated Rust structs
            match crate::analysis::askama::find_askama_structs_for_template(
                template_path,
                project_root,
            ) {
                Ok(structs) if !structs.is_empty() => {
                    log::info!(
                        "Found {} Askama struct(s) for template {}",
                        structs.len(),
                        template_path.display()
                    );
                    Some(structs)
                }
                Ok(_) => {
                    log::debug!(
                        "No Askama structs found for template {}",
                        template_path.display()
                    );
                    None
                }
                Err(e) => {
                    log::warn!(
                        "Failed to find Askama structs for {}: {}",
                        template_path.display(),
                        e
                    );
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    }
}
