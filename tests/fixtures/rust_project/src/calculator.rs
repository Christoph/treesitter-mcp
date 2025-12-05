//! Calculator module with basic arithmetic operations

use crate::models::{Calculator, Point};
use std::ops::Add;

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
///
/// # Arguments
/// * `a` - The minuend
/// * `b` - The subtrahend
///
/// # Returns
/// The difference a - b
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

/// A function that uses a closure
pub fn apply_operation<F>(a: i32, b: i32, op: F) -> i32
where
    F: Fn(i32, i32) -> i32,
{
    let formatter = |x: i32| -> String { format!("Result is: {}", x) };
    let result = op(a, b);
    println!("{}", formatter(result));
    result
}

/// Uses the Calculator struct from models
pub fn create_calculator() -> Calculator {
    Calculator::new()
}

/// Creates a calculator with an initial value
pub fn create_calculator_with_value(value: i32) -> Calculator {
    Calculator::with_value(value)
}

/// Performs a sequence of operations on a calculator
pub fn perform_sequence(mut calc: Calculator, operations: &[(i32, char)]) -> i32 {
    for (value, op) in operations {
        match op {
            '+' => {
                calc.add(*value);
            }
            '-' => {
                calc.subtract(*value);
            }
            _ => {}
        }
    }
    calc.value
}

/// Calculates the distance between two points
pub fn point_distance(p1: Point, p2: Point) -> f64 {
    let dx = (p1.x - p2.x) as f64;
    let dy = (p1.y - p2.y) as f64;
    (dx * dx + dy * dy).sqrt()
}

/// A complex operation with nested closures
pub fn complex_operation(base: i32) -> i32 {
    let multiplier = 2;

    // First closure
    let double = |x: i32| -> i32 { x * multiplier };

    // Nested closure inside the first
    let apply_twice = |x: i32| -> i32 {
        let first_pass = double(x);
        // Another nested closure
        let add_base = |y: i32| -> i32 { y + base };
        add_base(first_pass)
    };

    apply_twice(base)
}
