// Level 1 nested type
pub struct Statistics {
    pub total_users: u32,
    pub active_sessions: u32,
    pub performance: PerformanceMetrics, // Level 2 nesting
}

// Level 2 nested type
pub struct PerformanceMetrics {
    pub avg_response_time_ms: f64,
    pub error_rate: f64,
    pub detailed_stats: DetailedStats, // Level 3 nesting
}

// Level 3 nested type (deepest we resolve)
pub struct DetailedStats {
    pub p95_latency: f64,
    pub p99_latency: f64,
    pub requests_per_second: f64,
}

pub struct Item {
    pub id: u64,
    pub name: String,
    pub metadata: ItemMetadata, // Level 2 nesting
}

pub struct ItemMetadata {
    pub created_at: String,
    pub updated_at: String,
}

pub struct User {
    pub id: u64,
    pub email: String,
}
