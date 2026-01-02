//! Domain events for event sourcing

use super::models::{OrderId, UserId};
use super::value_objects::Email;
use serde::{Deserialize, Serialize};

/// Domain events representing state changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainEvent {
    UserCreated { user_id: UserId },
    UserDeactivated { user_id: UserId },
    UserEmailChanged { user_id: UserId, new_email: Email },
    OrderCreated { order_id: OrderId },
    OrderConfirmed { order_id: OrderId },
    OrderShipped { order_id: OrderId },
}

impl DomainEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::UserCreated { .. } => "UserCreated",
            Self::UserDeactivated { .. } => "UserDeactivated",
            Self::UserEmailChanged { .. } => "UserEmailChanged",
            Self::OrderCreated { .. } => "OrderCreated",
            Self::OrderConfirmed { .. } => "OrderConfirmed",
            Self::OrderShipped { .. } => "OrderShipped",
        }
    }
}
