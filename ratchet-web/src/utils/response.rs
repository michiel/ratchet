use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use ratchet_api_types::{ListResponse, pagination::PaginationMeta};
use serde::{Deserialize, Serialize};

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

impl<T> ApiResponse<T> {
    /// Create a simple response with just data
    pub fn new(data: T) -> Self {
        Self { data, meta: None }
    }

    /// Create response with metadata
    pub fn with_meta(data: T, meta: ResponseMeta) -> Self {
        Self {
            data,
            meta: Some(meta),
        }
    }

    /// Create response with pagination metadata
    pub fn with_pagination(data: T, pagination: PaginationMeta) -> Self {
        Self {
            data,
            meta: Some(ResponseMeta {
                pagination: Some(pagination),
                request_id: None,
                timestamp: Some(chrono::Utc::now()),
            }),
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

/// Response builder for creating API responses
pub struct ResponseBuilder<T> {
    data: T,
    status: StatusCode,
    pagination: Option<PaginationMeta>,
    request_id: Option<String>,
}

impl<T> ResponseBuilder<T> {
    /// Create a new response builder
    pub fn new(data: T) -> Self {
        Self {
            data,
            status: StatusCode::OK,
            pagination: None,
            request_id: None,
        }
    }

    /// Set HTTP status code
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Add pagination metadata
    pub fn pagination(mut self, pagination: PaginationMeta) -> Self {
        self.pagination = Some(pagination);
        self
    }

    /// Add request ID
    pub fn request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Build the response
    pub fn build(self) -> impl IntoResponse
    where
        T: Serialize,
    {
        let meta = if self.pagination.is_some() || self.request_id.is_some() {
            Some(ResponseMeta {
                pagination: self.pagination,
                request_id: self.request_id,
                timestamp: Some(chrono::Utc::now()),
            })
        } else {
            None
        };

        let response = ApiResponse {
            data: self.data,
            meta,
        };

        (self.status, Json(response))
    }
}

/// Helper functions for common response patterns

/// Create a successful response with data
pub fn ok<T: Serialize>(data: T) -> impl IntoResponse {
    ResponseBuilder::new(data).build()
}

/// Create a successful response with pagination
pub fn ok_with_pagination<T: Serialize>(
    data: T,
    pagination: PaginationMeta,
) -> impl IntoResponse {
    ResponseBuilder::new(data).pagination(pagination).build()
}

/// Create a created response (201)
pub fn created<T: Serialize>(data: T) -> impl IntoResponse {
    ResponseBuilder::new(data).status(StatusCode::CREATED).build()
}

/// Create a no content response (204)
pub fn no_content() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

/// Helper to convert ListResponse to API response
impl<T: Serialize> From<ListResponse<T>> for ApiResponse<Vec<T>> {
    fn from(list_response: ListResponse<T>) -> Self {
        ApiResponse::with_pagination(list_response.items, list_response.meta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct TestData {
        id: u32,
        name: String,
    }

    #[test]
    fn test_api_response_creation() {
        let data = TestData {
            id: 1,
            name: "test".to_string(),
        };

        let response = ApiResponse::new(data);
        assert!(response.meta.is_none());
    }

    #[test]
    fn test_response_builder() {
        let data = vec![
            TestData {
                id: 1,
                name: "test1".to_string(),
            },
            TestData {
                id: 2,
                name: "test2".to_string(),
            },
        ];

        let pagination = PaginationMeta {
            page: 1,
            limit: 25,
            total: 100,
            total_pages: 4,
            has_next: true,
            has_previous: false,
            offset: 0,
        };

        let _response = ResponseBuilder::new(data)
            .status(StatusCode::OK)
            .pagination(pagination)
            .request_id("test-123".to_string())
            .build();
    }
}