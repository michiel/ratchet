use axum::{
    response::{IntoResponse, Response},
    Json,
};

/// Add pagination headers to response for Refine.dev compatibility
pub fn add_pagination_headers(
    mut response: Response,
    total: u64,
    offset: u64,
    limit: u64,
    resource: &str,
) -> Response {
    let headers = response.headers_mut();
    
    // Add x-total-count header
    if let Ok(total_header) = total.to_string().parse() {
        headers.insert("x-total-count", total_header);
    }
    
    // Add content-range header
    let end = offset + limit - 1;
    let content_range = format!("{} {}-{}/{}", resource, offset, end.min(total.saturating_sub(1)), total);
    if let Ok(range_header) = content_range.parse() {
        headers.insert("content-range", range_header);
    }
    
    response
}

/// Helper trait to add pagination headers to any response
pub trait WithPaginationHeaders: IntoResponse {
    fn with_pagination_headers(
        self,
        total: u64,
        offset: u64,
        limit: u64,
        resource: &str,
    ) -> Response 
    where 
        Self: Sized {
        let response = self.into_response();
        add_pagination_headers(response, total, offset, limit, resource)
    }
}

// Implement the trait for common response types
impl<T> WithPaginationHeaders for Json<T>
where
    T: serde::Serialize,
{
}

impl WithPaginationHeaders for Response {}

impl WithPaginationHeaders for &'static str {}

impl WithPaginationHeaders for String {}