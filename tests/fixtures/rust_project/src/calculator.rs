//! Calculator module with basic arithmetic operations

use crate::models::Calculator;

/// Adds two numbers together
///
/// # Examples
/// ```
/// let result = add(2, 3);
/// assert_eq!(result, 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Subtracts b from a
pub fn subtract(a: i32, b: i32) -> i32 {
    a - b
}

/// Multiplies two numbers
pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

/// Divides a by b, returns None if b is zero
pub fn divide(a: i32, b: i32) -> Option<i32> {
    if b == 0 {
        None
    } else {
        Some(a / b)
    }
}

/// Private helper function
fn validate_input(x: i32) -> bool {
    x >= 0
}

/// A function that uses a closure
pub fn apply_operation<F>(a: i32, b: i32, op: F) -> i32
where
    F: Fn(i32, i32) -> i32,
{
    let result = op(a, b);

    // Nested closure
    let formatter = |x: i32| -> String { format!("Result is: {}", x) };

    println!("{}", formatter(result));
    result
}

/// Uses the Calculator struct from models
pub fn create_calculator() -> Calculator {
    Calculator::new()
}
