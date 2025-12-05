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

    /// Creates a point at the origin
    pub fn origin() -> Self {
        Point { x: 0, y: 0 }
    }

    /// Calculates distance from origin
    pub fn distance_from_origin(&self) -> f64 {
        ((self.x.pow(2) + self.y.pow(2)) as f64).sqrt()
    }

    /// Calculates distance to another point
    pub fn distance_to(&self, other: Point) -> f64 {
        let dx = (self.x - other.x) as f64;
        let dy = (self.y - other.y) as f64;
        (dx * dx + dy * dy).sqrt()
    }

    /// Translates the point by the given offset
    pub fn translate(&mut self, dx: i32, dy: i32) {
        self.x += dx;
        self.y += dy;
    }

    /// Returns a new point translated by the given offset
    pub fn translated(&self, dx: i32, dy: i32) -> Self {
        Point {
            x: self.x + dx,
            y: self.y + dy,
        }
    }
}

/// A line segment between two points
#[derive(Debug, Clone, Copy)]
pub struct LineSegment {
    pub start: Point,
    pub end: Point,
}

impl LineSegment {
    /// Creates a new line segment
    pub fn new(start: Point, end: Point) -> Self {
        LineSegment { start, end }
    }

    /// Calculates the length of the line segment
    pub fn length(&self) -> f64 {
        self.start.distance_to(self.end)
    }

    /// Calculates the midpoint of the line segment
    pub fn midpoint(&self) -> Point {
        Point {
            x: (self.start.x + self.end.x) / 2,
            y: (self.start.y + self.end.y) / 2,
        }
    }
}
