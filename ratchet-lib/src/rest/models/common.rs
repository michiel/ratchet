use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use axum::http::StatusCode;

/// Standard API response wrapper for Refine.dev compatibility
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

/// Standard API error response for Refine.dev compatibility
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub message: String,
    #[serde(rename = "statusCode")]
    pub status_code: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<String>>,
}

impl ApiError {
    pub fn new(status_code: StatusCode, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status_code: status_code.as_u16(),
            errors: None,
        }
    }

    pub fn with_errors(mut self, errors: Vec<String>) -> Self {
        self.errors = Some(errors);
        self
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    pub fn method_not_allowed(message: impl Into<String>) -> Self {
        Self::new(StatusCode::METHOD_NOT_ALLOWED, message)
    }
}

/// Pagination query parameters for Refine.dev compatibility
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(rename = "_start")]
    pub start: Option<u64>,
    #[serde(rename = "_end")]
    pub end: Option<u64>,
}

impl PaginationQuery {
    pub fn offset(&self) -> u64 {
        self.start.unwrap_or(0)
    }

    pub fn limit(&self) -> u64 {
        let start = self.start.unwrap_or(0);
        let end = self.end.unwrap_or(start + 25);
        if end > start { end - start } else { 25 }
    }
}

/// Sorting query parameters for Refine.dev compatibility
#[derive(Debug, Deserialize)]
pub struct SortQuery {
    #[serde(rename = "_sort")]
    pub sort: Option<String>,
    #[serde(rename = "_order")]
    pub order: Option<String>,
}

impl SortQuery {
    pub fn sort_field(&self) -> Option<&str> {
        self.sort.as_deref()
    }

    pub fn sort_direction(&self) -> SortDirection {
        match self.order.as_deref() {
            Some("DESC") | Some("desc") => SortDirection::Desc,
            _ => SortDirection::Asc,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// Generic filter parameters
#[derive(Debug, Deserialize)]
pub struct FilterQuery {
    #[serde(flatten)]
    pub filters: HashMap<String, String>,
}

impl FilterQuery {
    pub fn get_filter(&self, key: &str) -> Option<&String> {
        self.filters.get(key)
    }

    pub fn get_like_filter(&self, key: &str) -> Option<&String> {
        self.filters.get(&format!("{}_like", key))
    }

    pub fn get_gte_filter(&self, key: &str) -> Option<&String> {
        self.filters.get(&format!("{}_gte", key))
    }

    pub fn get_lte_filter(&self, key: &str) -> Option<&String> {
        self.filters.get(&format!("{}_lte", key))
    }
}

/// Combined query parameters for list endpoints
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(flatten)]
    pub pagination: PaginationQuery,
    #[serde(flatten)]
    pub sort: SortQuery,
    #[serde(flatten)]
    pub filter: FilterQuery,
}

/// Standard list response metadata
#[derive(Debug, Serialize)]
pub struct ListMeta {
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
}