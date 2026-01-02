//! Application services orchestrating business logic

use crate::domain::models::{Order, OrderId, User, UserId, Product, ProductId};
use crate::domain::repositories::{OrderRepository, UserRepository, ProductRepository};
use crate::domain::value_objects::Money;
use crate::infrastructure::messaging::EventPublisher;

/// Order service with complex business logic
pub struct OrderService<OR, UR, PR, EP> 
where
    OR: OrderRepository,
    UR: UserRepository,
    PR: ProductRepository,
    EP: EventPublisher,
{
    order_repo: OR,
    user_repo: UR,
    product_repo: PR,
    event_publisher: EP,
}

impl<OR, UR, PR, EP> OrderService<OR, UR, PR, EP>
where
    OR: OrderRepository,
    UR: UserRepository,
    PR: ProductRepository,
    EP: EventPublisher,
{
    pub fn new(
        order_repo: OR,
        user_repo: UR,
        product_repo: PR,
        event_publisher: EP,
    ) -> Self {
        Self {
            order_repo,
            user_repo,
            product_repo,
            event_publisher,
        }
    }
    
    /// Creates a new order with validation
    pub async fn create_order(
        &mut self,
        order_id: OrderId,
        user_id: UserId,
    ) -> Result<Order, ServiceError> {
        // Verify user exists and is active
        let user = self.user_repo.find_by_id(user_id).await
            .map_err(|e| ServiceError::RepositoryError(e.to_string()))?
            .ok_or(ServiceError::UserNotFound)?;
            
        if !user.is_active {
            return Err(ServiceError::UserNotActive);
        }
        
        let order = Order::new(order_id, user_id);
        Ok(order)
    }
    
    /// Adds a product to an order with stock validation
    pub async fn add_product_to_order(
        &mut self,
        order_id: OrderId,
        product_id: ProductId,
        quantity: u32,
    ) -> Result<(), ServiceError> {
        // Load order
        let mut order = self.order_repo.find_by_id(order_id).await
            .map_err(|e| ServiceError::RepositoryError(e.to_string()))?
            .ok_or(ServiceError::OrderNotFound)?;
        
        // Load product and check stock
        let mut product = self.product_repo.find_by_id(product_id).await
            .map_err(|e| ServiceError::RepositoryError(e.to_string()))?
            .ok_or(ServiceError::ProductNotFound)?;
            
        product.reserve_stock(quantity)
            .map_err(|e| ServiceError::StockError(e.to_string()))?;
        
        // Add to order
        order.add_item(&product, quantity)
            .map_err(|e| ServiceError::OrderError(e.to_string()))?;
        
        // Save changes
        self.product_repo.save(product).await
            .map_err(|e| ServiceError::RepositoryError(e.to_string()))?;
        self.order_repo.save(order).await
            .map_err(|e| ServiceError::RepositoryError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Confirms an order and publishes events
    pub async fn confirm_order(
        &mut self,
        order_id: OrderId,
    ) -> Result<(), ServiceError> {
        let mut order = self.order_repo.find_by_id(order_id).await
            .map_err(|e| ServiceError::RepositoryError(e.to_string()))?
            .ok_or(ServiceError::OrderNotFound)?;
        
        order.confirm()
            .map_err(|e| ServiceError::OrderError(e.to_string()))?;
        
        let events = order.take_events();
        
        self.order_repo.save(order).await
            .map_err(|e| ServiceError::RepositoryError(e.to_string()))?;
        
        self.event_publisher.publish_batch(events).await
            .map_err(|e| ServiceError::EventPublishError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Calculates total revenue from all orders
    pub async fn calculate_total_revenue(&self) -> Result<Money, ServiceError> {
        let orders = self.order_repo.find_pending_orders().await
            .map_err(|e| ServiceError::RepositoryError(e.to_string()))?;
        
        let total = orders.iter()
            .map(|o| o.total)
            .sum();
        
        Ok(total)
    }
}

/// Service errors
#[derive(Debug)]
pub enum ServiceError {
    UserNotFound,
    UserNotActive,
    OrderNotFound,
    ProductNotFound,
    StockError(String),
    OrderError(String),
    RepositoryError(String),
    EventPublishError(String),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserNotFound => write!(f, "User not found"),
            Self::UserNotActive => write!(f, "User is not active"),
            Self::OrderNotFound => write!(f, "Order not found"),
            Self::ProductNotFound => write!(f, "Product not found"),
            Self::StockError(e) => write!(f, "Stock error: {}", e),
            Self::OrderError(e) => write!(f, "Order error: {}", e),
            Self::RepositoryError(e) => write!(f, "Repository error: {}", e),
            Self::EventPublishError(e) => write!(f, "Event publish error: {}", e),
        }
    }
}

impl std::error::Error for ServiceError {}
