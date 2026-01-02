//! API handlers (would integrate with web framework)

use crate::api::dto::{OrderDto, CreateOrderRequest, AddProductRequest};
use crate::application::services::OrderService;
use crate::domain::models::{OrderId, UserId, ProductId};
use crate::domain::repositories::{OrderRepository, UserRepository, ProductRepository};
use crate::infrastructure::messaging::EventPublisher;

/// Order API handler
pub struct OrderHandler<OR, UR, PR, EP>
where
    OR: OrderRepository,
    UR: UserRepository,
    PR: ProductRepository,
    EP: EventPublisher,
{
    service: OrderService<OR, UR, PR, EP>,
}

impl<OR, UR, PR, EP> OrderHandler<OR, UR, PR, EP>
where
    OR: OrderRepository,
    UR: UserRepository,
    PR: ProductRepository,
    EP: EventPublisher,
{
    pub fn new(service: OrderService<OR, UR, PR, EP>) -> Self {
        Self { service }
    }
    
    /// POST /orders
    pub async fn create_order(
        &mut self,
        request: CreateOrderRequest,
    ) -> Result<OrderDto, HandlerError> {
        let order_id = OrderId(generate_id());
        let user_id = UserId(request.user_id);
        
        let order = self.service.create_order(order_id, user_id).await
            .map_err(|e| HandlerError::ServiceError(e.to_string()))?;
        
        Ok(order.into())
    }
    
    /// POST /orders/{id}/items
    pub async fn add_product(
        &mut self,
        order_id: u64,
        request: AddProductRequest,
    ) -> Result<(), HandlerError> {
        self.service.add_product_to_order(
            OrderId(order_id),
            ProductId(request.product_id),
            request.quantity,
        ).await
        .map_err(|e| HandlerError::ServiceError(e.to_string()))?;
        
        Ok(())
    }
    
    /// POST /orders/{id}/confirm
    pub async fn confirm_order(
        &mut self,
        order_id: u64,
    ) -> Result<(), HandlerError> {
        self.service.confirm_order(OrderId(order_id)).await
            .map_err(|e| HandlerError::ServiceError(e.to_string()))?;
        
        Ok(())
    }
}

#[derive(Debug)]
pub enum HandlerError {
    ServiceError(String),
    ValidationError(String),
}

impl std::fmt::Display for HandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceError(e) => write!(f, "Service error: {}", e),
            Self::ValidationError(e) => write!(f, "Validation error: {}", e),
        }
    }
}

impl std::error::Error for HandlerError {}

fn generate_id() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
