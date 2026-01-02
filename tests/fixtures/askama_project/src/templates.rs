use crate::models::{Item, Statistics, User};
use askama::Template;

// Simple single-level template
#[derive(Template)]
#[template(path = "calculator.html")]
pub struct CalculatorTemplate {
    pub result: i32,
    pub history: Vec<String>,
}

// Nested path template with 3-level deep types
#[derive(Template)]
#[template(path = "admin/dashboard.html")]
pub struct DashboardTemplate {
    pub user_name: String,
    pub stats: Statistics, // Level 1 nesting
    pub recent_items: Vec<Item>,
}

// Generic template
#[derive(Template)]
#[template(path = "list.html")]
pub struct ListTemplate<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub total_pages: u32,
}

// Multiple structs same template
#[derive(Template)]
#[template(path = "shared/form.html")]
pub struct LoginForm {
    pub username: String,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "shared/form.html")]
pub struct RegisterForm {
    pub username: String,
    pub email: String,
    pub errors: Vec<String>,
}
