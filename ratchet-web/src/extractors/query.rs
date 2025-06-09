use axum::{
    async_trait,
    extract::{FromRequestParts, Query},
    http::request::Parts,
};
use serde::{Deserialize, Serialize};

use crate::errors::WebError;

/// Pagination query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationQuery {
    /// Page number (1-based)
    pub page: Option<u32>,
    /// Items per page (max 100)
    pub limit: Option<u32>,
    /// Alternative: Refine.dev style start offset
    #[serde(rename = "_start")]
    pub start: Option<u64>,
    /// Alternative: Refine.dev style end offset
    #[serde(rename = "_end")]
    pub end: Option<u64>,
}

impl Default for PaginationQuery {
    fn default() -> Self {
        Self {
            page: Some(1),
            limit: Some(25),
            start: None,
            end: None,
        }
    }
}

impl PaginationQuery {
    /// Convert to standard pagination input
    pub fn to_pagination_input(&self) -> ratchet_api_types::PaginationInput {
        if let (Some(start), Some(end)) = (self.start, self.end) {
            ratchet_api_types::PaginationInput::from_refine(Some(start), Some(end))
        } else {
            ratchet_api_types::PaginationInput {
                page: self.page,
                limit: self.limit,
                offset: None,
            }
        }
    }

    /// Validate pagination parameters
    pub fn validate(&self) -> Result<(), WebError> {
        // Check Refine.dev style parameters
        if let (Some(start), Some(end)) = (self.start, self.end) {
            if start >= end {
                return Err(WebError::bad_request(
                    "Invalid pagination: _start must be less than _end"
                ));
            }
            if end - start > 100 {
                return Err(WebError::bad_request(
                    "Invalid pagination: maximum limit is 100"
                ));
            }
        }

        // Check standard parameters
        if let Some(limit) = self.limit {
            if limit > 100 {
                return Err(WebError::bad_request(
                    "Invalid pagination: maximum limit is 100"
                ));
            }
            if limit == 0 {
                return Err(WebError::bad_request(
                    "Invalid pagination: limit must be greater than 0"
                ));
            }
        }

        if let Some(page) = self.page {
            if page == 0 {
                return Err(WebError::bad_request(
                    "Invalid pagination: page must be greater than 0"
                ));
            }
        }

        Ok(())
    }
}

/// Sort query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortQuery {
    /// Field to sort by
    #[serde(rename = "_sort")]
    pub sort: Option<String>,
    /// Sort order (ASC/DESC)
    #[serde(rename = "_order")]
    pub order: Option<String>,
}

impl SortQuery {
    /// Convert to standard sort input
    pub fn to_sort_input(&self) -> Option<ratchet_api_types::pagination::SortInput> {
        ratchet_api_types::pagination::SortInput::from_refine(
            self.sort.clone(),
            self.order.clone()
        )
    }
}

/// Filter query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterQuery {
    /// Generic filter fields (field_name=value)
    #[serde(flatten)]
    pub filters: std::collections::HashMap<String, String>,
}

impl FilterQuery {
    /// Convert to standard filter inputs
    pub fn to_filter_inputs(&self) -> Vec<ratchet_api_types::pagination::FilterInput> {
        self.filters
            .iter()
            .filter_map(|(field, value)| {
                // Skip pagination and sort fields
                if field.starts_with('_') {
                    return None;
                }

                Some(ratchet_api_types::pagination::FilterInput {
                    field: field.clone(),
                    operator: ratchet_api_types::pagination::FilterOperator::Eq, // Default to equality
                    value: value.clone(),
                })
            })
            .collect()
    }
}

/// Combined query parameters for list endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListQuery {
    #[serde(flatten)]
    pub pagination: PaginationQuery,
    #[serde(flatten)]
    pub sort: SortQuery,
    #[serde(flatten)]
    pub filter: FilterQuery,
}

impl ListQuery {
    /// Convert to standard list input
    pub fn to_list_input(&self) -> ratchet_api_types::pagination::ListInput {
        ratchet_api_types::pagination::ListInput {
            pagination: Some(self.pagination.to_pagination_input()),
            sort: self.sort.to_sort_input(),
            filters: Some(self.filter.to_filter_inputs()),
        }
    }

    /// Validate all query parameters
    pub fn validate(&self) -> Result<(), WebError> {
        self.pagination.validate()
    }
}

/// Extract and validate query parameters
#[derive(Debug)]
pub struct QueryParams(pub ListQuery);

#[async_trait]
impl<S> FromRequestParts<S> for QueryParams
where
    S: Send + Sync,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<ListQuery>::from_request_parts(parts, state)
            .await
            .map_err(|err| WebError::bad_request(format!("Invalid query parameters: {}", err)))?;

        // Validate query parameters
        query.validate()?;

        Ok(QueryParams(query))
    }
}

/// Extract pagination parameters only
#[derive(Debug)]
pub struct PaginationParams(pub PaginationQuery);

#[async_trait]
impl<S> FromRequestParts<S> for PaginationParams
where
    S: Send + Sync,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(pagination) = Query::<PaginationQuery>::from_request_parts(parts, state)
            .await
            .map_err(|err| WebError::bad_request(format!("Invalid pagination parameters: {}", err)))?;

        // Validate pagination parameters
        pagination.validate()?;

        Ok(PaginationParams(pagination))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_validation() {
        // Valid pagination
        let valid = PaginationQuery {
            page: Some(1),
            limit: Some(25),
            start: None,
            end: None,
        };
        assert!(valid.validate().is_ok());

        // Invalid limit too high
        let invalid_limit = PaginationQuery {
            page: Some(1),
            limit: Some(200),
            start: None,
            end: None,
        };
        assert!(invalid_limit.validate().is_err());

        // Invalid Refine.dev style
        let invalid_refine = PaginationQuery {
            page: None,
            limit: None,
            start: Some(10),
            end: Some(5), // end < start
        };
        assert!(invalid_refine.validate().is_err());
    }

    #[test]
    fn test_sort_conversion() {
        let sort = SortQuery {
            sort: Some("name".to_string()),
            order: Some("DESC".to_string()),
        };

        let sort_input = sort.to_sort_input().unwrap();
        assert_eq!(sort_input.field, "name");
        assert_eq!(
            sort_input.get_direction(),
            ratchet_api_types::pagination::SortDirection::Desc
        );
    }
}