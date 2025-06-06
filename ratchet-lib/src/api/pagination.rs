/// Unified pagination system for both REST and GraphQL APIs
use async_graphql::*;
use serde::{Deserialize, Serialize};

/// Unified pagination input that works for both APIs
#[derive(Debug, Clone, Serialize, Deserialize, InputObject)]
#[serde(rename_all = "camelCase")]
pub struct PaginationInput {
    /// Page number (1-based, default: 1)
    pub page: Option<u32>,
    /// Items per page (default: 25, max: 100)
    pub limit: Option<u32>,
    /// Offset-based pagination (alternative to page)
    pub offset: Option<u32>,
}

impl Default for PaginationInput {
    fn default() -> Self {
        Self {
            page: Some(1),
            limit: Some(25),
            offset: None,
        }
    }
}

impl PaginationInput {
    /// Get the calculated offset
    pub fn get_offset(&self) -> u32 {
        if let Some(offset) = self.offset {
            offset
        } else {
            let page = self.page.unwrap_or(1);
            let limit = self.get_limit();
            (page.saturating_sub(1)) * limit
        }
    }

    /// Get the limit with validation
    pub fn get_limit(&self) -> u32 {
        self.limit.unwrap_or(25).min(100).max(1)
    }

    /// Get the page number
    pub fn get_page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    /// Convert from Refine.dev style parameters
    pub fn from_refine(start: Option<u64>, end: Option<u64>) -> Self {
        let offset = start.unwrap_or(0) as u32;
        let limit = if let (Some(start), Some(end)) = (start, end) {
            (end.saturating_sub(start) as u32).min(100).max(1)
        } else {
            25
        };

        Self {
            page: None,
            limit: Some(limit),
            offset: Some(offset),
        }
    }

    /// Convert to Refine.dev style parameters for compatibility
    pub fn to_refine(&self) -> (u64, u64) {
        let offset = self.get_offset() as u64;
        let limit = self.get_limit() as u64;
        (offset, offset + limit)
    }
}

/// Unified sorting input
#[derive(Debug, Clone, Serialize, Deserialize, InputObject)]
#[serde(rename_all = "camelCase")]
pub struct SortInput {
    /// Field to sort by
    pub field: String,
    /// Sort direction (default: ASC)
    pub direction: Option<SortDirection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Enum)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SortDirection {
    Asc,
    Desc,
}

impl Default for SortDirection {
    fn default() -> Self {
        Self::Asc
    }
}

impl SortInput {
    /// Convert from Refine.dev style parameters
    pub fn from_refine(sort: Option<String>, order: Option<String>) -> Option<Self> {
        sort.map(|field| Self {
            field,
            direction: match order.as_deref() {
                Some("DESC") | Some("desc") => Some(SortDirection::Desc),
                _ => Some(SortDirection::Asc),
            },
        })
    }

    /// Get the direction with default
    pub fn get_direction(&self) -> SortDirection {
        self.direction.unwrap_or_default()
    }
}

/// Unified pagination metadata for responses
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct PaginationMeta {
    /// Current page number (1-based)
    pub page: u32,
    /// Items per page
    pub limit: u32,
    /// Total number of items
    pub total: u64,
    /// Total number of pages
    pub total_pages: u32,
    /// Whether there is a next page
    pub has_next: bool,
    /// Whether there is a previous page
    pub has_previous: bool,
    /// Current offset
    pub offset: u32,
}

impl PaginationMeta {
    pub fn new(pagination: &PaginationInput, total: u64) -> Self {
        let limit = pagination.get_limit();
        let offset = pagination.get_offset();
        let page = pagination.get_page();
        let total_pages = ((total as f64) / (limit as f64)).ceil() as u32;

        Self {
            page,
            limit,
            total,
            total_pages,
            has_next: (offset + limit) < total as u32,
            has_previous: page > 1,
            offset,
        }
    }

    /// Convert to headers for REST API compatibility
    pub fn to_headers(&self) -> Vec<(String, String)> {
        vec![
            ("X-Total-Count".to_string(), self.total.to_string()),
            ("X-Page-Offset".to_string(), self.offset.to_string()),
            ("X-Page-Limit".to_string(), self.limit.to_string()),
            ("X-Total-Pages".to_string(), self.total_pages.to_string()),
        ]
    }
}

/// Unified list response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResponse<T> {
    /// The list of items
    pub items: Vec<T>,
    /// Pagination metadata
    pub meta: PaginationMeta,
}

impl<T> ListResponse<T> {
    pub fn new(items: Vec<T>, pagination: &PaginationInput, total: u64) -> Self {
        let meta = PaginationMeta::new(pagination, total);
        Self { items, meta }
    }
}

// Manual GraphQL implementation to provide unique type names
#[async_graphql::Object(name = "TaskListResponse")]
impl ListResponse<crate::api::types::UnifiedTask> {
    async fn items(&self) -> &Vec<crate::api::types::UnifiedTask> {
        &self.items
    }

    async fn meta(&self) -> &PaginationMeta {
        &self.meta
    }
}

#[async_graphql::Object(name = "ExecutionListResponse")]
impl ListResponse<crate::api::types::UnifiedExecution> {
    async fn items(&self) -> &Vec<crate::api::types::UnifiedExecution> {
        &self.items
    }

    async fn meta(&self) -> &PaginationMeta {
        &self.meta
    }
}

#[async_graphql::Object(name = "JobListResponse")]
impl ListResponse<crate::api::types::UnifiedJob> {
    async fn items(&self) -> &Vec<crate::api::types::UnifiedJob> {
        &self.items
    }

    async fn meta(&self) -> &PaginationMeta {
        &self.meta
    }
}

#[async_graphql::Object(name = "ScheduleListResponse")]
impl ListResponse<crate::api::types::UnifiedSchedule> {
    async fn items(&self) -> &Vec<crate::api::types::UnifiedSchedule> {
        &self.items
    }

    async fn meta(&self) -> &PaginationMeta {
        &self.meta
    }
}

/// Filter input for generic filtering
#[derive(Debug, Clone, Serialize, Deserialize, InputObject)]
pub struct FilterInput {
    /// Field name to filter by
    pub field: String,
    /// Filter operator
    pub operator: FilterOperator,
    /// Value to filter by (as string, will be parsed based on field type)
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Enum)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FilterOperator {
    /// Exact match
    Eq,
    /// Not equal
    Ne,
    /// Contains (for strings)
    Contains,
    /// Starts with (for strings)  
    StartsWith,
    /// Ends with (for strings)
    EndsWith,
    /// Greater than
    Gt,
    /// Greater than or equal
    Gte,
    /// Less than
    Lt,
    /// Less than or equal
    Lte,
    /// In list of values
    In,
    /// Not in list of values
    NotIn,
}

/// Combined query input for list operations
#[derive(Debug, Clone, Serialize, Deserialize, InputObject)]
#[serde(rename_all = "camelCase")]
pub struct ListInput {
    /// Pagination parameters
    pub pagination: Option<PaginationInput>,
    /// Sorting parameters
    pub sort: Option<SortInput>,
    /// Filter parameters
    pub filters: Option<Vec<FilterInput>>,
}

impl Default for ListInput {
    fn default() -> Self {
        Self {
            pagination: Some(PaginationInput::default()),
            sort: None,
            filters: None,
        }
    }
}

impl ListInput {
    /// Get pagination with defaults
    pub fn get_pagination(&self) -> PaginationInput {
        self.pagination.clone().unwrap_or_default()
    }
}
