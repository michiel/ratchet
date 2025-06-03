//! Storage entity definitions

pub mod task;
pub mod execution;
pub mod job;
pub mod schedule;
pub mod delivery_result;

// Common traits and utilities for entities
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Common fields for all entities
pub trait Entity {
    /// Get the entity's primary key
    fn id(&self) -> i32;
    
    /// Get the entity's UUID
    fn uuid(&self) -> Uuid;
    
    /// Get when the entity was created
    fn created_at(&self) -> DateTime<Utc>;
    
    /// Get when the entity was last updated
    fn updated_at(&self) -> DateTime<Utc>;
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    /// Page number (0-based)
    pub page: u32,
    
    /// Number of items per page
    pub limit: u32,
    
    /// Total number of items (if known)
    pub total: Option<u32>,
}

impl Pagination {
    /// Create new pagination parameters
    pub fn new(page: u32, limit: u32) -> Self {
        Self {
            page,
            limit: limit.min(1000), // Cap at 1000 items per page
            total: None,
        }
    }
    
    /// Calculate offset for database queries
    pub fn offset(&self) -> u32 {
        self.page * self.limit
    }
    
    /// Create pagination with total count
    pub fn with_total(mut self, total: u32) -> Self {
        self.total = Some(total);
        self
    }
}

/// Sorting parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sort {
    /// Field to sort by
    pub field: String,
    
    /// Sort direction
    pub direction: SortDirection,
}

/// Sort direction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Asc,
    Desc,
}

impl Default for Sort {
    fn default() -> Self {
        Self {
            field: "created_at".to_string(),
            direction: SortDirection::Desc,
        }
    }
}

/// Filter parameters for queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Text search query
    pub search: Option<String>,
    
    /// Status filters
    pub status: Option<Vec<String>>,
    
    /// Date range filters
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    
    /// Additional custom filters
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

/// Query builder for combining pagination, sorting, and filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    /// Pagination parameters
    pub pagination: Pagination,
    
    /// Sort parameters
    pub sort: Sort,
    
    /// Filter parameters
    pub filter: Filter,
}

impl Query {
    /// Create a new query with default parameters
    pub fn new() -> Self {
        Self {
            pagination: Pagination::new(0, 50),
            sort: Sort::default(),
            filter: Filter::default(),
        }
    }
    
    /// Set pagination
    pub fn paginate(mut self, page: u32, limit: u32) -> Self {
        self.pagination = Pagination::new(page, limit);
        self
    }
    
    /// Set sorting
    pub fn sort_by(mut self, field: impl Into<String>, direction: SortDirection) -> Self {
        self.sort = Sort {
            field: field.into(),
            direction,
        };
        self
    }
    
    /// Add text search
    pub fn search(mut self, query: impl Into<String>) -> Self {
        self.filter.search = Some(query.into());
        self
    }
    
    /// Add status filter
    pub fn with_status(mut self, status: Vec<String>) -> Self {
        self.filter.status = Some(status);
        self
    }
    
    /// Add date range filter
    pub fn created_between(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.filter.created_after = Some(start);
        self.filter.created_before = Some(end);
        self
    }
}