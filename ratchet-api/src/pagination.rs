//! Unified pagination for REST and GraphQL APIs

use serde::{Deserialize, Serialize};

/// Pagination input parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInput {
    /// Page number (0-based)
    pub page: Option<u32>,
    
    /// Number of items per page
    pub limit: Option<u32>,
    
    /// Offset for cursor-based pagination
    pub offset: Option<u32>,
    
    /// Sort field
    pub sort: Option<String>,
    
    /// Sort order
    pub order: Option<SortOrder>,
    
    /// Legacy Refine.dev parameters
    pub _start: Option<u32>,
    pub _end: Option<u32>,
    pub _sort: Option<String>,
    pub _order: Option<String>,
}

/// Sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

/// Pagination metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMeta {
    /// Current page number
    pub page: u32,
    
    /// Number of items per page
    pub limit: u32,
    
    /// Total number of items
    pub total: u64,
    
    /// Total number of pages
    pub pages: u32,
    
    /// Whether there are more pages
    pub has_next: bool,
    
    /// Whether there are previous pages
    pub has_prev: bool,
    
    /// Offset of first item
    pub offset: u32,
}

/// List response with pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse<T> {
    /// List of items
    pub data: Vec<T>,
    
    /// Pagination metadata
    pub meta: PaginationMeta,
}

impl Default for PaginationInput {
    fn default() -> Self {
        Self {
            page: Some(0),
            limit: Some(20),
            offset: None,
            sort: None,
            order: Some(SortOrder::Asc),
            _start: None,
            _end: None,
            _sort: None,
            _order: None,
        }
    }
}


impl PaginationInput {
    /// Create new pagination input
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set page number
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }
    
    /// Set limit
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
    
    /// Set sorting
    pub fn sort(mut self, field: impl Into<String>, order: SortOrder) -> Self {
        self.sort = Some(field.into());
        self.order = Some(order);
        self
    }
    
    /// Get effective page number
    pub fn effective_page(&self) -> u32 {
        self.page.unwrap_or(0)
    }
    
    /// Get effective limit
    pub fn effective_limit(&self) -> u32 {
        self.limit.unwrap_or(20).min(100) // Cap at 100 items
    }
    
    /// Get effective offset
    pub fn effective_offset(&self) -> u32 {
        if let Some(offset) = self.offset {
            offset
        } else if let (Some(_start), Some(_end)) = (self._start, self._end) {
            // Legacy Refine.dev pagination
            self._start.unwrap_or(0)
        } else {
            self.effective_page() * self.effective_limit()
        }
    }
    
    /// Get effective sort field
    pub fn effective_sort(&self) -> Option<String> {
        self.sort.clone().or_else(|| self._sort.clone())
    }
    
    /// Get effective sort order
    pub fn effective_order(&self) -> SortOrder {
        if let Some(order) = self.order {
            order
        } else if let Some(ref order_str) = self._order {
            match order_str.to_lowercase().as_str() {
                "desc" | "descending" => SortOrder::Desc,
                _ => SortOrder::Asc,
            }
        } else {
            SortOrder::Asc
        }
    }
    
    /// Convert to storage query parameters
    /// TODO: Re-implement when storage integration is ready
    pub fn to_storage_query(&self) -> String {
        format!(
            "LIMIT {} OFFSET {} ORDER BY {} {}",
            self.effective_limit(),
            self.effective_offset(),
            self.effective_sort().unwrap_or_else(|| "id".to_string()),
            match self.effective_order() {
                SortOrder::Asc => "ASC",
                SortOrder::Desc => "DESC",
            }
        )
    }
    
    /// Validate pagination parameters
    pub fn validate(&self) -> Result<(), String> {
        if let Some(limit) = self.limit {
            if limit == 0 {
                return Err("Limit cannot be zero".to_string());
            }
            if limit > 100 {
                return Err("Limit cannot exceed 100".to_string());
            }
        }
        
        if let Some(page) = self.page {
            if page > 10000 {
                return Err("Page number too large".to_string());
            }
        }
        
        // Validate Refine.dev parameters
        if let (Some(start), Some(end)) = (self._start, self._end) {
            if start >= end {
                return Err("Start must be less than end".to_string());
            }
            if end - start > 100 {
                return Err("Range cannot exceed 100 items".to_string());
            }
        }
        
        Ok(())
    }
}

impl PaginationMeta {
    /// Create pagination metadata
    pub fn new(page: u32, limit: u32, total: u64) -> Self {
        let pages = if total == 0 { 1 } else { ((total - 1) / limit as u64 + 1) as u32 };
        let offset = page * limit;
        
        Self {
            page,
            limit,
            total,
            pages,
            has_next: page + 1 < pages,
            has_prev: page > 0,
            offset,
        }
    }
    
    // Create from storage pagination
    // TODO: Re-implement when storage integration is ready
    // pub fn from_storage_pagination(
    //     storage_pagination: &ratchet_storage::entities::Pagination,
    //     total: u64,
    // ) -> Self {
    //     Self::new(storage_pagination.page, storage_pagination.limit, total)
    // }
}

