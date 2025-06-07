//! HTTP client functionality for Ratchet
//! 
//! This crate provides HTTP client capabilities with mock support, request recording,
//! and integration with the Ratchet configuration system.

pub mod client;
pub mod config;
pub mod errors;
pub mod types;

#[cfg(feature = "recording")]
pub mod recording;

// Re-export main types for convenience
pub use client::{HttpClient, HttpManager};
pub use config::HttpConfig;
pub use errors::HttpError;
pub use types::{HttpMethod, HttpMethodError};

#[cfg(feature = "recording")]
pub use recording::{
    finalize_recording, get_recording_dir, is_recording, record_http_request, record_input,
    record_output, set_recording_dir,
};

// Backward compatibility function
pub async fn call_http(
    url: &str,
    params: Option<&serde_json::Value>,
    body: Option<&serde_json::Value>,
) -> Result<serde_json::Value, HttpError> {
    let manager = HttpManager::new();
    manager.call_http(url, params, body).await
}