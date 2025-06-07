// Re-export HTTP functionality from ratchet-http crate
pub use ratchet_http::{call_http, HttpClient, HttpConfig, HttpError, HttpManager, HttpMethod, HttpMethodError};

// Keep fetch functionality that's specific to JS integration
pub mod fetch;

#[cfg(test)]
mod tests;

pub use fetch::register_fetch;

// Backward compatibility alias
pub fn create_http_manager() -> HttpManager {
    HttpManager::new()
}