impl<T> ListResponse<T> {
    /// Create a new list response
    pub fn new(data: Vec<T>, meta: PaginationMeta) -> Self {
        Self { data, meta }
    }
    
    /// Create from paginated data
    pub fn paginated(data: Vec<T>, page: u32, limit: u32, total: u64) -> Self {
        let meta = PaginationMeta::new(page, limit, total);
        Self::new(data, meta)
    }
    
    /// Create empty response
    pub fn empty() -> Self {
        Self::new(Vec::new(), PaginationMeta::new(0, 0, 0))
    }
    
    /// Map the data to another type
    pub fn map<U, F>(self, f: F) -> ListResponse<U>
    where
        F: FnMut(T) -> U,
    {
        ListResponse {
            data: self.data.into_iter().map(f).collect(),
            meta: self.meta,
        }
    }
    
    /// Get total count for headers
    pub fn total_count(&self) -> u64 {
        self.meta.total
    }
}

// GraphQL types
#[cfg(feature = "graphql")]
use async_graphql::InputObject;

#[cfg(feature = "graphql")]
#[derive(InputObject)]
pub struct PaginationInputGql {
    /// Page number (0-based)
    pub page: Option<u32>,
    /// Number of items per page
    pub limit: Option<u32>,
    /// Sort field
    pub sort: Option<String>,
    /// Sort order
    pub order: Option<SortOrderGql>,
}

#[cfg(feature = "graphql")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, async_graphql::Enum)]
pub enum SortOrderGql {
    Asc,
    Desc,
}

#[cfg(feature = "graphql")]
impl From<PaginationInputGql> for PaginationInput {
    fn from(gql: PaginationInputGql) -> Self {
        Self {
            page: gql.page,
            limit: gql.limit,
            offset: None,
            sort: gql.sort,
            order: gql.order.map(|o| match o {
                SortOrderGql::Asc => SortOrder::Asc,
                SortOrderGql::Desc => SortOrder::Desc,
            }),
            _start: None,
            _end: None,
            _sort: None,
            _order: None,
        }
    }
}

#[cfg(feature = "graphql")]
impl Default for PaginationInputGql {
    fn default() -> Self {
        Self {
            page: Some(0),
            limit: Some(20),
            sort: None,
            order: Some(SortOrderGql::Asc),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pagination_input_defaults() {
        let input = PaginationInput::default();
        assert_eq!(input.effective_page(), 0);
        assert_eq!(input.effective_limit(), 20);
        assert_eq!(input.effective_order(), SortOrder::Asc);
    }
    
    #[test]
    fn test_pagination_input_refine_compatibility() {
        let input = PaginationInput {
            page: None,
            limit: None,
            offset: None,
            sort: None,
            order: None,
            _start: Some(10),
            _end: Some(30),
            _sort: Some("name".to_string()),
            _order: Some("DESC".to_string()),
        };
        
        assert_eq!(input.effective_offset(), 10);
        assert_eq!(input.effective_sort(), Some("name".to_string()));
        assert_eq!(input.effective_order(), SortOrder::Desc);
    }
    
    #[test]
    fn test_pagination_meta() {
        let meta = PaginationMeta::new(1, 10, 25);
        
        assert_eq!(meta.page, 1);
        assert_eq!(meta.limit, 10);
        assert_eq!(meta.total, 25);
        assert_eq!(meta.pages, 3);
        assert!(meta.has_next);
        assert!(meta.has_prev);
        assert_eq!(meta.offset, 10);
    }
    
    #[test]
    fn test_list_response() {
        let data = vec![1, 2, 3, 4, 5];
        let response = ListResponse::paginated(data, 0, 5, 10);
        
        assert_eq!(response.data.len(), 5);
        assert_eq!(response.meta.total, 10);
        assert_eq!(response.total_count(), 10);
    }
    
    #[test]
    fn test_list_response_map() {
        let data = vec![1, 2, 3];
        let response = ListResponse::paginated(data, 0, 3, 3);
        let mapped = response.map(|x| x.to_string());
        
        assert_eq!(mapped.data, vec!["1", "2", "3"]);
        assert_eq!(mapped.meta.total, 3);
    }
    
    #[test]
    fn test_pagination_validation() {
        let mut input = PaginationInput::default();
        assert!(input.validate().is_ok());
        
        input.limit = Some(0);
        assert!(input.validate().is_err());
        
        input.limit = Some(200);
        assert!(input.validate().is_err());
        
        input.limit = Some(50);
        input._start = Some(10);
        input._end = Some(5);
        assert!(input.validate().is_err());
    }
    
    #[test]
    fn test_sort_order_serialization() {
        // Test serialization instead of Display since we use serde rename_all
        assert_eq!(serde_json::to_string(&SortOrder::Asc).unwrap(), "\"asc\"");
        assert_eq!(serde_json::to_string(&SortOrder::Desc).unwrap(), "\"desc\"");
    }
}