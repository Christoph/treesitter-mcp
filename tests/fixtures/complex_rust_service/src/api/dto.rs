//! Data Transfer Objects for API layer

use crate::domain::models::{Order, OrderId, Product, ProductId, User, UserId};
use crate::domain::value_objects::{Money, OrderStatus};
use serde::{Deserialize, Serialize};

/// User DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDto {
    pub id: u64,
    pub email: String,
    pub name: String,
    pub is_active: bool,
}

impl From<User> for UserDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id.0,
            email: user.email.as_str().to_string(),
            name: user.name,
            is_active: user.is_active,
        }
    }
}

/// Order DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderDto {
    pub id: u64,
    pub user_id: u64,
    pub items: Vec<OrderItemDto>,
    pub status: String,
    pub total_cents: i64,
}

impl From<Order> for OrderDto {
    fn from(order: Order) -> Self {
        Self {
            id: order.id.0,
            user_id: order.user_id.0,
            items: order.items.into_iter().map(Into::into).collect(),
            status: order.status.to_string(),
            total_cents: order.total.cents(),
        }
    }
}

/// Order item DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemDto {
    pub product_id: u64,
    pub product_name: String,
    pub unit_price_cents: i64,
    pub quantity: u32,
}

impl From<crate::domain::models::OrderItem> for OrderItemDto {
    fn from(item: crate::domain::models::OrderItem) -> Self {
        Self {
            product_id: item.product_id.0,
            product_name: item.product_name,
            unit_price_cents: item.unit_price.cents(),
            quantity: item.quantity,
        }
    }
}

/// Product DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductDto {
    pub id: u64,
    pub name: String,
    pub price_cents: i64,
    pub stock: u32,
}

impl From<Product> for ProductDto {
    fn from(product: Product) -> Self {
        Self {
            id: product.id.0,
            name: product.name,
            price_cents: product.price.cents(),
            stock: product.stock,
        }
    }
}

/// Create order request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrderRequest {
    pub user_id: u64,
}

/// Add product request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddProductRequest {
    pub product_id: u64,
    pub quantity: u32,
}
