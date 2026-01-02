//! Dependency Resolution Module
//!
//! Handles finding file dependencies for different languages.
//! Supports both module declarations and import statements.

use crate::parser::Language;
use std::path::{Path, PathBuf};

// Re-export dependency resolution functions from file_shape
pub use crate::analysis::file_shape::{
    find_js_ts_dependencies, find_python_dependencies, find_rust_dependencies,
};

/// Resolve all file dependencies for a given source file
///
/// Returns a list of absolute paths to dependency files.
/// Only includes files that exist on the filesystem.
pub fn resolve_dependencies(
    language: Language,
    source: &str,
    file_path: &Path,
    project_root: &Path,
) -> Vec<PathBuf> {
    match language {
        Language::Rust => find_rust_dependencies(source, file_path, project_root),
        Language::Python => find_python_dependencies(source, file_path, project_root),
        Language::JavaScript | Language::TypeScript => {
            find_js_ts_dependencies(source, file_path, project_root)
        }
        _ => vec![],
    }
}
