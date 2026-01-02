//! Complex web service demonstrating realistic Rust patterns
//! 
//! This fixture tests LLM agent ability to navigate:
//! - Async/await patterns
//! - Trait hierarchies and generic bounds
//! - Error handling with custom types
//! - Macro usage
//! - Module organization

pub mod domain;
pub mod infrastructure;
pub mod application;
pub mod api;

pub use domain::models::{User, Order, Product};
pub use domain::repositories::{Repository, UserRepository};
pub use application::services::OrderService;
