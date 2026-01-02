//! Code Analysis Tools
//!
//! This module provides various tree-sitter based code analysis tools including:
//! - File shape extraction (functions, classes, imports)
//! - Code mapping with token budget awareness
//! - Symbol usage finding
//! - Context extraction at specific positions
//! - Custom query pattern execution
//! - Askama template struct context resolution

pub mod askama;
pub mod code_map;
pub mod dependencies;
pub mod diff;
pub mod file_shape;
pub mod find_usages;
pub mod path_utils;
pub mod query_pattern;
pub mod shape;
pub mod symbol_at_line;
pub mod view_code;

#[cfg(test)]
mod shape_tests;
