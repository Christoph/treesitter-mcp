//! In-memory repository implementations for testing

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::domain::models::{User, UserId, Order, OrderId, Product, ProductId};
use crate::domain::repositories::{Repository, UserRepository, OrderRepository, ProductRepository};

#[derive(Debug, Clone)]
pub struct RepositoryError(String);

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Repository error: {}", self.0)
    }
}

impl std::error::Error for RepositoryError {}

/// In-memory user repository
#[derive(Clone)]
pub struct InMemoryUserRepository {
    users: Arc<RwLock<HashMap<UserId, User>>>,
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Repository<User, UserId> for InMemoryUserRepository {
    type Error = RepositoryError;
    
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, Self::Error> {
        let users = self.users.read()
            .map_err(|e| RepositoryError(format!("Lock error: {}", e)))?;
        Ok(users.get(&id).cloned())
    }
    
    async fn save(&mut self, entity: User) -> Result<(), Self::Error> {
        let mut users = self.users.write()
            .map_err(|e| RepositoryError(format!("Lock error: {}", e)))?;
        users.insert(entity.id, entity);
        Ok(())
    }
    
    async fn delete(&mut self, id: UserId) -> Result<(), Self::Error> {
        let mut users = self.users.write()
            .map_err(|e| RepositoryError(format!("Lock error: {}", e)))?;
        users.remove(&id);
        Ok(())
    }
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, Self::Error> {
        let users = self.users.read()
            .map_err(|e| RepositoryError(format!("Lock error: {}", e)))?;
        Ok(users.values()
            .find(|u| u.email.as_str() == email)
            .cloned())
    }
    
    async fn find_active_users(&self) -> Result<Vec<User>, Self::Error> {
        let users = self.users.read()
            .map_err(|e| RepositoryError(format!("Lock error: {}", e)))?;
        Ok(users.values()
            .filter(|u| u.is_active)
            .cloned()
            .collect())
    }
}

/// In-memory order repository
#[derive(Clone)]
pub struct InMemoryOrderRepository {
    orders: Arc<RwLock<HashMap<OrderId, Order>>>,
}

impl InMemoryOrderRepository {
    pub fn new() -> Self {
        Self {
            orders: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Repository<Order, OrderId> for InMemoryOrderRepository {
    type Error = RepositoryError;
    
    async fn find_by_id(&self, id: OrderId) -> Result<Option<Order>, Self::Error> {
        let orders = self.orders.read()
            .map_err(|e| RepositoryError(format!("Lock error: {}", e)))?;
        Ok(orders.get(&id).cloned())
    }
    
    async fn save(&mut self, entity: Order) -> Result<(), Self::Error> {
        let mut orders = self.orders.write()
            .map_err(|e| RepositoryError(format!("Lock error: {}", e)))?;
        orders.insert(entity.id, entity);
        Ok(())
    }
    
    async fn delete(&mut self, id: OrderId) -> Result<(), Self::Error> {
        let mut orders = self.orders.write()
            .map_err(|e| RepositoryError(format!("Lock error: {}", e)))?;
        orders.remove(&id);
        Ok(())
    }
}

#[async_trait]
impl OrderRepository for InMemoryOrderRepository {
    async fn find_by_user(&self, user_id: UserId) -> Result<Vec<Order>, Self::Error> {
        let orders = self.orders.read()
            .map_err(|e| RepositoryError(format!("Lock error: {}", e)))?;
        Ok(orders.values()
            .filter(|o| o.user_id == user_id)
            .cloned()
            .collect())
    }
    
    async fn find_pending_orders(&self) -> Result<Vec<Order>, Self::Error> {
        let orders = self.orders.read()
            .map_err(|e| RepositoryError(format!("Lock error: {}", e)))?;
        Ok(orders.values()
            .filter(|o| o.status == crate::domain::value_objects::OrderStatus::Pending)
            .cloned()
            .collect())
    }
}

/// In-memory product repository
#[derive(Clone)]
pub struct InMemoryProductRepository {
    products: Arc<RwLock<HashMap<ProductId, Product>>>,
}

impl InMemoryProductRepository {
    pub fn new() -> Self {
        Self {
            products: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Repository<Product, ProductId> for InMemoryProductRepository {
    type Error = RepositoryError;
    
    async fn find_by_id(&self, id: ProductId) -> Result<Option<Product>, Self::Error> {
        let products = self.products.read()
            .map_err(|e| RepositoryError(format!("Lock error: {}", e)))?;
        Ok(products.get(&id).cloned())
    }
    
    async fn save(&mut self, entity: Product) -> Result<(), Self::Error> {
        let mut products = self.products.write()
            .map_err(|e| RepositoryError(format!("Lock error: {}", e)))?;
        products.insert(entity.id, entity);
        Ok(())
    }
    
    async fn delete(&mut self, id: ProductId) -> Result<(), Self::Error> {
        let mut products = self.products.write()
            .map_err(|e| RepositoryError(format!("Lock error: {}", e)))?;
        products.remove(&id);
        Ok(())
    }
}

#[async_trait]
impl ProductRepository for InMemoryProductRepository {
    async fn find_in_stock(&self) -> Result<Vec<Product>, Self::Error> {
        let products = self.products.read()
            .map_err(|e| RepositoryError(format!("Lock error: {}", e)))?;
        Ok(products.values()
            .filter(|p| p.stock > 0)
            .cloned()
            .collect())
    }
    
    async fn search_by_name(&self, query: &str) -> Result<Vec<Product>, Self::Error> {
        let products = self.products.read()
            .map_err(|e| RepositoryError(format!("Lock error: {}", e)))?;
        let query_lower = query.to_lowercase();
        Ok(products.values()
            .filter(|p| p.name.to_lowercase().contains(&query_lower))
            .cloned()
            .collect())
    }
}
