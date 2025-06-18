use axum::{
    extract::{FromRequestParts, Query},
    http::request::Parts,
};
use serde::{Deserialize, Serialize};

use crate::errors::WebError;

/// Pagination query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
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


impl PaginationQuery {
    /// Convert to standard pagination input
    pub fn to_pagination_input(&self) -> ratchet_api_types::PaginationInput {
        if let (Some(start), Some(end)) = (self.start, self.end) {
            ratchet_api_types::PaginationInput::from_refine(Some(start), Some(end))
        } else {
            ratchet_api_types::PaginationInput {
                page: self.page.or(Some(1)), // Default to page 1 if not specified
                limit: self.limit.or(Some(25)), // Default to 25 items if not specified
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
#[derive(Default)]
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
#[derive(Debug, Clone, Serialize, Default)]
pub struct FilterQuery {
    /// Generic filter fields (field_name=value)
    #[serde(flatten)]
    pub filters: std::collections::HashMap<String, String>,
}

impl<'de> serde::Deserialize<'de> for FilterQuery {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map = std::collections::HashMap::<String, String>::deserialize(deserializer)?;
        
        // Filter out pagination and sort fields to avoid conflicts
        let filters: std::collections::HashMap<String, String> = map
            .into_iter()
            .filter(|(key, _)| {
                // Exclude explicit pagination and sort fields
                !matches!(key.as_str(), "_start" | "_end" | "page" | "limit" | "_sort" | "_order")
            })
            .collect();
        
        Ok(FilterQuery { filters })
    }
}

impl FilterQuery {
    /// Convert to standard filter inputs with advanced Refine.dev operator support
    pub fn to_filter_inputs(&self) -> Vec<ratchet_api_types::pagination::FilterInput> {
        self.filters
            .iter()
            .filter_map(|(field, value)| {
                // Skip pagination and sort fields
                if field.starts_with('_') && !self.is_advanced_filter_field(field) {
                    return None;
                }
                
                // Skip standard pagination fields to avoid conflicts
                if matches!(field.as_str(), "page" | "limit" | "start" | "end") {
                    return None;
                }

                // Parse field name and operator from Refine.dev style suffixes
                let (base_field, operator) = self.parse_field_and_operator(field);

                Some(ratchet_api_types::pagination::FilterInput {
                    field: base_field,
                    operator,
                    value: value.clone(),
                })
            })
            .collect()
    }

    /// Check if a field starting with _ is actually an advanced filter field (not pagination/sort)
    fn is_advanced_filter_field(&self, field: &str) -> bool {
        // These are advanced filter operators that start with _
        field.ends_with("_like") || 
        field.ends_with("_ne") || 
        field.ends_with("_gte") || 
        field.ends_with("_lte") || 
        field.ends_with("_gt") || 
        field.ends_with("_lt") || 
        field.ends_with("_in") || 
        field.ends_with("_not_in") ||
        field.ends_with("_starts_with") ||
        field.ends_with("_ends_with") ||
        field.ends_with("_contains")
    }

    /// Parse field name and determine the appropriate FilterOperator from Refine.dev style suffixes
    fn parse_field_and_operator(&self, field: &str) -> (String, ratchet_api_types::pagination::FilterOperator) {
        use ratchet_api_types::pagination::FilterOperator;

        // Check for Refine.dev style operator suffixes
        if let Some(base) = field.strip_suffix("_like") {
            (base.to_string(), FilterOperator::Contains)
        } else if let Some(base) = field.strip_suffix("_ne") {
            (base.to_string(), FilterOperator::Ne)
        } else if let Some(base) = field.strip_suffix("_gte") {
            (base.to_string(), FilterOperator::Gte)
        } else if let Some(base) = field.strip_suffix("_lte") {
            (base.to_string(), FilterOperator::Lte)
        } else if let Some(base) = field.strip_suffix("_gt") {
            (base.to_string(), FilterOperator::Gt)
        } else if let Some(base) = field.strip_suffix("_lt") {
            (base.to_string(), FilterOperator::Lt)
        } else if let Some(base) = field.strip_suffix("_in") {
            (base.to_string(), FilterOperator::In)
        } else if let Some(base) = field.strip_suffix("_not_in") {
            (base.to_string(), FilterOperator::NotIn)
        } else if let Some(base) = field.strip_suffix("_starts_with") {
            (base.to_string(), FilterOperator::StartsWith)
        } else if let Some(base) = field.strip_suffix("_ends_with") {
            (base.to_string(), FilterOperator::EndsWith)
        } else if let Some(base) = field.strip_suffix("_contains") {
            (base.to_string(), FilterOperator::Contains)
        } else {
            // Default to equality for exact field names
            (field.to_string(), FilterOperator::Eq)
        }
    }
}

/// Combined query parameters for list endpoints - Full Refine.dev Support
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct ListQuery {
    /// Alternative: Refine.dev style start offset
    #[serde(rename = "_start")]
    pub start: Option<u64>,
    /// Alternative: Refine.dev style end offset
    #[serde(rename = "_end")]
    pub end: Option<u64>,
    /// Page number (1-based)
    pub page: Option<u32>,
    /// Items per page (max 100)
    pub limit: Option<u32>,
    /// Field to sort by
    #[serde(rename = "_sort")]
    pub sort: Option<String>,
    /// Sort order (ASC/DESC)
    #[serde(rename = "_order")]
    pub order: Option<String>,
    /// Generic filter fields (field_name=value)
    #[serde(flatten)]
    pub filters: std::collections::HashMap<String, String>,
}


impl ListQuery {
    /// Convert to standard list input
    pub fn to_list_input(&self) -> ratchet_api_types::pagination::ListInput {
        let pagination_input = if let (Some(start), Some(end)) = (self.start, self.end) {
            ratchet_api_types::PaginationInput::from_refine(Some(start), Some(end))
        } else {
            ratchet_api_types::PaginationInput {
                page: self.page.or(Some(1)),
                limit: self.limit.or(Some(25)),
                offset: None,
            }
        };
        
        let sort_input = ratchet_api_types::pagination::SortInput::from_refine(
            self.sort.clone(),
            self.order.clone()
        );
        
        let filter_inputs = self.to_filter_inputs();
        
        ratchet_api_types::pagination::ListInput {
            pagination: Some(pagination_input),
            sort: sort_input,
            filters: Some(filter_inputs),
        }
    }

    /// Validate all query parameters
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
    
    /// Convert to filter inputs with advanced Refine.dev operator support
    pub fn to_filter_inputs(&self) -> Vec<ratchet_api_types::pagination::FilterInput> {
        self.filters
            .iter()
            .filter_map(|(field, value)| {
                // Skip pagination and sort fields
                if matches!(field.as_str(), "_start" | "_end" | "page" | "limit" | "_sort" | "_order") {
                    return None;
                }

                // Parse field name and operator from Refine.dev style suffixes
                let (base_field, operator) = self.parse_field_and_operator(field);

                Some(ratchet_api_types::pagination::FilterInput {
                    field: base_field,
                    operator,
                    value: value.clone(),
                })
            })
            .collect()
    }

    /// Parse field name and determine the appropriate FilterOperator from Refine.dev style suffixes
    fn parse_field_and_operator(&self, field: &str) -> (String, ratchet_api_types::pagination::FilterOperator) {
        use ratchet_api_types::pagination::FilterOperator;

        // Check for Refine.dev style operator suffixes
        if let Some(base) = field.strip_suffix("_like") {
            (base.to_string(), FilterOperator::Contains)
        } else if let Some(base) = field.strip_suffix("_ne") {
            (base.to_string(), FilterOperator::Ne)
        } else if let Some(base) = field.strip_suffix("_gte") {
            (base.to_string(), FilterOperator::Gte)
        } else if let Some(base) = field.strip_suffix("_lte") {
            (base.to_string(), FilterOperator::Lte)
        } else if let Some(base) = field.strip_suffix("_gt") {
            (base.to_string(), FilterOperator::Gt)
        } else if let Some(base) = field.strip_suffix("_lt") {
            (base.to_string(), FilterOperator::Lt)
        } else if let Some(base) = field.strip_suffix("_in") {
            (base.to_string(), FilterOperator::In)
        } else if let Some(base) = field.strip_suffix("_not_in") {
            (base.to_string(), FilterOperator::NotIn)
        } else if let Some(base) = field.strip_suffix("_starts_with") {
            (base.to_string(), FilterOperator::StartsWith)
        } else if let Some(base) = field.strip_suffix("_ends_with") {
            (base.to_string(), FilterOperator::EndsWith)
        } else if let Some(base) = field.strip_suffix("_contains") {
            (base.to_string(), FilterOperator::Contains)
        } else {
            // Default to equality for exact field names
            (field.to_string(), FilterOperator::Eq)
        }
    }
    
    // Helper methods for compatibility
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

/// Extract and validate query parameters
#[derive(Debug)]
pub struct QueryParams(pub ListQuery);

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

    #[test]
    fn test_filter_operator_parsing() {
        use std::collections::HashMap;
        use ratchet_api_types::pagination::FilterOperator;

        let mut filters = HashMap::new();
        filters.insert("name".to_string(), "test".to_string());
        filters.insert("name_like".to_string(), "partial".to_string());
        filters.insert("age_gte".to_string(), "18".to_string());
        filters.insert("status_ne".to_string(), "inactive".to_string());
        filters.insert("tags_in".to_string(), "tag1,tag2".to_string());
        filters.insert("email_contains".to_string(), "@example".to_string());
        filters.insert("title_starts_with".to_string(), "Mr".to_string());
        filters.insert("suffix_ends_with".to_string(), ".com".to_string());

        let filter_query = FilterQuery { filters };
        let filter_inputs = filter_query.to_filter_inputs();

        // Find specific filters
        let name_eq_filter = filter_inputs.iter().find(|f| f.field == "name" && f.operator == FilterOperator::Eq).unwrap();
        assert_eq!(name_eq_filter.value, "test");

        let name_like_filter = filter_inputs.iter().find(|f| f.field == "name" && f.operator == FilterOperator::Contains).unwrap();
        assert_eq!(name_like_filter.value, "partial");

        let age_filter = filter_inputs.iter().find(|f| f.field == "age").unwrap();
        assert_eq!(age_filter.operator, FilterOperator::Gte);
        assert_eq!(age_filter.value, "18");

        let status_filter = filter_inputs.iter().find(|f| f.field == "status").unwrap();
        assert_eq!(status_filter.operator, FilterOperator::Ne);
        assert_eq!(status_filter.value, "inactive");

        let tags_filter = filter_inputs.iter().find(|f| f.field == "tags").unwrap();
        assert_eq!(tags_filter.operator, FilterOperator::In);
        assert_eq!(tags_filter.value, "tag1,tag2");

        let email_filter = filter_inputs.iter().find(|f| f.field == "email").unwrap();
        assert_eq!(email_filter.operator, FilterOperator::Contains);
        assert_eq!(email_filter.value, "@example");

        let title_filter = filter_inputs.iter().find(|f| f.field == "title").unwrap();
        assert_eq!(title_filter.operator, FilterOperator::StartsWith);
        assert_eq!(title_filter.value, "Mr");

        let suffix_filter = filter_inputs.iter().find(|f| f.field == "suffix").unwrap();
        assert_eq!(suffix_filter.operator, FilterOperator::EndsWith);
        assert_eq!(suffix_filter.value, ".com");
    }

    #[test]
    fn test_advanced_filter_field_detection() {
        let filter_query = FilterQuery::default();

        // Should detect advanced filter fields
        assert!(filter_query.is_advanced_filter_field("name_like"));
        assert!(filter_query.is_advanced_filter_field("age_gte"));
        assert!(filter_query.is_advanced_filter_field("status_ne"));
        assert!(filter_query.is_advanced_filter_field("tags_in"));
        assert!(filter_query.is_advanced_filter_field("values_not_in"));
        assert!(filter_query.is_advanced_filter_field("title_starts_with"));
        assert!(filter_query.is_advanced_filter_field("suffix_ends_with"));
        assert!(filter_query.is_advanced_filter_field("description_contains"));

        // Should not detect pagination/sort fields
        assert!(!filter_query.is_advanced_filter_field("_start"));
        assert!(!filter_query.is_advanced_filter_field("_end"));
        assert!(!filter_query.is_advanced_filter_field("_sort"));
        assert!(!filter_query.is_advanced_filter_field("_order"));

        // Should not detect regular fields
        assert!(!filter_query.is_advanced_filter_field("name"));
        assert!(!filter_query.is_advanced_filter_field("age"));
        assert!(!filter_query.is_advanced_filter_field("_invalid"));
    }

    #[test]
    fn test_filter_skips_pagination_fields() {
        use std::collections::HashMap;

        let mut filters = HashMap::new();
        filters.insert("name".to_string(), "test".to_string());
        filters.insert("_start".to_string(), "0".to_string());
        filters.insert("_end".to_string(), "100".to_string());
        filters.insert("_sort".to_string(), "name".to_string());
        filters.insert("_order".to_string(), "ASC".to_string());
        filters.insert("page".to_string(), "1".to_string());
        filters.insert("limit".to_string(), "25".to_string());
        filters.insert("name_like".to_string(), "partial".to_string()); // This should be included

        let filter_query = FilterQuery { filters };
        let filter_inputs = filter_query.to_filter_inputs();

        // Should only have 2 filters: name (eq) and name (contains)
        assert_eq!(filter_inputs.len(), 2);
        
        // Should not include pagination/sort fields
        assert!(!filter_inputs.iter().any(|f| f.field == "_start"));
        assert!(!filter_inputs.iter().any(|f| f.field == "_end"));
        assert!(!filter_inputs.iter().any(|f| f.field == "_sort"));
        assert!(!filter_inputs.iter().any(|f| f.field == "_order"));
        assert!(!filter_inputs.iter().any(|f| f.field == "page"));
        assert!(!filter_inputs.iter().any(|f| f.field == "limit"));

        // Should include valid filters
        assert!(filter_inputs.iter().any(|f| f.field == "name" && f.operator == ratchet_api_types::pagination::FilterOperator::Eq));
        assert!(filter_inputs.iter().any(|f| f.field == "name" && f.operator == ratchet_api_types::pagination::FilterOperator::Contains));
    }
}