//! Repository trait definitions for persistence abstraction

use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;

use super::models::{User, UserId, Order, OrderId, Product, ProductId};

/// Generic repository trait with async operations
#[async_trait]
pub trait Repository<T, ID> {
    type Error: std::error::Error + Send + Sync + 'static;
    
    async fn find_by_id(&self, id: ID) -> Result<Option<T>, Self::Error>;
    async fn save(&mut self, entity: T) -> Result<(), Self::Error>;
    async fn delete(&mut self, id: ID) -> Result<(), Self::Error>;
}

/// User-specific repository with custom queries
#[async_trait]
pub trait UserRepository: Repository<User, UserId> {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, Self::Error>;
    async fn find_active_users(&self) -> Result<Vec<User>, Self::Error>;
}

/// Order-specific repository
#[async_trait]
pub trait OrderRepository: Repository<Order, OrderId> {
    async fn find_by_user(&self, user_id: UserId) -> Result<Vec<Order>, Self::Error>;
    async fn find_pending_orders(&self) -> Result<Vec<Order>, Self::Error>;
}

/// Product-specific repository
#[async_trait]
pub trait ProductRepository: Repository<Product, ProductId> {
    async fn find_in_stock(&self) -> Result<Vec<Product>, Self::Error>;
    async fn search_by_name(&self, query: &str) -> Result<Vec<Product>, Self::Error>;
}

/// Unit of work pattern for transaction management
pub trait UnitOfWork {
    type Error: std::error::Error + Send + Sync + 'static;
    
    fn commit(&mut self) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>>;
    fn rollback(&mut self) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>>;
}
