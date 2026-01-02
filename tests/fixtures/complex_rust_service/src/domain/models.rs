//! Domain models representing core business entities

use serde::{Deserialize, Serialize};
use std::fmt;

use super::events::DomainEvent;
use super::value_objects::{Email, Money, OrderStatus};

/// User entity with validation and business logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: Email,
    pub name: String,
    pub is_active: bool,
    #[serde(skip)]
    events: Vec<DomainEvent>,
}

impl User {
    /// Creates a new user with validation
    pub fn new(id: UserId, email: Email, name: String) -> Result<Self, ValidationError> {
        if name.trim().is_empty() {
            return Err(ValidationError::EmptyName);
        }

        let mut user = Self {
            id,
            email,
            name,
            is_active: true,
            events: vec![],
        };

        user.record_event(DomainEvent::UserCreated { user_id: id });
        Ok(user)
    }

    /// Deactivates the user account
    pub fn deactivate(&mut self) {
        if self.is_active {
            self.is_active = false;
            self.record_event(DomainEvent::UserDeactivated { user_id: self.id });
        }
    }

    /// Updates user email with validation
    pub fn update_email(&mut self, new_email: Email) -> Result<(), ValidationError> {
        if self.email == new_email {
            return Ok(());
        }

        self.email = new_email.clone();
        self.record_event(DomainEvent::UserEmailChanged {
            user_id: self.id,
            new_email,
        });
        Ok(())
    }

    fn record_event(&mut self, event: DomainEvent) {
        self.events.push(event);
    }

    pub fn take_events(&mut self) -> Vec<DomainEvent> {
        std::mem::take(&mut self.events)
    }
}

/// Product entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: ProductId,
    pub name: String,
    pub price: Money,
    pub stock: u32,
}

impl Product {
    pub fn new(id: ProductId, name: String, price: Money) -> Self {
        Self {
            id,
            name,
            price,
            stock: 0,
        }
    }

    pub fn add_stock(&mut self, quantity: u32) {
        self.stock += quantity;
    }

    pub fn reserve_stock(&mut self, quantity: u32) -> Result<(), StockError> {
        if self.stock < quantity {
            return Err(StockError::InsufficientStock {
                available: self.stock,
                requested: quantity,
            });
        }
        self.stock -= quantity;
        Ok(())
    }
}

/// Order aggregate root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub user_id: UserId,
    pub items: Vec<OrderItem>,
    pub status: OrderStatus,
    pub total: Money,
    #[serde(skip)]
    events: Vec<DomainEvent>,
}

impl Order {
    pub fn new(id: OrderId, user_id: UserId) -> Self {
        let mut order = Self {
            id,
            user_id,
            items: vec![],
            status: OrderStatus::Pending,
            total: Money::zero(),
            events: vec![],
        };
        order.record_event(DomainEvent::OrderCreated { order_id: id });
        order
    }

    pub fn add_item(&mut self, product: &Product, quantity: u32) -> Result<(), OrderError> {
        if self.status != OrderStatus::Pending {
            return Err(OrderError::OrderNotEditable);
        }

        let item = OrderItem {
            product_id: product.id,
            product_name: product.name.clone(),
            unit_price: product.price,
            quantity,
        };

        self.items.push(item);
        self.recalculate_total();
        Ok(())
    }

    pub fn confirm(&mut self) -> Result<(), OrderError> {
        if self.status != OrderStatus::Pending {
            return Err(OrderError::InvalidStatusTransition);
        }

        if self.items.is_empty() {
            return Err(OrderError::EmptyOrder);
        }

        self.status = OrderStatus::Confirmed;
        self.record_event(DomainEvent::OrderConfirmed { order_id: self.id });
        Ok(())
    }

    pub fn ship(&mut self) -> Result<(), OrderError> {
        if self.status != OrderStatus::Confirmed {
            return Err(OrderError::InvalidStatusTransition);
        }

        self.status = OrderStatus::Shipped;
        self.record_event(DomainEvent::OrderShipped { order_id: self.id });
        Ok(())
    }

    fn recalculate_total(&mut self) {
        self.total = self
            .items
            .iter()
            .map(|item| item.unit_price * item.quantity)
            .sum();
    }

    fn record_event(&mut self, event: DomainEvent) {
        self.events.push(event);
    }

    pub fn take_events(&mut self) -> Vec<DomainEvent> {
        std::mem::take(&mut self.events)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub product_id: ProductId,
    pub product_name: String,
    pub unit_price: Money,
    pub quantity: u32,
}

// Type-safe IDs using newtype pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProductId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderId(pub u64);

// Error types
#[derive(Debug, Clone)]
pub enum ValidationError {
    EmptyName,
    InvalidEmail,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "Name cannot be empty"),
            Self::InvalidEmail => write!(f, "Invalid email format"),
        }
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug, Clone)]
pub enum StockError {
    InsufficientStock { available: u32, requested: u32 },
}

impl fmt::Display for StockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientStock {
                available,
                requested,
            } => {
                write!(
                    f,
                    "Insufficient stock: {} available, {} requested",
                    available, requested
                )
            }
        }
    }
}

impl std::error::Error for StockError {}

#[derive(Debug, Clone)]
pub enum OrderError {
    OrderNotEditable,
    InvalidStatusTransition,
    EmptyOrder,
}

impl fmt::Display for OrderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OrderNotEditable => write!(f, "Order cannot be edited in current status"),
            Self::InvalidStatusTransition => write!(f, "Invalid status transition"),
            Self::EmptyOrder => write!(f, "Cannot confirm empty order"),
        }
    }
}

impl std::error::Error for OrderError {}
