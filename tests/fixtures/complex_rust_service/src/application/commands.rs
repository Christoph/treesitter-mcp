//! Command pattern for CQRS

use crate::domain::models::{OrderId, ProductId, UserId};
use serde::{Deserialize, Serialize};

/// Command trait for command handlers
pub trait Command {
    type Result;
    type Error: std::error::Error;
}

/// Create order command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrderCommand {
    pub order_id: OrderId,
    pub user_id: UserId,
}

impl Command for CreateOrderCommand {
    type Result = OrderId;
    type Error = crate::application::services::ServiceError;
}

/// Add product to order command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddProductToOrderCommand {
    pub order_id: OrderId,
    pub product_id: ProductId,
    pub quantity: u32,
}

impl Command for AddProductToOrderCommand {
    type Result = ();
    type Error = crate::application::services::ServiceError;
}

/// Confirm order command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmOrderCommand {
    pub order_id: OrderId,
}

impl Command for ConfirmOrderCommand {
    type Result = ();
    type Error = crate::application::services::ServiceError;
}
