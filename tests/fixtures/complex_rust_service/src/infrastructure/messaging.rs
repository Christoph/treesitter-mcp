//! Event publishing infrastructure

use async_trait::async_trait;
use crate::domain::events::DomainEvent;

/// Event publisher trait
#[async_trait]
pub trait EventPublisher {
    type Error: std::error::Error + Send + Sync + 'static;
    
    async fn publish(&self, event: DomainEvent) -> Result<(), Self::Error>;
    async fn publish_batch(&self, events: Vec<DomainEvent>) -> Result<(), Self::Error>;
}

/// In-memory event publisher for testing
pub struct InMemoryEventPublisher {
    events: std::sync::Arc<std::sync::RwLock<Vec<DomainEvent>>>,
}

impl InMemoryEventPublisher {
    pub fn new() -> Self {
        Self {
            events: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }
    
    pub fn published_events(&self) -> Vec<DomainEvent> {
        self.events.read().unwrap().clone()
    }
    
    pub fn clear(&self) {
        self.events.write().unwrap().clear();
    }
}

#[derive(Debug)]
pub struct PublishError(String);

impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Publish error: {}", self.0)
    }
}

impl std::error::Error for PublishError {}

#[async_trait]
impl EventPublisher for InMemoryEventPublisher {
    type Error = PublishError;
    
    async fn publish(&self, event: DomainEvent) -> Result<(), Self::Error> {
        self.events.write()
            .map_err(|e| PublishError(format!("Lock error: {}", e)))?
            .push(event);
        Ok(())
    }
    
    async fn publish_batch(&self, events: Vec<DomainEvent>) -> Result<(), Self::Error> {
        self.events.write()
            .map_err(|e| PublishError(format!("Lock error: {}", e)))?
            .extend(events);
        Ok(())
    }
}
