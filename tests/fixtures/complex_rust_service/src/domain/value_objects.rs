//! Value objects - immutable types with validation

use serde::{Deserialize, Serialize};
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, Mul};

/// Email value object with validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Email(String);

impl Email {
    pub fn new(email: impl Into<String>) -> Result<Self, super::models::ValidationError> {
        let email = email.into();
        if email.contains('@') && email.len() > 3 {
            Ok(Self(email))
        } else {
            Err(super::models::ValidationError::InvalidEmail)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Money value object with currency-safe arithmetic
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    cents: i64,
}

impl Money {
    pub fn from_cents(cents: i64) -> Self {
        Self { cents }
    }

    pub fn from_dollars(dollars: i64) -> Self {
        Self {
            cents: dollars * 100,
        }
    }

    pub fn zero() -> Self {
        Self { cents: 0 }
    }

    pub fn cents(&self) -> i64 {
        self.cents
    }

    pub fn dollars(&self) -> f64 {
        self.cents as f64 / 100.0
    }
}

impl Add for Money {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            cents: self.cents + other.cents,
        }
    }
}

impl Mul<u32> for Money {
    type Output = Self;

    fn mul(self, quantity: u32) -> Self {
        Self {
            cents: self.cents * quantity as i64,
        }
    }
}

impl Sum for Money {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Money::zero(), |acc, m| acc + m)
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${:.2}", self.dollars())
    }
}

/// Order status state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Confirmed,
    Shipped,
    Delivered,
    Cancelled,
}

impl OrderStatus {
    pub fn can_transition_to(&self, next: OrderStatus) -> bool {
        use OrderStatus::*;
        matches!(
            (self, next),
            (Pending, Confirmed)
                | (Pending, Cancelled)
                | (Confirmed, Shipped)
                | (Confirmed, Cancelled)
                | (Shipped, Delivered)
        )
    }
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Confirmed => write!(f, "Confirmed"),
            Self::Shipped => write!(f, "Shipped"),
            Self::Delivered => write!(f, "Delivered"),
            Self::Cancelled => write!(f, "Cancelled"),
        }
    }
}
