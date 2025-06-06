//! HTTP response cache implementation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::{
    cache::Cache,
    config::HttpCacheConfig,
    stores::{MokaCache, TtlCache},
    CacheError, CacheResult, CacheStats,
};

/// HTTP cache key
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct HttpCacheKey {
    /// HTTP method
    pub method: String,

    /// URL
    pub url: String,

    /// Query parameters (sorted for consistency)
    pub query_params: Vec<(String, String)>,

    /// Relevant headers for cache key
    pub headers: Vec<(String, String)>,
}

impl HttpCacheKey {
    /// Create a new cache key
    pub fn new(method: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            url: url.into(),
            query_params: Vec::new(),
            headers: Vec::new(),
        }
    }

    /// Add query parameters
    pub fn with_query_params(mut self, params: Vec<(String, String)>) -> Self {
        let mut params = params;
        params.sort_by(|a, b| a.0.cmp(&b.0));
        self.query_params = params;
        self
    }

    /// Add headers
    pub fn with_headers(mut self, headers: Vec<(String, String)>) -> Self {
        let mut headers = headers;
        headers.sort_by(|a, b| a.0.cmp(&b.0));
        self.headers = headers;
        self
    }
}

/// Cached HTTP response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedHttpResponse {
    /// Status code
    pub status_code: u16,

    /// Response headers
    pub headers: HashMap<String, String>,

    /// Response body
    pub body: Vec<u8>,

    /// When the response was cached
    pub cached_at: DateTime<Utc>,

    /// When the response expires
    pub expires_at: Option<DateTime<Utc>>,

    /// ETag if present
    pub etag: Option<String>,

    /// Last-Modified if present
    pub last_modified: Option<String>,

    /// Response size in bytes
    pub size_bytes: usize,
}

impl CachedHttpResponse {
    /// Check if the response is stale
    pub fn is_stale(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Get age of the cached response
    pub fn age(&self) -> Duration {
        let now = Utc::now();
        let age_secs = (now - self.cached_at).num_seconds().max(0) as u64;
        Duration::from_secs(age_secs)
    }
}

/// HTTP cache implementation
pub struct HttpCache {
    /// Inner cache
    inner: HttpCacheImpl,

    /// Configuration
    config: HttpCacheConfig,
}

/// Inner cache implementation
enum HttpCacheImpl {
    Ttl(TtlCache<HttpCacheKey, Arc<CachedHttpResponse>>),
    Moka(MokaCache<HttpCacheKey, Arc<CachedHttpResponse>>),
}

impl HttpCache {
    /// Create a new HTTP cache from configuration
    pub fn from_config(config: HttpCacheConfig) -> Self {
        let inner = match config.cache_type {
            crate::config::CacheType::Moka => {
                let mut builder = MokaCache::builder();
                builder = builder
                    .max_capacity(config.max_entries as u64)
                    .time_to_live(Duration::from_secs(config.default_ttl_seconds))
                    .weigher(|_k, v: &Arc<CachedHttpResponse>| {
                        (v.size_bytes / 1024) as u32 // KB
                    });

                HttpCacheImpl::Moka(builder.build())
            }
            _ => {
                // Default to TTL cache for HTTP
                let ttl = Duration::from_secs(config.default_ttl_seconds);
                HttpCacheImpl::Ttl(TtlCache::new(ttl))
            }
        };

        Self { inner, config }
    }

    /// Get a cached response
    pub async fn get(&self, key: &HttpCacheKey) -> CacheResult<Option<Arc<CachedHttpResponse>>> {
        let response = match &self.inner {
            HttpCacheImpl::Ttl(cache) => cache.get(key).await?,
            HttpCacheImpl::Moka(cache) => cache.get(key).await?,
        };

        // Check if response is stale
        if let Some(ref resp) = response {
            if resp.is_stale() {
                // Remove stale entry
                self.remove(key).await?;
                return Ok(None);
            }
        }

        Ok(response)
    }

    /// Cache a response
    pub async fn put(
        &self,
        key: HttpCacheKey,
        response: CachedHttpResponse,
        custom_ttl: Option<Duration>,
    ) -> CacheResult<()> {
        // Check size limit
        if response.size_bytes > self.config.max_response_size {
            return Err(CacheError::CapacityExceeded(format!(
                "Response size {} exceeds limit",
                response.size_bytes
            )));
        }

        let response = Arc::new(response);

        match &self.inner {
            HttpCacheImpl::Ttl(cache) => {
                if let Some(ttl) = custom_ttl {
                    cache.put_with_ttl(key, response, ttl).await
                } else {
                    cache.put(key, response).await
                }
            }
            HttpCacheImpl::Moka(cache) => {
                if let Some(ttl) = custom_ttl {
                    cache.put_with_ttl(key, response, ttl).await
                } else {
                    cache.put(key, response).await
                }
            }
        }
    }

