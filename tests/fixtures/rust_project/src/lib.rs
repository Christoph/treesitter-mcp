//! Rust Project Test Fixture
//!
//! This is a test fixture for testing tree-sitter parsing of Rust code.

pub mod calculator;
pub mod models;

/// Re-export the main calculator functions
pub use calculator::{add, multiply, subtract};
pub use models::Calculator;

/// A helper function at the module level
pub fn format_result(value: i32) -> String {
    format!("Result: {}", value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_result() {
        assert_eq!(format_result(42), "Result: 42");
    }
}
