use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::api::pagination::{PaginationInput, PaginationMeta};

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

/// Re-export unified list response
pub use crate::api::pagination::ListResponse as ApiListResponse;

// Unified API error is already imported above

/// Legacy API error response for backward compatibility
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[cfg(debug_assertions)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_info: Option<String>,
}

impl ApiError {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            error_code: None,
            request_id: None,
            timestamp: Utc::now(),
            path: None,
            #[cfg(debug_assertions)]
            debug_info: None,
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.error_code = Some(code.into());
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    #[cfg(debug_assertions)]
    pub fn with_debug_info(mut self, debug_info: impl Into<String>) -> Self {
        self.debug_info = Some(debug_info.into());
        self
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(message).with_code("NOT_FOUND")
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(message).with_code("BAD_REQUEST")
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(message).with_code("INTERNAL_ERROR")
    }

    pub fn method_not_allowed(message: impl Into<String>) -> Self {
        Self::new(message).with_code("METHOD_NOT_ALLOWED")
    }

    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(message).with_code("SERVICE_UNAVAILABLE")
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(message).with_code("CONFLICT")
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(message).with_code("TIMEOUT")
    }
}

/// Legacy pagination query parameters for Refine.dev compatibility
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
        if end > start {
            end - start
        } else {
            25
        }
    }

    /// Convert to unified pagination input
    pub fn to_unified(&self) -> PaginationInput {
        PaginationInput::from_refine(self.start, self.end)
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
    // Pagination parameters
    #[serde(rename = "_start")]
    pub start: Option<u64>,
    #[serde(rename = "_end")]
    pub end: Option<u64>,

    // Sort parameters
    #[serde(rename = "_sort")]
    pub sort: Option<String>,
    #[serde(rename = "_order")]
    pub order: Option<String>,

    // All other parameters as filters
    #[serde(flatten)]
    pub filters: HashMap<String, String>,
}

impl ListQuery {
    pub fn pagination(&self) -> PaginationQuery {
        PaginationQuery {
            start: self.start,
            end: self.end,
        }
    }

    pub fn sort(&self) -> SortQuery {
        SortQuery {
            sort: self.sort.clone(),
            order: self.order.clone(),
        }
    }

    pub fn filter(&self) -> FilterQuery {
        FilterQuery {
            filters: self.filters.clone(),
        }
    }
}

/// Legacy list response metadata (use PaginationMeta instead)
#[derive(Debug, Serialize)]
pub struct ListMeta {
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
}

impl From<PaginationMeta> for ListMeta {
    fn from(meta: PaginationMeta) -> Self {
        Self {
            total: meta.total,
            offset: meta.offset as u64,
            limit: meta.limit as u64,
        }
    }
}
