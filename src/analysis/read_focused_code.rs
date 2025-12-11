use crate::analysis::path_utils;
use crate::analysis::shape::extract_enhanced_shape;
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code};
use serde_json::Value;
use std::fs;
use std::io;

/// Execute the read_focused_code tool
pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path = arguments["file_path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'file_path' argument",
        )
    })?;

    let focus_symbol = arguments["focus_symbol"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'focus_symbol' argument",
        )
    })?;

    let context_radius = arguments["context_radius"]
        .as_u64()
        .map(|v| v as usize)
        .unwrap_or(0);

    log::info!("Reading focused code for '{focus_symbol}' in: {file_path}");

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

    // First pass: extract with all code blocks
    let mut shape = extract_enhanced_shape(&tree, &source, language, Some(file_path), true)?;

    // Convert to relative path
    if let Some(ref path) = shape.path {
        shape.path = Some(path_utils::to_relative_path(path));
    }

    // Find the target symbol and its neighbors
    let focus_indices = find_focus_indices(&shape, focus_symbol, context_radius);

    // Second pass: selectively remove code blocks
    remove_unfocused_code(&mut shape, &focus_indices);

    let shape_json = serde_json::to_string(&shape).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize shape to JSON: {e}"),
        )
    })?;

    Ok(CallToolResult::success(shape_json))
}

/// Find indices of symbols to include full code for
fn find_focus_indices(
    shape: &crate::analysis::shape::EnhancedFileShape,
    focus_symbol: &str,
    context_radius: usize,
) -> FocusIndices {
    let mut indices = FocusIndices {
        functions: vec![],
        structs: vec![],
        classes: vec![],
    };

    // Find focus in functions
    if let Some(pos) = shape.functions.iter().position(|f| f.name == focus_symbol) {
        let start = pos.saturating_sub(context_radius);
        let end = (pos + context_radius + 1).min(shape.functions.len());
        indices.functions.extend(start..end);
    }

    // Find focus in structs
    if let Some(pos) = shape.structs.iter().position(|s| s.name == focus_symbol) {
        let start = pos.saturating_sub(context_radius);
        let end = (pos + context_radius + 1).min(shape.structs.len());
        indices.structs.extend(start..end);
    }

    // Find focus in classes
    if let Some(pos) = shape.classes.iter().position(|c| c.name == focus_symbol) {
        let start = pos.saturating_sub(context_radius);
        let end = (pos + context_radius + 1).min(shape.classes.len());
        indices.classes.extend(start..end);
    }

    indices
}

/// Remove code blocks from symbols not in focus
fn remove_unfocused_code(
    shape: &mut crate::analysis::shape::EnhancedFileShape,
    focus_indices: &FocusIndices,
) {
    // Remove code from unfocused functions
    for (i, func) in shape.functions.iter_mut().enumerate() {
        if !focus_indices.functions.contains(&i) {
            func.code = None;
        }
    }

    // Remove code from unfocused structs
    for (i, struct_) in shape.structs.iter_mut().enumerate() {
        if !focus_indices.structs.contains(&i) {
            struct_.code = None;
        }
    }

    // Remove code from unfocused classes
    for (i, class) in shape.classes.iter_mut().enumerate() {
        if !focus_indices.classes.contains(&i) {
            class.code = None;
        }
    }
}

struct FocusIndices {
    functions: Vec<usize>,
    structs: Vec<usize>,
    classes: Vec<usize>,
}
