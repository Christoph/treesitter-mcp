//! Query pattern for CQRS

use crate::domain::models::{Order, OrderId, UserId};
use crate::domain::value_objects::Money;
use serde::{Deserialize, Serialize};

/// Query trait for query handlers
pub trait Query {
    type Result;
    type Error: std::error::Error;
}

/// Get order by ID query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOrderQuery {
    pub order_id: OrderId,
}

impl Query for GetOrderQuery {
    type Result = Option<Order>;
    type Error = crate::application::services::ServiceError;
}

/// Get orders by user query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUserOrdersQuery {
    pub user_id: UserId,
}

impl Query for GetUserOrdersQuery {
    type Result = Vec<Order>;
    type Error = crate::application::services::ServiceError;
}

/// Calculate revenue query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculateRevenueQuery;

impl Query for CalculateRevenueQuery {
    type Result = Money;
    type Error = crate::application::services::ServiceError;
}
