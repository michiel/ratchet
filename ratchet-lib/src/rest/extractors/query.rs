use axum::{
    async_trait,
    extract::{FromRequestParts, Query},
    http::request::Parts,
};

use crate::rest::{
    middleware::RestError,
    models::common::{FilterQuery, ListQuery, PaginationQuery, SortQuery},
};

/// Extract list query parameters with validation
#[derive(Debug)]
pub struct ListQueryExtractor(pub ListQuery);

#[async_trait]
impl<S> FromRequestParts<S> for ListQueryExtractor
where
    S: Send + Sync,
{
    type Rejection = RestError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<ListQuery>::from_request_parts(parts, state)
            .await
            .map_err(|err| RestError::BadRequest(format!("Invalid query parameters: {}", err)))?;

        // Validate pagination parameters
        if let (Some(start), Some(end)) = (query.start, query.end) {
            if start >= end {
                return Err(RestError::BadRequest(
                    "Invalid pagination: _start must be less than _end".to_string(),
                ));
            }
            if end - start > 100 {
                return Err(RestError::BadRequest(
                    "Invalid pagination: maximum limit is 100".to_string(),
                ));
            }
        }

        Ok(ListQueryExtractor(query))
    }
}

/// Extract pagination query parameters
#[derive(Debug)]
pub struct PaginationExtractor(pub PaginationQuery);

#[async_trait]
impl<S> FromRequestParts<S> for PaginationExtractor
where
    S: Send + Sync,
{
    type Rejection = RestError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<PaginationQuery>::from_request_parts(parts, state)
            .await
            .map_err(|err| {
                RestError::BadRequest(format!("Invalid pagination parameters: {}", err))
            })?;

        Ok(PaginationExtractor(query))
    }
}

/// Extract sort query parameters
#[derive(Debug)]
pub struct SortExtractor(pub SortQuery);

#[async_trait]
impl<S> FromRequestParts<S> for SortExtractor
where
    S: Send + Sync,
{
    type Rejection = RestError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<SortQuery>::from_request_parts(parts, state)
            .await
            .map_err(|err| RestError::BadRequest(format!("Invalid sort parameters: {}", err)))?;

        Ok(SortExtractor(query))
    }
}

/// Extract filter query parameters
#[derive(Debug)]
pub struct FilterExtractor(pub FilterQuery);

#[async_trait]
impl<S> FromRequestParts<S> for FilterExtractor
where
    S: Send + Sync,
{
    type Rejection = RestError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<FilterQuery>::from_request_parts(parts, state)
            .await
            .map_err(|err| RestError::BadRequest(format!("Invalid filter parameters: {}", err)))?;

        Ok(FilterExtractor(query))
    }
}