    /// Remove a cached response
    pub async fn remove(&self, key: &HttpCacheKey) -> CacheResult<Option<Arc<CachedHttpResponse>>> {
        match &self.inner {
            HttpCacheImpl::Ttl(cache) => cache.remove(key).await,
            HttpCacheImpl::Moka(cache) => cache.remove(key).await,
        }
    }

    /// Clear all cached responses
    pub async fn clear(&self) -> CacheResult<()> {
        match &self.inner {
            HttpCacheImpl::Ttl(cache) => cache.clear().await,
            HttpCacheImpl::Moka(cache) => cache.clear().await,
        }
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheResult<CacheStats> {
        match &self.inner {
            HttpCacheImpl::Ttl(cache) => cache.stats().await,
            HttpCacheImpl::Moka(cache) => cache.stats().await,
        }
    }

    /// Parse cache control header
    pub fn parse_cache_control(header: &str) -> CacheControl {
        let mut control = CacheControl::default();

        for directive in header.split(',') {
            let directive = directive.trim();

            if directive == "no-cache" {
                control.no_cache = true;
            } else if directive == "no-store" {
                control.no_store = true;
            } else if let Some(max_age) = directive.strip_prefix("max-age=") {
                if let Ok(seconds) = max_age.parse::<u64>() {
                    control.max_age = Some(Duration::from_secs(seconds));
                }
            } else if let Some(s_maxage) = directive.strip_prefix("s-maxage=") {
                if let Ok(seconds) = s_maxage.parse::<u64>() {
                    control.s_maxage = Some(Duration::from_secs(seconds));
                }
            } else if directive == "private" {
                control.private = true;
            } else if directive == "public" {
                control.public = true;
            }
        }

        control
    }
}

/// Cache control directives
#[derive(Debug, Default)]
pub struct CacheControl {
    pub no_cache: bool,
    pub no_store: bool,
    pub max_age: Option<Duration>,
    pub s_maxage: Option<Duration>,
    pub private: bool,
    pub public: bool,
}

impl CacheControl {
    /// Check if response is cacheable
    pub fn is_cacheable(&self) -> bool {
        !self.no_store && !self.no_cache
    }

    /// Get effective TTL
    pub fn get_ttl(&self, default: Duration) -> Duration {
        self.s_maxage.or(self.max_age).unwrap_or(default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_cache() {
        let config = HttpCacheConfig {
            enabled: true,
            max_entries: 100,
            max_response_size: 1024 * 1024,
            default_ttl_seconds: 300,
            honor_cache_control: true,
            cache_type: crate::config::CacheType::Ttl,
        };

        let cache = HttpCache::from_config(config);

        let key = HttpCacheKey::new("GET", "https://api.example.com/data");

        let response = CachedHttpResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: b"test response".to_vec(),
            cached_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::minutes(5)),
            etag: Some("\"123456\"".to_string()),
            last_modified: None,
            size_bytes: 13,
        };

        // Put and get
        cache.put(key.clone(), response, None).await.unwrap();
        let cached = cache.get(&key).await.unwrap();

        assert!(cached.is_some());
        assert_eq!(cached.unwrap().status_code, 200);
    }

    #[tokio::test]
    async fn test_cache_control_parsing() {
        let header = "max-age=3600, public, s-maxage=7200";
        let control = HttpCache::parse_cache_control(header);

        assert!(control.is_cacheable());
        assert!(control.public);
        assert_eq!(control.max_age, Some(Duration::from_secs(3600)));
        assert_eq!(control.s_maxage, Some(Duration::from_secs(7200)));

        let ttl = control.get_ttl(Duration::from_secs(300));
        assert_eq!(ttl, Duration::from_secs(7200)); // s-maxage takes precedence
    }

    #[tokio::test]
    async fn test_size_limit() {
        let config = HttpCacheConfig {
            enabled: true,
            max_entries: 100,
            max_response_size: 100, // Small limit
            default_ttl_seconds: 300,
            honor_cache_control: true,
            cache_type: crate::config::CacheType::Ttl,
        };

        let cache = HttpCache::from_config(config);
        let key = HttpCacheKey::new("GET", "https://api.example.com/large");

        let large_response = CachedHttpResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: vec![0; 1000], // Exceeds limit
            cached_at: Utc::now(),
            expires_at: None,
            etag: None,
            last_modified: None,
            size_bytes: 1000,
        };

        let result = cache.put(key, large_response, None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CacheError::CapacityExceeded(_)
        ));
    }
}
