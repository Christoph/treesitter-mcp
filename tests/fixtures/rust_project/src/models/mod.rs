//! Models module containing data structures

use std::fmt;

/// A simple calculator struct
///
/// This struct maintains state for calculator operations.
pub struct Calculator {
    /// The current value
    pub value: i32,
    /// History of operations
    history: Vec<String>,
}

impl Calculator {
    /// Creates a new Calculator with value 0
    pub fn new() -> Self {
        Calculator {
            value: 0,
            history: Vec::new(),
        }
    }

    /// Creates a calculator with an initial value
    pub fn with_value(value: i32) -> Self {
        Calculator {
            value,
            history: Vec::new(),
        }
    }

    /// Adds a number to the current value
    pub fn add(&mut self, n: i32) -> i32 {
        self.value += n;
        self.history.push(format!("add {}", n));
        self.value
    }

    /// Subtracts a number from the current value
    pub fn subtract(&mut self, n: i32) -> i32 {
        self.value -= n;
        self.history.push(format!("subtract {}", n));
        self.value
    }

    /// Resets the calculator to zero
    pub fn reset(&mut self) {
        self.value = 0;
        self.history.clear();
    }

    /// Gets the operation history
    pub fn get_history(&self) -> &[String] {
        &self.history
    }

    /// Private helper method
    fn log_operation(&mut self, op: &str) {
        self.history.push(op.to_string());
    }
}

impl Default for Calculator {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Calculator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Calculator(value: {})", self.value)
    }
}

/// A point structure for testing
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    /// Creates a new point
    pub fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }

    /// Calculates distance from origin
    pub fn distance_from_origin(&self) -> f64 {
        ((self.x.pow(2) + self.y.pow(2)) as f64).sqrt()
    }
}
